//! Contract-typed, read-only server snapshot acquisition.

use super::publication::SaveError;
use super::*;

/// A contract-bound server snapshot. The variant determines which reverse
/// mapper can consume the payload, so a save selector cannot be paired with a
/// response for another entity kind.
#[derive(Debug, Clone)]
pub(super) enum EntitySnapshot {
    Assistant(AssistantSnapshot),
    Workflow(WorkflowSnapshot),
    Skill(SkillSnapshot),
    Datasource(DatasourceSnapshot),
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
    let snapshot =
        crate::adapters::workflow::resolve_snapshot(client, url, project, slug, workflow_id)
            .await?;
    Ok(EntitySnapshot::Workflow(snapshot))
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
    let snapshot = client.get(&path).await?;
    Ok(EntitySnapshot::Assistant(snapshot))
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
    Ok(EntitySnapshot::Datasource(snapshot))
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
