/// Datasource entity adapter — D-001.
///
/// Visibility preflight (`GET /v1/user`) required before any write (ADR-012 Option A).
/// Exhaustive `(project, repo_name, index_type)` resolution via paginated
/// `GET /v1/index?full_response=true&page={page}&per_page=100&filters={json}`.
/// Pagination is zero-indexed. Cap: 1,000 pages / 100,000 items → `E_API_INCOMPATIBLE`.
/// File Datasource: multipart transport with parts cap (10) and basename safety.
use std::path::Path;

use serde::Deserialize;

use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::{preflight_visibility, ApiClient, encode_query_value};
use crate::parse::ParsedDeclaration;
use crate::projection::{project, ExistingEntity, RequestBody, WritePlan};

use super::{ApplyAction, ApplyResult};

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
    pages: u32,
}

#[derive(Deserialize, Clone)]
struct DatasourceItem {
    id: String,
    repo_name: String,
    project_name: String,
    index_type: String,
}

// ---------------------------------------------------------------------------
// Public adapter entry point
// ---------------------------------------------------------------------------

pub async fn apply(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    decl: &ParsedDeclaration,
    project_name: &str,
    repo_name: &str,
    index_type: &str,
    repo_root: &Path,
    follow_symlinks: bool,
) -> Result<ApplyResult, AppError> {
    // ADR-012 Option A: preflight before any write
    preflight_visibility(client, base_url).await?;

    let matches = enumerate(client, base_url, project_name, repo_name, index_type).await?;

    match matches.as_slice() {
        [] => {
            let plan = project(decl, None, None, repo_root, follow_symlinks)?;
            dispatch(client, base_url, plan, index_type, project_name, repo_name, decl, repo_root)
                .await
        }
        [single] => {
            let existing = ExistingEntity {
                server_id: single.id.clone(),
                meta_config: None,
            };
            let plan = project(decl, Some(&existing), None, repo_root, follow_symlinks)?;
            dispatch(client, base_url, plan, index_type, project_name, repo_name, decl, repo_root)
                .await
        }
        _ => Err(AppError::Reconciliation(format!(
            "Datasource: {} matches for (repo_name={repo_name:?}, \
             project={project_name:?}, type={index_type:?}); manual resolution required",
            matches.len()
        ))),
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
) -> Result<Vec<DatasourceItem>, AppError> {
    let filter = serde_json::to_string(&serde_json::json!({ "project_name": project_name }))
        .map_err(|_| AppError::Internal("datasource: failed to encode filter JSON".into()))?;

    let mut all_matches = Vec::new();
    let mut page = 0u32;
    let mut total_seen = 0u32;

    loop {
        let path = format!(
            "/v1/index?full_response=true&page={}&per_page=100&filters={}",
            page,
            encode_query_value(&filter)
        );
        let resp: DatasourcePage = client.get(base_url, &path).await?;
        let total_pages = resp.pagination.pages;

        for item in resp.data {
            total_seen += 1;
            if total_seen > MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "datasource enumeration exceeded 100,000-item cap".into(),
                ));
            }
            if item.repo_name == repo_name
                && item.project_name == project_name
                && item.index_type == index_type
            {
                all_matches.push(item);
            }
        }

        // Zero-indexed: stop when we've seen all pages (pages=N means indices 0..N-1)
        let next = page + 1;
        if next >= total_pages || next > MAX_PAGES {
            break;
        }
        page = next;
    }

    Ok(all_matches)
}

// ---------------------------------------------------------------------------
// Route helpers (per-kind)
// ---------------------------------------------------------------------------

fn create_route(kind: &str, project: &str) -> String {
    match kind {
        "git" => format!("/v1/application/{}/index", encode_query_value(project)),
        "svn" => format!("/v1/application/{}/index/svn", encode_query_value(project)),
        _ => format!("/v1/index/knowledge_base/{}", encode_query_value(kind)),
    }
}

fn update_route(kind: &str, project: &str, repo_name: &str) -> String {
    match kind {
        "git" => format!(
            "/v1/application/{}/index/{}",
            encode_query_value(project),
            encode_query_value(repo_name)
        ),
        "svn" => format!(
            "/v1/application/{}/index/svn/{}",
            encode_query_value(project),
            encode_query_value(repo_name)
        ),
        // knowledge_base types identify by project_name + name in query params
        _ => format!("/v1/index/knowledge_base/{}", encode_query_value(kind)),
    }
}

// ---------------------------------------------------------------------------
// Dispatch: JSON or multipart, create or update
// ---------------------------------------------------------------------------

async fn dispatch(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    plan: WritePlan,
    kind: &str,
    project_name: &str,
    repo_name: &str,
    decl: &ParsedDeclaration,
    repo_root: &Path,
) -> Result<ApplyResult, AppError> {
    match plan {
        WritePlan::Create { request: RequestBody::Json(body) } => {
            let route = create_route(kind, project_name);
            let resp: serde_json::Value = client.post(base_url, &route, &body).await?;
            let id = extract_id(&resp)?;
            Ok(ApplyResult { action: ApplyAction::Created, server_id: id })
        }
        WritePlan::Update { request: RequestBody::Json(body), .. } => {
            let route = update_route(kind, project_name, repo_name);
            let resp: serde_json::Value = client.put(base_url, &route, &body).await?;
            let id = extract_id(&resp)?;
            Ok(ApplyResult { action: ApplyAction::Updated, server_id: id })
        }
        WritePlan::Create { request: RequestBody::FileMultipart { query_params, .. } } => {
            let route = create_route(kind, project_name);
            let file_parts = read_file_parts(decl, repo_root)?;
            let resp =
                client.post_multipart(base_url, &route, &query_params, file_parts).await?;
            let id = extract_id(&resp)?;
            Ok(ApplyResult { action: ApplyAction::Created, server_id: id })
        }
        WritePlan::Update { request: RequestBody::FileMultipart { mut query_params, .. }, .. } => {
            let route = update_route(kind, project_name, repo_name);
            // Inject name=repo_name for server identity: knowledge_base file update uses
            // (project_name, name) to locate the datasource (no ID in route path).
            if !query_params.iter().any(|(k, _)| k == "name") {
                query_params.push(("name".to_owned(), repo_name.to_owned()));
            }
            let file_parts = read_file_parts(decl, repo_root)?;
            let resp =
                client.put_multipart(base_url, &route, &query_params, file_parts).await?;
            let id = extract_id(&resp)?;
            Ok(ApplyResult { action: ApplyAction::Updated, server_id: id })
        }
    }
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
        return Err(AppError::Schema("file datasource: basename must not be empty".into()));
    }
    for ch in basename.chars() {
        let cp = ch as u32;
        if cp <= 0x1F          // C0 controls (NUL=0x00, CR=0x0D, LF=0x0A)
            || (cp >= 0x7F && cp <= 0x9F)  // DEL + C1 controls
            || ch == '/'       // POSIX path separator
            || ch == '\\'      // Windows path separator
        {
            return Err(AppError::Schema(format!(
                "file datasource: basename {:?} contains unsafe character U+{cp:04X}",
                basename
            )));
        }
    }
    Ok(())
}

fn read_file_parts(
    decl: &ParsedDeclaration,
    repo_root: &Path,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
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
        let bytes = std::fs::read(&full).map_err(|e| {
            AppError::Schema(format!(
                "file datasource: cannot read {:?}: {}",
                rel_path,
                e.kind()
            ))
        })?;

        result.push((basename.to_owned(), bytes));
    }

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

    fn user_ok_mock(server: &mut mockito::Server) -> impl std::future::Future<Output = mockito::Mock> + '_ {
        server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"is_admin":true}"#)
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
            r#"{{"data":[{{"id":"{id}","repo_name":"{repo}","project_name":"{project}","index_type":"{kind}"}}],"pagination":{{"page":0,"per_page":100,"total":1,"pages":1}}}}"#
        )
    }

    fn two_match_page(repo: &str, project: &str, kind: &str) -> String {
        format!(
            r#"{{"data":[{{"id":"id-1","repo_name":"{repo}","project_name":"{project}","index_type":"{kind}"}},{{"id":"id-2","repo_name":"{repo}","project_name":"{project}","index_type":"{kind}"}}],"pagination":{{"page":0,"per_page":100,"total":2,"pages":1}}}}"#
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
            &decl,
            "my-project",
            "my-repo",
            "git",
            Path::new("."),
            false,
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
            &decl,
            "my-project",
            "my-repo",
            "git",
            Path::new("."),
            false,
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

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = git_decl("my-project", "my-repo");

        let err = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-repo",
            "git",
            Path::new("."),
            false,
        )
        .await
        .expect_err("multiple matches must error");

        assert!(matches!(err, AppError::Reconciliation(_)));
        assert_eq!(err.exit_code(), 1);
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
        let files: Vec<serde_json::Value> =
            (0..11).map(|i| json!(format!("f{i}.txt"))).collect();
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
}
