use std::path::{Component, Path, PathBuf};

use crate::adapters::workflow::{MarkerClassification, classify_marker};
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value};
use crate::output::Outcome;
use base64::Engine;

const SKILL_PAGE_SIZE: u32 = 100;
const SKILL_MAX_PAGES: u32 = 1_000;
const SKILL_MAX_ITEMS: u32 = 100_000;
const WORKFLOW_RESERVED_KEY: &str = "codemie.epam.com/gitops/workflow-identity";
const WORKFLOW_PAGE_SIZE: u32 = 100;
const WORKFLOW_MAX_PAGES: u32 = 1_000;
const WORKFLOW_MAX_ITEMS: u32 = 100_000;
const DATASOURCE_PAGE_SIZE: u32 = 100;
const DATASOURCE_MAX_PAGES: u32 = 1_000;
const DATASOURCE_MAX_ITEMS: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveKind {
    Assistant,
    Workflow,
    Skill,
    Datasource,
}

impl std::str::FromStr for SaveKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Assistant" => Ok(Self::Assistant),
            "Workflow" => Ok(Self::Workflow),
            "Skill" => Ok(Self::Skill),
            "Datasource" => Ok(Self::Datasource),
            _ => Err("kind must be Assistant, Workflow, Skill, or Datasource".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCommand {
    pub kind: SaveKind,
    pub project: Option<String>,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub repo_name: Option<String>,
    pub workflow_id: Option<String>,
    pub file: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub url: Option<String>,
    pub follow_symlinks: bool,
}

impl SaveCommand {
    pub fn validate(self) -> Result<Self, AppError> {
        let selector_count =
            self.slug.is_some() as u8 + self.name.is_some() as u8 + self.repo_name.is_some() as u8;
        let selector_is_valid = match self.kind {
            SaveKind::Assistant | SaveKind::Workflow => self.slug.is_some(),
            SaveKind::Skill => self.name.is_some(),
            SaveKind::Datasource => self.repo_name.is_some(),
        };
        if selector_count != 1 || !selector_is_valid {
            return Err(AppError::Usage("selector does not match save kind".into()));
        }
        if self.workflow_id.is_some() && self.kind != SaveKind::Workflow {
            return Err(AppError::Usage("--id is valid only for Workflow".into()));
        }
        if self.file.as_os_str().is_empty() {
            return Err(AppError::Usage("--file is required".into()));
        }
        Ok(self)
    }
}

pub async fn save(command: SaveCommand) -> Result<Outcome, AppError> {
    let repo_root = command
        .repo_root
        .clone()
        .ok_or_else(|| AppError::Configuration("repository root is required for save".into()))?;
    let url = command
        .url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("target URL is required for save".into()))?;
    let validated_url = crate::config::ValidatedUrl::try_from(url)?;
    let output_path = validate_output_path(&repo_root, &command.file)?;
    let sidecar_path = if command.kind == SaveKind::Skill {
        Some(validate_output_path(
            &repo_root,
            &skill_sidecar_path(&command.file)?,
        )?)
    } else {
        None
    };
    let client = ApiClient::new(
        validated_url.clone(),
        std::env::var("CODEMIE_TOKEN").unwrap_or_default(),
    )?;
    let project = command
        .project
        .as_deref()
        .ok_or_else(|| AppError::Configuration("project is required for save".into()))?;
    let response: serde_json::Value = if command.kind == SaveKind::Skill {
        read_skill_snapshot(
            &client,
            &validated_url,
            project,
            command
                .name
                .as_deref()
                .ok_or_else(|| AppError::Usage("name is required".into()))?,
        )
        .await?
    } else if command.kind == SaveKind::Datasource {
        read_datasource(
            &client,
            &validated_url,
            project,
            command
                .repo_name
                .as_deref()
                .ok_or_else(|| AppError::Usage("repo-name is required".into()))?,
        )
        .await?
    } else if command.kind == SaveKind::Workflow {
        read_workflow(
            &client,
            &validated_url,
            project,
            command
                .slug
                .as_deref()
                .ok_or_else(|| AppError::Usage("slug is required".into()))?,
            command.workflow_id.as_deref(),
        )
        .await?
    } else {
        let path = detail_path(&command, project)?;
        client.get(&validated_url, &path).await?
    };
    let (declaration, sidecar, adoption_required) = project_response(&command, project, response)?;
    let yaml = canonical_yaml(&declaration)?;
    let cancellation = crate::cancellation::CancellationToken::default();
    let overlay = crate::repository::OverlayRepositoryView::new(
        &repo_root,
        command.follow_symlinks,
        &output_path,
        yaml.as_bytes(),
        sidecar_path.as_deref(),
        sidecar.as_deref(),
    )?;
    crate::repository::validate_overlay(&overlay, Some(project), &cancellation)?;
    publish(
        &output_path,
        yaml.as_bytes(),
        sidecar_path.as_deref(),
        sidecar.as_deref(),
    )?;
    outcome(&command, project, adoption_required)
}

#[derive(serde::Deserialize)]
struct WorkflowListPage {
    data: Vec<serde_json::Value>,
    pagination: WorkflowPagination,
}

#[derive(serde::Deserialize)]
struct WorkflowPagination {
    page: u32,
    pages: u32,
    total: u32,
    per_page: u32,
}

async fn read_workflow(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    project: &str,
    slug: &str,
    workflow_id: Option<&str>,
) -> Result<serde_json::Value, AppError> {
    let visibility = crate::http::preflight_visibility(client, url, project).await?;
    let creator_user_id = visibility.authenticated_user_id();
    let mut exact_ids = Vec::new();
    let mut unmarked_ids = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for scope in [None, Some("marketplace")] {
        let mut page = 0;
        let mut fingerprint = None;
        let mut total_seen = 0;
        loop {
            let path = match scope {
                None => format!(
                    "/v1/workflows?minimal_response=false&page={page}&per_page={WORKFLOW_PAGE_SIZE}"
                ),
                Some(scope) => format!(
                    "/v1/workflows?minimal_response=false&page={page}&per_page={WORKFLOW_PAGE_SIZE}&scope={}",
                    encode_query_value(scope)
                ),
            };
            let response: WorkflowListPage = client.get(url, &path).await?;
            validate_workflow_pagination(page, &response.pagination, fingerprint)?;
            fingerprint = Some((
                response.pagination.pages,
                response.pagination.total,
                response.pagination.per_page,
            ));
            let total_pages = response.pagination.pages;
            for item in response.data {
                total_seen += 1;
                if total_seen > WORKFLOW_MAX_ITEMS {
                    return Err(AppError::ApiIncompatible(
                        "workflow enumeration exceeded its item limit".into(),
                    ));
                }
                let object = item.as_object().ok_or_else(|| {
                    AppError::ApiIncompatible("workflow list item is not an object".into())
                })?;
                let id = object
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| AppError::ApiIncompatible("workflow id is missing".into()))?;
                if !seen_ids.insert(id.to_owned()) {
                    continue;
                }
                let row_project = object
                    .get("project")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        AppError::ApiIncompatible("workflow project is missing".into())
                    })?;
                let row_creator = object
                    .get("created_by")
                    .and_then(|value| value.get("id").or_else(|| value.get("user_id")))
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        AppError::ApiIncompatible("workflow creator is missing".into())
                    })?;
                if row_project != project || row_creator != creator_user_id {
                    continue;
                }
                let marker = object.get("meta_config").and_then(|value| value.as_str());
                match classify_marker(marker, row_project, project, slug, creator_user_id) {
                    MarkerClassification::Exact => {
                        if workflow_is_not_exportable(object) {
                            return Err(AppError::EntityNotExportable);
                        }
                        exact_ids.push(id.to_owned());
                    }
                    MarkerClassification::Invalid => {
                        return Err(AppError::Reconciliation(
                            "workflow target project contains invalid identity metadata".into(),
                        ));
                    }
                    MarkerClassification::Unmarked => {
                        if object.get("name").and_then(serde_json::Value::as_str) == Some(slug) {
                            unmarked_ids.push(id.to_owned());
                        }
                    }
                    MarkerClassification::OtherValid => {}
                }
            }
            if total_pages == 0 || page + 1 >= total_pages {
                if total_seen != response.pagination.total {
                    return Err(AppError::Reconciliation(
                        "workflow enumeration ended before the advertised total".into(),
                    ));
                }
                break;
            }
            page += 1;
        }
    }

    if let Some(workflow_id) = workflow_id {
        if !exact_ids.is_empty() {
            return Err(AppError::Reconciliation(
                "workflow marker conflict requires natural selection".into(),
            ));
        }
        let detail: serde_json::Value = client
            .get(
                url,
                &format!("/v1/workflows/id/{}", encode_query_value(workflow_id)),
            )
            .await?;
        validate_workflow_detail(&detail, project, workflow_id)?;
        let object = detail.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("workflow response must be an object".into())
        })?;
        if workflow_is_not_exportable(object) {
            return Err(AppError::EntityNotExportable);
        }
        return Ok(detail);
    }

    match exact_ids.as_slice() {
        [id] => {
            client
                .get(url, &format!("/v1/workflows/id/{}", encode_query_value(id)))
                .await
        }
        [] if !unmarked_ids.is_empty() => Err(AppError::Reconciliation(
            "workflow has an unmarked display-name match; use --id".into(),
        )),
        [] => Err(AppError::EntityNotFound),
        _ => Err(AppError::Reconciliation(
            "workflow marker identity is ambiguous".into(),
        )),
    }
}

fn workflow_is_not_exportable(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    object.get("mode").and_then(serde_json::Value::as_str) == Some("Autonomous")
        || object
            .get("deprecated")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
}

fn validate_workflow_detail(
    detail: &serde_json::Value,
    project: &str,
    workflow_id: &str,
) -> Result<(), AppError> {
    let object = detail
        .as_object()
        .ok_or_else(|| AppError::ApiIncompatible("workflow response must be an object".into()))?;
    if object.get("id").and_then(serde_json::Value::as_str) != Some(workflow_id) {
        return Err(AppError::ApiIncompatible(
            "workflow detail id does not match --id".into(),
        ));
    }
    if object.get("project").and_then(serde_json::Value::as_str) != Some(project) {
        return Err(AppError::ApiIncompatible(
            "workflow detail project does not match selector".into(),
        ));
    }
    let marker = object
        .get("meta_config")
        .and_then(serde_json::Value::as_str);
    if !matches!(
        classify_marker(marker, project, project, "", ""),
        MarkerClassification::Unmarked
    ) {
        return Err(AppError::WorkflowAlreadyMarked);
    }
    Ok(())
}

fn validate_workflow_pagination(
    requested_page: u32,
    pagination: &WorkflowPagination,
    fingerprint: Option<(u32, u32, u32)>,
) -> Result<(), AppError> {
    if pagination.page != requested_page
        || pagination.per_page != WORKFLOW_PAGE_SIZE
        || pagination.pages > WORKFLOW_MAX_PAGES
        || (pagination.pages == 0) != (pagination.total == 0)
        || pagination.pages != pagination.total.div_ceil(pagination.per_page)
    {
        return Err(AppError::ApiIncompatible(
            "workflow pagination violates the save contract".into(),
        ));
    }
    if fingerprint.is_some_and(|expected| {
        expected != (pagination.pages, pagination.total, pagination.per_page)
    }) {
        return Err(AppError::Reconciliation(
            "workflow pagination changed during enumeration".into(),
        ));
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct DatasourceListPage {
    data: Vec<serde_json::Value>,
    pagination: DatasourcePagination,
}

#[derive(serde::Deserialize)]
struct DatasourcePagination {
    page: u32,
    per_page: u32,
    total: u32,
    pages: u32,
}

async fn read_datasource(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    project: &str,
    repo_name: &str,
) -> Result<serde_json::Value, AppError> {
    crate::http::preflight_visibility(client, url, project).await?;
    let filters = serde_json::to_string(&serde_json::json!({"project": project}))
        .map_err(|_| AppError::Internal("failed to encode Datasource filters".into()))?;
    let mut matches = Vec::new();
    let mut page = 0;
    let mut total_seen = 0;
    let mut fingerprint = None;
    let mut seen_ids = std::collections::HashSet::new();
    loop {
        let path = format!(
            "/v1/index?full_response=true&page={page}&per_page={DATASOURCE_PAGE_SIZE}&filters={}",
            encode_query_value(&filters)
        );
        let response: DatasourceListPage = client.get(url, &path).await?;
        if response.pagination.page != page
            || response.pagination.per_page != DATASOURCE_PAGE_SIZE
            || response.pagination.pages > DATASOURCE_MAX_PAGES
            || (response.pagination.pages == 0) != (response.pagination.total == 0)
            || response.pagination.pages
                != response
                    .pagination
                    .total
                    .div_ceil(response.pagination.per_page)
        {
            return Err(AppError::ApiIncompatible(
                "Datasource pagination violates the save contract".into(),
            ));
        }
        let current = (
            response.pagination.pages,
            response.pagination.total,
            response.pagination.per_page,
        );
        if fingerprint.is_some_and(|expected| expected != current) {
            return Err(AppError::Reconciliation(
                "Datasource pagination changed during enumeration".into(),
            ));
        }
        fingerprint = Some(current);
        for item in response.data {
            total_seen += 1;
            if total_seen > DATASOURCE_MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "Datasource enumeration exceeded its item limit".into(),
                ));
            }
            let object = item.as_object().ok_or_else(|| {
                AppError::ApiIncompatible("Datasource list item is not an object".into())
            })?;
            let id = object
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AppError::ApiIncompatible("Datasource list item is missing id".into())
                })?;
            if !seen_ids.insert(id.to_owned()) {
                return Err(AppError::Reconciliation(
                    "Datasource enumeration repeated an entity id".into(),
                ));
            }
            if object
                .get("project_name")
                .and_then(serde_json::Value::as_str)
                == Some(project)
                && object.get("repo_name").and_then(serde_json::Value::as_str) == Some(repo_name)
            {
                matches.push(id.to_owned());
            }
        }
        if response.pagination.pages == 0 || page + 1 >= response.pagination.pages {
            break;
        }
        page += 1;
    }
    if total_seen != fingerprint.map_or(0, |(_, total, _)| total) {
        return Err(AppError::Reconciliation(
            "Datasource enumeration ended before the advertised total".into(),
        ));
    }
    let id = select_datasource_id(&matches)?;
    client
        .get(url, &format!("/v1/index/{}", encode_query_value(id)))
        .await
}

fn select_datasource_id(matches: &[String]) -> Result<&str, AppError> {
    match matches {
        [id] => Ok(id),
        [] => Err(AppError::EntityNotFound),
        _ => Err(AppError::Reconciliation(
            "Datasource natural identity is ambiguous".into(),
        )),
    }
}

fn detail_path(command: &SaveCommand, project: &str) -> Result<String, AppError> {
    match command.kind {
        SaveKind::Assistant => Ok(format!(
            "/v1/assistants/slug/{}?project={}",
            encode_query_value(
                command
                    .slug
                    .as_deref()
                    .ok_or_else(|| AppError::Usage("slug is required".into()))?
            ),
            encode_query_value(project)
        )),
        SaveKind::Workflow => {
            if let Some(id) = command.workflow_id.as_deref() {
                Ok(format!("/v1/workflows/id/{}", encode_query_value(id)))
            } else {
                Ok(format!(
                    "/v1/workflows?project={}&page=0&per_page=100",
                    encode_query_value(project)
                ))
            }
        }
        SaveKind::Skill => Err(AppError::Internal(
            "Skill detail reads must use the snapshot reader".into(),
        )),
        SaveKind::Datasource => Ok(format!(
            "/v1/index/{}?project={}",
            encode_query_value(
                command
                    .repo_name
                    .as_deref()
                    .ok_or_else(|| AppError::Usage("repo-name is required".into()))?
            ),
            encode_query_value(project)
        )),
    }
}

#[derive(serde::Deserialize)]
struct SkillListPage {
    skills: Vec<serde_json::Value>,
    page: u32,
    #[serde(rename = "perPage")]
    per_page: u32,
    total: u32,
    pages: u32,
}

async fn read_skill_snapshot(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    project: &str,
    name: &str,
) -> Result<serde_json::Value, AppError> {
    crate::http::preflight_visibility(client, url, project).await?;
    let filters = serde_json::to_string(&serde_json::json!({
        "project": project,
        "scope": "project_with_marketplace",
        "search": name,
    }))
    .map_err(|_| AppError::Internal("failed to encode Skill filters".into()))?;
    let mut page = 0;
    let mut seen = std::collections::HashSet::new();
    let mut selected = Vec::new();
    let mut fingerprint = None;
    let mut total_seen = 0u32;

    loop {
        let path = format!(
            "/v1/skills?filters={}&page={page}&per_page={SKILL_PAGE_SIZE}",
            encode_query_value(&filters)
        );
        let response: SkillListPage = client.get(url, &path).await?;
        if response.page != page
            || response.per_page != SKILL_PAGE_SIZE
            || response.pages > SKILL_MAX_PAGES
            || (response.pages == 0) != (response.total == 0)
            || response.pages != response.total.div_ceil(response.per_page)
        {
            return Err(AppError::ApiIncompatible(
                "Skill pagination violates the save contract".into(),
            ));
        }
        let current = (response.pages, response.total, response.per_page);
        if fingerprint.is_some_and(|expected| expected != current) {
            return Err(AppError::Reconciliation(
                "Skill pagination changed during enumeration".into(),
            ));
        }
        fingerprint = Some(current);
        for item in response.skills {
            total_seen += 1;
            if total_seen > SKILL_MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "Skill enumeration exceeded its item limit".into(),
                ));
            }
            let object = item.as_object().ok_or_else(|| {
                AppError::ApiIncompatible("Skill list item is not an object".into())
            })?;
            let id = object
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| AppError::ApiIncompatible("Skill list item is missing id".into()))?;
            if !seen.insert(id.to_owned()) {
                return Err(AppError::Reconciliation(
                    "Skill enumeration repeated an entity id".into(),
                ));
            }
            if object.get("name").and_then(serde_json::Value::as_str) == Some(name)
                && object.get("project").and_then(serde_json::Value::as_str) == Some(project)
            {
                selected.push(id.to_owned());
            }
        }
        if response.pages == 0 || page + 1 >= response.pages {
            break;
        }
        page += 1;
    }
    if total_seen != fingerprint.map_or(0, |(_, total, _)| total) {
        return Err(AppError::Reconciliation(
            "Skill enumeration ended before the advertised total".into(),
        ));
    }
    if selected.len() != 1 {
        return Err(AppError::Reconciliation(
            "Skill natural identity is missing or ambiguous".into(),
        ));
    }

    let id = &selected[0];
    let detail_a = read_skill_detail(client, url, id).await?;
    let payload_a = read_companion_payloads(client, url, id, &detail_a).await?;
    let detail_b = read_skill_detail(client, url, id).await?;
    let payload_b = read_companion_payloads(client, url, id, &detail_b).await?;
    let detail_c = read_skill_detail(client, url, id).await?;
    if skill_fingerprint(&detail_a) != skill_fingerprint(&detail_b)
        || skill_fingerprint(&detail_a) != skill_fingerprint(&detail_c)
        || payload_a != payload_b
    {
        return Err(AppError::Reconciliation(
            "Skill snapshot was unstable during save".into(),
        ));
    }
    let mut result = detail_a;
    result["companion_files"] = serde_json::Value::Array(payload_a);
    Ok(result)
}

async fn read_skill_detail(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    id: &str,
) -> Result<serde_json::Value, AppError> {
    let path = format!("/v1/skills/{}", encode_query_value(id));
    let value: serde_json::Value = client.get(url, &path).await?;
    validate_skill_detail(&value)?;
    Ok(value)
}

fn validate_skill_detail(value: &serde_json::Value) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::ApiIncompatible("Skill detail response must be an object".into())
    })?;
    let required = [
        "id",
        "name",
        "project",
        "updatedDate",
        "description",
        "content",
        "visibility",
        "categories",
        "toolkits",
        "mcp_servers",
        "companion_files",
        "enabled_builtin_subagents",
    ];
    if required.iter().any(|field| !object.contains_key(*field)) {
        return Err(AppError::ApiIncompatible(
            "Skill detail is missing a contracted field".into(),
        ));
    }
    for (key, child) in object {
        if !required.contains(&key.as_str())
            && !matches!(
                key.as_str(),
                "display_name"
                    | "created_by"
                    | "createdDate"
                    | "assistants_count"
                    | "user_abilities"
                    | "unique_likes_count"
                    | "unique_dislikes_count"
            )
        {
            return Err(AppError::ApiIncompatible(
                "Skill detail contains an unknown field".into(),
            ));
        }
        reject_secret_keys(key, child)?;
    }
    if !object["id"].is_string()
        || !object["name"].is_string()
        || !object["project"].is_string()
        || !(object["updatedDate"].is_string() || object["updatedDate"].is_null())
        || !object["content"].is_string()
        || !object["companion_files"].is_array()
    {
        return Err(AppError::ApiIncompatible(
            "Skill detail has an invalid contracted field".into(),
        ));
    }
    reject_secret_values(&object["toolkits"])?;
    reject_secret_values(&object["mcp_servers"])?;
    Ok(())
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

fn reject_secret_values(value: &serde_json::Value) -> Result<(), AppError> {
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

fn skill_fingerprint(value: &serde_json::Value) -> serde_json::Value {
    let object = value.as_object().expect("validated Skill detail object");
    serde_json::json!({
        "id": object["id"], "name": object["name"], "project": object["project"],
        "updatedDate": object["updatedDate"], "description": object["description"],
        "content": object["content"], "visibility": object["visibility"],
        "categories": object["categories"], "toolkits": object["toolkits"],
        "mcp_servers": object["mcp_servers"], "companion_files": normalized_metadata(object["companion_files"].as_array().unwrap()),
        "enabled_builtin_subagents": object["enabled_builtin_subagents"]
    })
}

fn normalized_metadata(values: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut values = values.to_vec();
    values.sort_by_key(|value| {
        value
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    });
    values
}

async fn read_companion_payloads(
    client: &ApiClient,
    url: &crate::config::ValidatedUrl,
    id: &str,
    detail: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, AppError> {
    let metadata = detail["companion_files"].as_array().ok_or_else(|| {
        AppError::ApiIncompatible("Skill companion metadata is not an array".into())
    })?;
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in metadata {
        let object = item.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("Skill companion metadata is not an object".into())
        })?;
        let raw_path = object
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::ApiIncompatible("Skill companion path is missing".into()))?;
        let path = normalize_companion_path(raw_path)?;
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
        let payload: serde_json::Value = client.get(url, &query).await?;
        let payload_object = payload.as_object().ok_or_else(|| {
            AppError::ApiIncompatible("Skill companion payload is not an object".into())
        })?;
        if payload_object
            .get("path")
            .and_then(serde_json::Value::as_str)
            != Some(path.as_str())
        {
            return Err(AppError::ApiIncompatible(
                "Skill companion response path mismatch".into(),
            ));
        }
        let encoding = metadata["encoding"].as_str().ok_or_else(|| {
            AppError::ApiIncompatible("Skill companion encoding is missing".into())
        })?;
        let content = payload_object
            .get("content")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                AppError::ApiIncompatible("Skill companion content is missing".into())
            })?;
        let bytes = match encoding {
            "text" => content.as_bytes().to_vec(),
            "base64" => base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|_| {
                    AppError::ApiIncompatible("Skill companion base64 is invalid".into())
                })?,
            _ => {
                return Err(AppError::ApiIncompatible(
                    "Skill companion encoding is unsupported".into(),
                ));
            }
        };
        if metadata["mime_type"] != payload["mime_type"]
            || metadata["encoding"] != payload["encoding"]
            || metadata["size_bytes"].as_u64() != Some(bytes.len() as u64)
            || payload["size_bytes"].as_u64() != Some(bytes.len() as u64)
        {
            return Err(AppError::ApiIncompatible(
                "Skill companion metadata mismatch".into(),
            ));
        }
        let mut item = metadata;
        item["content"] = serde_json::Value::String(content.to_owned());
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

fn project_response(
    command: &SaveCommand,
    project: &str,
    response: serde_json::Value,
) -> Result<(serde_json::Value, Option<Vec<u8>>, bool), AppError> {
    if command.kind == SaveKind::Workflow {
        let mode = response.get("mode").and_then(serde_json::Value::as_str);
        let deprecated = response
            .get("deprecated")
            .and_then(serde_json::Value::as_bool);
        if mode == Some("Autonomous") || deprecated == Some(true) {
            return Err(AppError::EntityNotExportable);
        }
    }
    let adoption_required = command.kind == SaveKind::Workflow && command.workflow_id.is_some();
    let sidecar = if command.kind == SaveKind::Skill {
        response
            .get("content")
            .and_then(serde_json::Value::as_str)
            .map(|value| value.as_bytes().to_vec())
    } else {
        None
    };
    let declaration = reverse_project(command, project, &response)?;
    Ok((declaration, sidecar, adoption_required))
}

fn reverse_project(
    command: &SaveCommand,
    project: &str,
    response: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let source = response
        .as_object()
        .ok_or_else(|| AppError::ApiIncompatible("entity response must be an object".into()))?;
    type Projection<'a> = (
        &'a str,
        (&'a str, Option<&'a str>),
        &'a [&'a str],
        &'a [&'a str],
    );
    let (kind, identity, fields, forbidden): Projection<'_> = match command.kind {
        SaveKind::Assistant => (
            "Assistant",
            ("slug", command.slug.as_deref()),
            &[
                "name",
                "description",
                "system_prompt",
                "type",
                "context",
                "toolkits",
                "icon_url",
                "llm_model_type",
                "enable_image_generation",
                "image_generation_model",
                "conversation_starters",
                "shared",
                "is_global",
                "agent_mode",
                "plan_prompt",
                "temperature",
                "top_p",
                "tools_tokens_size_limit",
                "smart_tool_selection_enabled",
                "hedging_config",
                "interactive_features",
                "mcp_servers",
                "sub_assistants",
                "enabled_builtin_subagents",
                "skills",
                "categories",
                "prompt_variables",
                "custom_metadata",
                "guardrail_assignments",
            ],
            &[
                "assistant_ids",
                "skill_ids",
                "access_token",
                "mcp_connect_auth_token",
            ],
        ),
        SaveKind::Workflow => (
            "Workflow",
            ("slug", command.slug.as_deref()),
            &[
                "name",
                "description",
                "mode",
                "start_hint",
                "icon_url",
                "shared",
                "supervisor_prompt",
                "meta_config",
                "yaml_config",
                "guardrail_assignments",
            ],
            &[
                "access_token",
                "assistant_ids",
                "skill_ids",
                "datasource_ids",
            ],
        ),
        SaveKind::Skill => (
            "Skill",
            ("name", command.name.as_deref()),
            &[
                "description",
                "visibility",
                "categories",
                "toolkits",
                "mcp_servers",
                "companion_files",
                "enabled_builtin_subagents",
            ],
            &["access_token", "mcp_connect_auth_token"],
        ),
        SaveKind::Datasource => (
            "Datasource",
            ("repo_name", command.repo_name.as_deref()),
            &[],
            &[
                "access_token",
                "provider_fields",
                "sharepoint_access_token",
                "expires_at",
            ],
        ),
    };
    for field in forbidden {
        if source.get(*field).is_some_and(|value| {
            !value.is_null()
                && !matches!(value, serde_json::Value::Array(items) if items.is_empty())
        }) {
            return Err(AppError::EntityNotExportable);
        }
    }
    if let Some(value) = source.get("project").or_else(|| source.get("project_name"))
        && value.as_str() != Some(project)
    {
        return Err(AppError::ApiIncompatible(
            "response project does not match selector".into(),
        ));
    }
    let (identity_field, identity_value) = identity;
    let identity_value = identity_value.ok_or(AppError::EntityNotExportable)?;
    if command.kind != SaveKind::Workflow
        && source
            .get(identity_field)
            .and_then(serde_json::Value::as_str)
            != Some(identity_value)
    {
        return Err(AppError::EntityNotExportable);
    }
    let mut metadata = serde_json::Map::new();
    metadata.insert("project".into(), project.into());
    metadata.insert(identity_field.into(), identity_value.into());
    if command.kind == SaveKind::Datasource {
        let spec = reverse_datasource(source)?;
        return Ok(serde_json::json!({
            "apiVersion": "codemie.epam.com/v1alpha1",
            "kind": kind,
            "metadata": metadata,
            "spec": spec,
        }));
    }
    let mut spec = serde_json::Map::new();
    if command.kind == SaveKind::Assistant {
        let empty_references = serde_json::Value::Array(Vec::new());
        let assistant_ids = source.get("assistant_ids").unwrap_or(&empty_references);
        let skill_ids = source.get("skill_ids").unwrap_or(&empty_references);
        if !matches!(assistant_ids, serde_json::Value::Array(items) if items.is_empty())
            || !matches!(skill_ids, serde_json::Value::Array(items) if items.is_empty())
        {
            return Err(AppError::EntityNotExportable);
        }
        spec.insert("sub_assistants".into(), serde_json::json!([]));
        spec.insert("skills".into(), serde_json::json!([]));
    }
    for field in fields {
        if let Some(value) = source.get(*field) {
            if command.kind == SaveKind::Workflow {
                match *field {
                    "meta_config" => {
                        spec.insert(
                            "meta_config".into(),
                            reverse_workflow_meta(value, project, identity_value)?,
                        );
                    }
                    "yaml_config" => {
                        spec.insert(
                            "execution_config".into(),
                            reverse_workflow_yaml_config(value)?,
                        );
                    }
                    _ => {
                        spec.insert((*field).into(), value.clone());
                    }
                }
            } else {
                spec.insert((*field).into(), value.clone());
            }
        }
    }
    if command.kind == SaveKind::Skill && source.contains_key("content") {
        spec.insert(
            "contentFrom".into(),
            serde_json::Value::String(format!(
                "{}.md",
                command.file.file_stem().unwrap().to_string_lossy()
            )),
        );
    }
    Ok(serde_json::json!({
        "apiVersion": "codemie.epam.com/v1alpha1",
        "kind": kind,
        "metadata": metadata,
        "spec": spec,
    }))
}

fn reverse_datasource(
    source: &serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, AppError> {
    reject_datasource_secrets(source)?;
    let index_type = source
        .get("index_type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::ApiIncompatible("Datasource index_type is invalid".into()))?;
    let mut spec = serde_json::Map::new();
    let branch = match index_type {
        "code" | "summary" | "chunk-summary" => {
            let vcs_type = source
                .get("vcs_type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AppError::ApiIncompatible("Datasource vcs_type is invalid".into())
                })?;
            let branch = match vcs_type {
                "git" | "svn" => vcs_type,
                _ => {
                    return Err(AppError::ApiIncompatible(
                        "Datasource vcs_type is invalid".into(),
                    ));
                }
            };
            required_source(source, "description", &mut spec, "description")?;
            required_source(source, "link", &mut spec, "link")?;
            required_source(source, "branch", &mut spec, "branch")?;
            required_source(
                source,
                "project_space_visible",
                &mut spec,
                "projectSpaceVisible",
            )?;
            spec.insert("index_type".into(), branch.into());
            spec.insert("indexType".into(), index_type.into());
            copy_fields(
                source,
                &mut spec,
                &[
                    ("files_filter", "filesFilter"),
                    ("embeddings_model", "embeddingsModel"),
                    ("summarization_model", "summarizationModel"),
                    ("prompt", "prompt"),
                    ("docs_generation", "docsGeneration"),
                    ("setting_id", "setting_id"),
                    ("guardrail_assignments", "guardrail_assignments"),
                    ("cron_expression", "cron_expression"),
                    ("timezone", "timezone"),
                ],
            );
            branch
        }
        "knowledge_base_confluence" => {
            reverse_nested_branch(source, "confluence", "confluence", "cql", &mut spec)?;
            "confluence"
        }
        "knowledge_base_jira" => {
            reverse_nested_branch(source, "jira", "jira", "jql", &mut spec)?;
            "jira"
        }
        "knowledge_base_xray" => {
            reverse_nested_branch(source, "xray", "xray", "jql", &mut spec)?;
            "xray"
        }
        "knowledge_base_azure_devops_wiki" => {
            reverse_nested_branch(
                source,
                "azure_devops_wiki",
                "azure_devops_wiki",
                "wiki_query",
                &mut spec,
            )?;
            copy_nested_optional(source, "azure_devops_wiki", &mut spec, "wiki_name");
            "azure_devops_wiki"
        }
        "knowledge_base_azure_devops_work_item" => {
            reverse_nested_branch(
                source,
                "azure_devops_work_item",
                "azure_devops_work_item",
                "wiql_query",
                &mut spec,
            )?;
            "azure_devops_work_item"
        }
        "knowledge_base_sharepoint" => {
            let nested = required_object(source, "sharepoint")?;
            required_source(nested, "site_url", &mut spec, "site_url")?;
            required_source(nested, "auth_type", &mut spec, "auth_type")?;
            copy_fields(
                nested,
                &mut spec,
                &[
                    ("include_pages", "include_pages"),
                    ("include_documents", "include_documents"),
                    ("include_lists", "include_lists"),
                    ("max_file_size_mb", "max_file_size_mb"),
                    ("files_filter", "files_filter"),
                    ("oauth_client_id", "oauth_client_id"),
                    ("oauth_tenant_id", "oauth_tenant_id"),
                ],
            );
            "sharepoint"
        }
        "llm_routing_google" => {
            required_source(source, "description", &mut spec, "description")?;
            required_source(source, "google_doc_link", &mut spec, "googleDoc")?;
            required_source(source, "setting_id", &mut spec, "setting_id")?;
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
    copy_fields(
        source,
        &mut spec,
        &[
            ("setting_id", "setting_id"),
            ("guardrail_assignments", "guardrail_assignments"),
            ("cron_expression", "cron_expression"),
            ("timezone", "timezone"),
        ],
    );
    Ok(serde_json::Value::Object(spec))
}

fn reverse_nested_branch(
    source: &serde_json::Map<String, serde_json::Value>,
    nested_key: &str,
    _branch: &str,
    required_key: &str,
    spec: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    let nested = required_object(source, nested_key)?;
    required_source(source, "description", spec, "description")?;
    required_source(nested, required_key, spec, required_key)?;
    copy_fields(
        source,
        spec,
        &[
            ("project_space_visible", "project_space_visible"),
            ("setting_id", "setting_id"),
            ("guardrail_assignments", "guardrail_assignments"),
            ("cron_expression", "cron_expression"),
            ("timezone", "timezone"),
        ],
    );
    copy_fields(
        nested,
        spec,
        &[
            ("cql", "cql"),
            ("jql", "jql"),
            ("wiki_query", "wiki_query"),
            ("wiql_query", "wiql_query"),
            ("include_restricted_content", "include_restricted_content"),
            ("include_archived_content", "include_archived_content"),
            ("include_attachments", "include_attachments"),
            ("include_comments", "include_comments"),
            ("keep_markdown_format", "keep_markdown_format"),
            ("keep_newlines", "keep_newlines"),
            ("embedding_model", "embedding_model"),
            ("wiki_name", "wiki_name"),
        ],
    );
    Ok(())
}

fn required_object<'a>(
    source: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, AppError> {
    source
        .get(key)
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| AppError::ApiIncompatible(format!("Datasource {key} is missing")))
}

fn required_source(
    source: &serde_json::Map<String, serde_json::Value>,
    source_key: &str,
    target: &mut serde_json::Map<String, serde_json::Value>,
    target_key: &str,
) -> Result<(), AppError> {
    let value = source
        .get(source_key)
        .filter(|value| !value.is_null())
        .ok_or_else(|| AppError::ApiIncompatible(format!("Datasource {source_key} is missing")))?;
    target.insert(target_key.into(), value.clone());
    Ok(())
}

fn copy_nested_optional(
    source: &serde_json::Map<String, serde_json::Value>,
    nested_key: &str,
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) {
    if let Some(nested) = source
        .get(nested_key)
        .and_then(serde_json::Value::as_object)
        && let Some(value) = nested.get(key)
    {
        target.insert(key.into(), value.clone());
    }
}

fn copy_fields(
    source: &serde_json::Map<String, serde_json::Value>,
    target: &mut serde_json::Map<String, serde_json::Value>,
    fields: &[(&str, &str)],
) {
    for (source_key, target_key) in fields {
        if let Some(value) = source.get(*source_key) {
            target.insert((*target_key).into(), value.clone());
        }
    }
}

fn reject_datasource_secrets(
    source: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), AppError> {
    for key in ["access_token", "expires_at", "provider_fields", "bedrock"] {
        if let Some(value) = source.get(key) {
            if value.is_null() {
                continue;
            }
            reject_secret_values(value)?;
            return Err(AppError::EntityNotExportable);
        }
    }
    if let Some(sharepoint) = source.get("sharepoint") {
        reject_secret_values(sharepoint)?;
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
    let mut decoded: serde_json::Value = serde_json::from_str(raw)
        .map_err(|_| AppError::ApiIncompatible("Workflow meta_config is not valid JSON".into()))?;
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
    let decoded: serde_yaml::Value = serde_yaml::from_str(raw)
        .map_err(|_| AppError::ApiIncompatible("Workflow yaml_config is not valid YAML".into()))?;
    if decoded.is_null() {
        return Ok(serde_json::Value::Null);
    }
    if !decoded.is_mapping() {
        return Err(AppError::ApiIncompatible(
            "Workflow yaml_config must decode to an object".into(),
        ));
    }
    let execution: serde_json::Value = serde_json::to_value(decoded).map_err(|_| {
        AppError::ApiIncompatible("Workflow yaml_config contains unsupported YAML values".into())
    })?;
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

fn canonical_yaml(value: &serde_json::Value) -> Result<String, AppError> {
    let mut yaml = serde_yaml::to_string(value)
        .map_err(|_| AppError::Internal("failed to serialize declaration".into()))?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml.replace("\r\n", "\n"))
}

fn validate_output_path(repo_root: &Path, path: &Path) -> Result<PathBuf, AppError> {
    let relative = path.strip_prefix(repo_root).unwrap_or(path);
    if path.is_absolute() && path.strip_prefix(repo_root).is_err() {
        return Err(AppError::Configuration(
            "output path must be inside repository".into(),
        ));
    }
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(AppError::Configuration("output path is unsafe".into()));
    }
    let target = repo_root.join(relative);
    if target.exists() || std::fs::symlink_metadata(&target).is_ok() {
        return Err(AppError::EntityNotExportable);
    }
    if !target.parent().is_some_and(Path::is_dir) {
        return Err(AppError::EntityNotExportable);
    }
    Ok(target)
}

fn skill_sidecar_path(file: &Path) -> Result<PathBuf, AppError> {
    let stem = file
        .file_stem()
        .ok_or_else(|| AppError::Configuration("output filename is invalid".into()))?;
    Ok(file.with_file_name(format!("{}.md", stem.to_string_lossy())))
}

fn publish(
    yaml_path: &Path,
    yaml: &[u8],
    sidecar_path: Option<&Path>,
    sidecar: Option<&[u8]>,
) -> Result<(), AppError> {
    let staged_sidecar = sidecar_path
        .zip(sidecar)
        .map(|(path, bytes)| stage_one(path, bytes))
        .transpose()?;
    let staged_yaml = match stage_one(yaml_path, yaml) {
        Ok(staged) => staged,
        Err(error) => {
            if let Some((_, staging)) = staged_sidecar {
                let _ = std::fs::remove_file(staging);
            }
            return Err(error);
        }
    };
    if let Some((path, staging)) = staged_sidecar
        && let Err(error) = publish_staged(&staging, &path)
    {
        let _ = std::fs::remove_file(&staged_yaml.1);
        return Err(error);
    }
    publish_staged(&staged_yaml.1, &staged_yaml.0)
}

#[cfg(test)]
fn publish_one(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let staged = stage_one(path, bytes)?;
    publish_staged(&staged.1, &staged.0)
}

fn stage_one(path: &Path, bytes: &[u8]) -> Result<(PathBuf, PathBuf), AppError> {
    let parent = path.parent().ok_or(AppError::EntityNotExportable)?;
    if !parent.is_dir() {
        return Err(AppError::EntityNotExportable);
    }
    let staging = parent.join(format!(
        ".{}.{}.save",
        std::process::id(),
        unique_stage_id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .map_err(|_| AppError::EntityNotExportable)?;
    if std::io::Write::write_all(&mut file, bytes).is_err() || file.sync_all().is_err() {
        drop(file);
        let _ = std::fs::remove_file(&staging);
        return Err(AppError::EntityNotExportable);
    }
    Ok((path.to_path_buf(), staging))
}

fn unique_stage_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn publish_staged(staging: &Path, destination: &Path) -> Result<(), AppError> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        let source = std::ffi::CString::new(staging.as_os_str().as_bytes())
            .map_err(|_| AppError::EntityNotExportable)?;
        let target = std::ffi::CString::new(destination.as_os_str().as_bytes())
            .map_err(|_| AppError::EntityNotExportable)?;
        let result = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if result == 0 {
            return Ok(());
        }
        let _ = std::fs::remove_file(staging);
        Err(AppError::EntityNotExportable)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (staging, destination);
        Err(AppError::Configuration(
            "save publication is unsupported on this platform".into(),
        ))
    }
}

fn outcome(
    command: &SaveCommand,
    project: &str,
    adoption_required: bool,
) -> Result<Outcome, AppError> {
    Ok(match command.kind {
        SaveKind::Assistant => Outcome::new(
            crate::output::Action::Saved,
            "Assistant".into(),
            project.into(),
            command.slug.clone().unwrap(),
        ),
        SaveKind::Workflow => Outcome::saved_workflow(
            project.into(),
            command.slug.clone().unwrap(),
            adoption_required,
        ),
        SaveKind::Skill => Outcome::new_skill(
            crate::output::Action::Saved,
            project.into(),
            command.name.clone().unwrap(),
        ),
        SaveKind::Datasource => Outcome::new_datasource(
            crate::output::Action::Saved,
            project.into(),
            command.repo_name.clone().unwrap(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(kind: SaveKind) -> SaveCommand {
        SaveCommand {
            kind,
            project: Some("demo".into()),
            slug: None,
            name: None,
            repo_name: None,
            workflow_id: None,
            file: "entity.yaml".into(),
            repo_root: None,
            url: None,
            follow_symlinks: false,
        }
    }

    #[test]
    fn accepts_exact_kind_selectors() {
        let mut assistant = command(SaveKind::Assistant);
        assistant.slug = Some("assistant".into());
        assert!(assistant.validate().is_ok());

        let mut workflow = command(SaveKind::Workflow);
        workflow.slug = Some("workflow".into());
        workflow.workflow_id = Some("reviewed-id".into());
        assert!(workflow.validate().is_ok());

        let mut skill = command(SaveKind::Skill);
        skill.name = Some("skill".into());
        assert!(skill.validate().is_ok());

        let mut datasource = command(SaveKind::Datasource);
        datasource.repo_name = Some("repo".into());
        assert!(datasource.validate().is_ok());
    }

    #[test]
    fn rejects_cross_kind_and_multiple_selectors() {
        let mut wrong_kind = command(SaveKind::Assistant);
        wrong_kind.name = Some("wrong".into());
        assert!(wrong_kind.validate().is_err());

        let mut multiple = command(SaveKind::Assistant);
        multiple.slug = Some("assistant".into());
        multiple.name = Some("wrong".into());
        assert!(multiple.validate().is_err());
    }

    #[test]
    fn rejects_workflow_id_for_other_kinds() {
        let mut command = command(SaveKind::Skill);
        command.name = Some("skill".into());
        command.workflow_id = Some("server-id".into());
        assert!(command.validate().is_err());
    }

    #[test]
    fn rejects_autonomous_workflow_before_projection() {
        let mut command = command(SaveKind::Workflow);
        command.slug = Some("autonomous".into());
        let result = project_response(
            &command,
            "demo",
            serde_json::json!({"mode":"Autonomous","name":"autonomous"}),
        );
        assert!(matches!(result, Err(AppError::EntityNotExportable)));
    }

    fn workflow_list_item(
        id: &str,
        name: &str,
        meta_config: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "project": "demo",
            "name": name,
            "meta_config": meta_config,
            "created_by": {"id": "user-1"},
            "mode": "Guided",
            "deprecated": false
        })
    }

    fn workflow_detail(id: &str, slug: &str, meta_config: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "project": "demo",
            "slug": slug,
            "name": slug,
            "meta_config": meta_config,
            "yaml_config": "{}",
            "mode": "Guided"
        })
    }

    async fn mock_workflow_scan(
        server: &mut mockito::Server,
        project_items: serde_json::Value,
        marketplace_items: serde_json::Value,
    ) {
        let project_total = project_items.as_array().map_or(0, Vec::len);
        let marketplace_total = marketplace_items.as_array().map_or(0, Vec::len);
        server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "user_id": "user-1",
                    "projects": [{"name": "demo"}]
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/v1/workflows")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("minimal_response".into(), "false".into()),
                mockito::Matcher::UrlEncoded("page".into(), "0".into()),
                mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            ]))
            .with_status(200)
            .with_body(serde_json::json!({
                "data": project_items,
                "pagination": {"page": 0, "pages": u32::from(project_total > 0), "total": project_total, "per_page": 100}
            }).to_string())
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/v1/workflows")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("minimal_response".into(), "false".into()),
                mockito::Matcher::UrlEncoded("page".into(), "0".into()),
                mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
                mockito::Matcher::UrlEncoded("scope".into(), "marketplace".into()),
            ]))
            .with_status(200)
            .with_body(serde_json::json!({
                "data": marketplace_items,
                "pagination": {"page": 0, "pages": u32::from(marketplace_total > 0), "total": marketplace_total, "per_page": 100}
            }).to_string())
            .expect(1)
            .create_async()
            .await;
    }

    #[tokio::test]
    async fn workflow_natural_selection_uses_exact_reserved_marker() {
        let mut server = mockito::Server::new_async().await;
        let marker = serde_json::json!({"codemie.epam.com/gitops/workflow-identity": {
            "version": 2, "project": "demo", "creator_user_id": "user-1", "slug": "workflow"
        }})
        .to_string();
        mock_workflow_scan(
            &mut server,
            serde_json::json!([workflow_list_item(
                "wf-1",
                "workflow",
                serde_json::Value::String(marker.clone())
            )]),
            serde_json::json!([]),
        )
        .await;
        let detail = server
            .mock("GET", "/v1/workflows/id/wf-1")
            .with_status(200)
            .with_body(
                workflow_detail("wf-1", "workflow", serde_json::Value::String(marker)).to_string(),
            )
            .expect(1)
            .create_async()
            .await;
        let result = read_workflow(
            &ApiClient::new(
                crate::config::ValidatedUrl::try_from(server.url()).unwrap(),
                String::new(),
            )
            .unwrap(),
            &crate::config::ValidatedUrl::try_from(server.url()).unwrap(),
            "demo",
            "workflow",
            None,
        )
        .await
        .unwrap();
        assert_eq!(result["id"], "wf-1");
        detail.assert_async().await;
    }

    #[tokio::test]
    async fn workflow_id_selection_accepts_unmarked_detail() {
        let mut server = mockito::Server::new_async().await;
        mock_workflow_scan(&mut server, serde_json::json!([]), serde_json::json!([])).await;
        let detail = server
            .mock("GET", "/v1/workflows/id/wf-2")
            .with_status(200)
            .with_body(workflow_detail("wf-2", "workflow", serde_json::Value::Null).to_string())
            .expect(1)
            .create_async()
            .await;
        let url = crate::config::ValidatedUrl::try_from(server.url()).unwrap();
        let client = ApiClient::new(url.clone(), String::new()).unwrap();
        let result = read_workflow(&client, &url, "demo", "workflow", Some("wf-2"))
            .await
            .unwrap();
        assert_eq!(result["id"], "wf-2");
        detail.assert_async().await;
    }

    #[tokio::test]
    async fn workflow_marker_conflict_is_rejected_before_detail() {
        let mut server = mockito::Server::new_async().await;
        let marker = serde_json::json!({"codemie.epam.com/gitops/workflow-identity": {
            "version": 2, "project": "demo", "creator_user_id": "user-1", "slug": "workflow"
        }})
        .to_string();
        mock_workflow_scan(
            &mut server,
            serde_json::json!([workflow_list_item(
                "wf-1",
                "workflow",
                serde_json::Value::String(marker.clone())
            )]),
            serde_json::json!([workflow_list_item(
                "wf-2",
                "workflow",
                serde_json::Value::String(marker)
            )]),
        )
        .await;
        let url = crate::config::ValidatedUrl::try_from(server.url()).unwrap();
        let client = ApiClient::new(url.clone(), String::new()).unwrap();
        let result = read_workflow(&client, &url, "demo", "workflow", None).await;
        assert!(matches!(result, Err(AppError::Reconciliation(_))));
    }

    #[tokio::test]
    async fn autonomous_workflow_is_non_exportable_during_resolution() {
        let mut server = mockito::Server::new_async().await;
        let marker = serde_json::json!({"codemie.epam.com/gitops/workflow-identity": {
            "version": 2, "project": "demo", "creator_user_id": "user-1", "slug": "workflow"
        }})
        .to_string();
        let mut item = workflow_list_item("wf-3", "workflow", serde_json::Value::String(marker));
        item["mode"] = "Autonomous".into();
        mock_workflow_scan(
            &mut server,
            serde_json::json!([item]),
            serde_json::json!([]),
        )
        .await;
        let url = crate::config::ValidatedUrl::try_from(server.url()).unwrap();
        let client = ApiClient::new(url.clone(), String::new()).unwrap();
        let result = read_workflow(&client, &url, "demo", "workflow", None).await;
        assert!(matches!(result, Err(AppError::EntityNotExportable)));
    }

    #[tokio::test]
    async fn workflow_resolution_performs_exact_get_only_sequence() {
        let mut server = mockito::Server::new_async().await;
        mock_workflow_scan(&mut server, serde_json::json!([]), serde_json::json!([])).await;
        let detail = server
            .mock("GET", "/v1/workflows/id/wf-4")
            .with_status(200)
            .with_body(workflow_detail("wf-4", "workflow", serde_json::Value::Null).to_string())
            .expect(1)
            .create_async()
            .await;
        let post = server
            .mock("POST", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let url = crate::config::ValidatedUrl::try_from(server.url()).unwrap();
        let client = ApiClient::new(url.clone(), String::new()).unwrap();
        read_workflow(&client, &url, "demo", "workflow", Some("wf-4"))
            .await
            .unwrap();
        detail.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
    }

    #[test]
    fn reverse_projection_preserves_explicit_nulls() {
        let mut command = command(SaveKind::Assistant);
        command.slug = Some("assistant".into());
        let (declaration, _, _) = project_response(
            &command,
            "demo",
            serde_json::json!({
                "project": "demo",
                "slug": "assistant",
                "name": "Assistant",
                "description": null
            }),
        )
        .unwrap();
        assert!(declaration["spec"]["description"].is_null());
        assert!(declaration["spec"].get("id").is_none());
    }

    #[test]
    fn datasource_code_mapping_covers_all_strategies_and_vcs_branches() {
        for strategy in ["code", "summary", "chunk-summary"] {
            for vcs in ["git", "svn"] {
                let response = serde_json::json!({
                    "index_type": strategy,
                    "vcs_type": vcs,
                    "description": "Description",
                    "link": "https://example.test/repository",
                    "branch": "main",
                    "project_space_visible": true
                });
                let spec = reverse_datasource(response.as_object().unwrap()).unwrap();
                assert_eq!(spec["index_type"], vcs);
                assert_eq!(spec["indexType"], strategy);
            }
        }
    }

    #[test]
    fn datasource_duplicate_natural_identity_is_not_resolved_by_discriminators() {
        let ids = ["git-id".into(), "svn-id".into()];
        let result = select_datasource_id(&ids);
        assert!(matches!(result, Err(AppError::Reconciliation(_))));
    }

    #[test]
    fn datasource_unsupported_branches_are_non_exportable() {
        for index_type in ["knowledge_base_file", "provider", "bedrock"] {
            let response = serde_json::json!({"index_type": index_type});
            assert!(matches!(
                reverse_datasource(response.as_object().unwrap()),
                Err(AppError::EntityNotExportable)
            ));
        }
        let response = serde_json::json!({"index_type": "unknown"});
        assert!(matches!(
            reverse_datasource(response.as_object().unwrap()),
            Err(AppError::ApiIncompatible(_))
        ));
    }

    #[test]
    fn datasource_preserves_optional_nulls_and_rejects_nested_secrets() {
        let response = serde_json::json!({
            "index_type": "summary",
            "vcs_type": "git",
            "description": "Description",
            "link": "https://example.test/repository",
            "branch": "main",
            "project_space_visible": true,
            "files_filter": null,
            "prompt": null
        });
        let spec = reverse_datasource(response.as_object().unwrap()).unwrap();
        assert!(spec["filesFilter"].is_null());
        assert!(spec["prompt"].is_null());

        let response = serde_json::json!({
            "index_type": "knowledge_base_sharepoint",
            "description": "Description",
            "sharepoint": {
                "site_url": "https://example.test/site",
                "auth_type": "integration",
                "access_token": "masked"
            }
        });
        assert!(matches!(
            reverse_datasource(response.as_object().unwrap()),
            Err(AppError::EntityNotExportable)
        ));
    }

    #[test]
    fn workflow_yaml_config_decodes_at_root_and_preserves_local_ids() {
        let mut command = command(SaveKind::Workflow);
        command.slug = Some("workflow".into());
        let (declaration, _, _) = project_response(
            &command,
            "demo",
            serde_json::json!({
                "project": "demo",
                "slug": "workflow",
                "yaml_config": "assistants:\n  - id: actor-1\n    system_prompt: hello\nstates:\n  - id: state-1\n    assistant_id: actor-1\ntools: []\n"
            }),
        )
        .unwrap();

        assert_eq!(
            declaration["spec"]["execution_config"]["assistants"][0]["id"],
            "actor-1"
        );
        assert_eq!(
            declaration["spec"]["execution_config"]["states"][0]["assistant_id"],
            "actor-1"
        );
        assert!(
            declaration["spec"]["execution_config"]
                .get("execution_config")
                .is_none()
        );
    }

    #[test]
    fn workflow_meta_config_preserves_explicit_null() {
        let mut command = command(SaveKind::Workflow);
        command.slug = Some("workflow".into());
        let (declaration, _, _) = project_response(
            &command,
            "demo",
            serde_json::json!({
                "project": "demo",
                "slug": "workflow",
                "meta_config": null
            }),
        )
        .unwrap();

        assert!(declaration["spec"]["meta_config"].is_null());
    }

    #[test]
    fn workflow_reverse_projection_removes_reserved_marker_only() {
        let mut command = command(SaveKind::Workflow);
        command.slug = Some("workflow".into());
        let meta_config = serde_json::json!({
            WORKFLOW_RESERVED_KEY: {
                "version": 2,
                "creator_user_id": "user-1",
                "project": "demo",
                "slug": "workflow"
            },
            "owner_note": "keep me"
        })
        .to_string();
        let (declaration, _, _) = project_response(
            &command,
            "demo",
            serde_json::json!({
                "project": "demo",
                "slug": "workflow",
                "meta_config": meta_config
            }),
        )
        .unwrap();

        assert!(
            declaration["spec"]["meta_config"]
                .get(WORKFLOW_RESERVED_KEY)
                .is_none()
        );
        assert_eq!(declaration["spec"]["meta_config"]["owner_note"], "keep me");
    }

    #[test]
    fn workflow_reverse_projection_fails_closed_on_managed_ids() {
        let mut command = command(SaveKind::Workflow);
        command.slug = Some("workflow".into());
        let result = project_response(
            &command,
            "demo",
            serde_json::json!({
                "project": "demo",
                "slug": "workflow",
                "yaml_config": "assistants:\n  - id: actor-1\n    assistant_id: managed-assistant-id\n"
            }),
        );

        assert!(matches!(result, Err(AppError::EntityNotExportable)));
    }

    #[test]
    fn skill_yaml_failure_leaves_only_complete_orphan_sidecar() {
        let directory = tempfile::tempdir().unwrap();
        let yaml_path = directory.path().join("skill.yaml");
        let sidecar_path = directory.path().join("skill.md");
        std::fs::write(&yaml_path, b"pre-existing\n").unwrap();

        let result = publish(
            &yaml_path,
            b"yaml\n",
            Some(&sidecar_path),
            Some(b"skill content\n"),
        );

        assert!(result.is_err());
        assert_eq!(std::fs::read(&yaml_path).unwrap(), b"pre-existing\n");
        assert_eq!(std::fs::read(&sidecar_path).unwrap(), b"skill content\n");
    }

    #[tokio::test]
    async fn save_reads_only_and_does_not_modify_server() {
        let mut server = mockito::Server::new_async().await;
        let visibility = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "user_id": "user-1",
                    "projects": [{"name": "demo"}]
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;
        let list = server
            .mock("GET", "/v1/skills")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("page".into(), "0".into()),
                mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            ]))
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "skills": [{"id": "skill-1", "name": "demo-skill", "project": "demo"}],
                    "page": 0, "perPage": 100, "total": 1, "pages": 1
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;
        let _legacy_detail = server
            .mock("GET", "/v1/skills/demo-skill?project=demo")
            .with_status(404)
            .expect(0)
            .create_async()
            .await;
        let detail = server
            .mock("GET", "/v1/skills/skill-1")
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "id": "skill-1",
                    "project": "demo",
                    "name": "demo-skill",
                    "updatedDate": "2026-01-01T00:00:00Z",
                    "description": "A sufficiently descriptive skill response for testing.",
                    "visibility": "private",
                    "categories": [],
                    "toolkits": [],
                    "mcp_servers": [],
                    "companion_files": [],
                    "enabled_builtin_subagents": [],
                    "content": "This is sufficiently long skill content for the save test. It contains enough authored text to satisfy the closed declaration resource limit while remaining deterministic."
                })
                .to_string(),
            )
            .expect(3)
            .create_async()
            .await;
        let post = server
            .mock("POST", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let root = tempfile::tempdir().unwrap();
        let command = SaveCommand {
            kind: SaveKind::Skill,
            project: Some("demo".into()),
            slug: None,
            name: Some("demo-skill".into()),
            repo_name: None,
            workflow_id: None,
            file: "skill.yaml".into(),
            repo_root: Some(root.path().to_path_buf()),
            url: Some(server.url()),
            follow_symlinks: false,
        };

        save(command).await.unwrap();
        visibility.assert_async().await;
        list.assert_async().await;
        detail.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
    }

    #[test]
    fn publication_does_not_overwrite_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("entity.yaml");
        std::fs::write(&path, b"original\n").unwrap();
        let result = publish_one(&path, b"replacement\n");
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original\n");
    }

    #[test]
    fn validation_failure_does_not_create_output_files() {
        let root = tempfile::tempdir().unwrap();
        let yaml_path = root.path().join("generated.yaml");
        let sidecar_path = root.path().join("generated.md");
        let yaml = b"apiVersion: codemie.epam.com/v1alpha1
kind: Skill
metadata:
  project: demo
  name: generated-skill
spec:
  description: A sufficiently descriptive skill response for testing.
  visibility: private
  categories: []
  toolkits: []
  mcp_servers: []
  companion_files: []
  enabled_builtin_subagents: []
  contentFrom: generated.md
";
        let overlay = crate::repository::OverlayRepositoryView::new(
            root.path(),
            false,
            &yaml_path,
            yaml,
            Some(&sidecar_path),
            Some(b"short"),
        )
        .unwrap();

        assert!(
            crate::repository::validate_overlay(
                &overlay,
                Some("demo"),
                &crate::cancellation::CancellationToken::default(),
            )
            .is_err()
        );
        assert!(!yaml_path.exists());
        assert!(!sidecar_path.exists());
    }
}
