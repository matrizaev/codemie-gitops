/// Skill entity adapter — S-001.
///
/// Exhaustive `(project, name)` resolution via paginated `GET /v1/skills?filters=...`.
/// Zero matches → POST with one 409 re-resolution attempt.
/// One match  → unconditional PUT.
/// Many matches → `AppError::Reconciliation`.
use std::path::Path;

use serde::Deserialize;

use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value};
use crate::parse::ParsedDeclaration;
use crate::projection::{project, ExistingEntity, RequestBody, WritePlan};

use super::{ApplyAction, ApplyResult};

const MAX_PAGES: u32 = 1_000;
const MAX_ITEMS: u32 = 100_000;

#[derive(Deserialize)]
struct SkillPage {
    skills: Vec<SkillItem>,
    pages: u32,
}

#[derive(Deserialize, Clone)]
struct SkillItem {
    id: String,
    name: String,
    project: String,
}

#[derive(Deserialize)]
struct SkillIdResponse {
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
    skill_name: &str,
    repo_root: &Path,
    follow_symlinks: bool,
) -> Result<ApplyResult, AppError> {
    let matches = enumerate(client, base_url, project_name, skill_name).await?;
    match matches.as_slice() {
        [] => {
            create_with_reresolution(
                client, base_url, decl, project_name, skill_name, repo_root, follow_symlinks,
            )
            .await
        }
        [single] => {
            let existing = ExistingEntity {
                server_id: single.id.clone(),
                meta_config: None,
            };
            let plan = project(decl, Some(&existing), None, repo_root, follow_symlinks)?;
            dispatch_update(client, base_url, plan).await
        }
        _ => Err(AppError::Reconciliation(format!(
            "Skill: {} matches for (name={skill_name:?}, project={project_name:?}); \
             manual resolution required",
            matches.len()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Exhaustive enumerate — all pages, exact client-side filter
// ---------------------------------------------------------------------------

async fn enumerate(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    skill_name: &str,
) -> Result<Vec<SkillItem>, AppError> {
    let filter = serde_json::to_string(&serde_json::json!({
        "project": project_name,
        "scope": "project_with_marketplace",
        "search": skill_name,
    }))
    .map_err(|_| AppError::Internal("skill: failed to encode filter JSON".into()))?;

    let mut all_matches = Vec::new();
    let mut page = 1u32;
    let mut total_seen = 0u32;

    loop {
        let path = format!(
            "/v1/skills?filters={}&page={}&per_page=100",
            encode_query_value(&filter),
            page
        );
        let resp: SkillPage = client.get(base_url, &path).await?;
        let total_pages = resp.pages.max(1);

        for item in resp.skills {
            total_seen += 1;
            if total_seen > MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "skill enumeration exceeded 100,000-item cap".into(),
                ));
            }
            if item.name == skill_name && item.project == project_name {
                all_matches.push(item);
            }
        }

        if page >= total_pages || page >= MAX_PAGES {
            break;
        }
        page += 1;
    }

    Ok(all_matches)
}

// ---------------------------------------------------------------------------
// Create with one 409 re-resolution (S-001)
// ---------------------------------------------------------------------------

async fn create_with_reresolution(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    decl: &ParsedDeclaration,
    project_name: &str,
    skill_name: &str,
    repo_root: &Path,
    follow_symlinks: bool,
) -> Result<ApplyResult, AppError> {
    let plan = project(decl, None, None, repo_root, follow_symlinks)?;
    let body = match plan {
        WritePlan::Create { request: RequestBody::Json(b) } => b,
        _ => {
            return Err(AppError::Internal(
                "skill: projection produced unexpected create variant".into(),
            ))
        }
    };

    match client
        .post_or_conflict::<_, SkillIdResponse>(base_url, "/v1/skills", &body)
        .await?
    {
        Some(resp) => Ok(ApplyResult { action: ApplyAction::Created, server_id: resp.id }),
        None => {
            // 409: re-enumerate once; no second POST attempt (S-001)
            let second = enumerate(client, base_url, project_name, skill_name).await?;
            match second.as_slice() {
                [single] => {
                    let existing = ExistingEntity {
                        server_id: single.id.clone(),
                        meta_config: None,
                    };
                    let update_plan =
                        project(decl, Some(&existing), None, repo_root, follow_symlinks)?;
                    dispatch_update(client, base_url, update_plan).await
                }
                [] => Err(AppError::Reconciliation(
                    "skill: POST returned 409 but re-enumeration found no match; \
                     unstable server state"
                        .into(),
                )),
                _ => Err(AppError::Reconciliation(
                    "skill: POST returned 409 and re-enumeration found multiple matches; \
                     manual resolution required"
                        .into(),
                )),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Update dispatch
// ---------------------------------------------------------------------------

async fn dispatch_update(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    plan: WritePlan,
) -> Result<ApplyResult, AppError> {
    match plan {
        WritePlan::Update { server_id, request: RequestBody::Json(body) } => {
            let path = format!("/v1/skills/{}", encode_query_value(&server_id));
            let resp: SkillIdResponse = client.put(base_url, &path, &body).await?;
            Ok(ApplyResult { action: ApplyAction::Updated, server_id: resp.id })
        }
        _ => Err(AppError::Internal(
            "skill: projection produced unexpected update variant".into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Tests (S-001)
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

    fn skill_decl(project: &str, name: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Skill,
            value: json!({
                "metadata": { "project": project, "name": name },
                "spec": {
                    "description": "test description",
                    "visibility": "private",
                    "categories": [],
                    "toolkits": [],
                    "mcp_servers": [],
                    "companion_files": [],
                    "enabled_builtin_subagents": [],
                    "content": "# Test\nThis is a test skill."
                }
            }),
            source_path: PathBuf::from("test.yaml"),
        }
    }

    fn empty_page() -> &'static str {
        r#"{"skills":[],"page":1,"perPage":100,"total":0,"pages":0}"#
    }

    fn one_match_page(id: &str, name: &str, project: &str) -> String {
        format!(
            r#"{{"skills":[{{"id":"{id}","name":"{name}","project":"{project}"}}],"page":1,"perPage":100,"total":1,"pages":1}}"#
        )
    }

    fn two_match_page(project: &str, name: &str) -> String {
        format!(
            r#"{{"skills":[{{"id":"id-1","name":"{name}","project":"{project}"}},{{"id":"id-2","name":"{name}","project":"{project}"}}],"page":1,"perPage":100,"total":2,"pages":1}}"#
        )
    }

    #[tokio::test]
    async fn apply_creates_when_not_found() {
        let mut server = mockito::Server::new_async().await;

        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/v1/skills")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"new-uuid"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = skill_decl("my-project", "my-skill");

        let result =
            apply(&client, &url, &decl, "my-project", "my-skill", Path::new("."), false)
                .await
                .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "new-uuid");
        _enum.assert_async().await;
        _create.assert_async().await;
    }

    #[tokio::test]
    async fn apply_updates_when_found() {
        let mut server = mockito::Server::new_async().await;

        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(one_match_page("existing-id", "my-skill", "my-project"))
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/skills/existing-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"existing-id"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = skill_decl("my-project", "my-skill");

        let result =
            apply(&client, &url, &decl, "my-project", "my-skill", Path::new("."), false)
                .await
                .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "existing-id");
        _enum.assert_async().await;
        _update.assert_async().await;
    }

    #[tokio::test]
    async fn apply_reconciliation_on_multiple_matches() {
        let mut server = mockito::Server::new_async().await;

        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(two_match_page("my-project", "my-skill"))
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = skill_decl("my-project", "my-skill");

        let err =
            apply(&client, &url, &decl, "my-project", "my-skill", Path::new("."), false)
                .await
                .expect_err("multiple matches must error");

        assert!(matches!(err, AppError::Reconciliation(_)));
        assert_eq!(err.exit_code(), 1);
    }

    #[tokio::test]
    async fn apply_resolves_409_via_reenumeration() {
        let mut server = mockito::Server::new_async().await;

        // First enumerate: empty (races with concurrent create)
        let _enum1 = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/v1/skills")
            .with_status(409)
            .create_async()
            .await;

        // Second enumerate after 409: one match
        let _enum2 = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(one_match_page("conflict-id", "my-skill", "my-project"))
            .expect(1)
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/skills/conflict-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"conflict-id"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = skill_decl("my-project", "my-skill");

        let result =
            apply(&client, &url, &decl, "my-project", "my-skill", Path::new("."), false)
                .await
                .expect("409 re-resolution must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "conflict-id");
        _create.assert_async().await;
    }
}
