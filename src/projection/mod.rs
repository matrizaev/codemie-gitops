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
use std::path::Path;

use serde_json::{Map, Value};

use crate::error::AppError;
use crate::parse::{EntityKind, ParsedDeclaration};

// ---------------------------------------------------------------------------
// Public plan types (data-model.md §7)
// ---------------------------------------------------------------------------

/// A file part for multipart/form-data upload (File Datasource only).
#[derive(Debug, Clone)]
pub struct FilePart {
    /// Filename for the multipart part (basename only, SEC-005).
    pub filename: String,
    /// Raw file bytes (never echoed to output or logs).
    pub content: Vec<u8>,
}

/// The request payload for a single POST or PUT operation.
#[derive(Debug, Clone)]
pub enum RequestBody {
    /// Ordinary JSON body.
    Json(Value),
    /// `multipart/form-data` with scalar query parameters.
    /// Used only for File Datasource (data-model.md §6).
    FileMultipart {
        /// Repeated `files` multipart parts.
        parts: Vec<FilePart>,
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
/// `repo_root` and `follow_symlinks` are forwarded to file-load helpers for
/// File Datasource projections.
///
/// Fails with `AppError::Schema` when a required authored field is absent or
/// null, or when a Workflow `meta_config` string is malformed.
pub fn project(
    decl: &ParsedDeclaration,
    existing: Option<&ExistingEntity>,
    _adopt_workflow_id: Option<&str>,
    _repo_root: &Path,
    _follow_symlinks: bool,
) -> Result<WritePlan, AppError> {
    let server_id = existing.map(|e| e.server_id.clone());

    let request = match decl.kind {
        EntityKind::Assistant => project_assistant(&decl.value, server_id.is_some()),
        EntityKind::Workflow => project_workflow(
            &decl.value,
            server_id.is_some(),
            existing.and_then(|e| e.meta_config.as_deref()),
        ),
        EntityKind::Skill => project_skill(&decl.value, server_id.is_some()),
        EntityKind::Datasource => project_datasource(&decl.value, server_id.is_some()),
    }?;

    Ok(match server_id {
        None => WritePlan::Create { request },
        Some(id) => WritePlan::Update { server_id: id, request },
    })
}

// ---------------------------------------------------------------------------
// Assistant projection
// ---------------------------------------------------------------------------

/// Projection policy constants for the Assistant entity.
///
/// Source: contracts/adapter-manifest-v2.42.0.json §entities.Assistant.
mod assistant {
    /// Fields the client MUST NOT send regardless of projection (manifest
    /// §excludedRequestFields). These are server-owned vendor extensions.
    pub const EXCLUDED: &[&str] =
        &["is_react", "bedrock", "bedrock_agentcore_runtime", "agent_card", "skip_integration_validation"];

    /// Fields in the authored `spec` that map to request body fields.
    /// Source: manifest §requestFields minus §excludedRequestFields.
    pub const REQUEST_FIELDS: &[&str] = &[
        "name", "description", "system_prompt", "project", "context",
        "icon_url", "llm_model_type", "enable_image_generation", "image_generation_model",
        "toolkits", "conversation_starters", "shared", "is_global", "agent_mode",
        "plan_prompt", "slug", "temperature", "top_p", "tools_tokens_size_limit",
        "smart_tool_selection_enabled", "hedging_config", "interactive_features",
        "mcp_servers", "assistant_ids", "enabled_builtin_subagents", "skill_ids",
        "type", "categories", "prompt_variables", "custom_metadata", "guardrail_assignments",
    ];

    /// Optional-null spec fields → explicit JSON null when absent/null.
    /// Source: manifest §fieldClasses.optionalNull (spec.* paths).
    pub const OPTIONAL_NULL: &[&str] = &[
        "description", "icon_url", "enable_image_generation", "image_generation_model",
        "is_global", "agent_mode", "plan_prompt", "temperature", "top_p",
        "tools_tokens_size_limit", "smart_tool_selection_enabled", "hedging_config",
        "interactive_features", "prompt_variables", "custom_metadata", "guardrail_assignments",
    ];

    /// Required spec fields (authoringRequired from manifest).
    pub const REQUIRED: &[&str] = &[
        "name", "system_prompt", "llm_model_type", "type",
        "context", "toolkits", "conversation_starters", "shared",
        "mcp_servers", "enabled_builtin_subagents", "categories",
    ];
}

fn project_assistant(decl_value: &Value, _is_update: bool) -> Result<RequestBody, AppError> {
    let metadata = get_obj(decl_value, "metadata")?;
    let spec = get_obj(decl_value, "spec")?;

    let mut body = Map::new();

    // authorTransform: metadata.project → project, metadata.slug → slug
    insert_required_from_obj(&mut body, metadata, "project", "project")?;
    insert_required_from_obj(&mut body, metadata, "slug", "slug")?;

    // Spec fields per manifest requestFields (minus excluded)
    for field in assistant::REQUEST_FIELDS {
        // Skip excluded fields
        if assistant::EXCLUDED.contains(field) {
            continue;
        }
        // Skip identity-injected fields already handled above
        if *field == "project" || *field == "slug" {
            continue;
        }

        if assistant::REQUIRED.contains(field) {
            insert_required_spec(&mut body, spec, field)?;
        } else if assistant::OPTIONAL_NULL.contains(field) {
            insert_optional_null_spec(&mut body, spec, field);
        }
        // Fields not in REQUIRED or OPTIONAL_NULL are authoringOnly/operationInapplicable —
        // they do not appear in the request body under their spec name.
    }

    // authorTransform: spec.sub_assistants → assistant_ids (natural refs, resolved by adapter)
    // authorTransform: spec.skills → skill_ids (natural refs, resolved by adapter)
    // These are authoring-only source fields. If the adapter has pre-resolved them into
    // assistant_ids / skill_ids in the spec JSON, they pass through as REQUEST_FIELDS.
    // context[].ref is authoringOnly and excluded; context array passes through.

    Ok(RequestBody::Json(Value::Object(body)))
}

// ---------------------------------------------------------------------------
// Workflow projection
// ---------------------------------------------------------------------------

mod workflow {
    /// Fields sent on POST (create). Source: manifest §createRequestFields.
    /// `id` is in §clientExcludedRequestFields — never emitted.
    pub const CREATE_FIELDS: &[&str] = &[
        "name", "mode", "description", "start_hint", "project",
        "icon_url", "yaml_config", "shared", "assistants", "tools",
        "states", "supervisor_prompt", "meta_config", "guardrail_assignments",
    ];

    /// Fields sent on PUT (update). Source: manifest §updateRequestFields.
    /// `id` is in §clientExcludedRequestFields — never emitted.
    pub const UPDATE_FIELDS: &[&str] = &[
        "name", "description", "start_hint", "project", "mode",
        "icon_url", "shared", "yaml_config", "supervisor_prompt",
        "meta_config", "guardrail_assignments",
    ];

    /// Optional-null fields → explicit JSON null when absent/null.
    pub const OPTIONAL_NULL: &[&str] = &[
        "start_hint", "icon_url", "supervisor_prompt", "guardrail_assignments",
    ];

    /// Required on create and update.
    pub const REQUIRED: &[&str] = &["name", "mode", "description", "shared"];

    pub const RESERVED_KEY: &str = "codemie.epam.com/gitops/workflow-identity";
}

fn project_workflow(
    decl_value: &Value,
    is_update: bool,
    server_meta_config: Option<&str>,
) -> Result<RequestBody, AppError> {
    let metadata = get_obj(decl_value, "metadata")?;
    let spec = get_obj(decl_value, "spec")?;

    let mut body = Map::new();

    // authorTransform: metadata.project → project
    insert_required_from_obj(&mut body, metadata, "project", "project")?;

    let active_fields = if is_update { workflow::UPDATE_FIELDS } else { workflow::CREATE_FIELDS };

    for field in active_fields {
        if *field == "project" {
            // already inserted from metadata
            continue;
        }

        match *field {
            "meta_config" => {
                // mixedOwned: decode/merge/encode per metaConfigCodec
                let authored_meta = spec.get("meta_config");
                let merged = merge_meta_config(server_meta_config, authored_meta, metadata)?;
                body.insert("meta_config".to_owned(), merged);
            }
            "yaml_config" | "assistants" | "tools" | "states" => {
                // authorTransform: execution_config → yaml_config + flattened assistants/tools/states
                // For create, adapter must pre-encode execution_config into yaml_config etc.
                // For update, only yaml_config is used. The adapter fills these from execution_config
                // before calling project(). Pass through any values already placed in spec by adapter.
                if let Some(v) = spec.get(*field) {
                    body.insert((*field).to_owned(), v.clone());
                }
                // If absent: operationInapplicable on update (states/assistants/tools not in
                // updateRequestFields). For create, yaml_config is required by adapter transform.
            }
            f if workflow::REQUIRED.contains(&f) => {
                insert_required_spec(&mut body, spec, f)?;
            }
            f if workflow::OPTIONAL_NULL.contains(&f) => {
                insert_optional_null_spec(&mut body, spec, f);
            }
            _ => {
                // Not required, not optional-null, not specially handled:
                // operationInapplicable or authoringOnly — skip.
            }
        }
    }

    Ok(RequestBody::Json(Value::Object(body)))
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
fn merge_meta_config(
    server_raw: Option<&str>,
    authored_value: Option<&Value>,
    metadata: &Map<String, Value>,
) -> Result<Value, AppError> {
    // Step 1: decode server side (null → empty object)
    let mut merged: BTreeMap<String, Value> = match server_raw {
        None | Some("null") => BTreeMap::new(),
        Some(s) => {
            let decoded: Value = serde_json::from_str(s)
                .map_err(|_| AppError::Schema("meta_config: server value is not valid JSON".into()))?;
            let obj = decoded
                .as_object()
                .ok_or_else(|| AppError::Schema("meta_config: server value is not a JSON object".into()))?;
            // Reject invalid reserved record upfront; the reserved key value is overwritten below
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        }
    };

    // Step 2: overlay authored non-reserved keys
    if let Some(authored) = authored_value {
        match authored {
            Value::Object(authored_map) => {
                for (k, v) in authored_map {
                    if k == workflow::RESERVED_KEY {
                        return Err(AppError::Schema(
                            "meta_config: author must not set the reserved identity key".into(),
                        ));
                    }
                    merged.insert(k.clone(), v.clone());
                }
            }
            Value::Null => {
                // authored explicit null → treat as empty authored object (no overlay)
            }
            _ => {
                return Err(AppError::Schema(
                    "meta_config: authored value must be a JSON object or null".into(),
                ));
            }
        }
    }

    // Step 3: install exact reserved identity record
    // WorkflowIdentityV1 = {version: 1, project: NonEmptyString, slug: NonEmptyString}
    let project = metadata
        .get("project")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Schema("meta_config: metadata.project required for identity".into()))?;
    let slug = metadata
        .get("slug")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Schema("meta_config: metadata.slug required for identity".into()))?;

    let identity = serde_json::json!({
        "project": project,
        "slug": slug,
        "version": 1
    });
    merged.insert(workflow::RESERVED_KEY.to_owned(), identity);

    // Step 4: encode — compact JSON, BTreeMap ensures keys sorted by Unicode scalar value
    let encoded = serde_json::to_string(&merged)
        .map_err(|_| AppError::Internal("meta_config: failed to encode merged object".into()))?;

    Ok(Value::String(encoded))
}

// ---------------------------------------------------------------------------
// Skill projection
// ---------------------------------------------------------------------------

mod skill {
    pub const CREATE_FIELDS: &[&str] = &[
        "name", "description", "content", "project", "visibility",
        "categories", "toolkits", "mcp_servers", "companion_files",
        "enabled_builtin_subagents",
    ];
    // create == update for Skill
    pub const UPDATE_FIELDS: &[&str] = CREATE_FIELDS;

    pub const REQUIRED: &[&str] = &[
        "name", "description", "content", "project", "visibility",
        "categories", "toolkits", "mcp_servers", "companion_files",
        "enabled_builtin_subagents",
    ];
    // No optionalNull fields for Skill (manifest §fieldClasses.optionalNull = [])
}

fn project_skill(decl_value: &Value, is_update: bool) -> Result<RequestBody, AppError> {
    let metadata = get_obj(decl_value, "metadata")?;
    let spec = get_obj(decl_value, "spec")?;

    let mut body = Map::new();

    // authorTransform: metadata.name → name, metadata.project → project
    insert_required_from_obj(&mut body, metadata, "name", "name")?;
    insert_required_from_obj(&mut body, metadata, "project", "project")?;

    let active_fields = if is_update { skill::UPDATE_FIELDS } else { skill::CREATE_FIELDS };

    for field in active_fields {
        if *field == "name" || *field == "project" {
            continue; // already inserted from metadata
        }
        if *field == "content" {
            // authorTransform: contentFrom → content (already resolved by parse/F-004)
            // spec.content holds the final bytes; spec.contentFrom is authoringOnly
            insert_required_spec(&mut body, spec, "content")?;
        } else if skill::REQUIRED.contains(field) {
            insert_required_spec(&mut body, spec, field)?;
        }
        // No optional-null fields for Skill
    }

    Ok(RequestBody::Json(Value::Object(body)))
}

// ---------------------------------------------------------------------------
// Datasource projection
// ---------------------------------------------------------------------------

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

/// Create-only fields by datasource kind (manifest §types.*.createOnlyFields).
fn create_only_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "git" | "svn" => &["indexType", "summarizationModel"],
        "confluence" => &[
            "include_restricted_content", "include_archived_content", "include_attachments",
            "include_comments", "keep_markdown_format", "keep_newlines", "embedding_model",
        ],
        "jira" | "xray" | "azure_devops_wiki" | "azure_devops_work_item" => &["embedding_model"],
        "google" => &["googleDoc", "setting_id", "embedding_model"],
        _ => &[],
    }
}

/// Request fields per kind for create.
fn create_request_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "git" | "svn" => &[
            "name", "description", "link", "branch", "filesFilter",
            "indexType", "embeddingsModel", "summarizationModel", "prompt",
            "docsGeneration", "projectSpaceVisible", "setting_id",
            "guardrail_assignments", "cron_expression", "timezone",
        ],
        "confluence" => &[
            "name", "project_name", "description", "project_space_visible", "cql",
            "setting_id", "include_restricted_content", "include_archived_content",
            "include_attachments", "include_comments", "keep_markdown_format",
            "keep_newlines", "embedding_model", "cron_expression", "timezone",
            "guardrail_assignments",
        ],
        "jira" | "xray" => &[
            "name", "project_name", "description", "project_space_visible", "jql",
            "setting_id", "embedding_model", "cron_expression", "timezone",
            "guardrail_assignments",
        ],
        "azure_devops_wiki" => &[
            "name", "project_name", "description", "project_space_visible",
            "wiki_query", "wiki_name", "setting_id", "embedding_model",
            "cron_expression", "timezone", "guardrail_assignments",
        ],
        "azure_devops_work_item" => &[
            "name", "project_name", "description", "project_space_visible",
            "wiql_query", "setting_id", "embedding_model",
            "cron_expression", "timezone", "guardrail_assignments",
        ],
        "sharepoint" => &[
            "name", "project_name", "description", "project_space_visible",
            "site_url", "include_pages", "include_documents", "include_lists",
            "max_file_size_mb", "files_filter", "auth_type", "setting_id",
            "embedding_model", "cron_expression", "timezone",
            "oauth_client_id", "oauth_tenant_id", "guardrail_assignments",
        ],
        "google" => &[
            "name", "project_name", "description", "project_space_visible",
            "googleDoc", "setting_id", "embedding_model",
            "cron_expression", "timezone", "guardrail_assignments",
        ],
        _ => &[],
    }
}

/// Request fields per kind for update.
fn update_request_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "git" | "svn" => &[
            "name", "description", "prompt", "embeddingsModel", "projectSpaceVisible",
            "docsGeneration", "branch", "link", "filesFilter", "setting_id",
            "guardrail_assignments", "cron_expression", "timezone",
        ],
        "confluence" => &[
            "name", "project_name", "description", "project_space_visible",
            "cql", "setting_id", "cron_expression", "timezone", "guardrail_assignments",
        ],
        "jira" | "xray" => &[
            "name", "project_name", "description", "project_space_visible",
            "jql", "setting_id", "cron_expression", "timezone", "guardrail_assignments",
        ],
        "azure_devops_wiki" => &[
            "name", "project_name", "description", "project_space_visible",
            "wiki_query", "wiki_name", "setting_id", "cron_expression",
            "timezone", "guardrail_assignments",
        ],
        "azure_devops_work_item" => &[
            "name", "project_name", "description", "project_space_visible",
            "wiql_query", "setting_id", "cron_expression", "timezone",
            "guardrail_assignments",
        ],
        "sharepoint" => &[
            "name", "project_name", "description", "project_space_visible",
            "site_url", "include_pages", "include_documents", "include_lists",
            "max_file_size_mb", "files_filter", "auth_type", "setting_id",
            "embedding_model", "cron_expression", "timezone",
            "oauth_client_id", "oauth_tenant_id", "guardrail_assignments",
        ],
        "google" => &[
            "name", "project_name", "description", "project_space_visible",
            "cron_expression", "timezone", "guardrail_assignments",
        ],
        _ => &[],
    }
}

fn project_json_datasource(
    spec: &Map<String, Value>,
    kind: &str,
    project: &str,
    repo_name: &str,
    is_update: bool,
) -> Result<RequestBody, AppError> {
    let active_fields = if is_update {
        update_request_fields(kind)
    } else {
        create_request_fields(kind)
    };
    let create_only = create_only_fields(kind);

    let mut body = Map::new();

    // Identity injection: repo_name → name (create) for git/svn; project_name for others
    match kind {
        "git" | "svn" => {
            // `name` = repo_name (injected on create; on update it's a route param)
            if !is_update {
                body.insert("name".to_owned(), Value::String(repo_name.to_owned()));
            }
        }
        _ => {
            // project_name in body
            body.insert("project_name".to_owned(), Value::String(project.to_owned()));
        }
    }

    for field in active_fields {
        // Skip identity-injected fields handled above
        if *field == "name" && (kind == "git" || kind == "svn") && !is_update {
            continue;
        }
        if *field == "project_name" {
            continue; // already inserted
        }
        // Skip create-only fields on update
        if is_update && create_only.contains(field) {
            continue;
        }

        // Emit from spec: pass through present values; absent = omit (no optionalNull rule
        // for datasource from commonFieldClasses — optionalNull says "JSON-body properties
        // typed Optional in every applicable operation"; we pass through whatever is present)
        if let Some(v) = spec.get(*field) {
            body.insert((*field).to_owned(), v.clone());
        }
        // Fields absent from authored spec are simply absent from the request body.
        // The manifest commonFieldClasses.optionalNull says "JSON-body properties typed
        // Optional" — these fields accept null on the wire; omission is also valid.
    }

    Ok(RequestBody::Json(Value::Object(body)))
}

fn project_file_datasource(
    spec: &Map<String, Value>,
    _metadata: &Map<String, Value>,
    project: &str,
    repo_name: &str,
    is_update: bool,
) -> Result<RequestBody, AppError> {
    // File Datasource: multipart/form-data, all scalar fields as query params.
    // - `files`: each path → FilePart (bytes read by adapter; here we encode paths)
    // - `uploaded_files`: update-only, compact JSON array in query param
    // - `guardrail_assignments`: compact JSON array in query param
    // - other scalars: query parameters
    // Source: manifest §entities.Datasource.types.file.transportTransform

    let mut query_params: Vec<(String, String)> = Vec::new();
    let mut parts: Vec<FilePart> = Vec::new();

    // Identity injection: project_name and repo_name as query params
    query_params.push(("project_name".to_owned(), project.to_owned()));
    if !is_update {
        query_params.push(("name".to_owned(), repo_name.to_owned()));
    }

    let active_fields: &[&str] = if is_update {
        &[
            "name", "project_name", "description", "project_space_visible",
            "uploaded_files", "files", "csv_separator", "csv_start_row",
            "csv_rows_per_document", "embedding_model", "guardrail_assignments",
            "include_email_attachments",
        ]
    } else {
        &[
            "name", "project_name", "description", "project_space_visible",
            "files", "csv_separator", "csv_start_row", "csv_rows_per_document",
            "embedding_model", "guardrail_assignments", "include_email_attachments",
        ]
    };

    for field in active_fields {
        // Skip identity fields already handled
        if *field == "project_name" || (*field == "name" && !is_update) {
            continue;
        }

        match *field {
            "files" => {
                // Each path entry → a FilePart (adapter reads bytes from disk)
                // The projection layer records the filename; the adapter fills `content`.
                if let Some(Value::Array(paths)) = spec.get("files") {
                    for path_val in paths {
                        if let Some(path_str) = path_val.as_str() {
                            let filename = std::path::Path::new(path_str)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(path_str)
                                .to_owned();
                            // Content is empty here; adapter fills before upload.
                            parts.push(FilePart { filename, content: Vec::new() });
                        }
                    }
                }
            }
            "uploaded_files" => {
                if is_update {
                    if let Some(v) = spec.get("uploaded_files") {
                        let encoded = serde_json::to_string(v).map_err(|_| {
                            AppError::Internal("file datasource: uploaded_files encode failed".into())
                        })?;
                        query_params.push(("uploaded_files".to_owned(), encoded));
                    }
                }
            }
            "guardrail_assignments" => {
                if let Some(v) = spec.get("guardrail_assignments") {
                    let encoded = serde_json::to_string(v).map_err(|_| {
                        AppError::Internal("file datasource: guardrail_assignments encode failed".into())
                    })?;
                    query_params.push(("guardrail_assignments".to_owned(), encoded));
                }
            }
            f => {
                if let Some(v) = spec.get(f) {
                    // Scalar query parameter
                    let s = scalar_to_string(v)?;
                    query_params.push((f.to_owned(), s));
                }
            }
        }
    }

    Ok(RequestBody::FileMultipart { parts, query_params })
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

fn get_obj<'a>(v: &'a Value, key: &str) -> Result<&'a Map<String, Value>, AppError> {
    v.get(key)
        .and_then(|v| v.as_object())
        .ok_or_else(|| AppError::Schema(format!("declaration is missing required '{key}' object")))
}

/// Insert a required field from one JSON object into the request body under
/// a (possibly different) output key. Fails if the source field is absent or null.
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
fn insert_optional_null_spec(body: &mut Map<String, Value>, spec: &Map<String, Value>, field: &str) {
    let value = spec.get(field).cloned().unwrap_or(Value::Null);
    body.insert(field.to_owned(), value);
}

// ---------------------------------------------------------------------------
// Tests (F-006)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // meta_config merge tests (metaConfigCodec, data-model.md §4)
    // -----------------------------------------------------------------------

    fn meta_metadata(project: &str, slug: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("project".to_owned(), Value::String(project.to_owned()));
        m.insert("slug".to_owned(), Value::String(slug.to_owned()));
        m
    }

    #[test]
    fn meta_config_null_server_no_authored_installs_reserved_only() {
        let meta = meta_metadata("p", "s");
        let result = merge_meta_config(None, None, &meta).unwrap();
        let s = result.as_str().unwrap();
        let obj: serde_json::Value = serde_json::from_str(s).unwrap();
        let reserved = obj[workflow::RESERVED_KEY].as_object().unwrap();
        assert_eq!(reserved["project"].as_str().unwrap(), "p");
        assert_eq!(reserved["slug"].as_str().unwrap(), "s");
        assert_eq!(reserved["version"].as_u64().unwrap(), 1);
        // No other keys
        assert_eq!(obj.as_object().unwrap().len(), 1);
    }

    #[test]
    fn meta_config_authored_value_merged_and_reserved_overwritten() {
        let meta = meta_metadata("p", "s");
        let authored = serde_json::json!({"foo": "bar", "baz": 42});
        let result = merge_meta_config(None, Some(&authored), &meta).unwrap();
        let s = result.as_str().unwrap();
        let obj: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(obj["foo"].as_str().unwrap(), "bar");
        assert_eq!(obj["baz"].as_u64().unwrap(), 42);
        assert!(obj[workflow::RESERVED_KEY].is_object());
    }

    #[test]
    fn meta_config_server_non_reserved_preserved_when_author_absent() {
        let meta = meta_metadata("p", "s");
        // Server has a custom key "server_key"
        let server_obj = serde_json::json!({"server_key": "value"});
        let server_str = serde_json::to_string(&server_obj).unwrap();
        let result = merge_meta_config(Some(&server_str), None, &meta).unwrap();
        let s = result.as_str().unwrap();
        let obj: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(obj["server_key"].as_str().unwrap(), "value");
    }

    #[test]
    fn meta_config_author_wins_over_server_for_non_reserved_key() {
        let meta = meta_metadata("p", "s");
        let server_obj = serde_json::json!({"shared_key": "server_value"});
        let server_str = serde_json::to_string(&server_obj).unwrap();
        let authored = serde_json::json!({"shared_key": "author_value"});
        let result = merge_meta_config(Some(&server_str), Some(&authored), &meta).unwrap();
        let s = result.as_str().unwrap();
        let obj: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(obj["shared_key"].as_str().unwrap(), "author_value");
    }

    #[test]
    fn meta_config_author_cannot_set_reserved_key() {
        let meta = meta_metadata("p", "s");
        let authored = serde_json::json!({
            workflow::RESERVED_KEY: {"version": 1, "project": "evil", "slug": "evil"}
        });
        let err = merge_meta_config(None, Some(&authored), &meta).unwrap_err();
        assert!(matches!(err, AppError::Schema(_)));
    }

    #[test]
    fn meta_config_keys_sorted_by_unicode_scalar() {
        let meta = meta_metadata("p", "s");
        let authored = serde_json::json!({"z_key": 1, "a_key": 2});
        let result = merge_meta_config(None, Some(&authored), &meta).unwrap();
        let s = result.as_str().unwrap();
        // BTreeMap guarantees lexicographic order; verify the encoded string
        // has a_key before z_key before the reserved key
        // BTreeMap sorts lexicographically:
        //   "a_key" (a=0x61) < "codemie.epam.com/..." (c=0x63) < "z_key" (z=0x7A)
        let a_pos = s.find("a_key").unwrap();
        let z_pos = s.find("z_key").unwrap();
        let r_pos = s.find("codemie.epam.com").unwrap();
        assert!(a_pos < r_pos, "a_key must come before reserved key");
        assert!(r_pos < z_pos, "reserved key must come before z_key");
    }

    #[test]
    fn meta_config_malformed_server_string_is_schema_error() {
        let meta = meta_metadata("p", "s");
        let err = merge_meta_config(Some("not-json{{{"), None, &meta).unwrap_err();
        assert!(matches!(err, AppError::Schema(_)));
    }

    #[test]
    fn meta_config_server_non_object_root_is_schema_error() {
        let meta = meta_metadata("p", "s");
        let err = merge_meta_config(Some("[1,2,3]"), None, &meta).unwrap_err();
        assert!(matches!(err, AppError::Schema(_)));
    }

    #[test]
    fn meta_config_authored_explicit_null_keeps_server_non_reserved() {
        let meta = meta_metadata("p", "s");
        let server_obj = serde_json::json!({"srv": "keep"});
        let server_str = serde_json::to_string(&server_obj).unwrap();
        let result = merge_meta_config(Some(&server_str), Some(&Value::Null), &meta).unwrap();
        let s = result.as_str().unwrap();
        let obj: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(obj["srv"].as_str().unwrap(), "keep");
    }

    // -----------------------------------------------------------------------
    // WritePlan variant tests
    // -----------------------------------------------------------------------

    #[test]
    fn write_plan_create_variant() {
        let plan = WritePlan::Create { request: RequestBody::Json(serde_json::json!({})) };
        assert!(matches!(plan, WritePlan::Create { .. }));
    }

    #[test]
    fn write_plan_update_variant() {
        let plan = WritePlan::Update {
            server_id: "uuid".into(),
            request: RequestBody::Json(serde_json::json!({})),
        };
        assert!(matches!(plan, WritePlan::Update { .. }));
    }

    // -----------------------------------------------------------------------
    // Optional-null helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn optional_null_absent_field_becomes_explicit_null() {
        let spec = Map::new();
        let mut body = Map::new();
        insert_optional_null_spec(&mut body, &spec, "description");
        assert_eq!(body["description"], Value::Null);
    }

    #[test]
    fn optional_null_authored_null_stays_null() {
        let mut spec = Map::new();
        spec.insert("description".to_owned(), Value::Null);
        let mut body = Map::new();
        insert_optional_null_spec(&mut body, &spec, "description");
        assert_eq!(body["description"], Value::Null);
    }

    #[test]
    fn optional_null_authored_value_passes_through() {
        let mut spec = Map::new();
        spec.insert("description".to_owned(), Value::String("desc".into()));
        let mut body = Map::new();
        insert_optional_null_spec(&mut body, &spec, "description");
        assert_eq!(body["description"].as_str().unwrap(), "desc");
    }

    // -----------------------------------------------------------------------
    // Required field helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn required_absent_field_returns_schema_error() {
        let spec = Map::new();
        let mut body = Map::new();
        let err = insert_required_spec(&mut body, &spec, "system_prompt").unwrap_err();
        assert!(matches!(err, AppError::Schema(_)));
    }

    #[test]
    fn required_null_field_returns_schema_error() {
        let mut spec = Map::new();
        spec.insert("system_prompt".to_owned(), Value::Null);
        let mut body = Map::new();
        let err = insert_required_spec(&mut body, &spec, "system_prompt").unwrap_err();
        assert!(matches!(err, AppError::Schema(_)));
    }

    #[test]
    fn required_present_field_inserted() {
        let mut spec = Map::new();
        spec.insert("system_prompt".to_owned(), Value::String("prompt".into()));
        let mut body = Map::new();
        insert_required_spec(&mut body, &spec, "system_prompt").unwrap();
        assert_eq!(body["system_prompt"].as_str().unwrap(), "prompt");
    }

    // -----------------------------------------------------------------------
    // Excluded fields test (Assistant)
    // -----------------------------------------------------------------------

    #[test]
    fn assistant_excluded_fields_not_in_request_fields() {
        for excluded in assistant::EXCLUDED {
            assert!(
                !assistant::REQUEST_FIELDS.contains(excluded),
                "excluded field '{excluded}' must not appear in REQUEST_FIELDS"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Skill: no optional-null fields
    // -----------------------------------------------------------------------

    #[test]
    fn skill_create_and_update_fields_are_identical() {
        assert_eq!(skill::CREATE_FIELDS, skill::UPDATE_FIELDS);
    }

    // -----------------------------------------------------------------------
    // Workflow: create-only fields absent from update
    // -----------------------------------------------------------------------

    #[test]
    fn workflow_create_only_fields_absent_from_update() {
        let create_only = ["assistants", "tools", "states"];
        for f in create_only {
            assert!(
                !workflow::UPDATE_FIELDS.contains(&f),
                "create-only field '{f}' must not be in workflow UPDATE_FIELDS"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Workflow: client-excluded `id` field absent from all field lists
    // -----------------------------------------------------------------------

    #[test]
    fn workflow_client_excluded_id_absent_from_all_field_lists() {
        assert!(!workflow::CREATE_FIELDS.contains(&"id"));
        assert!(!workflow::UPDATE_FIELDS.contains(&"id"));
    }

    // -----------------------------------------------------------------------
    // Datasource create-only field exclusion on update
    // -----------------------------------------------------------------------

    #[test]
    fn datasource_create_only_fields_absent_from_update_fields() {
        for kind in &["git", "svn", "confluence", "jira", "xray", "google"] {
            let co = create_only_fields(kind);
            let uf = update_request_fields(kind);
            for f in co {
                assert!(
                    !uf.contains(f),
                    "create-only field '{f}' for kind '{kind}' must not be in update fields"
                );
            }
        }
    }
}
