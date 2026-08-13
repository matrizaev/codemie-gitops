//! Contract-driven reverse mapping and canonical declaration serialization.

use super::publication::SaveError;
use super::snapshot::{
    AssistantSnapshot, DatasourceSnapshot, EntitySnapshot, OpenResponseValue, ResponseField,
    SkillSnapshot, WorkflowSnapshot, reject_secret_values,
};
use super::*;

pub(super) fn project_response(
    command: &SaveCommand,
    project: &str,
    response: EntitySnapshot,
) -> Result<(serde_json::Value, bool), AppError> {
    let adoption_required =
        command.selector.kind() == SaveKind::Workflow && command.selector.workflow_id().is_some();
    let declaration = match (command.selector.kind(), response) {
        (SaveKind::Assistant, EntitySnapshot::Assistant(snapshot)) => {
            reverse_assistant(command, project, *snapshot)?
        }
        (SaveKind::Workflow, EntitySnapshot::Workflow(snapshot)) => {
            reverse_workflow(command, project, snapshot)?
        }
        (SaveKind::Skill, EntitySnapshot::Skill(snapshot)) => {
            reverse_skill(command, project, snapshot)?
        }
        (SaveKind::Datasource, EntitySnapshot::Datasource(snapshot)) => {
            reverse_datasource(command, project, *snapshot)?
        }
        _ => {
            return Err(AppError::Internal(
                "save snapshot kind does not match validated selector".into(),
            ));
        }
    };
    Ok((declaration, adoption_required))
}

fn declaration(
    kind: &str,
    metadata: serde_json::Value,
    spec: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "codemie.epam.com/v1alpha1",
        "kind": kind,
        "metadata": metadata,
        "spec": spec,
    })
}

fn require_matching_project(actual: Option<&String>, expected: &str) -> Result<(), AppError> {
    if actual.is_some_and(|actual| actual != expected) {
        return Err(AppError::ApiIncompatible(
            "response project does not match selector".into(),
        ));
    }
    Ok(())
}

fn insert_field(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    field: ResponseField<OpenResponseValue>,
) {
    if let Some(value) = field.into_option() {
        target.insert(key.into(), value.0);
    }
}

fn forbidden_extension(
    extensions: &super::snapshot::ResponseExtensions,
    key: &str,
) -> Result<(), AppError> {
    if extensions.0.get(key).is_some_and(|value| {
        !value.0.is_null()
            && !matches!(&value.0, serde_json::Value::Array(items) if items.is_empty())
    }) {
        return Err(AppError::EntityNotExportable);
    }
    Ok(())
}

fn reverse_assistant(
    command: &SaveCommand,
    project: &str,
    snapshot: AssistantSnapshot,
) -> Result<serde_json::Value, AppError> {
    require_matching_project(snapshot.project.as_ref(), project)?;
    let slug = command
        .selector
        .slug()
        .ok_or(AppError::EntityNotExportable)?;
    if snapshot.slug.as_ref().is_some_and(|actual| actual != slug) {
        return Err(AppError::EntityNotExportable);
    }
    forbidden_extension(&snapshot.extensions, "access_token")?;
    forbidden_extension(&snapshot.extensions, "mcp_connect_auth_token")?;
    let mut spec = serde_json::Map::new();
    insert_field(&mut spec, "name", snapshot.name);
    insert_field(&mut spec, "description", snapshot.description);
    insert_field(&mut spec, "system_prompt", snapshot.system_prompt);
    insert_field(&mut spec, "type", snapshot.type_);
    insert_field(&mut spec, "context", snapshot.context);
    insert_field(&mut spec, "toolkits", snapshot.toolkits);
    insert_field(&mut spec, "icon_url", snapshot.icon_url);
    insert_field(&mut spec, "llm_model_type", snapshot.llm_model_type);
    insert_field(
        &mut spec,
        "enable_image_generation",
        snapshot.enable_image_generation,
    );
    insert_field(
        &mut spec,
        "image_generation_model",
        snapshot.image_generation_model,
    );
    insert_field(
        &mut spec,
        "conversation_starters",
        snapshot.conversation_starters,
    );
    insert_field(&mut spec, "shared", snapshot.shared);
    insert_field(&mut spec, "is_global", snapshot.is_global);
    insert_field(&mut spec, "agent_mode", snapshot.agent_mode);
    insert_field(&mut spec, "plan_prompt", snapshot.plan_prompt);
    insert_field(&mut spec, "temperature", snapshot.temperature);
    insert_field(&mut spec, "top_p", snapshot.top_p);
    insert_field(
        &mut spec,
        "tools_tokens_size_limit",
        snapshot.tools_tokens_size_limit,
    );
    insert_field(
        &mut spec,
        "smart_tool_selection_enabled",
        snapshot.smart_tool_selection_enabled,
    );
    insert_field(&mut spec, "hedging_config", snapshot.hedging_config);
    insert_field(
        &mut spec,
        "interactive_features",
        snapshot.interactive_features,
    );
    insert_field(&mut spec, "mcp_servers", snapshot.mcp_servers);
    insert_field(
        &mut spec,
        "enabled_builtin_subagents",
        snapshot.enabled_builtin_subagents,
    );
    insert_field(&mut spec, "categories", snapshot.categories);
    insert_field(&mut spec, "prompt_variables", snapshot.prompt_variables);
    insert_field(&mut spec, "custom_metadata", snapshot.custom_metadata);
    insert_field(
        &mut spec,
        "guardrail_assignments",
        snapshot.guardrail_assignments,
    );
    spec.insert(
        "sub_assistants".into(),
        serde_json::Value::Array(snapshot.sub_assistants),
    );
    spec.insert("skills".into(), serde_json::Value::Array(snapshot.skills));
    Ok(declaration(
        "Assistant",
        serde_json::json!({"project": project, "slug": slug}),
        serde_json::Value::Object(spec),
    ))
}

fn reverse_workflow(
    command: &SaveCommand,
    project: &str,
    snapshot: WorkflowSnapshot,
) -> Result<serde_json::Value, AppError> {
    require_matching_project(snapshot.project.as_ref(), project)?;
    let slug = command
        .selector
        .slug()
        .ok_or(AppError::EntityNotExportable)?;
    if snapshot.slug.as_ref().is_some_and(|actual| actual != slug) {
        return Err(AppError::EntityNotExportable);
    }
    if snapshot
        .mode
        .as_ref()
        .is_some_and(|mode| mode == "Autonomous")
        || snapshot.deprecated.as_ref() == Some(&true)
    {
        return Err(AppError::EntityNotExportable);
    }
    for key in [
        "access_token",
        "assistant_ids",
        "skill_ids",
        "datasource_ids",
    ] {
        forbidden_extension(&snapshot.extensions, key)?;
    }
    let mut spec = serde_json::Map::new();
    insert_field(&mut spec, "name", snapshot.name);
    insert_field(&mut spec, "description", snapshot.description);
    insert_field(
        &mut spec,
        "mode",
        snapshot.mode.map(|value| OpenResponseValue(value.into())),
    );
    insert_field(&mut spec, "start_hint", snapshot.start_hint);
    insert_field(&mut spec, "icon_url", snapshot.icon_url);
    insert_field(&mut spec, "shared", snapshot.shared);
    insert_field(&mut spec, "supervisor_prompt", snapshot.supervisor_prompt);
    if let Some(value) = snapshot.meta_config.into_option() {
        spec.insert(
            "meta_config".into(),
            reverse_workflow_meta(&value.0, project, slug)?,
        );
    }
    if let Some(value) = snapshot.yaml_config.into_option() {
        spec.insert(
            "execution_config".into(),
            reverse_workflow_yaml_config(&value.0)?,
        );
    }
    insert_field(
        &mut spec,
        "guardrail_assignments",
        snapshot.guardrail_assignments,
    );
    Ok(declaration(
        "Workflow",
        serde_json::json!({"project": project, "slug": slug}),
        serde_json::Value::Object(spec),
    ))
}

impl<T> ResponseField<T> {
    fn map<U>(self, mapper: impl FnOnce(T) -> U) -> ResponseField<U> {
        match self {
            Self::Missing => ResponseField::Missing,
            Self::Present(value) => ResponseField::Present(mapper(value)),
        }
    }
}

fn reverse_skill(
    command: &SaveCommand,
    project: &str,
    snapshot: SkillSnapshot,
) -> Result<serde_json::Value, AppError> {
    let name = command
        .selector
        .name()
        .ok_or(AppError::EntityNotExportable)?;
    if snapshot.detail.project != project || snapshot.detail.name != name {
        return Err(AppError::EntityNotExportable);
    }
    let mut spec = serde_json::Map::new();
    spec.insert("description".into(), snapshot.detail.description.0);
    spec.insert("visibility".into(), snapshot.detail.visibility.0);
    spec.insert(
        "categories".into(),
        serde_json::to_value(snapshot.detail.categories).map_err(SaveError::JsonSerialization)?,
    );
    spec.insert(
        "toolkits".into(),
        serde_json::to_value(snapshot.detail.toolkits).map_err(SaveError::JsonSerialization)?,
    );
    spec.insert(
        "mcp_servers".into(),
        serde_json::to_value(snapshot.detail.mcp_servers).map_err(SaveError::JsonSerialization)?,
    );
    spec.insert(
        "companion_files".into(),
        serde_json::to_value(snapshot.companion_files).map_err(SaveError::JsonSerialization)?,
    );
    spec.insert(
        "enabled_builtin_subagents".into(),
        serde_json::to_value(snapshot.detail.enabled_builtin_subagents)
            .map_err(SaveError::JsonSerialization)?,
    );
    spec.insert("content".into(), snapshot.detail.content.into());
    Ok(declaration(
        "Skill",
        serde_json::json!({"project": project, "name": name}),
        serde_json::Value::Object(spec),
    ))
}

pub(super) fn reverse_datasource(
    command: &SaveCommand,
    project: &str,
    snapshot: DatasourceSnapshot,
) -> Result<serde_json::Value, AppError> {
    require_matching_project(snapshot.project_name.as_ref(), project)?;
    let repo_name = command
        .selector
        .repo_name()
        .ok_or(AppError::EntityNotExportable)?;
    if snapshot
        .repo_name
        .as_ref()
        .is_some_and(|actual| actual != repo_name)
    {
        return Err(AppError::EntityNotExportable);
    }
    reject_datasource_secrets(&snapshot)?;
    let index_type = required_string_field(&snapshot.index_type, "index_type")?;
    let mut spec = serde_json::Map::new();
    let branch = match index_type.as_str() {
        "code" | "summary" | "chunk-summary" => {
            let vcs_type = required_string_field(&snapshot.vcs_type, "vcs_type")?;
            let branch = match vcs_type.as_str() {
                "git" => "git",
                "svn" => "svn",
                _ => {
                    return Err(AppError::ApiIncompatible(
                        "Datasource vcs_type is invalid".into(),
                    ));
                }
            };
            spec.insert(
                "description".into(),
                required_value_field(&snapshot.description, "description")?,
            );
            spec.insert("link".into(), required_value_field(&snapshot.link, "link")?);
            spec.insert(
                "branch".into(),
                required_value_field(&snapshot.branch, "branch")?,
            );
            spec.insert(
                "projectSpaceVisible".into(),
                required_value_field(&snapshot.project_space_visible, "project_space_visible")?,
            );
            spec.insert("indexType".into(), index_type.clone().into());
            insert_field(&mut spec, "filesFilter", snapshot.files_filter.clone());
            insert_field(
                &mut spec,
                "embeddingsModel",
                snapshot.embeddings_model.clone(),
            );
            insert_field(
                &mut spec,
                "summarizationModel",
                snapshot.summarization_model.clone(),
            );
            insert_field(&mut spec, "prompt", snapshot.prompt.clone());
            insert_field(
                &mut spec,
                "docsGeneration",
                snapshot.docs_generation.clone(),
            );
            branch
        }
        "knowledge_base_confluence" => {
            reverse_nested_branch(
                &snapshot,
                &snapshot.confluence,
                "confluence",
                "cql",
                &mut spec,
            )?;
            "confluence"
        }
        "knowledge_base_jira" => {
            reverse_nested_branch(&snapshot, &snapshot.jira, "jira", "jql", &mut spec)?;
            "jira"
        }
        "knowledge_base_xray" => {
            reverse_nested_branch(&snapshot, &snapshot.xray, "xray", "jql", &mut spec)?;
            "xray"
        }
        "knowledge_base_azure_devops_wiki" => {
            reverse_nested_branch(
                &snapshot,
                &snapshot.azure_devops_wiki,
                "azure_devops_wiki",
                "wiki_query",
                &mut spec,
            )?;
            "azure_devops_wiki"
        }
        "knowledge_base_azure_devops_work_item" => {
            reverse_nested_branch(
                &snapshot,
                &snapshot.azure_devops_work_item,
                "azure_devops_work_item",
                "wiql_query",
                &mut spec,
            )?;
            "azure_devops_work_item"
        }
        "knowledge_base_sharepoint" => {
            let nested = required_object_field(&snapshot.sharepoint, "sharepoint")?;
            reject_secret_values(&serde_json::Value::Object(nested.clone()))?;
            copy_required_nested(nested, "site_url", &mut spec)?;
            copy_required_nested(nested, "auth_type", &mut spec)?;
            copy_optional_nested(nested, "include_pages", &mut spec);
            copy_optional_nested(nested, "include_documents", &mut spec);
            copy_optional_nested(nested, "include_lists", &mut spec);
            copy_optional_nested(nested, "max_file_size_mb", &mut spec);
            copy_optional_nested(nested, "files_filter", &mut spec);
            copy_optional_nested(nested, "oauth_client_id", &mut spec);
            copy_optional_nested(nested, "oauth_tenant_id", &mut spec);
            "sharepoint"
        }
        "llm_routing_google" => {
            spec.insert(
                "description".into(),
                required_value_field(&snapshot.description, "description")?,
            );
            spec.insert(
                "googleDoc".into(),
                required_value_field(&snapshot.google_doc_link, "google_doc_link")?,
            );
            spec.insert(
                "setting_id".into(),
                required_value_field(&snapshot.setting_id, "setting_id")?,
            );
            "google"
        }
        "knowledge_base_file" | "provider" | "bedrock" => {
            return Err(AppError::EntityNotExportable);
        }
        _ => {
            return Err(AppError::ApiIncompatible(
                "Datasource index_type is unsupported".into(),
            ));
        }
    };
    spec.insert("index_type".into(), branch.into());
    insert_field(&mut spec, "setting_id", snapshot.setting_id);
    insert_field(
        &mut spec,
        "guardrail_assignments",
        snapshot.guardrail_assignments,
    );
    insert_field(&mut spec, "cron_expression", snapshot.cron_expression);
    insert_field(&mut spec, "timezone", snapshot.timezone);
    Ok(declaration(
        "Datasource",
        serde_json::json!({"project": project, "repo_name": repo_name}),
        serde_json::Value::Object(spec),
    ))
}

fn reverse_nested_branch(
    snapshot: &DatasourceSnapshot,
    nested: &ResponseField<OpenResponseValue>,
    nested_key: &str,
    required_key: &str,
    spec: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    let nested = required_object_field(nested, nested_key)?;
    spec.insert(
        "description".into(),
        required_value_field(&snapshot.description, "description")?,
    );
    copy_required_nested(nested, required_key, spec)?;
    insert_field(
        spec,
        "project_space_visible",
        snapshot.project_space_visible.clone(),
    );
    copy_optional_nested(nested, "include_restricted_content", spec);
    copy_optional_nested(nested, "include_archived_content", spec);
    copy_optional_nested(nested, "include_attachments", spec);
    copy_optional_nested(nested, "include_comments", spec);
    copy_optional_nested(nested, "keep_markdown_format", spec);
    copy_optional_nested(nested, "keep_newlines", spec);
    copy_optional_nested(nested, "embedding_model", spec);
    copy_optional_nested(nested, "wiki_name", spec);
    Ok(())
}

fn required_object_field<'a>(
    field: &'a ResponseField<OpenResponseValue>,
    label: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, AppError> {
    field
        .as_ref()
        .map(|value| &value.0)
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| AppError::ApiIncompatible(format!("Datasource {label} is missing")))
}

fn required_value_field(
    field: &ResponseField<OpenResponseValue>,
    label: &str,
) -> Result<serde_json::Value, AppError> {
    field
        .as_ref()
        .map(|value| &value.0)
        .filter(|value| !value.is_null())
        .cloned()
        .ok_or_else(|| AppError::ApiIncompatible(format!("Datasource {label} is missing")))
}

fn required_string_field(field: &ResponseField<String>, label: &str) -> Result<String, AppError> {
    field
        .as_ref()
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or_else(|| AppError::ApiIncompatible(format!("Datasource {label} is invalid")))
}

fn copy_required_nested(
    source: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    target: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    let value = source
        .get(key)
        .filter(|value| !value.is_null())
        .ok_or_else(|| AppError::ApiIncompatible(format!("Datasource {key} is missing")))?;
    target.insert(key.into(), value.clone());
    Ok(())
}

fn copy_optional_nested(
    source: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    target: &mut serde_json::Map<String, serde_json::Value>,
) {
    if let Some(value) = source.get(key) {
        target.insert(key.into(), value.clone());
    }
}

fn reject_datasource_secrets(snapshot: &DatasourceSnapshot) -> Result<(), AppError> {
    for field in [&snapshot.provider_fields, &snapshot.bedrock] {
        if let Some(value) = field.as_ref()
            && !value.0.is_null()
        {
            reject_secret_values(&value.0)?;
            return Err(AppError::EntityNotExportable);
        }
    }
    for key in ["access_token", "expires_at", "sharepoint_access_token"] {
        forbidden_extension(&snapshot.extensions, key)?;
    }
    Ok(())
}

fn reverse_workflow_meta(
    value: &serde_json::Value,
    project: &str,
    slug: &str,
) -> Result<serde_json::Value, AppError> {
    let Some(raw) = value.as_str() else {
        return if value.is_null() {
            Ok(serde_json::Value::Null)
        } else {
            Err(AppError::ApiIncompatible(
                "Workflow meta_config must be a JSON string or null".into(),
            ))
        };
    };
    let mut decoded: serde_json::Value =
        serde_json::from_str(raw).map_err(SaveError::SnapshotJson)?;
    let object = decoded.as_object_mut().ok_or_else(|| {
        AppError::ApiIncompatible("Workflow meta_config must decode to an object".into())
    })?;
    if let Some(marker) = object.get(WORKFLOW_RESERVED_KEY) {
        let marker_object = marker.as_object().ok_or_else(|| {
            AppError::Reconciliation("Workflow reserved identity marker is invalid".into())
        })?;
        let valid = marker_object.len() == 4
            && marker_object
                .get("version")
                .and_then(serde_json::Value::as_u64)
                == Some(2)
            && marker_object
                .get("creator_user_id")
                .and_then(serde_json::Value::as_str)
                .is_some()
            && marker_object
                .get("project")
                .and_then(serde_json::Value::as_str)
                == Some(project)
            && marker_object
                .get("slug")
                .and_then(serde_json::Value::as_str)
                == Some(slug);
        if !valid {
            return Err(AppError::Reconciliation(
                "Workflow reserved identity marker is invalid".into(),
            ));
        }
        object.remove(WORKFLOW_RESERVED_KEY);
    }
    Ok(decoded)
}

fn reverse_workflow_yaml_config(value: &serde_json::Value) -> Result<serde_json::Value, AppError> {
    let raw = value.as_str().ok_or_else(|| {
        AppError::ApiIncompatible("Workflow yaml_config must be a YAML string".into())
    })?;
    let decoded: serde_yaml::Value = serde_yaml::from_str(raw).map_err(SaveError::SnapshotYaml)?;
    if decoded.is_null() {
        return Ok(serde_json::Value::Null);
    }
    if !decoded.is_mapping() {
        return Err(AppError::ApiIncompatible(
            "Workflow yaml_config must decode to an object".into(),
        ));
    }
    let execution: serde_json::Value =
        serde_json::to_value(decoded).map_err(SaveError::SnapshotJson)?;
    reject_workflow_managed_ids(&execution)?;
    Ok(execution)
}

fn reject_workflow_managed_ids(value: &serde_json::Value) -> Result<(), AppError> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    if let Some(actors) = object
        .get("assistants")
        .and_then(serde_json::Value::as_array)
    {
        for actor in actors {
            let actor = actor.as_object().ok_or(AppError::EntityNotExportable)?;
            if actor.contains_key("assistant_id")
                || actor.contains_key("skill_ids")
                || actor.contains_key("datasource_ids")
            {
                return Err(AppError::EntityNotExportable);
            }
        }
    }
    Ok(())
}

pub(super) fn canonical_yaml(value: &serde_json::Value) -> Result<String, AppError> {
    let mut yaml = serde_yaml::to_string(value).map_err(SaveError::Serialization)?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml.replace("\r\n", "\n"))
}
