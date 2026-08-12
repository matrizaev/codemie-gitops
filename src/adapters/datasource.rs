/// Datasource entity adapter — D-001.
///
/// Visibility preflight (`GET /v1/user`) required before any write (ADR-012 Option A).
/// Exhaustive `(project, repo_name, index_type)` resolution via paginated
/// `GET /v1/index?full_response=true&page={page}&per_page=100&filters={json}`.
/// Pagination is zero-indexed. Cap: 1,000 pages / 100,000 items → `E_API_INCOMPATIBLE`.
/// File Datasource: multipart transport with parts cap (10) and basename safety.
use std::path::Path;

use serde::Deserialize;

use crate::cancellation::CancellationToken;
use crate::config::ValidatedUrl;
use crate::discovery::{MAX_SIDECAR_FILE_BYTES, load_sidecar_file_cancellable};
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value, preflight_visibility};
use crate::parse::ParsedDeclaration;
use crate::projection::{ExistingEntity, RequestBody, project};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, PreparedWriteResponse, ResolutionTarget,
    WriteAbilityEvidence, prove_write,
};

const MAX_PAGES: u32 = 1_000;
const MAX_ITEMS: u32 = 100_000;
const MAX_FILE_PARTS: usize = 10;

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DatasourcePage {
    data: Vec<DatasourceItem>,
    pagination: DatasourcePagination,
}

#[derive(Deserialize)]
struct DatasourcePagination {
    page: u32,
    per_page: u32,
    total: u32,
    pages: u32,
}

#[derive(Deserialize, Clone)]
struct DatasourceItem {
    id: String,
    repo_name: String,
    project_name: String,
    index_type: String,
    user_abilities: Vec<String>,
}

#[derive(Debug)]
struct ScanEvidence {
    _pages_requested: u32,
    _items_seen: u32,
    _advertised_total: u32,
}

struct ProjectEnumeration {
    items: Vec<DatasourceItem>,
    evidence: ScanEvidence,
}

struct Enumeration {
    matches: Vec<DatasourceItem>,
    evidence: ScanEvidence,
}

/// Concrete exhaustive Datasource resolution evidence owned by `PreparedWrite`.
#[derive(Debug)]
pub(super) struct CompletedResolution {
    effective_project: String,
    repo_name: String,
    index_type: String,
    target: ResolutionTarget,
    _scan: ScanEvidence,
    _write_ability: Option<WriteAbilityEvidence>,
}

impl CompletedResolution {
    pub(super) fn effective_project(&self) -> &str {
        &self.effective_project
    }

    pub(super) fn repo_name(&self) -> &str {
        &self.repo_name
    }

    pub(super) fn index_type(&self) -> &str {
        &self.index_type
    }

    pub(super) fn target(&self) -> &ResolutionTarget {
        &self.target
    }
}

// ---------------------------------------------------------------------------
// Public adapter entry point
// ---------------------------------------------------------------------------

pub struct ApplyRequest<'a> {
    pub declaration: &'a ParsedDeclaration,
    pub project_name: &'a str,
    pub repo_name: &'a str,
    pub index_type: &'a str,
    pub repo_root: &'a Path,
    pub follow_symlinks: bool,
}

/// Apply one Datasource while propagating coordinator cancellation into File
/// Datasource reads.
pub async fn apply(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    request: ApplyRequest<'_>,
    cancellation: &CancellationToken,
) -> Result<ApplyResult, AppError> {
    // ADR-012 Option A: exact-effective-project preflight before any write.
    let _initial_visibility = preflight_visibility(client, base_url, request.project_name).await?;

    let enumeration = enumerate(
        client,
        base_url,
        request.project_name,
        request.repo_name,
        request.index_type,
    )
    .await?;

    let (existing, target, write_ability) = match enumeration.matches.as_slice() {
        [] => (None, ResolutionTarget::Create, None),
        [single] => {
            let write_ability = prove_write(&single.user_abilities, "Datasource")?;
            (
                Some(ExistingEntity {
                    server_id: single.id.clone(),
                    meta_config: None,
                }),
                ResolutionTarget::Update {
                    server_id: single.id.clone(),
                },
                Some(write_ability),
            )
        }
        _ => Err(AppError::Reconciliation(format!(
            "Datasource: {} matches for (repo_name={:?}, project={:?}, type={:?}); manual resolution required",
            enumeration.matches.len(),
            request.repo_name,
            request.project_name,
            request.index_type
        )))?,
    };

    let plan = project(
        request.declaration,
        existing.as_ref(),
        None,
        request.repo_root,
        request.follow_symlinks,
    )?;
    let file_parts = if matches!(
        &plan,
        crate::projection::WritePlan::Create {
            request: RequestBody::FileMultipart { .. }
        } | crate::projection::WritePlan::Update {
            request: RequestBody::FileMultipart { .. },
            ..
        }
    ) {
        Some(read_file_parts_async(request.declaration, request.repo_root, cancellation).await?)
    } else {
        None
    };
    let resolution = CompletedResolution {
        effective_project: request.project_name.to_owned(),
        repo_name: request.repo_name.to_owned(),
        index_type: request.index_type.to_owned(),
        target,
        _scan: enumeration.evidence,
        _write_ability: write_ability,
    };
    let visibility = preflight_visibility(client, base_url, request.project_name).await?;
    let prepared = PreparedWrite::datasource(client, visibility, resolution, plan, file_parts)?;
    dispatch(prepared).await
}

/// Resolve a Datasource natural reference without requiring the target
/// declaration's `index_type` (DR-003/W-002).
///
/// Workflow references identify Datasources by exact `(project, repo_name)`.
/// If more than one visible server row has that identity, including rows of
/// different persisted kinds, resolution is ambiguous and no ID is selected.
pub async fn resolve_reference(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    repo_name: &str,
) -> Result<String, AppError> {
    let matches: Vec<String> = enumerate_project(client, base_url, project_name)
        .await?
        .items
        .into_iter()
        .filter(|item| item.repo_name == repo_name && item.project_name == project_name)
        .map(|item| item.id)
        .collect();

    match matches.as_slice() {
        [single] => Ok(single.clone()),
        [] => Err(AppError::Reconciliation(
            "referenced Datasource is missing on the target server".into(),
        )),
        _ => Err(AppError::Reconciliation(
            "referenced Datasource identity is ambiguous on the target server".into(),
        )),
    }
}

/// Post-write exact identity verification for the coordinator (R-001).
pub async fn verify_identity(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    repo_name: &str,
    index_type: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let enumeration = enumerate(client, base_url, project_name, repo_name, index_type).await?;
    match enumeration.matches.as_slice() {
        [single] if single.id == expected_server_id => Ok(()),
        _ => Err(AppError::Reconciliation(
            "Datasource write may have committed but identity verification did not match exactly once"
                .into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Exhaustive enumerate — zero-indexed pages, exact client-side filter
// ---------------------------------------------------------------------------

async fn enumerate(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    repo_name: &str,
    index_type: &str,
) -> Result<Enumeration, AppError> {
    let project = enumerate_project(client, base_url, project_name).await?;
    Ok(Enumeration {
        matches: project
            .items
            .into_iter()
            .filter(|item| {
                item.repo_name == repo_name
                    && item.project_name == project_name
                    && item.index_type == index_type
            })
            .collect(),
        evidence: project.evidence,
    })
}

async fn enumerate_project(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
) -> Result<ProjectEnumeration, AppError> {
    let filter = serde_json::to_string(&serde_json::json!({ "project_name": project_name }))
        .map_err(|_| AppError::Internal("datasource: failed to encode filter JSON".into()))?;

    let mut all_items = Vec::new();
    let mut page = 0u32;
    let mut pages_requested = 0u32;
    let mut total_seen = 0u32;
    let mut fingerprint: Option<(u32, u32, u32)> = None;
    let mut seen_ids = std::collections::HashSet::new();

    loop {
        let path = format!(
            "/v1/index?full_response=true&page={}&per_page=100&filters={}",
            page,
            encode_query_value(&filter)
        );
        let resp: DatasourcePage = client.get(base_url, &path).await?;
        pages_requested += 1;
        validate_pagination(page, &resp.pagination, fingerprint)?;
        fingerprint = Some((
            resp.pagination.pages,
            resp.pagination.total,
            resp.pagination.per_page,
        ));
        let total_pages = resp.pagination.pages;

        for item in resp.data {
            total_seen += 1;
            if total_seen > MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "datasource enumeration exceeded 100,000-item cap".into(),
                ));
            }
            if !seen_ids.insert(item.id.clone()) {
                return Err(AppError::Reconciliation(
                    "datasource enumeration repeated an entity id".into(),
                ));
            }
            all_items.push(item);
        }

        // Zero-indexed: stop when we've seen all pages (pages=N means indices 0..N-1)
        let next = page + 1;
        if next >= total_pages {
            break;
        }
        page = next;
    }

    let expected_total = fingerprint.map_or(0, |(_, total, _)| total);
    if total_seen != expected_total {
        return Err(AppError::Reconciliation(
            "datasource enumeration ended before the advertised total".into(),
        ));
    }

    Ok(ProjectEnumeration {
        items: all_items,
        evidence: ScanEvidence {
            _pages_requested: pages_requested,
            _items_seen: total_seen,
            _advertised_total: expected_total,
        },
    })
}

fn validate_pagination(
    requested_page: u32,
    pagination: &DatasourcePagination,
    fingerprint: Option<(u32, u32, u32)>,
) -> Result<(), AppError> {
    if pagination.page != requested_page || pagination.per_page != 100 {
        return Err(AppError::ApiIncompatible(
            "datasource pagination origin or page size changed".into(),
        ));
    }
    if pagination.pages > MAX_PAGES {
        return Err(AppError::ApiIncompatible(
            "datasource enumeration exceeded 1,000-page cap".into(),
        ));
    }
    if (pagination.pages == 0) != (pagination.total == 0) {
        return Err(AppError::ApiIncompatible(
            "datasource zero page count does not match zero total".into(),
        ));
    }
    if pagination.pages != pagination.total.div_ceil(pagination.per_page) {
        return Err(AppError::ApiIncompatible(
            "datasource pagination total and page count disagree".into(),
        ));
    }
    if let Some(expected) = fingerprint
        && expected != (pagination.pages, pagination.total, pagination.per_page)
    {
        return Err(AppError::Reconciliation(
            "datasource pagination changed during enumeration".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Dispatch: the HTTP client accepts only the evidence-bearing aggregate
// ---------------------------------------------------------------------------

async fn dispatch(prepared: PreparedWrite<'_>) -> Result<ApplyResult, AppError> {
    let action = match prepared.target() {
        ResolutionTarget::Create => ApplyAction::Created,
        ResolutionTarget::Update { .. } => ApplyAction::Updated,
    };
    let response = ApiClient::dispatch_prepared(prepared).await?;
    let PreparedWriteResponse::Success(response) = response else {
        return Err(AppError::ServerRejected(
            "Datasource create collided with an existing server identity".into(),
        ));
    };
    let id = extract_id(&response)?;
    Ok(ApplyResult {
        action,
        server_id: id,
    })
}

fn extract_id(resp: &serde_json::Value) -> Result<String, AppError> {
    resp.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::ApiIncompatible("datasource response missing id field".into()))
        .map(|s| s.to_owned())
}

// ---------------------------------------------------------------------------
// File Datasource: basename safety + disk read (D-001)
// ---------------------------------------------------------------------------

fn check_basename_safety(basename: &str) -> Result<(), AppError> {
    if basename.is_empty() {
        return Err(AppError::Schema(
            "file datasource: basename must not be empty".into(),
        ));
    }
    for ch in basename.chars() {
        let cp = ch as u32;
        if cp <= 0x1F          // C0 controls (NUL=0x00, CR=0x0D, LF=0x0A)
            || (0x7F..=0x9F).contains(&cp)  // DEL + C1 controls
            || ch == '/'       // POSIX path separator
            || ch == '\\'
        // Windows path separator
        {
            return Err(AppError::Schema(format!(
                "file datasource: basename {:?} contains unsafe character U+{cp:04X}",
                basename
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
fn read_file_parts(
    decl: &ParsedDeclaration,
    repo_root: &Path,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    read_file_parts_cancellable(decl, repo_root, &CancellationToken::default())
}

async fn read_file_parts_async(
    decl: &ParsedDeclaration,
    repo_root: &Path,
    cancellation: &CancellationToken,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let declaration = decl.clone();
    let root = repo_root.to_owned();
    let cancellation = cancellation.clone();
    tokio::task::spawn_blocking(move || {
        read_file_parts_cancellable(&declaration, &root, &cancellation)
    })
    .await
    .map_err(|_| AppError::Internal("file datasource reader task failed".into()))?
}

fn read_file_parts_cancellable(
    decl: &ParsedDeclaration,
    repo_root: &Path,
    cancellation: &CancellationToken,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    cancellation.checkpoint()?;
    let files = decl.value["spec"]["files"]
        .as_array()
        .ok_or_else(|| AppError::Schema("file datasource: spec.files must be an array".into()))?;

    if files.len() > MAX_FILE_PARTS {
        return Err(AppError::Schema(format!(
            "file datasource: {} files exceeds the {MAX_FILE_PARTS}-part cap",
            files.len()
        )));
    }

    let mut result = Vec::with_capacity(files.len());
    for path_val in files {
        cancellation.checkpoint()?;
        let rel_path = path_val.as_str().ok_or_else(|| {
            AppError::Schema("file datasource: each spec.files entry must be a string".into())
        })?;

        let basename = std::path::Path::new(rel_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                AppError::Schema(format!(
                    "file datasource: path {:?} has no valid basename",
                    rel_path
                ))
            })?;

        check_basename_safety(basename)?;

        let full = repo_root.join(rel_path);
        let bytes = load_sidecar_file_cancellable(&full, MAX_SIDECAR_FILE_BYTES, cancellation)?;

        result.push((basename.to_owned(), bytes));
    }

    cancellation.checkpoint()?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests (D-001)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;
    use tempfile::TempDir;

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
            .expect_at_least(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"my-project","is_project_admin":true}]}"#,
            )
            .create_async()
    }

    fn git_decl(project: &str, repo_name: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Datasource,
            value: json!({
                "metadata": { "project": project, "repo_name": repo_name },
                "spec": {
                    "index_type": "git",
                    "url": "https://github.com/example/repo.git",
                    "branch": "main"
                }
            }),
            source_path: PathBuf::from("test.yaml"),
        }
    }

    fn empty_page() -> &'static str {
        r#"{"data":[],"pagination":{"page":0,"per_page":100,"total":0,"pages":0}}"#
    }

    fn one_match_page(id: &str, repo: &str, project: &str, kind: &str) -> String {
        format!(
            r#"{{"data":[{{"id":"{id}","repo_name":"{repo}","project_name":"{project}","index_type":"{kind}","user_abilities":["read","write"]}}],"pagination":{{"page":0,"per_page":100,"total":1,"pages":1}}}}"#
        )
    }

    fn two_match_page(repo: &str, project: &str, kind: &str) -> String {
        format!(
            r#"{{"data":[{{"id":"id-1","repo_name":"{repo}","project_name":"{project}","index_type":"{kind}","user_abilities":["write"]}},{{"id":"id-2","repo_name":"{repo}","project_name":"{project}","index_type":"{kind}","user_abilities":["write"]}}],"pagination":{{"page":0,"per_page":100,"total":2,"pages":1}}}}"#
        )
    }

    #[tokio::test]
    async fn apply_creates_git_datasource_when_not_found() {
        let mut server = mockito::Server::new_async().await;

        let _user = user_ok_mock(&mut server).await;
        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/index\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;
        let _create = server
            .mock("POST", "/v1/application/my-project/index")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"ds-new"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = git_decl("my-project", "my-repo");

        let result = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &decl,
                project_name: "my-project",
                repo_name: "my-repo",
                index_type: "git",
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "ds-new");
        _user.assert_async().await;
        _enum.assert_async().await;
        _create.assert_async().await;
    }

    #[tokio::test]
    async fn file_multipart_409_is_one_post_with_no_followup_request() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("document.txt"), b"secret file bytes").unwrap();
        let declaration = ParsedDeclaration {
            kind: EntityKind::Datasource,
            value: json!({
                "metadata": { "project": "my-project", "repo_name": "my-files" },
                "spec": { "index_type": "file", "files": ["document.txt"] }
            }),
            source_path: PathBuf::from("test.yaml"),
        };
        let mut server = mockito::Server::new_async().await;
        let user = user_ok_mock(&mut server).await;
        let scan = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/index\?".to_owned()))
            .with_status(200)
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;
        let create = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"^/v1/index/knowledge_base/file\?".to_owned()),
            )
            .with_status(409)
            .with_body("must-not-leak")
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
            ApplyRequest {
                declaration: &declaration,
                project_name: "my-project",
                repo_name: "my-files",
                index_type: "file",
                repo_root: tmp.path(),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
        )
        .await
        .expect_err("multipart 409 must be terminal");
        assert!(matches!(error, AppError::ServerRejected(_)));
        user.assert_async().await;
        scan.assert_async().await;
        create.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn apply_updates_git_datasource_when_found() {
        let mut server = mockito::Server::new_async().await;

        let _user = user_ok_mock(&mut server).await;
        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/index\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(one_match_page("ds-exist", "my-repo", "my-project", "git"))
            .create_async()
            .await;
        let _update = server
            .mock("PUT", "/v1/application/my-project/index/my-repo")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"ds-exist"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = git_decl("my-project", "my-repo");

        let result = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &decl,
                project_name: "my-project",
                repo_name: "my-repo",
                index_type: "git",
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "ds-exist");
        _user.assert_async().await;
        _enum.assert_async().await;
        _update.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_other_project_admin_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let user = server
            .mock("GET", "/v1/user")
            .expect_at_least(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"other-project","is_project_admin":true}]}"#,
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
        let declaration = git_decl("my-project", "my-repo");
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &declaration,
                project_name: "my-project",
                repo_name: "my-repo",
                index_type: "git",
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
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
        let page = one_match_page("ds-exist", "my-repo", "my-project", "git")
            .replace(r#"["read","write"]"#, r#"["read"]"#);
        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/index\?".to_string()))
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
        let declaration = git_decl("my-project", "my-repo");
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &declaration,
                project_name: "my-project",
                repo_name: "my-repo",
                index_type: "git",
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
        )
        .await
        .expect_err("non-writing Datasource row must stop before PUT");
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
            .mock("GET", mockito::Matcher::Regex(r"^/v1/index\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":[],"pagination":{"page":1,"per_page":100,"total":0,"pages":0}}"#)
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
        let declaration = git_decl("my-project", "my-repo");
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &declaration,
                project_name: "my-project",
                repo_name: "my-repo",
                index_type: "git",
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
        )
        .await
        .expect_err("wrong zero-indexed page must fail compatibility");
        assert!(matches!(error, AppError::ApiIncompatible(_)));
        page.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_repeated_datasource_ids_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let repeated = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/index\?".to_owned()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":[{"id":"same","repo_name":"other-a","project_name":"my-project","index_type":"git","user_abilities":["read"]},{"id":"same","repo_name":"other-b","project_name":"my-project","index_type":"git","user_abilities":["read"]}],"pagination":{"page":0,"per_page":100,"total":2,"pages":1}}"#,
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
        let declaration = git_decl("my-project", "my-repo");
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &declaration,
                project_name: "my-project",
                repo_name: "my-repo",
                index_type: "git",
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
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
    fn datasource_response_requires_consumed_fields_and_allows_additions() {
        let baseline = json!({
            "data":[{
                "id":"ds",
                "repo_name":"repo",
                "project_name":"project",
                "index_type":"git",
                "user_abilities":["write"]
            }],
            "pagination":{"page":0,"per_page":100,"total":1,"pages":1}
        });
        let additive = json!({
            "data":[{
                "id":"ds",
                "repo_name":"repo",
                "project_name":"project",
                "index_type":"git",
                "user_abilities":["write"],
                "future_entity":true
            }],
            "pagination":{"page":0,"per_page":100,"total":1,"pages":1,"future_page":true},
            "future_top":true
        });
        let baseline_page = serde_json::from_value::<DatasourcePage>(baseline.clone())
            .expect("baseline must decode");
        let additive_page =
            serde_json::from_value::<DatasourcePage>(additive).expect("additions must decode");

        crate::adapters::assert_consumed_field_mutations::<DatasourcePage>(
            &baseline,
            &[
                "/data",
                "/data/0/id",
                "/data/0/repo_name",
                "/data/0/project_name",
                "/data/0/index_type",
                "/data/0/user_abilities",
                "/pagination",
                "/pagination/page",
                "/pagination/per_page",
                "/pagination/total",
                "/pagination/pages",
            ],
        );

        let project_response = |page: DatasourcePage| {
            let existing = ExistingEntity {
                server_id: page.data[0].id.clone(),
                meta_config: None,
            };
            project(
                &git_decl("project", "repo"),
                Some(&existing),
                None,
                Path::new("."),
                false,
            )
            .expect("decoded response must project")
        };
        let crate::projection::WritePlan::Update {
            request: RequestBody::Json(baseline_body),
            ..
        } = project_response(baseline_page)
        else {
            panic!("Datasource projection must be JSON");
        };
        let crate::projection::WritePlan::Update {
            request: RequestBody::Json(additive_body),
            ..
        } = project_response(additive_page)
        else {
            panic!("Datasource projection must be JSON");
        };
        assert_eq!(baseline_body, additive_body);
    }

    #[test]
    fn datasource_pagination_invariant_matrix() {
        let empty = || DatasourcePagination {
            page: 0,
            per_page: 100,
            total: 0,
            pages: 0,
        };
        assert!(validate_pagination(0, &empty(), None).is_ok());

        for invalid in [
            DatasourcePagination { page: 1, ..empty() },
            DatasourcePagination {
                per_page: 99,
                ..empty()
            },
            DatasourcePagination {
                total: 1,
                pages: 0,
                ..empty()
            },
            DatasourcePagination {
                total: 101,
                pages: 1,
                ..empty()
            },
            DatasourcePagination {
                total: 100_001,
                pages: 1_001,
                ..empty()
            },
        ] {
            assert!(matches!(
                validate_pagination(0, &invalid, None),
                Err(AppError::ApiIncompatible(_))
            ));
        }

        let stable_page = DatasourcePagination {
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

    #[tokio::test]
    async fn apply_reconciliation_on_multiple_matches() {
        let mut server = mockito::Server::new_async().await;

        let _user = user_ok_mock(&mut server).await;
        let _enum = server
            .mock("GET", mockito::Matcher::Regex(r"^/v1/index\?".to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(two_match_page("my-repo", "my-project", "git"))
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
        let decl = git_decl("my-project", "my-repo");

        let err = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &decl,
                project_name: "my-project",
                repo_name: "my-repo",
                index_type: "git",
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
            &CancellationToken::default(),
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

    // -----------------------------------------------------------------------
    // Basename safety checks
    // -----------------------------------------------------------------------

    #[test]
    fn basename_nul_rejected() {
        assert!(check_basename_safety("file\x00name").is_err());
    }

    #[test]
    fn basename_cr_rejected() {
        assert!(check_basename_safety("file\rname").is_err());
    }

    #[test]
    fn basename_lf_rejected() {
        assert!(check_basename_safety("file\nname").is_err());
    }

    #[test]
    fn basename_path_separator_rejected() {
        assert!(check_basename_safety("dir/file").is_err());
        assert!(check_basename_safety("dir\\file").is_err());
    }

    #[test]
    fn basename_c1_control_rejected() {
        assert!(check_basename_safety("file\u{0080}name").is_err());
    }

    #[test]
    fn basename_empty_rejected() {
        assert!(check_basename_safety("").is_err());
    }

    #[test]
    fn basename_safe_accepted() {
        assert!(check_basename_safety("document.pdf").is_ok());
        assert!(check_basename_safety("data file 2024.csv").is_ok());
        assert!(check_basename_safety("résumé.pdf").is_ok());
    }

    // -----------------------------------------------------------------------
    // Parts cap enforcement
    // -----------------------------------------------------------------------

    #[test]
    fn parts_cap_enforced() {
        let tmp = TempDir::new().unwrap();
        // Create 11 real files
        for i in 0..11 {
            std::fs::write(tmp.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        let files: Vec<serde_json::Value> = (0..11).map(|i| json!(format!("f{i}.txt"))).collect();
        let decl = ParsedDeclaration {
            kind: EntityKind::Datasource,
            value: json!({
                "metadata": { "project": "p", "repo_name": "r" },
                "spec": { "index_type": "file", "files": files }
            }),
            source_path: PathBuf::from("test.yaml"),
        };
        let err = read_file_parts(&decl, tmp.path()).expect_err("11 parts must exceed cap");
        assert!(matches!(err, AppError::Schema(_)));
    }

    #[test]
    fn file_streaming_observes_invocation_cancellation() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("document.txt"), b"content").unwrap();
        let decl = ParsedDeclaration {
            kind: EntityKind::Datasource,
            value: json!({
                "metadata": { "project": "p", "repo_name": "r" },
                "spec": { "index_type": "file", "files": ["document.txt"] }
            }),
            source_path: PathBuf::from("test.yaml"),
        };
        let cancellation = CancellationToken::default();
        cancellation.cancel();

        let error = read_file_parts_cancellable(&decl, tmp.path(), &cancellation)
            .expect_err("cancelled file streaming must stop before reading file data");
        assert!(matches!(error, AppError::Timeout(_)));
    }
}
