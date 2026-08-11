/// Workflow entity adapter — W-001.
///
/// Identity resolved from `meta_config["codemie.epam.com/gitops/workflow-identity"]`
/// = `{version:1, project, slug}`. Two-pass enumeration (pass 1: project-visible,
/// pass 2: `scope=marketplace`). Deduplicates across passes by server ID.
/// Optional `adopt_workflow_id` bypasses enumeration (explicit by-ID adoption).
use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value};
use crate::parse::ParsedDeclaration;
use crate::projection::{
    ExistingEntity, RequestBody, WorkflowReferenceMap, WritePlan, project_with_workflow_references,
};

use super::{ApplyAction, ApplyResult, assistant, datasource, skill};

const MAX_PAGES: u32 = 1_000;
const MAX_ITEMS: u32 = 100_000;

const IDENTITY_KEY: &str = "codemie.epam.com/gitops/workflow-identity";

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct WorkflowPage {
    data: Vec<WorkflowItem>,
    pagination: WorkflowPagination,
}

#[derive(Deserialize)]
struct WorkflowPagination {
    pages: u32,
}

#[derive(Deserialize, Clone)]
struct WorkflowItem {
    id: String,
    meta_config: Option<String>,
}

#[derive(Deserialize)]
struct WorkflowIdResponse {
    id: String,
}

// ---------------------------------------------------------------------------
// Public adapter entry point
// ---------------------------------------------------------------------------

pub async fn apply(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    decl: &ParsedDeclaration,
    project_name: &str,
    slug: &str,
    adopt_workflow_id: Option<&str>,
    repo_root: &Path,
    follow_symlinks: bool,
) -> Result<ApplyResult, AppError> {
    let reference_map = resolve_execution_references(client, base_url, decl).await?;

    let existing_entity = if let Some(adopt_id) = adopt_workflow_id {
        // Explicit by-ID adoption: fetch current meta_config for merge (W-001)
        let detail = fetch_detail(client, base_url, adopt_id).await?;
        Some(ExistingEntity {
            server_id: detail.id,
            meta_config: detail.meta_config,
        })
    } else {
        // Two-pass exhaustive enumeration
        let matches = enumerate_all(client, base_url, project_name, slug).await?;
        match matches.as_slice() {
            [] => None,
            [single] => Some(ExistingEntity {
                server_id: single.id.clone(),
                meta_config: single.meta_config.clone(),
            }),
            _ => {
                return Err(AppError::Reconciliation(format!(
                    "Workflow: {} matches for (project={project_name:?}, slug={slug:?}); \
                     use --adopt-workflow-id to select one",
                    matches.len()
                )));
            }
        }
    };

    let plan = project_with_workflow_references(
        decl,
        existing_entity.as_ref(),
        adopt_workflow_id,
        repo_root,
        follow_symlinks,
        Some(&reference_map),
    )?;

    match plan {
        WritePlan::Create {
            request: RequestBody::Json(body),
        } => {
            let resp: WorkflowIdResponse = client.post(base_url, "/v1/workflows", &body).await?;
            Ok(ApplyResult {
                action: ApplyAction::Created,
                server_id: resp.id,
            })
        }
        WritePlan::Update {
            server_id,
            request: RequestBody::Json(body),
        } => {
            let path = format!("/v1/workflows/{}", encode_query_value(&server_id));
            let resp: WorkflowIdResponse = client.put(base_url, &path, &body).await?;
            Ok(ApplyResult {
                action: ApplyAction::Updated,
                server_id: resp.id,
            })
        }
        _ => Err(AppError::Internal(
            "workflow: projection produced unexpected body variant".into(),
        )),
    }
}

/// Re-resolve a Workflow marker after a modifying request and require that it
/// identifies exactly the server route returned by that request (FR-034).
pub async fn verify_identity(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let matches = enumerate_all(client, base_url, project_name, slug).await?;
    match matches.as_slice() {
        [single] if single.id == expected_server_id => Ok(()),
        _ => Err(AppError::Reconciliation(
            "Workflow write may have committed but identity verification did not match exactly once"
                .into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// W-002 execution/reference projection
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct WorkflowReferenceKeys {
    assistants: BTreeSet<(String, String)>,
    skills: BTreeSet<(String, String)>,
    datasources: BTreeSet<(String, String)>,
}

/// Collect and resolve every distinct Workflow server-resource natural key.
/// Workflow-local actor/state IDs are deliberately not collected here.
async fn resolve_execution_references(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    decl: &ParsedDeclaration,
) -> Result<WorkflowReferenceMap, AppError> {
    let keys = collect_execution_reference_keys(decl)?;
    let mut resolved = WorkflowReferenceMap::default();

    for (project, slug) in keys.assistants {
        let server_id = assistant::resolve_reference(client, base_url, &project, &slug).await?;
        resolved.insert_assistant(project, slug, server_id);
    }
    for (project, name) in keys.skills {
        let server_id = skill::resolve_reference(client, base_url, &project, &name).await?;
        resolved.insert_skill(project, name, server_id);
    }
    for (project, repo_name) in keys.datasources {
        let server_id =
            datasource::resolve_reference(client, base_url, &project, &repo_name).await?;
        resolved.insert_datasource(project, repo_name, server_id);
    }

    Ok(resolved)
}

fn collect_execution_reference_keys(
    decl: &ParsedDeclaration,
) -> Result<WorkflowReferenceKeys, AppError> {
    let actors = decl
        .value
        .pointer("/spec/execution_config/assistants")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            AppError::Schema("workflow: execution_config.assistants must be an array".into())
        })?;

    let mut keys = WorkflowReferenceKeys::default();
    for actor in actors {
        let actor = actor.as_object().ok_or_else(|| {
            AppError::Schema("workflow: execution_config.assistants[] must be an object".into())
        })?;
        if let Some(reference) = actor.get("assistantRef") {
            keys.assistants
                .insert(reference_key(reference, "slug", "assistantRef")?);
            continue;
        }

        collect_reference_array(
            actor.get("skillRefs"),
            "name",
            "skillRefs",
            &mut keys.skills,
        )?;
        collect_reference_array(
            actor.get("datasourceRefs"),
            "repo_name",
            "datasourceRefs",
            &mut keys.datasources,
        )?;
    }
    Ok(keys)
}

fn collect_reference_array(
    value: Option<&serde_json::Value>,
    key_field: &str,
    field: &str,
    output: &mut BTreeSet<(String, String)>,
) -> Result<(), AppError> {
    let values = value.and_then(serde_json::Value::as_array).ok_or_else(|| {
        AppError::Schema(format!("workflow: inline actor {field} must be an array"))
    })?;
    for value in values {
        output.insert(reference_key(value, key_field, field)?);
    }
    Ok(())
}

fn reference_key(
    value: &serde_json::Value,
    key_field: &str,
    field: &str,
) -> Result<(String, String), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::Schema(format!("workflow: {field} must be an object")))?;
    let project = object
        .get("project")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Schema(format!("workflow: {field}.project is required")))?;
    let key = object
        .get(key_field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Schema(format!("workflow: {field}.{key_field} is required")))?;
    Ok((project.to_owned(), key.to_owned()))
}

// ---------------------------------------------------------------------------
// Two-pass enumerate with deduplication
// ---------------------------------------------------------------------------

async fn enumerate_all(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
) -> Result<Vec<WorkflowItem>, AppError> {
    let mut all_matches = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Pass 1: project-visible (no scope parameter)
    // Pass 2: globally published (scope=marketplace)
    for scope in [None, Some("marketplace")] {
        let pass = enumerate_pass(client, base_url, scope, project_name, slug).await?;
        for item in pass {
            if seen_ids.insert(item.id.clone()) {
                all_matches.push(item);
            }
        }
    }

    Ok(all_matches)
}

async fn enumerate_pass(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    scope: Option<&str>,
    project_name: &str,
    slug: &str,
) -> Result<Vec<WorkflowItem>, AppError> {
    let mut matches = Vec::new();
    let mut page = 1u32;
    let mut total_seen = 0u32;

    loop {
        let path = match scope {
            None => format!(
                "/v1/workflows?minimal_response=false&page={}&per_page=100",
                page
            ),
            Some(s) => format!(
                "/v1/workflows?minimal_response=false&page={}&per_page=100&scope={}",
                page,
                encode_query_value(s)
            ),
        };

        let resp: WorkflowPage = client.get(base_url, &path).await?;
        let total_pages = resp.pagination.pages.max(1);

        for item in resp.data {
            total_seen += 1;
            if total_seen > MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "workflow enumeration exceeded 100,000-item cap".into(),
                ));
            }
            if matches_identity(item.meta_config.as_deref(), project_name, slug) {
                matches.push(item);
            }
        }

        if page >= total_pages || page >= MAX_PAGES {
            break;
        }
        page += 1;
    }

    Ok(matches)
}

// ---------------------------------------------------------------------------
// Explicit adoption: fetch current state for meta_config merge
// ---------------------------------------------------------------------------

async fn fetch_detail(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    workflow_id: &str,
) -> Result<WorkflowItem, AppError> {
    let path = format!("/v1/workflows/id/{}", encode_query_value(workflow_id));
    client.get(base_url, &path).await
}

// ---------------------------------------------------------------------------
// Identity check: decode meta_config and verify reserved key (ADR-008)
// ---------------------------------------------------------------------------

fn matches_identity(meta_config: Option<&str>, project: &str, slug: &str) -> bool {
    let raw = match meta_config {
        Some(s) => s,
        None => return false,
    };
    let obj: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let identity = match obj.get(IDENTITY_KEY) {
        Some(v) => v,
        None => return false,
    };
    identity.get("version").and_then(|v| v.as_u64()) == Some(1)
        && identity.get("project").and_then(|v| v.as_str()) == Some(project)
        && identity.get("slug").and_then(|v| v.as_str()) == Some(slug)
}

// ---------------------------------------------------------------------------
// Tests (W-001)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;
    use crate::parse::{EntityKind, ParsedDeclaration};

    fn test_url(base: &str) -> ValidatedUrl {
        ValidatedUrl::try_from(base).expect("test URL must be valid")
    }

    fn test_client(base: &str) -> ApiClient {
        ApiClient::new(test_url(base), "test-token".into())
            .expect("ApiClient must construct in tests")
    }

    fn workflow_decl(project: &str, slug: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: json!({
                "metadata": { "project": project, "slug": slug },
                "spec": {
                    "name": "Test Workflow",
                    "description": "A test workflow",
                    "mode": "chat",
                    "shared": false,
                    "execution_config": {
                        "assistants": [],
                        "states": [],
                        "tools": []
                    }
                }
            }),
            source_path: PathBuf::from("test.yaml"),
        }
    }

    fn meta_config_for(project: &str, slug: &str) -> String {
        format!(r#"{{"{IDENTITY_KEY}":{{"version":1,"project":"{project}","slug":"{slug}"}}}}"#)
    }

    fn empty_page() -> &'static str {
        r#"{"data":[],"pagination":{"page":1,"per_page":100,"total":0,"pages":0}}"#
    }

    fn one_match_page(id: &str, project: &str, slug: &str) -> String {
        let mc = meta_config_for(project, slug);
        format!(
            r#"{{"data":[{{"id":"{id}","meta_config":{mc_json}}}],"pagination":{{"page":1,"per_page":100,"total":1,"pages":1}}}}"#,
            mc_json = serde_json::to_string(&mc).unwrap()
        )
    }

    fn two_match_page(project: &str, slug: &str) -> String {
        let mc = meta_config_for(project, slug);
        let mc_json = serde_json::to_string(&mc).unwrap();
        format!(
            r#"{{"data":[{{"id":"id-1","meta_config":{mc_json}}},{{"id":"id-2","meta_config":{mc_json}}}],"pagination":{{"page":1,"per_page":100,"total":2,"pages":1}}}}"#
        )
    }

    // -----------------------------------------------------------------------
    // W-001 AC: zero matches → POST/created
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_creates_when_not_found() {
        let mut server = mockito::Server::new_async().await;

        // Both passes return empty
        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=1&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;

        let _p2 = server
            .mock("GET", mockito::Matcher::Regex(
                r"^/v1/workflows\?minimal_response=false&page=1&per_page=100&scope=marketplace$".to_string()
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/v1/workflows")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"wf-new"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-slug",
            None,
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "wf-new");
        _p1.assert_async().await;
        _p2.assert_async().await;
        _create.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // W-001 AC: one match → PUT/updated
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_updates_when_found() {
        let mut server = mockito::Server::new_async().await;

        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=1&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(one_match_page("wf-exist", "my-project", "my-slug"))
            .create_async()
            .await;

        let _p2 = server
            .mock("GET", mockito::Matcher::Regex(
                r"^/v1/workflows\?minimal_response=false&page=1&per_page=100&scope=marketplace$".to_string()
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/workflows/wf-exist")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"wf-exist"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-slug",
            None,
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "wf-exist");
        _p1.assert_async().await;
        _p2.assert_async().await;
        _update.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // W-001 AC: multiple matches → Reconciliation error
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_reconciliation_on_multiple_matches() {
        let mut server = mockito::Server::new_async().await;

        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=1&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(two_match_page("my-project", "my-slug"))
            .create_async()
            .await;

        let _p2 = server
            .mock("GET", mockito::Matcher::Regex(
                r"^/v1/workflows\?minimal_response=false&page=1&per_page=100&scope=marketplace$".to_string()
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let err = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-slug",
            None,
            Path::new("."),
            false,
        )
        .await
        .expect_err("multiple matches must error");

        assert!(matches!(err, AppError::Reconciliation(_)));
        assert_eq!(err.exit_code(), 1);
    }

    // -----------------------------------------------------------------------
    // W-001 AC: adopt_workflow_id bypasses enumeration
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_adopt_id_skips_enumeration() {
        let mut server = mockito::Server::new_async().await;

        let _detail = server
            .mock("GET", "/v1/workflows/id/adopt-wf-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"id":"adopt-wf-id","meta_config":{}}}"#,
                serde_json::to_string(&meta_config_for("my-project", "my-slug")).unwrap()
            ))
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/workflows/adopt-wf-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"adopt-wf-id"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-slug",
            Some("adopt-wf-id"),
            Path::new("."),
            false,
        )
        .await
        .expect("adopt-id apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "adopt-wf-id");
        _detail.assert_async().await;
        _update.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // Identity match helpers
    // -----------------------------------------------------------------------

    #[test]
    fn matches_identity_correct() {
        let mc = meta_config_for("proj", "sl");
        assert!(matches_identity(Some(&mc), "proj", "sl"));
    }

    #[test]
    fn matches_identity_wrong_project() {
        let mc = meta_config_for("other", "sl");
        assert!(!matches_identity(Some(&mc), "proj", "sl"));
    }

    #[test]
    fn matches_identity_wrong_slug() {
        let mc = meta_config_for("proj", "other");
        assert!(!matches_identity(Some(&mc), "proj", "sl"));
    }

    #[test]
    fn matches_identity_missing_key() {
        assert!(!matches_identity(
            Some(r#"{"other": "value"}"#),
            "proj",
            "sl"
        ));
    }

    #[test]
    fn matches_identity_invalid_json() {
        assert!(!matches_identity(Some("not-json"), "proj", "sl"));
    }

    #[test]
    fn matches_identity_none() {
        assert!(!matches_identity(None, "proj", "sl"));
    }
}
