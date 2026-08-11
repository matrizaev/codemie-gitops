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
use crate::http::{ApiClient, ExactProjectVisibility, encode_query_value, preflight_visibility};
use crate::parse::ParsedDeclaration;
use crate::projection::{ExistingEntity, project};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, ResolutionTarget, WriteAbilityEvidence,
    decode_write_response, prove_write,
};

const MAX_PAGES: u32 = 1_000;
const MAX_ITEMS: u32 = 100_000;

#[derive(Deserialize)]
struct SkillPage {
    skills: Vec<SkillItem>,
    page: u32,
    #[serde(rename = "perPage")]
    per_page: u32,
    total: u32,
    pages: u32,
}

#[derive(Deserialize, Clone)]
struct SkillItem {
    id: String,
    name: String,
    project: String,
    #[serde(rename = "created_by")]
    _created_by: RequiredNullableObject,
    user_abilities: Vec<String>,
}

#[derive(Clone)]
struct RequiredNullableObject(Option<serde_json::Map<String, serde_json::Value>>);

impl<'de> Deserialize<'de> for RequiredNullableObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::Null => Ok(Self(None)),
            serde_json::Value::Object(value) => Ok(Self(Some(value))),
            _ => Err(serde::de::Error::custom(
                "created_by must be an object or null",
            )),
        }
    }
}

#[derive(Deserialize)]
struct SkillIdResponse {
    id: String,
}

#[derive(Debug)]
struct ScanEvidence {
    _pages_requested: u32,
    _items_seen: u32,
    _advertised_total: u32,
}

struct Enumeration {
    matches: Vec<SkillItem>,
    evidence: ScanEvidence,
}

/// Concrete exhaustive Skill resolution evidence owned by `PreparedWrite`.
#[derive(Debug)]
pub(super) struct CompletedResolution {
    effective_project: String,
    _name: String,
    target: ResolutionTarget,
    _scan: ScanEvidence,
    _write_ability: Option<WriteAbilityEvidence>,
}

impl CompletedResolution {
    pub(super) fn effective_project(&self) -> &str {
        &self.effective_project
    }

    pub(super) fn target(&self) -> &ResolutionTarget {
        &self.target
    }
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
    let visibility = preflight_visibility(client, base_url, project_name).await?;
    let enumeration = enumerate(client, base_url, project_name, skill_name).await?;
    match enumeration.matches.as_slice() {
        [] => {
            create_with_reresolution(
                client,
                base_url,
                CreateRequest {
                    declaration: decl,
                    project_name,
                    skill_name,
                    repo_root,
                    follow_symlinks,
                    visibility,
                    initial_scan: enumeration.evidence,
                },
            )
            .await
        }
        [single] => {
            let write_ability = prove_write(&single.user_abilities, "Skill")?;
            let existing = ExistingEntity {
                server_id: single.id.clone(),
                meta_config: None,
            };
            let plan = project(decl, Some(&existing), None, repo_root, follow_symlinks)?;
            let resolution = CompletedResolution {
                effective_project: project_name.to_owned(),
                _name: skill_name.to_owned(),
                target: ResolutionTarget::Update {
                    server_id: single.id.clone(),
                },
                _scan: enumeration.evidence,
                _write_ability: Some(write_ability),
            };
            let prepared = PreparedWrite::skill(visibility, resolution, plan)?;
            dispatch_update(client, base_url, prepared).await
        }
        _ => Err(AppError::Reconciliation(format!(
            "Skill: {} matches for (name={skill_name:?}, project={project_name:?}); \
             manual resolution required",
            enumeration.matches.len()
        ))),
    }
}

/// Resolve a Skill natural reference without creating or updating it
/// (FR-031/DR-003/W-002).
pub async fn resolve_reference(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    skill_name: &str,
) -> Result<String, AppError> {
    let enumeration = enumerate(client, base_url, project_name, skill_name).await?;
    match enumeration.matches.as_slice() {
        [single] => Ok(single.id.clone()),
        [] => Err(AppError::Reconciliation(
            "referenced Skill is missing on the target server".into(),
        )),
        _ => Err(AppError::Reconciliation(
            "referenced Skill identity is ambiguous on the target server".into(),
        )),
    }
}

/// Post-write exact identity verification for the coordinator (FR-034/R-001).
pub async fn verify_identity(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    skill_name: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let actual = resolve_reference(client, base_url, project_name, skill_name).await?;
    if actual == expected_server_id {
        Ok(())
    } else {
        Err(AppError::Reconciliation(
            "Skill write may have committed but identity verification did not match".into(),
        ))
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
) -> Result<Enumeration, AppError> {
    let filter = serde_json::to_string(&serde_json::json!({
        "project": project_name,
        "scope": "project_with_marketplace",
        "search": skill_name,
    }))
    .map_err(|_| AppError::Internal("skill: failed to encode filter JSON".into()))?;

    let mut all_matches = Vec::new();
    let mut page = 0u32;
    let mut pages_requested = 0u32;
    let mut total_seen = 0u32;
    let mut fingerprint: Option<(u32, u32, u32)> = None;
    let mut seen_ids = std::collections::HashSet::new();

    loop {
        let path = format!(
            "/v1/skills?filters={}&page={}&per_page=100",
            encode_query_value(&filter),
            page
        );
        let resp: SkillPage = client.get(base_url, &path).await?;
        pages_requested += 1;
        validate_pagination(page, &resp, fingerprint)?;
        fingerprint = Some((resp.pages, resp.total, resp.per_page));
        let total_pages = resp.pages;

        for item in resp.skills {
            // `created_by` participates in the pinned creator-scoped identity
            // shape. Its value may be null, but the member itself is required.
            let _creator_is_present = item._created_by.0.is_some();
            total_seen += 1;
            if total_seen > MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "skill enumeration exceeded 100,000-item cap".into(),
                ));
            }
            if !seen_ids.insert(item.id.clone()) {
                return Err(AppError::Reconciliation(
                    "skill enumeration repeated an entity id".into(),
                ));
            }
            if item.name == skill_name && item.project == project_name {
                all_matches.push(item);
            }
        }

        if total_pages == 0 || page + 1 >= total_pages {
            break;
        }
        page += 1;
    }

    let expected_total = fingerprint.map_or(0, |(_, total, _)| total);
    if total_seen != expected_total {
        return Err(AppError::Reconciliation(
            "skill enumeration ended before the advertised total".into(),
        ));
    }

    Ok(Enumeration {
        matches: all_matches,
        evidence: ScanEvidence {
            _pages_requested: pages_requested,
            _items_seen: total_seen,
            _advertised_total: expected_total,
        },
    })
}

fn validate_pagination(
    requested_page: u32,
    response: &SkillPage,
    fingerprint: Option<(u32, u32, u32)>,
) -> Result<(), AppError> {
    if response.page != requested_page || response.per_page != 100 {
        return Err(AppError::ApiIncompatible(
            "skill pagination origin or page size changed".into(),
        ));
    }
    if response.pages > MAX_PAGES {
        return Err(AppError::ApiIncompatible(
            "skill enumeration exceeded 1,000-page cap".into(),
        ));
    }
    if (response.pages == 0) != (response.total == 0) {
        return Err(AppError::ApiIncompatible(
            "skill zero page count does not match zero total".into(),
        ));
    }
    if response.pages != response.total.div_ceil(response.per_page) {
        return Err(AppError::ApiIncompatible(
            "skill pagination total and page count disagree".into(),
        ));
    }
    if let Some(expected) = fingerprint
        && expected != (response.pages, response.total, response.per_page)
    {
        return Err(AppError::Reconciliation(
            "skill pagination changed during enumeration".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Create with one 409 re-resolution (S-001)
// ---------------------------------------------------------------------------

struct CreateRequest<'a> {
    declaration: &'a ParsedDeclaration,
    project_name: &'a str,
    skill_name: &'a str,
    repo_root: &'a Path,
    follow_symlinks: bool,
    visibility: ExactProjectVisibility,
    initial_scan: ScanEvidence,
}

async fn create_with_reresolution(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    request: CreateRequest<'_>,
) -> Result<ApplyResult, AppError> {
    let plan = project(
        request.declaration,
        None,
        None,
        request.repo_root,
        request.follow_symlinks,
    )?;
    let resolution = CompletedResolution {
        effective_project: request.project_name.to_owned(),
        _name: request.skill_name.to_owned(),
        target: ResolutionTarget::Create,
        _scan: request.initial_scan,
        _write_ability: None,
    };
    let prepared = PreparedWrite::skill(request.visibility.clone(), resolution, plan)?;
    match dispatch_create(client, base_url, prepared).await? {
        Some(resp) => Ok(ApplyResult {
            action: ApplyAction::Created,
            server_id: resp.id,
        }),
        None => {
            // 409: re-enumerate once; no second POST attempt (S-001)
            let second =
                enumerate(client, base_url, request.project_name, request.skill_name).await?;
            match second.matches.as_slice() {
                [single] => {
                    let write_ability = prove_write(&single.user_abilities, "Skill")?;
                    let existing = ExistingEntity {
                        server_id: single.id.clone(),
                        meta_config: None,
                    };
                    let update_plan = project(
                        request.declaration,
                        Some(&existing),
                        None,
                        request.repo_root,
                        request.follow_symlinks,
                    )?;
                    let resolution = CompletedResolution {
                        effective_project: request.project_name.to_owned(),
                        _name: request.skill_name.to_owned(),
                        target: ResolutionTarget::Update {
                            server_id: single.id.clone(),
                        },
                        _scan: second.evidence,
                        _write_ability: Some(write_ability),
                    };
                    let prepared =
                        PreparedWrite::skill(request.visibility, resolution, update_plan)?;
                    dispatch_update(client, base_url, prepared).await
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

async fn dispatch_create(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    prepared: PreparedWrite,
) -> Result<Option<SkillIdResponse>, AppError> {
    let response = client.dispatch_prepared(base_url, prepared).await?;
    decode_write_response(response)
}

// ---------------------------------------------------------------------------
// Update dispatch
// ---------------------------------------------------------------------------

async fn dispatch_update(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    prepared: PreparedWrite,
) -> Result<ApplyResult, AppError> {
    if !matches!(prepared.target(), ResolutionTarget::Update { .. }) {
        return Err(AppError::Internal(
            "skill update dispatcher requires update resolution evidence".into(),
        ));
    }
    let response = client.dispatch_prepared(base_url, prepared).await?;
    let response: SkillIdResponse = decode_write_response(response)?
        .ok_or_else(|| AppError::Internal("Skill update cannot return a conflict signal".into()))?;
    Ok(ApplyResult {
        action: ApplyAction::Updated,
        server_id: response.id,
    })
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

    fn user_ok_mock(
        server: &mut mockito::Server,
    ) -> impl std::future::Future<Output = mockito::Mock> + '_ {
        server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"my-project","is_project_admin":true}]}"#,
            )
            .create_async()
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
        r#"{"skills":[],"page":0,"perPage":100,"total":0,"pages":0}"#
    }

    fn one_match_page(id: &str, name: &str, project: &str) -> String {
        format!(
            r#"{{"skills":[{{"id":"{id}","name":"{name}","project":"{project}","created_by":{{"id":"creator"}},"user_abilities":["read","write"]}}],"page":0,"perPage":100,"total":1,"pages":1}}"#
        )
    }

    fn two_match_page(project: &str, name: &str) -> String {
        format!(
            r#"{{"skills":[{{"id":"id-1","name":"{name}","project":"{project}","created_by":null,"user_abilities":["write"]}},{{"id":"id-2","name":"{name}","project":"{project}","created_by":{{"id":"creator"}},"user_abilities":["write"]}}],"page":0,"perPage":100,"total":2,"pages":1}}"#
        )
    }

    #[tokio::test]
    async fn apply_creates_when_not_found() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

        let _enum = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/skills\?filters=.*&page=0&per_page=100$".to_owned()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;
        let no_page_one = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/skills\?filters=.*&page=1&per_page=100$".to_owned()),
            )
            .expect(0)
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

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "new-uuid");
        _enum.assert_async().await;
        no_page_one.assert_async().await;
        _create.assert_async().await;
    }

    #[tokio::test]
    async fn apply_updates_when_found() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

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

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "existing-id");
        _enum.assert_async().await;
        _update.assert_async().await;
    }

    #[tokio::test]
    async fn apply_exhausts_zero_based_multi_page_skill_scan() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let first_page_items: Vec<_> = (0..100)
            .map(|index| {
                json!({
                    "id": format!("other-{index}"),
                    "name": format!("other-{index}"),
                    "project": "my-project",
                    "created_by": null,
                    "user_abilities": ["read"]
                })
            })
            .collect();
        let page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/skills\?.*&page=0&per_page=100$".to_owned()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "skills": first_page_items,
                    "page": 0,
                    "perPage": 100,
                    "total": 101,
                    "pages": 2
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;
        let page_one = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/skills\?.*&page=1&per_page=100$".to_owned()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                one_match_page("target", "my-skill", "my-project")
                    .replace(r#""page":0,"#, r#""page":1,"#)
                    .replace(r#""total":1,"pages":1"#, r#""total":101,"pages":2"#),
            )
            .expect(1)
            .create_async()
            .await;
        let update = server
            .mock("PUT", "/v1/skills/target")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"target"}"#)
            .expect(1)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let result = apply(
            &client,
            &url,
            &skill_decl("my-project", "my-skill"),
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect("page-1 target must be resolved after page 0");

        assert_eq!(result.action, ApplyAction::Updated);
        page_zero.assert_async().await;
        page_one.assert_async().await;
        update.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_other_project_admin_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let user = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"other-project","is_project_admin":true}]}"#,
            )
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
        let patch = server
            .mock("PATCH", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let delete = server
            .mock("DELETE", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            &skill_decl("my-project", "my-skill"),
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect_err("another project's admin entry is insufficient");
        assert!(matches!(error, AppError::VisibilityUnproven(_)));
        user.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_nonwriting_row_before_put() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let page = one_match_page("existing-id", "my-skill", "my-project")
            .replace(r#"["read","write"]"#, r#"["read"]"#);
        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let post = server
            .mock("POST", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let patch = server
            .mock("PATCH", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let delete = server
            .mock("DELETE", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            &skill_decl("my-project", "my-skill"),
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect_err("non-writing Skill row must stop before PUT");
        assert!(matches!(error, AppError::Authorization(_)));
        post.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_pagination_drift_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let page = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"skills":[],"page":1,"perPage":100,"total":0,"pages":0}"#)
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
        let patch = server
            .mock("PATCH", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let delete = server
            .mock("DELETE", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            &skill_decl("my-project", "my-skill"),
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect_err("wrong returned page must fail compatibility");
        assert!(matches!(error, AppError::ApiIncompatible(_)));
        page.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_repeated_skill_ids_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let repeated = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_owned()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"skills":[{"id":"same","name":"other-a","project":"my-project","created_by":null,"user_abilities":["read"]},{"id":"same","name":"other-b","project":"my-project","created_by":null,"user_abilities":["read"]}],"page":0,"perPage":100,"total":2,"pages":1}"#,
            )
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
        let patch = server
            .mock("PATCH", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let delete = server
            .mock("DELETE", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            &skill_decl("my-project", "my-skill"),
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect_err("repeated IDs indicate snapshot instability");

        assert!(matches!(error, AppError::Reconciliation(_)));
        repeated.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[test]
    fn skill_response_requires_consumed_fields_and_allows_additions() {
        let baseline = json!({
            "skills": [{
                "id":"skill",
                "name":"name",
                "project":"project",
                "created_by":null,
                "user_abilities":["write"]
            }],
            "page":0,
            "perPage":100,
            "total":1,
            "pages":1
        });
        let additive = json!({
            "skills": [{
                "id":"skill",
                "name":"name",
                "project":"project",
                "created_by":null,
                "user_abilities":["write"],
                "future_entity":true
            }],
            "page":0,
            "perPage":100,
            "total":1,
            "pages":1,
            "future_top":true
        });
        let baseline_page =
            serde_json::from_value::<SkillPage>(baseline.clone()).expect("baseline must decode");
        let additive_page =
            serde_json::from_value::<SkillPage>(additive).expect("additions must decode");

        crate::adapters::assert_consumed_field_mutations::<SkillPage>(
            &baseline,
            &[
                "/skills",
                "/skills/0/id",
                "/skills/0/name",
                "/skills/0/project",
                "/skills/0/created_by",
                "/skills/0/user_abilities",
                "/page",
                "/perPage",
                "/total",
                "/pages",
            ],
        );

        let project_response = |page: SkillPage| {
            let existing = ExistingEntity {
                server_id: page.skills[0].id.clone(),
                meta_config: None,
            };
            project(
                &skill_decl("project", "name"),
                Some(&existing),
                None,
                Path::new("."),
                false,
            )
            .expect("decoded response must project")
        };
        let crate::projection::WritePlan::Update {
            request: crate::projection::RequestBody::Json(baseline_body),
            ..
        } = project_response(baseline_page)
        else {
            panic!("Skill projection must be JSON");
        };
        let crate::projection::WritePlan::Update {
            request: crate::projection::RequestBody::Json(additive_body),
            ..
        } = project_response(additive_page)
        else {
            panic!("Skill projection must be JSON");
        };
        assert_eq!(baseline_body, additive_body);
    }

    #[test]
    fn skill_pagination_invariant_matrix() {
        let valid_empty = SkillPage {
            skills: Vec::new(),
            page: 0,
            per_page: 100,
            total: 0,
            pages: 0,
        };
        assert!(validate_pagination(0, &valid_empty, None).is_ok());

        for invalid in [
            SkillPage {
                page: 1,
                ..empty_skill_page()
            },
            SkillPage {
                per_page: 99,
                ..empty_skill_page()
            },
            SkillPage {
                total: 1,
                pages: 0,
                ..empty_skill_page()
            },
            SkillPage {
                total: 101,
                pages: 1,
                ..empty_skill_page()
            },
            SkillPage {
                total: 100_001,
                pages: 1_001,
                ..empty_skill_page()
            },
        ] {
            assert!(matches!(
                validate_pagination(0, &invalid, None),
                Err(AppError::ApiIncompatible(_))
            ));
        }

        let stable_page = SkillPage {
            skills: Vec::new(),
            page: 1,
            per_page: 100,
            total: 101,
            pages: 2,
        };
        assert!(validate_pagination(1, &stable_page, Some((2, 101, 100))).is_ok());
        assert!(matches!(
            validate_pagination(1, &stable_page, Some((2, 100, 100))),
            Err(AppError::Reconciliation(_))
        ));
    }

    fn empty_skill_page() -> SkillPage {
        SkillPage {
            skills: Vec::new(),
            page: 0,
            per_page: 100,
            total: 0,
            pages: 0,
        }
    }

    #[tokio::test]
    async fn verify_identity_uses_page_zero_and_stops_after_one_page() {
        let mut server = mockito::Server::new_async().await;
        let page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/skills\?filters=.*&page=0&per_page=100$".to_owned()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(one_match_page("expected-skill", "my-skill", "my-project"))
            .expect(1)
            .create_async()
            .await;
        let page_one = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/skills\?filters=.*&page=1&per_page=100$".to_owned()),
            )
            .expect(0)
            .create_async()
            .await;

        verify_identity(
            &test_client(&server.url()),
            &test_url(&server.url()),
            "my-project",
            "my-skill",
            "expected-skill",
        )
        .await
        .expect("post-write Skill scanner must begin at page zero");

        page_zero.assert_async().await;
        page_one.assert_async().await;
    }

    #[tokio::test]
    async fn verify_identity_rejects_nonzero_origin_without_modification() {
        let mut server = mockito::Server::new_async().await;
        let page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/skills\?filters=.*&page=0&per_page=100$".to_owned()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"skills":[],"page":1,"perPage":100,"total":0,"pages":0}"#)
            .expect(1)
            .create_async()
            .await;
        let mut modifications = Vec::new();
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            modifications.push(
                server
                    .mock(method, mockito::Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await,
            );
        }

        let error = verify_identity(
            &test_client(&server.url()),
            &test_url(&server.url()),
            "my-project",
            "my-skill",
            "expected-skill",
        )
        .await
        .expect_err("invalid post-write Skill origin must fail closed");

        assert!(matches!(error, AppError::ApiIncompatible(_)));
        page_zero.assert_async().await;
        for modification in modifications {
            modification.assert_async().await;
        }
    }

    #[tokio::test]
    async fn apply_reconciliation_on_multiple_matches() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(two_match_page("my-project", "my-skill"))
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
        let patch = server
            .mock("PATCH", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let delete = server
            .mock("DELETE", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = skill_decl("my-project", "my-skill");

        let err = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect_err("multiple matches must error");

        assert!(matches!(err, AppError::Reconciliation(_)));
        assert_eq!(err.exit_code(), 1);
        post.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn apply_resolves_409_via_reenumeration() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

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

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect("409 re-resolution must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "conflict-id");
        _create.assert_async().await;
    }

    #[tokio::test]
    async fn apply_409_reresolution_rejects_nonzero_origin_without_second_write() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let initial = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_owned()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;
        let create = server
            .mock("POST", "/v1/skills")
            .with_status(409)
            .expect(1)
            .create_async()
            .await;
        let reresolution = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/skills\?".to_owned()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"skills":[],"page":1,"perPage":100,"total":0,"pages":0}"#)
            .expect(1)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let patch = server
            .mock("PATCH", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let delete = server
            .mock("DELETE", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let error = apply(
            &test_client(&server.url()),
            &test_url(&server.url()),
            &skill_decl("my-project", "my-skill"),
            "my-project",
            "my-skill",
            Path::new("."),
            false,
        )
        .await
        .expect_err("409 re-resolution must validate page-zero origin");

        assert!(matches!(error, AppError::ApiIncompatible(_)));
        initial.assert_async().await;
        create.assert_async().await;
        reresolution.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }
}
