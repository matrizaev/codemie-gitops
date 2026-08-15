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

fn insert_assistant_context(
    target: &mut serde_json::Map<String, serde_json::Value>,
    field: ResponseField<OpenResponseValue>,
    project: &str,
) -> Result<(), AppError> {
    let Some(value) = field.into_option() else {
        return Ok(());
    };
    let contexts = value.0.as_array().ok_or_else(|| {
        AppError::ApiIncompatible("Assistant context response must be an array".into())
    })?;
    let mut declaration_context = Vec::with_capacity(contexts.len());
    for context in contexts {
        let object = context.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("Assistant context entry must be an object".into())
        })?;
        let context_type = object
            .get("context_type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::ApiIncompatible("Assistant context type is missing".into()))?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::ApiIncompatible("Assistant context name is missing".into()))?;
        declaration_context.push(serde_json::json!({
            "context_type": context_type,
            "ref": {"project": project, "repo_name": name},
        }));
    }
    target.insert(
        "context".into(),
        serde_json::Value::Array(declaration_context),
    );
    Ok(())
}

fn insert_assistant_categories(
    target: &mut serde_json::Map<String, serde_json::Value>,
    field: ResponseField<OpenResponseValue>,
) -> Result<(), AppError> {
    let Some(value) = field.into_option() else {
        return Ok(());
    };
    let categories = value.0.as_array().ok_or_else(|| {
        AppError::ApiIncompatible("Assistant categories response must be an array".into())
    })?;
    let mut declaration_categories = Vec::with_capacity(categories.len());
    for category in categories {
        let id = category
            .as_object()
            .and_then(|object| object.get("id"))
            .and_then(serde_json::Value::as_str)
            .or_else(|| category.as_str())
            .ok_or_else(|| AppError::ApiIncompatible("Assistant category id is missing".into()))?;
        declaration_categories.push(serde_json::Value::String(id.into()));
    }
    target.insert(
        "categories".into(),
        serde_json::Value::Array(declaration_categories),
    );
    Ok(())
}

fn declaration_settings(value: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let object = value?.as_object()?;
    let id = object.get("id")?.as_str()?.to_owned();
    let mut selection = serde_json::Map::new();
    selection.insert("id".into(), id.into());
    if let Some(alias) = object.get("alias").and_then(serde_json::Value::as_str) {
        selection.insert("alias".into(), alias.into());
    }
    Some(serde_json::Value::Object(selection))
}

fn normalize_toolkits(value: &serde_json::Value) -> Result<serde_json::Value, AppError> {
    let toolkits = value
        .as_array()
        .ok_or_else(|| AppError::ApiIncompatible("toolkits response must be an array".into()))?;
    let mut normalized = Vec::with_capacity(toolkits.len());
    for toolkit in toolkits {
        let object = toolkit.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("toolkit response entry must be an object".into())
        })?;
        let name = object
            .get("toolkit")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::ApiIncompatible("toolkit name is missing".into()))?;
        let tools = object
            .get("tools")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| AppError::ApiIncompatible("toolkit tools are missing".into()))?;
        let mut normalized_tools = Vec::with_capacity(tools.len());
        for tool in tools {
            let tool_object = tool.as_object().ok_or_else(|| {
                AppError::ApiIncompatible("tool response entry must be an object".into())
            })?;
            let tool_name = tool_object
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| AppError::ApiIncompatible("tool name is missing".into()))?;
            let mut normalized_tool = serde_json::Map::new();
            normalized_tool.insert("name".into(), tool_name.into());
            normalized_tool.insert(
                "settings_config".into(),
                tool_object
                    .get("settings_config")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                    .into(),
            );
            for key in ["label", "description", "user_description"] {
                if let Some(value) = tool_object.get(key) {
                    normalized_tool.insert(key.into(), value.clone());
                }
            }
            if let Some(settings) = declaration_settings(tool_object.get("settings")) {
                normalized_tool.insert("settings".into(), settings);
            }
            normalized_tools.push(serde_json::Value::Object(normalized_tool));
        }
        let mut normalized_toolkit = serde_json::Map::new();
        normalized_toolkit.insert("toolkit".into(), name.into());
        normalized_toolkit.insert("tools".into(), normalized_tools.into());
        normalized_toolkit.insert(
            "label".into(),
            object
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .into(),
        );
        normalized_toolkit.insert(
            "settings_config".into(),
            object
                .get("settings_config")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
                .into(),
        );
        for key in ["is_external"] {
            if let Some(value) = object.get(key) {
                normalized_toolkit.insert(key.into(), value.clone());
            }
        }
        if let Some(settings) = declaration_settings(object.get("settings")) {
            normalized_toolkit.insert("settings".into(), settings);
        }
        normalized.push(serde_json::Value::Object(normalized_toolkit));
    }
    Ok(serde_json::Value::Array(normalized))
}

fn normalize_mcp_servers(value: &serde_json::Value) -> Result<serde_json::Value, AppError> {
    let servers = value
        .as_array()
        .ok_or_else(|| AppError::ApiIncompatible("mcp_servers response must be an array".into()))?;
    let mut normalized = Vec::with_capacity(servers.len());
    for server in servers {
        let object = server.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("MCP server response entry must be an object".into())
        })?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::ApiIncompatible("MCP server name is missing".into()))?;
        let mut normalized_server = serde_json::Map::new();
        normalized_server.insert("name".into(), name.into());
        for key in [
            "description",
            "mcp_config_id",
            "mcp_connect_url",
            "tools_tokens_size_limit",
            "command",
            "arguments",
            "integration_alias",
            "tools",
        ] {
            if let Some(value) = object.get(key) {
                normalized_server.insert(key.into(), value.clone());
            }
        }
        normalized_server.insert(
            "enabled".into(),
            object
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
                .into(),
        );
        normalized_server.insert(
            "use_custom_config".into(),
            object
                .get("use_custom_config")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
                .into(),
        );
        normalized_server.insert(
            "resolve_dynamic_values_in_arguments".into(),
            object
                .get("resolve_dynamic_values_in_arguments")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
                .into(),
        );
        if let Some(settings) = declaration_settings(object.get("settings")) {
            normalized_server.insert("settings".into(), settings);
        }
        normalized.push(serde_json::Value::Object(normalized_server));
    }
    Ok(serde_json::Value::Array(normalized))
}

fn insert_normalized_field(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    field: ResponseField<OpenResponseValue>,
    normalizer: fn(&serde_json::Value) -> Result<serde_json::Value, AppError>,
) -> Result<(), AppError> {
    if let Some(value) = field.into_option() {
        target.insert(key.into(), normalizer(&value.0)?);
    }
    Ok(())
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
    insert_assistant_context(&mut spec, snapshot.context, project)?;
    insert_normalized_field(&mut spec, "toolkits", snapshot.toolkits, normalize_toolkits)?;
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
    insert_normalized_field(
        &mut spec,
        "mcp_servers",
        snapshot.mcp_servers,
        normalize_mcp_servers,
    )?;
    insert_field(
        &mut spec,
        "enabled_builtin_subagents",
        snapshot.enabled_builtin_subagents,
    );
    insert_assistant_categories(&mut spec, snapshot.categories)?;
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
        normalize_toolkits(
            &serde_json::to_value(snapshot.detail.toolkits)
                .map_err(SaveError::JsonSerialization)?,
        )?,
    );
    spec.insert(
        "mcp_servers".into(),
        normalize_mcp_servers(
            &serde_json::to_value(snapshot.detail.mcp_servers)
                .map_err(SaveError::JsonSerialization)?,
        )?,
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
        "knowledge_base_file" => {
            spec.insert(
                "description".into(),
                required_value_field(&snapshot.description, "description")?,
            );
            insert_field(
                &mut spec,
                "project_space_visible",
                snapshot.project_space_visible.clone(),
            );
            insert_field(&mut spec, "csv_separator", snapshot.csv_separator.clone());
            insert_field(&mut spec, "csv_start_row", snapshot.csv_start_row.clone());
            insert_field(
                &mut spec,
                "csv_rows_per_document",
                snapshot.csv_rows_per_document.clone(),
            );
            insert_field(
                &mut spec,
                "embedding_model",
                snapshot.embedding_model.clone(),
            );
            let uploaded_files = datasource_uploaded_files(&snapshot.uploaded_files)?;
            let placeholder_paths = datasource_placeholder_paths(command, &uploaded_files)?;
            spec.insert("files".into(), serde_json::Value::Array(placeholder_paths));
            spec.insert(
                "uploaded_files".into(),
                serde_json::Value::Array(uploaded_files),
            );
            spec.insert(
                "include_email_attachments".into(),
                snapshot
                    .include_email_attachments
                    .into_option()
                    .map_or(serde_json::Value::Bool(true), |value| value.0),
            );
            "file"
        }
        "provider" | "bedrock" => {
            return Err(AppError::EntityNotExportable);
        }
        _ => {
            return Err(AppError::ApiIncompatible(
                "Datasource index_type is unsupported".into(),
            ));
        }
    };
    spec.insert("index_type".into(), branch.into());
    if branch != "file" {
        insert_field(&mut spec, "setting_id", snapshot.setting_id);
        insert_field(
            &mut spec,
            "guardrail_assignments",
            snapshot.guardrail_assignments,
        );
        insert_field(&mut spec, "cron_expression", snapshot.cron_expression);
        insert_field(&mut spec, "timezone", snapshot.timezone);
    }
    Ok(declaration(
        "Datasource",
        serde_json::json!({"project": project, "repo_name": repo_name}),
        serde_json::Value::Object(spec),
    ))
}

fn datasource_uploaded_files(
    field: &ResponseField<OpenResponseValue>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let Some(value) = field.as_ref() else {
        return Ok(Vec::new());
    };
    value
        .0
        .as_array()
        .ok_or_else(|| {
            AppError::ApiIncompatible("Datasource uploaded_files must be an array".into())
        })?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|name| !name.is_empty())
                .map(|name| serde_json::Value::String(name.to_owned()))
                .ok_or(AppError::EntityNotExportable)
        })
        .collect()
}

fn datasource_placeholder_paths(
    command: &SaveCommand,
    uploaded_files: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, AppError> {
    let yaml_name = command
        .file
        .as_path()
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or(AppError::EntityNotExportable)?;
    let directory = format!("{yaml_name}.files");
    let source_names: Vec<&str> = uploaded_files
        .iter()
        .filter_map(serde_json::Value::as_str)
        .take(10)
        .collect();
    let names = if source_names.is_empty() {
        vec!["replace-content.txt"]
    } else {
        source_names
    };
    let mut seen = std::collections::BTreeSet::new();
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let safe = std::path::Path::new(name)
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .filter(|base| *base == name && !base.is_empty() && *base != ".")
                .filter(|base| !base.chars().any(char::is_control));
            let mut base = safe
                .map(str::to_owned)
                .unwrap_or_else(|| format!("replace-content-{}.txt", index + 1));
            if !seen.insert(base.clone()) {
                base = format!("replace-content-{}.txt", index + 1);
                seen.insert(base.clone());
            }
            Ok(serde_json::Value::String(format!("{directory}/{base}")))
        })
        .collect()
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
    let mut execution: serde_json::Value =
        serde_json::to_value(decoded).map_err(SaveError::SnapshotJson)?;
    normalize_workflow_execution(&mut execution)?;
    reject_workflow_managed_ids(&execution)?;
    Ok(execution)
}

fn normalize_workflow_execution(execution: &mut serde_json::Value) -> Result<(), AppError> {
    let execution = execution.as_object_mut().ok_or_else(|| {
        AppError::ApiIncompatible("Workflow yaml_config must decode to an object".into())
    })?;
    execution.remove("meta_states");
    insert_default(execution, "type", serde_json::json!("generic"));
    insert_default(execution, "verbose", serde_json::json!(false));
    insert_default(
        execution,
        "max_iteration_key_output_limit",
        serde_json::json!(100),
    );
    insert_default(execution, "tools", serde_json::json!([]));
    normalize_retry_policy(
        execution
            .entry("retry_policy")
            .or_insert_with(default_retry_policy),
    )?;

    let actors = execution
        .get_mut("assistants")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| {
            AppError::ApiIncompatible("Workflow yaml_config assistants must be an array".into())
        })?;
    for actor in actors {
        let actor = actor.as_object_mut().ok_or(AppError::EntityNotExportable)?;
        let local_id = actor
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(AppError::EntityNotExportable)?
            .to_owned();
        if actor.get("name").is_none_or(serde_json::Value::is_null) {
            actor.insert("name".into(), local_id.into());
        }
        insert_default(actor, "model", serde_json::json!(""));
        insert_default(actor, "limit_tool_output_tokens", serde_json::json!(8000));
        insert_default(actor, "tools", serde_json::json!([]));
        insert_default(
            actor,
            "exclude_extra_context_tools",
            serde_json::json!(false),
        );
        insert_default(actor, "mcp_servers", serde_json::json!([]));
        if let Some(mcp_servers) = actor.get_mut("mcp_servers") {
            *mcp_servers = normalize_mcp_servers(mcp_servers)?;
        }
    }

    let custom_nodes = execution
        .get_mut("custom_nodes")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| {
            AppError::ApiIncompatible("Workflow yaml_config custom_nodes must be an array".into())
        })?;
    for node in custom_nodes {
        let node = node.as_object_mut().ok_or(AppError::EntityNotExportable)?;
        insert_default(node, "name", serde_json::json!(""));
        insert_default(node, "model", serde_json::json!(""));
        insert_default(node, "system_prompt", serde_json::json!(""));
    }

    let states = execution
        .get_mut("states")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| {
            AppError::ApiIncompatible("Workflow yaml_config states must be an array".into())
        })?;
    for state in states {
        let state = state.as_object_mut().ok_or(AppError::EntityNotExportable)?;
        insert_default(state, "task", serde_json::json!(""));
        insert_default(state, "finish_iteration", serde_json::json!(false));
        insert_default(state, "interrupt_before", serde_json::json!(false));
        insert_default(
            state,
            "resolve_dynamic_values_in_prompt",
            serde_json::json!(false),
        );
        insert_default(state, "result_as_human_message", serde_json::json!(false));
        normalize_retry_policy(
            state
                .entry("retry_policy")
                .or_insert_with(default_retry_policy),
        )?;
        let next = state
            .get_mut("next")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or_else(|| AppError::ApiIncompatible("Workflow state next is missing".into()))?;
        insert_default(next, "override_task", serde_json::json!(false));
        insert_default(next, "store_in_context", serde_json::json!(false));
        insert_default(next, "include_in_llm_history", serde_json::json!(false));
        insert_default(next, "clear_prior_messages", serde_json::json!(false));
        insert_default(next, "clear_context_store", serde_json::json!(false));
        insert_default(next, "include_in_iterator_context", serde_json::json!([]));
        insert_default(next, "append_to_context", serde_json::json!(false));
    }
    Ok(())
}

fn insert_default(
    object: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
    default: serde_json::Value,
) {
    if object.get(field).is_none_or(serde_json::Value::is_null) {
        object.insert(field.into(), default);
    }
}

fn default_retry_policy() -> serde_json::Value {
    serde_json::json!({
        "initial_interval": 1000,
        "backoff_factor": 2,
        "max_interval": 60000,
        "max_attempts": 3
    })
}

fn normalize_retry_policy(value: &mut serde_json::Value) -> Result<(), AppError> {
    let policy = value.as_object_mut().ok_or_else(|| {
        AppError::ApiIncompatible("Workflow retry_policy must be an object".into())
    })?;
    insert_default(policy, "initial_interval", serde_json::json!(1000));
    insert_default(policy, "backoff_factor", serde_json::json!(2));
    insert_default(policy, "max_interval", serde_json::json!(60000));
    insert_default(policy, "max_attempts", serde_json::json!(3));
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_sparse_workflow_execution_without_changing_graph_ids() {
        let mut execution = serde_json::json!({
            "messages_limit_before_summarization": 10,
            "tokens_limit_before_summarization": 1000,
            "enable_summarization_node": false,
            "recursion_limit": 10,
            "max_concurrency": 1,
            "assistants": [{
                "id": "actor-1",
                "assistantRef": {"project": "p", "slug": "assistant-a"}
            }],
            "custom_nodes": [],
            "states": [{
                "id": "state-1",
                "assistant_id": "actor-1",
                "next": {"state_id": "end"}
            }],
            "meta_states": [{"id": "ui-only"}]
        });

        normalize_workflow_execution(&mut execution).expect("execution should normalize");

        assert!(execution.get("meta_states").is_none());
        assert_eq!(execution["assistants"][0]["name"], "actor-1");
        assert_eq!(execution["assistants"][0]["tools"], serde_json::json!([]));
        assert_eq!(execution["states"][0]["assistant_id"], "actor-1");
        assert_eq!(execution["states"][0]["retry_policy"]["max_attempts"], 3);
        assert_eq!(execution["states"][0]["next"]["override_task"], false);
    }

    #[test]
    fn normalizes_api_toolkit_and_settings_shape() {
        let value = serde_json::json!([{
            "toolkit": "github",
            "tools": [{
                "name": "search",
                "label": "Search",
                "settings_config": true,
                "settings": {
                    "id": "setting-1",
                    "alias": "github-prod",
                    "credential_values": ["must not leak"]
                }
            }],
            "config_class": "internal",
            "settings": {"id": "toolkit-setting", "project_name": "p"}
        }]);

        let normalized = normalize_toolkits(&value).expect("toolkit should normalize");
        assert_eq!(
            normalized,
            serde_json::json!([{
                "toolkit": "github",
                "tools": [{
                    "name": "search",
                    "label": "Search",
                    "settings_config": true,
                    "settings": {"id": "setting-1", "alias": "github-prod"}
                }],
                "label": "",
                "settings_config": false,
                "settings": {"id": "toolkit-setting"}
            }])
        );
    }

    #[test]
    fn normalizes_api_mcp_server_shape_and_defaults_required_flag() {
        let value = serde_json::json!([{
            "name": "interviews",
            "enabled": true,
            "use_custom_config": false,
            "config": {"auth_token": "must not leak"},
            "settings": {"id": "setting-2", "credential_values": ["secret"]}
        }]);

        let normalized = normalize_mcp_servers(&value).expect("MCP server should normalize");
        assert_eq!(
            normalized,
            serde_json::json!([{
                "name": "interviews",
                "enabled": true,
                "use_custom_config": false,
                "resolve_dynamic_values_in_arguments": false,
                "settings": {"id": "setting-2"}
            }])
        );
    }

    #[test]
    fn normalizes_api_context_and_categories() {
        let mut spec = serde_json::Map::new();
        insert_assistant_context(
            &mut spec,
            ResponseField::Present(OpenResponseValue(serde_json::json!([{
                "context_type": "knowledge_base",
                "name": "egi-interviews"
            }]))),
            "employees-gaps-identification",
        )
        .expect("context should normalize");
        insert_assistant_categories(
            &mut spec,
            ResponseField::Present(OpenResponseValue(serde_json::json!([
                {"id": "data-analytics", "name": "Data Analytics"},
                "Talent Acquisition"
            ]))),
        )
        .expect("categories should normalize");

        assert_eq!(
            serde_json::Value::Object(spec),
            serde_json::json!({
                "context": [{
                    "context_type": "knowledge_base",
                    "ref": {
                        "project": "employees-gaps-identification",
                        "repo_name": "egi-interviews"
                    }
                }],
                "categories": ["data-analytics", "Talent Acquisition"]
            })
        );
    }
}
