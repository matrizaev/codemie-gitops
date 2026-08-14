//! Contract-typed, read-only server snapshot acquisition.

use super::publication::SaveError;
use super::*;

/// A contract-bound server snapshot. The variant determines which reverse
/// mapper can consume the payload, so a save selector cannot be paired with a
/// response for another entity kind.
#[derive(Debug, Clone)]
pub(super) enum EntitySnapshot {
    Assistant(Box<AssistantSnapshot>),
    Workflow(WorkflowSnapshot),
    Skill(SkillSnapshot),
    Datasource(Box<DatasourceSnapshot>),
}

/// Presence-preserving response field. Unlike `Option<T>`, this distinguishes
/// an omitted additive field from an explicitly authored JSON `null`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) enum ResponseField<T> {
    #[default]
    Missing,
    Present(T),
}

impl<T> ResponseField<T> {
    pub(super) fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Missing => None,
            Self::Present(value) => Some(value),
        }
    }

    pub(super) fn into_option(self) -> Option<T> {
        match self {
            Self::Missing => None,
            Self::Present(value) => Some(value),
        }
    }
}

impl<'de, T> serde::Deserialize<'de> for ResponseField<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::Present)
    }
}

/// Additive response properties not consumed by save policy. Keeping these in
/// a named infrastructure wrapper prevents open JSON from entering application
/// orchestration while preserving forward-compatible response decoding.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct ResponseExtensions(
    pub(super) std::collections::BTreeMap<String, OpenResponseValue>,
);

#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct AssistantSnapshot {
    #[serde(default)]
    pub(super) project: ResponseField<String>,
    #[serde(default)]
    pub(super) slug: ResponseField<String>,
    #[serde(default)]
    pub(super) assistant_ids: ResponseField<Vec<String>>,
    #[serde(default)]
    pub(super) skill_ids: ResponseField<Option<Vec<String>>>,
    #[serde(skip)]
    pub(super) sub_assistants: Vec<serde_json::Value>,
    #[serde(skip)]
    pub(super) skills: Vec<serde_json::Value>,
    #[serde(default)]
    pub(super) name: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) description: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) system_prompt: ResponseField<OpenResponseValue>,
    #[serde(default, rename = "type")]
    pub(super) type_: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) context: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) toolkits: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) icon_url: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) llm_model_type: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) enable_image_generation: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) image_generation_model: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) conversation_starters: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) shared: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) is_global: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) agent_mode: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) plan_prompt: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) temperature: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) top_p: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) tools_tokens_size_limit: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) smart_tool_selection_enabled: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) hedging_config: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) interactive_features: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) mcp_servers: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) enabled_builtin_subagents: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) categories: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) prompt_variables: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) custom_metadata: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) guardrail_assignments: ResponseField<OpenResponseValue>,
    #[serde(flatten)]
    pub(super) extensions: ResponseExtensions,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct WorkflowSnapshot {
    #[serde(default)]
    pub(super) id: ResponseField<String>,
    #[serde(default)]
    pub(super) project: ResponseField<String>,
    #[serde(default)]
    pub(super) slug: ResponseField<String>,
    #[serde(default)]
    pub(super) mode: ResponseField<String>,
    #[serde(default)]
    pub(super) deprecated: ResponseField<bool>,
    #[serde(default)]
    pub(super) name: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) description: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) start_hint: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) icon_url: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) shared: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) supervisor_prompt: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) meta_config: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) yaml_config: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) guardrail_assignments: ResponseField<OpenResponseValue>,
    #[serde(flatten)]
    pub(super) extensions: ResponseExtensions,
}

impl crate::adapters::workflow::WorkflowSnapshotContract for WorkflowSnapshot {
    fn id(&self) -> Option<&str> {
        self.id.as_ref().map(String::as_str)
    }

    fn project(&self) -> Option<&str> {
        self.project.as_ref().map(String::as_str)
    }

    fn meta_config(&self) -> Option<&str> {
        self.meta_config.as_ref().and_then(|value| value.0.as_str())
    }

    fn mode(&self) -> Option<&str> {
        self.mode.as_ref().map(String::as_str)
    }

    fn deprecated(&self) -> bool {
        self.deprecated.as_ref() == Some(&true)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct DatasourceSnapshot {
    #[serde(default)]
    pub(super) project_name: ResponseField<String>,
    #[serde(default)]
    pub(super) repo_name: ResponseField<String>,
    #[serde(default)]
    pub(super) index_type: ResponseField<String>,
    #[serde(default)]
    pub(super) vcs_type: ResponseField<String>,
    #[serde(default)]
    pub(super) description: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) link: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) branch: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) project_space_visible: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) files_filter: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) embeddings_model: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) embedding_model: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) uploaded_files: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) csv_separator: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) csv_start_row: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) csv_rows_per_document: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) include_email_attachments: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) summarization_model: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) prompt: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) docs_generation: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) setting_id: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) guardrail_assignments: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) cron_expression: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) timezone: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) google_doc_link: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) confluence: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) jira: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) xray: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) azure_devops_wiki: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) azure_devops_work_item: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) sharepoint: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) provider_fields: ResponseField<OpenResponseValue>,
    #[serde(default)]
    pub(super) bedrock: ResponseField<OpenResponseValue>,
    #[serde(flatten)]
    pub(super) extensions: ResponseExtensions,
}

pub(super) async fn read_workflow(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    project: &str,
    slug: &str,
    workflow_id: Option<&str>,
) -> Result<EntitySnapshot, AppError> {
    let mut snapshot =
        crate::adapters::workflow::resolve_snapshot(client, url, project, slug, workflow_id)
            .await?;
    reverse_workflow_managed_references(client, &mut snapshot).await?;
    Ok(EntitySnapshot::Workflow(snapshot))
}

async fn reverse_workflow_managed_references(
    client: &ApiClient,
    snapshot: &mut WorkflowSnapshot,
) -> Result<(), AppError> {
    let ResponseField::Present(yaml_config) = &mut snapshot.yaml_config else {
        return Ok(());
    };
    let raw = yaml_config.0.as_str().ok_or_else(|| {
        AppError::ApiIncompatible("Workflow yaml_config must be a YAML string".into())
    })?;
    let decoded: serde_yaml::Value = serde_yaml::from_str(raw).map_err(SaveError::SnapshotYaml)?;
    let mut execution: serde_json::Value =
        serde_json::to_value(decoded).map_err(SaveError::SnapshotJson)?;
    let execution_object = execution.as_object_mut().ok_or_else(|| {
        AppError::ApiIncompatible("Workflow yaml_config must decode to an object".into())
    })?;
    for field in [
        "type",
        "verbose",
        "max_iteration_key_output_limit",
        "tools",
        "retry_policy",
    ] {
        if !execution_object.contains_key(field)
            && let Some(value) = snapshot.extensions.0.get(field)
        {
            execution_object.insert(field.into(), value.0.clone());
        }
    }
    let actors = execution
        .get_mut("assistants")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| {
            AppError::ApiIncompatible("Workflow yaml_config assistants must be an array".into())
        })?;

    for actor in actors {
        let actor = actor.as_object_mut().ok_or(AppError::EntityNotExportable)?;
        if let Some(assistant_id) = actor.remove("assistant_id") {
            if actor.contains_key("skill_ids") || actor.contains_key("datasource_ids") {
                return Err(AppError::EntityNotExportable);
            }
            let assistant_id = required_reference_id(assistant_id)?;
            actor.insert(
                "assistantRef".into(),
                read_reference_by_id(
                    client,
                    &format!("/v1/assistants/id/{}", encode_query_value(&assistant_id)),
                    &assistant_id,
                    "project",
                    "slug",
                )
                .await?,
            );
        } else {
            let skill_ids = take_reference_ids(actor, "skill_ids")?;
            let datasource_ids = take_reference_ids(actor, "datasource_ids")?;
            let mut skill_refs = Vec::with_capacity(skill_ids.len());
            for id in skill_ids {
                skill_refs.push(
                    read_reference_by_id(
                        client,
                        &format!("/v1/skills/{}", encode_query_value(&id)),
                        &id,
                        "project",
                        "name",
                    )
                    .await?,
                );
            }
            let mut datasource_refs = Vec::with_capacity(datasource_ids.len());
            for id in datasource_ids {
                datasource_refs.push(
                    read_reference_by_id(
                        client,
                        &format!("/v1/index/{}", encode_query_value(&id)),
                        &id,
                        "project_name",
                        "repo_name",
                    )
                    .await?,
                );
            }
            actor.insert("skillRefs".into(), serde_json::Value::Array(skill_refs));
            actor.insert(
                "datasourceRefs".into(),
                serde_json::Value::Array(datasource_refs),
            );
        }
    }

    yaml_config.0 = serde_json::Value::String(
        serde_yaml::to_string(&execution).map_err(SaveError::Serialization)?,
    );
    Ok(())
}

fn take_reference_ids(
    actor: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<Vec<String>, AppError> {
    let Some(value) = actor.remove(field) else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or(AppError::EntityNotExportable)?
        .iter()
        .cloned()
        .map(required_reference_id)
        .collect()
}

fn required_reference_id(value: serde_json::Value) -> Result<String, AppError> {
    value
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or(AppError::EntityNotExportable)
}

async fn read_reference_by_id(
    client: &ApiClient,
    path: &str,
    expected_id: &str,
    project_field: &str,
    selector_field: &str,
) -> Result<serde_json::Value, AppError> {
    let value: serde_json::Value = client.get(path).await?;
    let object = value.as_object().ok_or_else(|| {
        AppError::ApiIncompatible("Workflow reference response must be an object".into())
    })?;
    if object.get("id").and_then(serde_json::Value::as_str) != Some(expected_id) {
        return Err(AppError::Reconciliation(
            "Workflow reference detail returned a conflicting id".into(),
        ));
    }
    let project = object
        .get(project_field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(AppError::EntityNotExportable)?;
    let selector = object
        .get(selector_field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(AppError::EntityNotExportable)?;
    Ok(serde_json::json!({"project": project, selector_field: selector}))
}

pub(super) async fn read_assistant(
    client: &ApiClient,
    project: &str,
    slug: &str,
) -> Result<EntitySnapshot, AppError> {
    let path = format!(
        "/v1/assistants/slug/{}?project={}",
        encode_query_value(slug),
        encode_query_value(project)
    );
    let mut snapshot: AssistantSnapshot = client.get(&path).await?;
    snapshot.sub_assistants =
        resolve_assistant_ids(client, &snapshot.assistant_ids, project).await?;
    snapshot.skills = resolve_skill_ids(client, &snapshot.skill_ids, project).await?;
    Ok(EntitySnapshot::Assistant(Box::new(snapshot)))
}

async fn resolve_assistant_ids(
    client: &ApiClient,
    ids: &ResponseField<Vec<String>>,
    project: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    let mut refs = Vec::new();
    for id in ids.as_ref().into_iter().flatten() {
        let value: serde_json::Value = client
            .get(&format!("/v1/assistants/id/{}", encode_query_value(id)))
            .await?;
        let object = value.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("Assistant reference response must be an object".into())
        })?;
        let target_project = object
            .get("project")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                AppError::ApiIncompatible("Assistant reference project is missing".into())
            })?;
        let slug = object
            .get("slug")
            .and_then(serde_json::Value::as_str)
            .ok_or(AppError::EntityNotExportable)?;
        if target_project != project {
            return Err(AppError::EntityNotExportable);
        }
        refs.push(serde_json::json!({"project": target_project, "slug": slug}));
    }
    Ok(refs)
}

async fn resolve_skill_ids(
    client: &ApiClient,
    ids: &ResponseField<Option<Vec<String>>>,
    project: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    let mut refs = Vec::new();
    for id in ids
        .as_ref()
        .and_then(|value| value.as_ref())
        .into_iter()
        .flatten()
    {
        let value: serde_json::Value = client
            .get(&format!("/v1/skills/{}", encode_query_value(id)))
            .await?;
        let object = value.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("Skill reference response must be an object".into())
        })?;
        let target_project = object
            .get("project")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                AppError::ApiIncompatible("Skill reference project is missing".into())
            })?;
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or(AppError::EntityNotExportable)?;
        if target_project != project {
            return Err(AppError::EntityNotExportable);
        }
        refs.push(serde_json::json!({"project": target_project, "name": name}));
    }
    Ok(refs)
}

pub(super) async fn read_datasource(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    project: &str,
    repo_name: &str,
) -> Result<EntitySnapshot, AppError> {
    let id =
        crate::adapters::datasource::resolve_reference(client, url, project, repo_name).await?;
    let snapshot = client
        .get(&format!("/v1/index/{}", encode_query_value(&id)))
        .await?;
    Ok(EntitySnapshot::Datasource(Box::new(snapshot)))
}

pub(super) async fn read_skill_snapshot(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    project: &str,
    name: &str,
) -> Result<EntitySnapshot, AppError> {
    let id = crate::adapters::skill::resolve_reference(client, url, project, name).await?;
    let detail_a = read_skill_detail(client, url, &id).await?;
    let payload_a = read_companion_payloads(client, url, &id, &detail_a).await?;
    let detail_b = read_skill_detail(client, url, &id).await?;
    let payload_b = read_companion_payloads(client, url, &id, &detail_b).await?;
    let detail_c = read_skill_detail(client, url, &id).await?;
    if skill_fingerprint(&detail_a) != skill_fingerprint(&detail_b)
        || skill_fingerprint(&detail_a) != skill_fingerprint(&detail_c)
        || payload_a != payload_b
    {
        return Err(AppError::Reconciliation(
            "Skill snapshot was unstable during save".into(),
        ));
    }
    Ok(EntitySnapshot::Skill(SkillSnapshot {
        detail: detail_a,
        companion_files: payload_a,
    }))
}

#[derive(Debug, Clone)]
pub(super) struct SkillSnapshot {
    pub(super) detail: SkillDetailDto,
    pub(super) companion_files: Vec<SkillCompanionMetadataDto>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillDetailDto {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) project: String,
    #[serde(rename = "updatedDate")]
    updated_date: Option<String>,
    pub(super) description: OpenResponseValue,
    pub(super) content: String,
    pub(super) visibility: OpenResponseValue,
    pub(super) categories: Vec<OpenResponseValue>,
    pub(super) toolkits: Vec<OpenResponseValue>,
    pub(super) mcp_servers: Vec<OpenResponseValue>,
    pub(super) companion_files: Vec<SkillCompanionMetadataDto>,
    pub(super) enabled_builtin_subagents: Vec<OpenResponseValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_by: Option<OpenResponseValue>,
    #[serde(
        rename = "createdDate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    created_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assistants_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_abilities: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unique_likes_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unique_dislikes_count: Option<u64>,
}

/// Explicitly open response extension retained only for fields whose server
/// schema intentionally permits heterogeneous JSON values.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(transparent)]
pub(super) struct OpenResponseValue(pub(super) serde_json::Value);

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillCompanionMetadataDto {
    pub(super) path: String,
    pub(super) mime_type: String,
    pub(super) encoding: String,
    pub(super) size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) content: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillCompanionPayloadDto {
    path: String,
    mime_type: String,
    encoding: String,
    size_bytes: u64,
    content: String,
}

async fn read_skill_detail(
    client: &ApiClient,
    _url: &crate::config::ValidatedUrl,
    id: &str,
) -> Result<SkillDetailDto, AppError> {
    let path = format!("/v1/skills/{}", encode_query_value(id));
    let value: SkillDetailDto = client.get(&path).await?;
    reject_secret_values(
        &serde_json::to_value(&value.toolkits).map_err(SaveError::JsonSerialization)?,
    )?;
    reject_secret_values(
        &serde_json::to_value(&value.mcp_servers).map_err(SaveError::JsonSerialization)?,
    )?;
    Ok(value)
}

fn reject_secret_keys(key: &str, value: &serde_json::Value) -> Result<(), AppError> {
    if value.is_null() {
        return Ok(());
    }
    let normalized = key.to_ascii_lowercase();
    if normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("credential")
        || normalized.contains("mask")
    {
        return Err(AppError::EntityNotExportable);
    }
    reject_secret_values(value)
}

pub(super) fn reject_secret_values(value: &serde_json::Value) -> Result<(), AppError> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                reject_secret_keys(key, value)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                reject_secret_values(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn skill_fingerprint(value: &SkillDetailDto) -> SkillDetailDto {
    let mut value = value.clone();
    value
        .companion_files
        .sort_by(|left, right| left.path.cmp(&right.path));
    value
}

async fn read_companion_payloads(
    client: &ApiClient,
    _url: &crate::config::ValidatedUrl,
    id: &str,
    detail: &SkillDetailDto,
) -> Result<Vec<SkillCompanionMetadataDto>, AppError> {
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in &detail.companion_files {
        let path = normalize_companion_path(&item.path)?;
        if !seen.insert(path.clone()) {
            return Err(AppError::EntityNotExportable);
        }
        paths.push((path, item.clone()));
    }
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    let mut result = Vec::new();
    for (path, metadata) in paths {
        let query = format!(
            "/v1/skills/{id}/companion-files/content?path={}",
            encode_query_value(&path)
        );
        let payload: SkillCompanionPayloadDto = client.get(&query).await?;
        if payload.path != path {
            return Err(AppError::ApiIncompatible(
                "Skill companion response path mismatch".into(),
            ));
        }
        let bytes = match metadata.encoding.as_str() {
            "text" => payload.content.as_bytes().to_vec(),
            "base64" => base64::engine::general_purpose::STANDARD
                .decode(&payload.content)
                .map_err(SaveError::SnapshotBase64)?,
            _ => {
                return Err(AppError::ApiIncompatible(
                    "Skill companion encoding is unsupported".into(),
                ));
            }
        };
        if metadata.mime_type != payload.mime_type
            || metadata.encoding != payload.encoding
            || metadata.size_bytes != bytes.len() as u64
            || payload.size_bytes != bytes.len() as u64
        {
            return Err(AppError::ApiIncompatible(
                "Skill companion metadata mismatch".into(),
            ));
        }
        let mut item = metadata;
        item.content = Some(payload.content);
        result.push(item);
    }
    Ok(result)
}

fn normalize_companion_path(path: &str) -> Result<String, AppError> {
    if path.is_empty()
        || path == "SKILL.md"
        || path.starts_with('/')
        || path.contains('\0')
        || path.chars().any(|character| character.is_control())
    {
        return Err(AppError::EntityNotExportable);
    }
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => return Err(AppError::EntityNotExportable),
            ".." => {
                if parts.pop().is_none() {
                    return Err(AppError::EntityNotExportable);
                }
            }
            part => parts.push(part),
        }
    }
    let normalized = parts.join("/");
    if normalized.is_empty() || normalized == "SKILL.md" {
        return Err(AppError::EntityNotExportable);
    }
    Ok(normalized)
}
