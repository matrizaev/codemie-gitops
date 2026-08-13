/// Operation request projection (F-006).
///
/// Produces typed `Create` or `Update` plans from a parsed declaration and
/// optional identity resolution. Rules (ADR-002, data-model.md §7,
/// contracts/adapter-manifest-v2.42.0.json):
///
/// - `Omitted` and `AuthoredNull` optional-null fields → explicit JSON null.
/// - Required fields absent → `AppError::Schema` before write.
/// - `AuthoringOnly`, `OperationInapplicable`, `MixedOwned`, `ReadOnlyOrProhibited`
///   → no fabricated member; never emitted under the authored field name.
/// - No default filling; no equality-based write suppression.
/// - Create-only fields are absent from Update plans.
/// - Workflow `meta_config`: decode server string → merge (author wins, reserved
///   key preserved) → canonical encode (compact JSON, keys sorted by Unicode
///   scalar value).
/// - File Datasource: `files` → multipart parts; `uploaded_files` and
///   `guardrail_assignments` → query parameters.
///
/// ## Source traceability
///
/// - Requirements: FR-006/012/021/022/028/031/035/036, QR-002
/// - Architecture: ADR-002/007-009
/// - Task: F-006
/// - Manifest baseline: contracts/adapter-manifest-v2.42.0.json §entities
use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::declaration_schema::{
    AssistantDeclaration, DatasourceDeclaration, DatasourceSpec, FileDatasourceSpec,
    SkillDeclaration, WorkflowDeclaration,
};
use crate::error::AppError;
use crate::parse::{ParsedDeclaration, ParsedDeclarationRef};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProjectionError {
    #[error("failed to encode {context}")]
    Encode {
        context: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("server {context} is not valid JSON")]
    ServerJson {
        context: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to encode {context} as YAML")]
    YamlEncode {
        context: &'static str,
        #[source]
        source: serde_yaml::Error,
    },
}

impl ProjectionError {
    pub(crate) fn is_compatibility(&self) -> bool {
        matches!(self, Self::ServerJson { .. })
    }
}

// ---------------------------------------------------------------------------
// Public plan types (data-model.md §7)
// ---------------------------------------------------------------------------

/// The request payload for a single POST or PUT operation.
#[derive(Debug, Clone)]
pub enum RequestBody {
    /// Ordinary JSON body.
    Json(Value),
    /// `multipart/form-data` with scalar query parameters.
    /// Used only for File Datasource (data-model.md §6).
    FileMultipart {
        /// Scalar/JSON query parameters (e.g. `name`, `uploaded_files`,
        /// `guardrail_assignments`).
        query_params: Vec<(String, String)>,
    },
}

/// The complete projection result passed to an adapter for network dispatch.
///
/// Mirrors data-model.md §7: `WritePlan = Create{request} | Update{server_id, request}`.
#[derive(Debug)]
pub enum WritePlan {
    /// No existing identity found: issue a POST.
    Create {
        /// The POST request payload.
        request: RequestBody,
    },
    /// Existing identity found: issue a PUT.
    Update {
        /// The server-assigned entity UUID from identity resolution.
        server_id: String,
        /// The PUT request payload.
        request: RequestBody,
    },
}

/// Server-side state needed for operations that read before writing.
///
/// Currently only Workflow requires this (meta_config merge).
#[derive(Debug, Clone)]
pub struct ExistingEntity {
    /// Server UUID (identity resolution result).
    pub server_id: String,
    /// Server `meta_config` string for Workflow merge.
    /// `None` when absent from the server response (treated as `null`).
    pub meta_config: Option<String>,
}

/// Natural-key references resolved for one Workflow invocation (W-002).
///
/// Keys remain authored natural identities while values are transient server
/// handles. The map is constructed after exact online resolution and is never
/// serialized, logged, or persisted.
#[derive(Debug, Clone, Default)]
pub struct WorkflowReferenceMap {
    assistant_ids: BTreeMap<(String, String), String>,
    skill_ids: BTreeMap<(String, String), String>,
    datasource_ids: BTreeMap<(String, String), String>,
}

/// Server-validated Assistant reference projection.
#[derive(Debug, Default)]
pub struct AssistantReferenceMap {
    pub assistant_ids: Vec<String>,
    pub skill_ids: Vec<String>,
    pub context: Vec<Value>,
}

impl WorkflowReferenceMap {
    pub fn insert_assistant(
        &mut self,
        project: impl Into<String>,
        slug: impl Into<String>,
        server_id: impl Into<String>,
    ) {
        self.assistant_ids
            .insert((project.into(), slug.into()), server_id.into());
    }

    pub fn insert_skill(
        &mut self,
        project: impl Into<String>,
        name: impl Into<String>,
        server_id: impl Into<String>,
    ) {
        self.skill_ids
            .insert((project.into(), name.into()), server_id.into());
    }

    pub fn insert_datasource(
        &mut self,
        project: impl Into<String>,
        repo_name: impl Into<String>,
        server_id: impl Into<String>,
    ) {
        self.datasource_ids
            .insert((project.into(), repo_name.into()), server_id.into());
    }
}

// ---------------------------------------------------------------------------
// Public projection entry point
// ---------------------------------------------------------------------------

/// Project a parsed declaration into a typed write plan.
///
/// `existing` is `Some` when the identity resolver found a unique match and
/// an `Update` plan should be produced; `None` produces a `Create` plan.
///
/// `adopt_workflow_id` is an invocation-level adoption selector (apply
/// `--adopt-workflow-id`). Ignored for non-Workflow kinds.
///
/// Fails with `AppError::Schema` when a required authored field is absent or
/// null, or when a Workflow `meta_config` string is malformed.
pub fn project(
    decl: &ParsedDeclaration,
    existing: Option<&ExistingEntity>,
    _adopt_workflow_id: Option<&str>,
) -> Result<WritePlan, AppError> {
    project_with_workflow_references(decl, existing, _adopt_workflow_id, None)
}

/// Project an Assistant with every authored natural reference resolved online.
pub fn project_with_assistant_references(
    decl: &ParsedDeclaration,
    existing: Option<&ExistingEntity>,
    references: &AssistantReferenceMap,
) -> Result<WritePlan, AppError> {
    let server_id = existing.map(|entity| entity.server_id.clone());
    let ParsedDeclarationRef::Assistant(dto) = decl.typed() else {
        return Err(AppError::Internal(
            "Assistant reference projection requires an Assistant".into(),
        ));
    };
    let request = project_typed_assistant(dto, Some(references))?;
    Ok(match server_id {
        None => WritePlan::Create { request },
        Some(server_id) => WritePlan::Update { server_id, request },
    })
}

/// Project a declaration while supplying the exact invocation-local Workflow
/// reference map built by W-002's online resolution step.
pub fn project_with_workflow_references(
    decl: &ParsedDeclaration,
    existing: Option<&ExistingEntity>,
    _adopt_workflow_id: Option<&str>,
    workflow_references: Option<&WorkflowReferenceMap>,
) -> Result<WritePlan, AppError> {
    let server_id = existing.map(|e| e.server_id.clone());

    let request = match decl.typed() {
        ParsedDeclarationRef::Assistant(dto) => project_typed_assistant(dto, None),
        ParsedDeclarationRef::Workflow(dto) => project_typed_workflow(
            dto,
            server_id.is_some(),
            existing.and_then(|e| e.meta_config.as_deref()),
            workflow_references,
        ),
        ParsedDeclarationRef::Skill(dto) => project_typed_skill(dto),
        ParsedDeclarationRef::Datasource(dto) => project_typed_datasource(dto, server_id.is_some()),
        #[cfg(test)]
        ParsedDeclarationRef::Fixture(kind, value) => match kind {
            crate::parse::EntityKind::Assistant => project_assistant(value, server_id.is_some()),
            crate::parse::EntityKind::Workflow => project_workflow(
                value,
                server_id.is_some(),
                existing.and_then(|e| e.meta_config.as_deref()),
                workflow_references,
            ),
            crate::parse::EntityKind::Skill => project_skill(value, server_id.is_some()),
            crate::parse::EntityKind::Datasource => project_datasource(value, server_id.is_some()),
        },
    }?;

    Ok(match server_id {
        None => WritePlan::Create { request },
        Some(id) => WritePlan::Update {
            server_id: id,
            request,
        },
    })
}

fn encoded_value<T: serde::Serialize + ?Sized>(
    value: &T,
    context: &'static str,
) -> Result<Value, AppError> {
    serde_json::to_value(value)
        .map_err(|source| ProjectionError::Encode { context, source })
        .map_err(AppError::from)
}

fn insert_encoded<T: serde::Serialize + ?Sized>(
    body: &mut Map<String, Value>,
    key: &'static str,
    value: &T,
) -> Result<(), AppError> {
    body.insert(key.to_owned(), encoded_value(value, key)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// Assistant projection
// ---------------------------------------------------------------------------

fn project_typed_assistant(
    dto: &AssistantDeclaration,
    references: Option<&AssistantReferenceMap>,
) -> Result<RequestBody, AppError> {
    let project = &dto.metadata.project;
    let spec = &dto.spec;
    let mut body = Map::new();

    insert_encoded(&mut body, "project", project)?;
    insert_encoded(&mut body, "slug", &dto.metadata.slug)?;
    insert_encoded(&mut body, "name", &spec.name)?;
    insert_encoded(&mut body, "description", &spec.description)?;
    insert_encoded(&mut body, "system_prompt", &spec.system_prompt)?;
    let references = references.ok_or_else(|| {
        AppError::Internal("Assistant references were not resolved before projection".into())
    })?;
    body.insert("context".into(), Value::Array(references.context.clone()));
    body.insert(
        "assistant_ids".into(),
        Value::Array(
            references
                .assistant_ids
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    body.insert(
        "skill_ids".into(),
        Value::Array(
            references
                .skill_ids
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    insert_encoded(&mut body, "icon_url", &spec.icon_url)?;
    insert_encoded(&mut body, "llm_model_type", &spec.llm_model_type)?;
    insert_encoded(
        &mut body,
        "enable_image_generation",
        &spec.enable_image_generation,
    )?;
    insert_encoded(
        &mut body,
        "image_generation_model",
        &spec.image_generation_model,
    )?;
    insert_encoded(&mut body, "toolkits", &spec.toolkits)?;
    insert_encoded(
        &mut body,
        "conversation_starters",
        &spec.conversation_starters,
    )?;
    insert_encoded(&mut body, "shared", &spec.shared)?;
    insert_encoded(&mut body, "is_global", &spec.is_global)?;
    insert_encoded(&mut body, "agent_mode", &spec.agent_mode)?;
    insert_encoded(&mut body, "plan_prompt", &spec.plan_prompt)?;
    insert_encoded(&mut body, "temperature", &spec.temperature)?;
    insert_encoded(&mut body, "top_p", &spec.top_p)?;
    insert_encoded(
        &mut body,
        "tools_tokens_size_limit",
        &spec.tools_tokens_size_limit,
    )?;
    insert_encoded(
        &mut body,
        "smart_tool_selection_enabled",
        &spec.smart_tool_selection_enabled,
    )?;
    insert_encoded(&mut body, "hedging_config", &spec.hedging_config)?;
    insert_encoded(
        &mut body,
        "interactive_features",
        &spec.interactive_features,
    )?;
    insert_encoded(&mut body, "mcp_servers", &spec.mcp_servers)?;
    insert_encoded(
        &mut body,
        "enabled_builtin_subagents",
        &spec.enabled_builtin_subagents,
    )?;
    insert_encoded(&mut body, "type", &spec.type_)?;
    insert_encoded(&mut body, "categories", &spec.categories)?;
    insert_encoded(&mut body, "prompt_variables", &spec.prompt_variables)?;
    insert_encoded(&mut body, "custom_metadata", &spec.custom_metadata)?;
    insert_encoded(
        &mut body,
        "guardrail_assignments",
        &spec.guardrail_assignments,
    )?;

    Ok(RequestBody::Json(Value::Object(body)))
}

#[cfg(test)]
fn project_assistant(decl_value: &Value, _is_update: bool) -> Result<RequestBody, AppError> {
    let metadata = get_obj(decl_value, "metadata")?;
    let spec = get_obj(decl_value, "spec")?;

    let mut body = Map::new();

    // authorTransform: metadata.project → project, metadata.slug → slug
    insert_required_from_obj(&mut body, metadata, "project", "project")?;
    insert_required_from_obj(&mut body, metadata, "slug", "slug")?;

    insert_required_spec(&mut body, spec, "name")?;
    insert_required_spec(&mut body, spec, "system_prompt")?;
    insert_required_spec(&mut body, spec, "llm_model_type")?;
    insert_required_spec(&mut body, spec, "type")?;
    insert_required_spec(&mut body, spec, "context")?;
    insert_required_spec(&mut body, spec, "toolkits")?;
    insert_required_spec(&mut body, spec, "conversation_starters")?;
    insert_required_spec(&mut body, spec, "shared")?;
    insert_required_spec(&mut body, spec, "mcp_servers")?;
    insert_required_spec(&mut body, spec, "enabled_builtin_subagents")?;
    insert_required_spec(&mut body, spec, "prompt_variables")?;
    insert_required_spec(&mut body, spec, "categories")?;
    insert_optional_null_spec(&mut body, spec, "description");
    insert_optional_null_spec(&mut body, spec, "icon_url");
    insert_optional_null_spec(&mut body, spec, "enable_image_generation");
    insert_optional_null_spec(&mut body, spec, "image_generation_model");
    insert_optional_null_spec(&mut body, spec, "is_global");
    insert_optional_null_spec(&mut body, spec, "agent_mode");
    insert_optional_null_spec(&mut body, spec, "plan_prompt");
    insert_optional_null_spec(&mut body, spec, "temperature");
    insert_optional_null_spec(&mut body, spec, "top_p");
    insert_optional_null_spec(&mut body, spec, "tools_tokens_size_limit");
    insert_optional_null_spec(&mut body, spec, "smart_tool_selection_enabled");
    insert_optional_null_spec(&mut body, spec, "hedging_config");
    insert_optional_null_spec(&mut body, spec, "interactive_features");
    insert_optional_null_spec(&mut body, spec, "custom_metadata");
    insert_optional_null_spec(&mut body, spec, "guardrail_assignments");

    Ok(RequestBody::Json(Value::Object(body)))
}

// ---------------------------------------------------------------------------
// Workflow projection
// ---------------------------------------------------------------------------

mod workflow {
    pub const RESERVED_KEY: &str = "codemie.epam.com/gitops/workflow-identity";
}

fn project_typed_workflow(
    dto: &WorkflowDeclaration,
    is_update: bool,
    server_meta_config: Option<&str>,
    workflow_references: Option<&WorkflowReferenceMap>,
) -> Result<RequestBody, AppError> {
    let project = &dto.metadata.project;
    let spec = &dto.spec;
    let execution = encoded_value(&spec.execution_config, "workflow execution configuration")?;
    let execution = execution.as_object().cloned().ok_or_else(|| {
        AppError::Internal("typed workflow execution configuration was not an object".into())
    })?;
    let execution_config = transform_workflow_execution(execution, workflow_references)?;
    let yaml_config =
        serde_yaml::to_string(&execution_config).map_err(|source| ProjectionError::YamlEncode {
            context: "workflow execution configuration",
            source,
        })?;
    let meta_config = merge_meta_config_fields(
        server_meta_config,
        spec.meta_config.as_ref(),
        project.as_str(),
        dto.metadata.slug.as_str(),
    )?;

    let mut body = Map::new();
    insert_encoded(&mut body, "project", project)?;
    insert_encoded(&mut body, "name", &spec.name)?;
    insert_encoded(&mut body, "mode", &spec.mode)?;
    insert_encoded(&mut body, "description", &spec.description)?;
    insert_encoded(&mut body, "start_hint", &spec.start_hint)?;
    insert_encoded(&mut body, "icon_url", &spec.icon_url)?;
    body.insert("yaml_config".to_owned(), Value::String(yaml_config));
    insert_encoded(&mut body, "shared", &spec.shared)?;
    insert_encoded(&mut body, "supervisor_prompt", &spec.supervisor_prompt)?;
    body.insert("meta_config".to_owned(), meta_config);
    insert_encoded(
        &mut body,
        "guardrail_assignments",
        &spec.guardrail_assignments,
    )?;
    if !is_update {
        let required_execution = |field: &'static str| {
            execution_config.get(field).cloned().ok_or_else(|| {
                AppError::Internal(format!(
                    "typed workflow execution configuration omitted {field}"
                ))
            })
        };
        body.insert("assistants".to_owned(), required_execution("assistants")?);
        body.insert("tools".to_owned(), required_execution("tools")?);
        body.insert("states".to_owned(), required_execution("states")?);
    }

    Ok(RequestBody::Json(Value::Object(body)))
}

#[cfg(test)]
fn project_workflow(
    decl_value: &Value,
    is_update: bool,
    server_meta_config: Option<&str>,
    workflow_references: Option<&WorkflowReferenceMap>,
) -> Result<RequestBody, AppError> {
    let metadata = get_obj(decl_value, "metadata")?;
    let spec = get_obj(decl_value, "spec")?;

    let mut body = Map::new();
    let execution_config = project_workflow_execution(spec, workflow_references)?;
    let yaml_config =
        serde_yaml::to_string(&execution_config).map_err(|source| ProjectionError::YamlEncode {
            context: "workflow execution configuration",
            source,
        })?;

    // authorTransform: metadata.project → project
    insert_required_from_obj(&mut body, metadata, "project", "project")?;

    insert_required_spec(&mut body, spec, "name")?;
    insert_required_spec(&mut body, spec, "mode")?;
    insert_required_spec(&mut body, spec, "description")?;
    insert_required_spec(&mut body, spec, "shared")?;
    insert_optional_null_spec(&mut body, spec, "start_hint");
    insert_optional_null_spec(&mut body, spec, "icon_url");
    insert_optional_null_spec(&mut body, spec, "supervisor_prompt");
    insert_optional_null_spec(&mut body, spec, "guardrail_assignments");
    body.insert("yaml_config".to_owned(), Value::String(yaml_config));
    body.insert(
        "meta_config".to_owned(),
        merge_meta_config(server_meta_config, spec.get("meta_config"), metadata)?,
    );
    if !is_update {
        insert_required_from_obj(&mut body, &execution_config, "assistants", "assistants")?;
        insert_required_from_obj(&mut body, &execution_config, "tools", "tools")?;
        insert_required_from_obj(&mut body, &execution_config, "states", "states")?;
    }

    Ok(RequestBody::Json(Value::Object(body)))
}

/// Convert Workflow's author form into the exact server execution form.
///
/// Graph-local IDs (`assistants[].id` and `states[].assistant_id`) remain
/// unchanged. Only the three approved natural-reference positions are
/// replaced with server-resource fields.
#[cfg(test)]
fn project_workflow_execution(
    spec: &Map<String, Value>,
    references: Option<&WorkflowReferenceMap>,
) -> Result<Map<String, Value>, AppError> {
    let execution = spec
        .get("execution_config")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| AppError::Schema("workflow: spec.execution_config is required".into()))?;

    transform_workflow_execution(execution, references)
}

fn transform_workflow_execution(
    mut execution: Map<String, Value>,
    references: Option<&WorkflowReferenceMap>,
) -> Result<Map<String, Value>, AppError> {
    let assistants = execution
        .get_mut("assistants")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            AppError::Schema("workflow: execution_config.assistants must be an array".into())
        })?;

    for actor in assistants {
        let actor = actor.as_object_mut().ok_or_else(|| {
            AppError::Schema("workflow: execution_config.assistants[] must be an object".into())
        })?;

        // Defensive boundary: these positions are schema-prohibited, but the
        // projector also rejects them so callers cannot bypass W-002 by
        // constructing ParsedDeclaration directly.
        for prohibited in ["assistant_id", "skill_ids", "datasource_ids"] {
            if actor.contains_key(prohibited) {
                return Err(AppError::Schema(format!(
                    "workflow: authored server-resource field '{prohibited}' is prohibited"
                )));
            }
        }

        actor.entry("temperature".to_owned()).or_insert(Value::Null);

        if let Some(reference) = actor.remove("assistantRef") {
            if actor.contains_key("system_prompt")
                || actor.contains_key("skillRefs")
                || actor.contains_key("datasourceRefs")
            {
                return Err(AppError::Schema(
                    "workflow: persisted actor contains inline-only fields".into(),
                ));
            }
            let (project, slug) = reference_pair(&reference, "slug", "assistantRef")?;
            let server_id = references
                .and_then(|map| map.assistant_ids.get(&(project, slug)))
                .ok_or_else(|| {
                    AppError::Reconciliation(
                        "workflow: persisted Assistant reference did not resolve exactly once"
                            .into(),
                    )
                })?;
            actor.insert("assistant_id".to_owned(), Value::String(server_id.clone()));
        } else {
            if !actor.contains_key("system_prompt") {
                return Err(AppError::Schema(
                    "workflow: inline actor requires system_prompt".into(),
                ));
            }

            let skill_refs = actor.remove("skillRefs").ok_or_else(|| {
                AppError::Schema("workflow: inline actor requires skillRefs".into())
            })?;
            let datasource_refs = actor.remove("datasourceRefs").ok_or_else(|| {
                AppError::Schema("workflow: inline actor requires datasourceRefs".into())
            })?;
            let skill_ids = resolve_reference_array(
                &skill_refs,
                "name",
                "skillRefs",
                references.map(|map| &map.skill_ids),
            )?;
            let datasource_ids = resolve_reference_array(
                &datasource_refs,
                "repo_name",
                "datasourceRefs",
                references.map(|map| &map.datasource_ids),
            )?;
            actor.insert("skill_ids".to_owned(), Value::Array(skill_ids));
            actor.insert("datasource_ids".to_owned(), Value::Array(datasource_ids));
        }
    }

    let tools = execution
        .get_mut("tools")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            AppError::Schema("workflow: execution_config.tools must be an array".into())
        })?;
    for tool in tools {
        let tool = tool.as_object_mut().ok_or_else(|| {
            AppError::Schema("workflow: execution_config.tools[] must be an object".into())
        })?;
        for optional_null in [
            "tool_result_json_pointer",
            "mcp_server",
            "tokens_size_limit",
        ] {
            tool.entry(optional_null.to_owned()).or_insert(Value::Null);
        }
    }

    Ok(execution)
}

fn reference_pair(
    value: &Value,
    key_field: &str,
    field: &str,
) -> Result<(String, String), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::Schema(format!("workflow: {field} must be an object")))?;
    let project = object
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Schema(format!("workflow: {field}.project is required")))?;
    let key = object
        .get(key_field)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Schema(format!("workflow: {field}.{key_field} is required")))?;
    Ok((project.to_owned(), key.to_owned()))
}

fn resolve_reference_array(
    value: &Value,
    key_field: &str,
    field: &str,
    resolved: Option<&BTreeMap<(String, String), String>>,
) -> Result<Vec<Value>, AppError> {
    value
        .as_array()
        .ok_or_else(|| AppError::Schema(format!("workflow: {field} must be an array")))?
        .iter()
        .map(|reference| {
            let key = reference_pair(reference, key_field, field)?;
            resolved
                .and_then(|map| map.get(&key))
                .cloned()
                .map(Value::String)
                .ok_or_else(|| {
                    AppError::Reconciliation(format!(
                        "workflow: {field} reference did not resolve exactly once"
                    ))
                })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Workflow meta_config merge (metaConfigCodec)
// ---------------------------------------------------------------------------

/// Decode, merge, and re-encode the Workflow `meta_config` string.
///
/// Algorithm (manifest §metaConfigCodec):
/// 1. Decode server string (null → empty object).
/// 2. Overlay authored non-reserved keys (author wins).
/// 3. Install exact reserved identity record from declaration metadata.
/// 4. Encode: compact JSON, keys sorted by Unicode scalar value.
///
/// Reserved key: `codemie.epam.com/gitops/workflow-identity`.
#[cfg(test)]
fn merge_meta_config(
    server_raw: Option<&str>,
    authored_value: Option<&Value>,
    metadata: &Map<String, Value>,
) -> Result<Value, AppError> {
    let project = metadata
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::Schema("meta_config: metadata.project required for identity".into())
        })?;
    let slug = metadata
        .get("slug")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::Schema("meta_config: metadata.slug required for identity".into())
        })?;
    let authored = match authored_value {
        Some(Value::Object(value)) => Some(value),
        Some(Value::Null) | None => None,
        Some(_) => {
            return Err(AppError::Schema(
                "meta_config: authored value must be a JSON object or null".into(),
            ));
        }
    };
    merge_meta_config_fields(server_raw, authored, project, slug)
}

fn merge_meta_config_fields(
    server_raw: Option<&str>,
    authored: Option<&Map<String, Value>>,
    project: &str,
    slug: &str,
) -> Result<Value, AppError> {
    // Step 1: decode server side (null → empty object)
    let mut merged: BTreeMap<String, Value> = match server_raw {
        None | Some("null") => BTreeMap::new(),
        Some(s) => {
            let decoded: Value =
                serde_json::from_str(s).map_err(|source| ProjectionError::ServerJson {
                    context: "workflow meta_config",
                    source,
                })?;
            let obj = decoded.as_object().ok_or_else(|| {
                AppError::Schema("meta_config: server value is not a JSON object".into())
            })?;
            // Reject invalid reserved record upfront; the reserved key value is overwritten below
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        }
    };

    // Step 2: overlay authored non-reserved keys
    if let Some(authored_map) = authored {
        for (k, v) in authored_map {
            if k == workflow::RESERVED_KEY {
                return Err(AppError::Schema(
                    "meta_config: author must not set the reserved identity key".into(),
                ));
            }
            merged.insert(k.clone(), v.clone());
        }
    }

    // Step 3: install exact reserved identity record
    // WorkflowIdentityV2 is completed by the adapter with the authenticated creator.
    let identity = serde_json::json!({
        "project": project,
        "slug": slug,
        "version": 2
    });
    merged.insert(workflow::RESERVED_KEY.to_owned(), identity);

    // Step 4: encode — compact JSON, BTreeMap ensures keys sorted by Unicode scalar value
    let encoded = serde_json::to_string(&merged).map_err(|source| ProjectionError::Encode {
        context: "merged workflow meta_config",
        source,
    })?;

    Ok(Value::String(encoded))
}

// ---------------------------------------------------------------------------
// Skill projection
// ---------------------------------------------------------------------------

fn project_typed_skill(dto: &SkillDeclaration) -> Result<RequestBody, AppError> {
    let project = &dto.metadata.project;
    let spec = dto.spec.resolved().ok_or_else(|| {
        AppError::Internal("Skill contentFrom was not resolved at the parse boundary".into())
    })?;
    let mut body = Map::new();
    insert_encoded(&mut body, "name", &dto.metadata.name)?;
    insert_encoded(&mut body, "project", project)?;
    insert_encoded(&mut body, "description", spec.description)?;
    insert_encoded(&mut body, "content", spec.content)?;
    insert_encoded(&mut body, "visibility", spec.visibility)?;
    insert_encoded(&mut body, "categories", spec.categories)?;
    insert_encoded(&mut body, "toolkits", spec.toolkits)?;
    insert_encoded(&mut body, "mcp_servers", spec.mcp_servers)?;
    insert_encoded(&mut body, "companion_files", spec.companion_files)?;
    insert_encoded(
        &mut body,
        "enabled_builtin_subagents",
        spec.enabled_builtin_subagents,
    )?;
    Ok(RequestBody::Json(Value::Object(body)))
}

#[cfg(test)]
fn project_skill(decl_value: &Value, _is_update: bool) -> Result<RequestBody, AppError> {
    let metadata = get_obj(decl_value, "metadata")?;
    let spec = get_obj(decl_value, "spec")?;

    let mut body = Map::new();

    // authorTransform: metadata.name → name, metadata.project → project
    insert_required_from_obj(&mut body, metadata, "name", "name")?;
    insert_required_from_obj(&mut body, metadata, "project", "project")?;

    insert_required_spec(&mut body, spec, "description")?;
    insert_required_spec(&mut body, spec, "content")?;
    insert_required_spec(&mut body, spec, "visibility")?;
    insert_required_spec(&mut body, spec, "categories")?;
    insert_required_spec(&mut body, spec, "toolkits")?;
    insert_required_spec(&mut body, spec, "mcp_servers")?;
    insert_required_spec(&mut body, spec, "companion_files")?;
    insert_required_spec(&mut body, spec, "enabled_builtin_subagents")?;

    Ok(RequestBody::Json(Value::Object(body)))
}

// ---------------------------------------------------------------------------
// Datasource projection
// ---------------------------------------------------------------------------

fn insert_optional_encoded<T: serde::Serialize>(
    body: &mut Map<String, Value>,
    key: &'static str,
    value: &Option<T>,
) -> Result<(), AppError> {
    if let Some(value) = value {
        insert_encoded(body, key, value)?;
    }
    Ok(())
}

macro_rules! insert_fields {
    ($body:expr, $( $key:literal => $value:expr ),+ $(,)?) => {{
        $(insert_encoded($body, $key, $value)?;)+
    }};
}

macro_rules! insert_optional_fields {
    ($body:expr, $( $key:literal => $value:expr ),+ $(,)?) => {{
        $(insert_optional_encoded($body, $key, $value)?;)+
    }};
}

fn datasource_json_identity(
    dto: &DatasourceDeclaration,
    body: &mut Map<String, Value>,
    is_update: bool,
    project_scoped: bool,
) -> Result<(), AppError> {
    if !is_update {
        insert_encoded(body, "name", &dto.metadata.repo_name)?;
    }
    if project_scoped {
        insert_encoded(body, "project_name", &dto.metadata.project)?;
    }
    Ok(())
}

fn project_typed_datasource(
    dto: &DatasourceDeclaration,
    is_update: bool,
) -> Result<RequestBody, AppError> {
    let mut body = Map::new();
    match &dto.spec {
        DatasourceSpec::GitDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, false)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "branch" => &spec.branch,
                "link" => &spec.link,
                "projectSpaceVisible" => &spec.project_space_visible,
            );
            insert_optional_fields!(&mut body,
                "prompt" => &spec.prompt,
                "embeddingsModel" => &spec.embeddings_model,
                "docsGeneration" => &spec.docs_generation,
                "filesFilter" => &spec.files_filter,
                "setting_id" => &spec.setting_id,
                "guardrail_assignments" => &spec.guardrail_assignments,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
            );
            if !is_update {
                insert_encoded(&mut body, "indexType", &spec.index_type_camel)?;
                insert_optional_encoded(
                    &mut body,
                    "summarizationModel",
                    &spec.summarization_model,
                )?;
            }
        }
        DatasourceSpec::SvnDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, false)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "branch" => &spec.branch,
                "link" => &spec.link,
                "projectSpaceVisible" => &spec.project_space_visible,
            );
            insert_optional_fields!(&mut body,
                "prompt" => &spec.prompt,
                "embeddingsModel" => &spec.embeddings_model,
                "docsGeneration" => &spec.docs_generation,
                "filesFilter" => &spec.files_filter,
                "setting_id" => &spec.setting_id,
                "guardrail_assignments" => &spec.guardrail_assignments,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
            );
            if !is_update {
                insert_encoded(&mut body, "indexType", &spec.index_type_camel)?;
                insert_optional_encoded(
                    &mut body,
                    "summarizationModel",
                    &spec.summarization_model,
                )?;
            }
        }
        DatasourceSpec::ConfluenceDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, true)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "cql" => &spec.cql,
            );
            insert_optional_fields!(&mut body,
                "project_space_visible" => &spec.project_space_visible,
                "setting_id" => &spec.setting_id,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
                "guardrail_assignments" => &spec.guardrail_assignments,
            );
            if !is_update {
                insert_optional_fields!(&mut body,
                    "include_restricted_content" => &spec.include_restricted_content,
                    "include_archived_content" => &spec.include_archived_content,
                    "include_attachments" => &spec.include_attachments,
                    "include_comments" => &spec.include_comments,
                    "keep_markdown_format" => &spec.keep_markdown_format,
                    "keep_newlines" => &spec.keep_newlines,
                    "embedding_model" => &spec.embedding_model,
                );
            }
        }
        DatasourceSpec::JiraDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, true)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "jql" => &spec.jql,
            );
            insert_optional_fields!(&mut body,
                "project_space_visible" => &spec.project_space_visible,
                "setting_id" => &spec.setting_id,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
                "guardrail_assignments" => &spec.guardrail_assignments,
            );
            if !is_update {
                insert_optional_encoded(&mut body, "embedding_model", &spec.embedding_model)?;
            }
        }
        DatasourceSpec::XrayDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, true)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "jql" => &spec.jql,
            );
            insert_optional_fields!(&mut body,
                "project_space_visible" => &spec.project_space_visible,
                "setting_id" => &spec.setting_id,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
                "guardrail_assignments" => &spec.guardrail_assignments,
            );
            if !is_update {
                insert_optional_encoded(&mut body, "embedding_model", &spec.embedding_model)?;
            }
        }
        DatasourceSpec::FileDatasourceSpec(spec) => {
            return project_typed_file_datasource(dto, spec, is_update);
        }
        DatasourceSpec::GoogleDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, true)?;
            insert_encoded(&mut body, "description", &spec.description)?;
            insert_optional_fields!(&mut body,
                "project_space_visible" => &spec.project_space_visible,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
                "guardrail_assignments" => &spec.guardrail_assignments,
            );
            if !is_update {
                insert_fields!(&mut body,
                    "googleDoc" => &spec.google_doc,
                    "setting_id" => &spec.setting_id,
                );
                insert_optional_encoded(&mut body, "embedding_model", &spec.embedding_model)?;
            }
        }
        DatasourceSpec::AzureWikiDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, true)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "wiki_query" => &spec.wiki_query,
            );
            insert_optional_fields!(&mut body,
                "project_space_visible" => &spec.project_space_visible,
                "wiki_name" => &spec.wiki_name,
                "setting_id" => &spec.setting_id,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
                "guardrail_assignments" => &spec.guardrail_assignments,
            );
            if !is_update {
                insert_optional_encoded(&mut body, "embedding_model", &spec.embedding_model)?;
            }
        }
        DatasourceSpec::AzureWorkItemDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, true)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "wiql_query" => &spec.wiql_query,
            );
            insert_optional_fields!(&mut body,
                "project_space_visible" => &spec.project_space_visible,
                "setting_id" => &spec.setting_id,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
                "guardrail_assignments" => &spec.guardrail_assignments,
            );
            if !is_update {
                insert_optional_encoded(&mut body, "embedding_model", &spec.embedding_model)?;
            }
        }
        DatasourceSpec::SharepointDatasourceSpec(spec) => {
            datasource_json_identity(dto, &mut body, is_update, true)?;
            insert_fields!(&mut body,
                "description" => &spec.description,
                "site_url" => &spec.site_url,
                "auth_type" => &spec.auth_type,
            );
            insert_optional_fields!(&mut body,
                "project_space_visible" => &spec.project_space_visible,
                "include_pages" => &spec.include_pages,
                "include_documents" => &spec.include_documents,
                "include_lists" => &spec.include_lists,
                "max_file_size_mb" => &spec.max_file_size_mb,
                "files_filter" => &spec.files_filter,
                "setting_id" => &spec.setting_id,
                "embedding_model" => &spec.embedding_model,
                "cron_expression" => &spec.cron_expression,
                "timezone" => &spec.timezone,
                "oauth_client_id" => &spec.oauth_client_id,
                "oauth_tenant_id" => &spec.oauth_tenant_id,
                "guardrail_assignments" => &spec.guardrail_assignments,
            );
        }
    }
    Ok(RequestBody::Json(Value::Object(body)))
}

fn push_query_value<T: serde::Serialize + ?Sized>(
    query: &mut Vec<(String, String)>,
    key: &'static str,
    value: &T,
) -> Result<(), AppError> {
    let value = encoded_value(value, key)?;
    query.push((key.to_owned(), scalar_to_string(&value)?));
    Ok(())
}

fn push_optional_query_value<T: serde::Serialize>(
    query: &mut Vec<(String, String)>,
    key: &'static str,
    value: &Option<T>,
) -> Result<(), AppError> {
    if let Some(value) = value {
        push_query_value(query, key, value)?;
    }
    Ok(())
}

fn push_optional_json_query<T: serde::Serialize>(
    query: &mut Vec<(String, String)>,
    key: &'static str,
    value: &Option<T>,
) -> Result<(), AppError> {
    if let Some(value) = value {
        let encoded = serde_json::to_string(value).map_err(|source| ProjectionError::Encode {
            context: key,
            source,
        })?;
        query.push((key.to_owned(), encoded));
    }
    Ok(())
}

fn project_typed_file_datasource(
    dto: &DatasourceDeclaration,
    spec: &FileDatasourceSpec,
    is_update: bool,
) -> Result<RequestBody, AppError> {
    let mut query_params = Vec::new();
    push_query_value(&mut query_params, "project_name", &dto.metadata.project)?;
    if !is_update {
        push_query_value(&mut query_params, "name", &dto.metadata.repo_name)?;
    }
    push_query_value(&mut query_params, "description", &spec.description)?;
    push_optional_query_value(
        &mut query_params,
        "project_space_visible",
        &spec.project_space_visible,
    )?;
    push_optional_query_value(&mut query_params, "csv_separator", &spec.csv_separator)?;
    push_optional_query_value(&mut query_params, "csv_start_row", &spec.csv_start_row)?;
    push_optional_query_value(
        &mut query_params,
        "csv_rows_per_document",
        &spec.csv_rows_per_document,
    )?;
    push_optional_query_value(&mut query_params, "embedding_model", &spec.embedding_model)?;
    push_optional_json_query(
        &mut query_params,
        "guardrail_assignments",
        &spec.guardrail_assignments,
    )?;
    push_query_value(
        &mut query_params,
        "include_email_attachments",
        &spec.include_email_attachments,
    )?;
    if is_update {
        push_optional_json_query(&mut query_params, "uploaded_files", &spec.uploaded_files)?;
    }
    Ok(RequestBody::FileMultipart { query_params })
}

#[cfg(test)]
fn project_datasource(decl_value: &Value, is_update: bool) -> Result<RequestBody, AppError> {
    let metadata = get_obj(decl_value, "metadata")?;
    let spec = get_obj(decl_value, "spec")?;

    let index_type = spec
        .get("index_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Schema("datasource: spec.index_type is required".into()))?;

    let project = metadata
        .get("project")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Schema("datasource: metadata.project is required".into()))?;

    let repo_name = metadata
        .get("repo_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Schema("datasource: metadata.repo_name is required".into()))?;

    match index_type {
        "file" => project_file_datasource(spec, metadata, project, repo_name, is_update),
        kind => project_json_datasource(spec, kind, project, repo_name, is_update),
    }
}

#[cfg(test)]
fn project_json_datasource(
    spec: &Map<String, Value>,
    kind: &str,
    project: &str,
    repo_name: &str,
    is_update: bool,
) -> Result<RequestBody, AppError> {
    let mut body = spec.clone();
    body.remove("index_type");

    // The API uses `name` as the natural repository name for every create
    // route; project-scoped knowledge-base routes also require project_name.
    if !is_update {
        body.insert("name".to_owned(), Value::String(repo_name.to_owned()));
    }
    if kind != "git" && kind != "svn" {
        body.insert("project_name".to_owned(), Value::String(project.to_owned()));
    }

    if is_update {
        body.remove("name");
        match kind {
            "git" | "svn" => {
                body.remove("indexType");
                body.remove("summarizationModel");
            }
            "confluence" => {
                body.remove("include_restricted_content");
                body.remove("include_archived_content");
                body.remove("include_attachments");
                body.remove("include_comments");
                body.remove("keep_markdown_format");
                body.remove("keep_newlines");
                body.remove("embedding_model");
            }
            "jira" | "xray" | "azure_devops_wiki" | "azure_devops_work_item" => {
                body.remove("embedding_model");
            }
            "google" => {
                body.remove("googleDoc");
                body.remove("setting_id");
                body.remove("embedding_model");
            }
            _ => {}
        }
    }

    Ok(RequestBody::Json(Value::Object(body)))
}

#[cfg(test)]
fn project_file_datasource(
    spec: &Map<String, Value>,
    _metadata: &Map<String, Value>,
    project: &str,
    repo_name: &str,
    is_update: bool,
) -> Result<RequestBody, AppError> {
    // File Datasource: multipart/form-data, all scalar fields as query params.
    // - `files`: read at the selected-input boundary before server access
    // - `uploaded_files`: update-only, compact JSON array in query param
    // - `guardrail_assignments`: compact JSON array in query param
    // - other scalars: query parameters
    // Source: manifest §entities.Datasource.types.file.transportTransform

    let mut query_params: Vec<(String, String)> = Vec::new();

    // Identity injection: project_name and repo_name as query params
    query_params.push(("project_name".to_owned(), project.to_owned()));
    if !is_update {
        query_params.push(("name".to_owned(), repo_name.to_owned()));
    }

    push_fixture_scalar(&mut query_params, spec, "description")?;
    push_fixture_scalar(&mut query_params, spec, "project_space_visible")?;
    push_fixture_scalar(&mut query_params, spec, "csv_separator")?;
    push_fixture_scalar(&mut query_params, spec, "csv_start_row")?;
    push_fixture_scalar(&mut query_params, spec, "csv_rows_per_document")?;
    push_fixture_scalar(&mut query_params, spec, "embedding_model")?;
    push_fixture_scalar(&mut query_params, spec, "include_email_attachments")?;
    if is_update && let Some(value) = spec.get("uploaded_files") {
        let encoded = serde_json::to_string(value).map_err(|source| ProjectionError::Encode {
            context: "file datasource uploaded_files",
            source,
        })?;
        query_params.push(("uploaded_files".to_owned(), encoded));
    }
    if let Some(value) = spec.get("guardrail_assignments") {
        let encoded = serde_json::to_string(value).map_err(|source| ProjectionError::Encode {
            context: "file datasource guardrail assignments",
            source,
        })?;
        query_params.push(("guardrail_assignments".to_owned(), encoded));
    }

    Ok(RequestBody::FileMultipart { query_params })
}

#[cfg(test)]
fn push_fixture_scalar(
    query: &mut Vec<(String, String)>,
    spec: &Map<String, Value>,
    field: &'static str,
) -> Result<(), AppError> {
    if let Some(value) = spec.get(field) {
        query.push((field.to_owned(), scalar_to_string(value)?));
    }
    Ok(())
}

/// Convert a scalar JSON value to a query-parameter string.
fn scalar_to_string(v: &Value) -> Result<String, AppError> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Null => Ok("null".to_owned()),
        _ => Err(AppError::Schema(
            "datasource: query param field must be a scalar value".into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Shared projection helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
fn get_obj<'a>(v: &'a Value, key: &str) -> Result<&'a Map<String, Value>, AppError> {
    v.get(key)
        .and_then(|v| v.as_object())
        .ok_or_else(|| AppError::Schema(format!("declaration is missing required '{key}' object")))
}

/// Insert a required field from one JSON object into the request body under
/// a (possibly different) output key. Fails if the source field is absent or null.
#[cfg(test)]
fn insert_required_from_obj(
    body: &mut Map<String, Value>,
    source: &Map<String, Value>,
    source_key: &str,
    target_key: &str,
) -> Result<(), AppError> {
    match source.get(source_key) {
        Some(Value::Null) | None => Err(AppError::Schema(format!(
            "required field '{source_key}' is absent or null"
        ))),
        Some(v) => {
            body.insert(target_key.to_owned(), v.clone());
            Ok(())
        }
    }
}

/// Insert a required spec field. Fails if absent or null.
#[cfg(test)]
fn insert_required_spec(
    body: &mut Map<String, Value>,
    spec: &Map<String, Value>,
    field: &str,
) -> Result<(), AppError> {
    match spec.get(field) {
        Some(Value::Null) | None => Err(AppError::Schema(format!(
            "required spec field '{field}' is absent or null"
        ))),
        Some(v) => {
            body.insert(field.to_owned(), v.clone());
            Ok(())
        }
    }
}

/// Insert an optional-null spec field: explicit null when absent or authored null;
/// authored value otherwise.
#[cfg(test)]
fn insert_optional_null_spec(
    body: &mut Map<String, Value>,
    spec: &Map<String, Value>,
    field: &str,
) {
    let value = spec.get(field).cloned().unwrap_or(Value::Null);
    body.insert(field.to_owned(), value);
}

// ---------------------------------------------------------------------------
// Tests (F-006)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod assistant_reference_projection_tests {
    use super::*;

    #[test]
    fn resolved_assistant_references_replace_every_authoring_reference() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("assistant.yaml");
        let yaml = r#"apiVersion: codemie.epam.com/v1alpha1
kind: Assistant
metadata:
  project: project-a
  slug: assistant-a
spec:
  name: Assistant A
  system_prompt: Helpful
  llm_model_type: gpt
  type: codemie
  context:
    - context_type: knowledge_base
      ref: {project: project-a, repo_name: docs-a}
  toolkits: []
  conversation_starters: []
  shared: false
  mcp_servers: []
  sub_assistants:
    - {project: project-a, slug: nested-a}
  enabled_builtin_subagents: []
  prompt_variables: []
  skills:
    - {project: project-a, name: skill-a}
  categories: []
"#;
        let declaration = crate::parse::parse_and_validate(yaml, &path).unwrap();
        let references = AssistantReferenceMap {
            assistant_ids: vec!["assistant-id".into()],
            skill_ids: vec!["skill-id".into()],
            context: vec![serde_json::json!({
                "context_type": "knowledge_base",
                "name": "docs-a",
            })],
        };

        let WritePlan::Create {
            request: RequestBody::Json(body),
        } = project_with_assistant_references(&declaration, None, &references).unwrap()
        else {
            panic!("Assistant create must use a JSON request");
        };
        assert_eq!(body["assistant_ids"], serde_json::json!(["assistant-id"]));
        assert_eq!(body["skill_ids"], serde_json::json!(["skill-id"]));
        assert_eq!(
            body["context"],
            serde_json::json!([{"context_type":"knowledge_base","name":"docs-a"}])
        );
        assert!(body.get("sub_assistants").is_none());
        assert!(body.get("skills").is_none());
    }
}
