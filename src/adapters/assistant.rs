/// Assistant entity adapter — A-001.
///
/// Exact `(project, slug)` resolution via `GET /v1/assistants/slug/{slug}?project={project}`.
/// Absent identity → POST/created; present identity → unconditional PUT/updated.
/// Preflights (check_compatibility, preflight_visibility) are the caller's responsibility.
///
/// ## Source traceability
///
/// - Requirements: FR-005/006/015/021, DR-003/005
/// - Architecture: ADR-002
/// - Task: A-001
/// - Manifest: §entities.Assistant routes.resolve / create / update
use std::path::Path;

use serde::Deserialize;

use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value};
use crate::parse::ParsedDeclaration;
use crate::projection::{project, ExistingEntity, RequestBody, WritePlan};

use super::{ApplyAction, ApplyResult};

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

/// Minimal fields consumed from any Assistant server response.
///
/// Only `id` is needed for write dispatch; all other fields are discarded.
#[derive(Deserialize)]
struct AssistantIdResponse {
    id: String,
}

// ---------------------------------------------------------------------------
// Public adapter function
// ---------------------------------------------------------------------------

/// Apply an Assistant declaration to the target API.
///
/// 1. Calls `GET /v1/assistants/slug/{slug}?project={project}` to resolve identity.
/// 2. 404 → Create path; 200 → Update path.
/// 3. Projects the declaration into a `WritePlan`.
/// 4. Dispatches `POST /v1/assistants` (create) or `PUT /v1/assistants/{id}` (update).
///
/// `repo_root` and `follow_symlinks` are forwarded to the projection layer for
/// file-relative sidecar paths (not currently used for Assistants).
pub async fn apply(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    decl: &ParsedDeclaration,
    project_name: &str,
    slug: &str,
    repo_root: &Path,
    follow_symlinks: bool,
) -> Result<ApplyResult, AppError> {
    // Step 1: Resolve identity.
    let resolve_path = format!(
        "/v1/assistants/slug/{}?project={}",
        encode_query_value(slug),
        encode_query_value(project_name)
    );
    let existing: Option<AssistantIdResponse> =
        client.get_optional(base_url, &resolve_path).await?;

    let existing_entity = existing.as_ref().map(|e| ExistingEntity {
        server_id: e.id.clone(),
        meta_config: None,
    });

    // Step 2: Project.
    let plan = project(decl, existing_entity.as_ref(), None, repo_root, follow_symlinks)?;

    // Step 3: Dispatch.
    match plan {
        WritePlan::Create { request: RequestBody::Json(body) } => {
            let resp: AssistantIdResponse =
                client.post(base_url, "/v1/assistants", &body).await?;
            Ok(ApplyResult {
                action: ApplyAction::Created,
                server_id: resp.id,
            })
        }
        WritePlan::Update {
            server_id,
            request: RequestBody::Json(body),
        } => {
            let update_path = format!("/v1/assistants/{}", encode_query_value(&server_id));
            let resp: AssistantIdResponse =
                client.put(base_url, &update_path, &body).await?;
            Ok(ApplyResult {
                action: ApplyAction::Updated,
                server_id: resp.id,
            })
        }
        _ => Err(AppError::Internal(
            "Assistant projection produced unexpected body variant".into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Tests (A-001)
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

    fn assistant_decl(project: &str, slug: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Assistant,
            value: json!({
                "metadata": { "project": project, "slug": slug },
                "spec": {
                    "name": "Test Assistant",
                    "system_prompt": "You are helpful.",
                    "llm_model_type": "gpt-4",
                    "type": "codemie",
                    "context": [],
                    "toolkits": [],
                    "conversation_starters": [],
                    "shared": true,
                    "mcp_servers": [],
                    "enabled_builtin_subagents": [],
                    "categories": []
                }
            }),
            source_path: PathBuf::from("test.yaml"),
        }
    }

    // -----------------------------------------------------------------------
    // A-001 AC: 404 → POST/created
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_creates_when_not_found() {
        let mut server = mockito::Server::new_async().await;

        let _resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(404)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/v1/assistants")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"uuid-1"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = assistant_decl("my-project", "my-assistant");

        let result = apply(
            &client, &url, &decl, "my-project", "my-assistant",
            Path::new("."), false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "uuid-1");
        _resolve.assert_async().await;
        _create.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // A-001 AC: 200 → PUT/updated
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_updates_when_found() {
        let mut server = mockito::Server::new_async().await;

        let _resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"existing-uuid"}"#)
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/assistants/existing-uuid")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"existing-uuid"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = assistant_decl("my-project", "my-assistant");

        let result = apply(
            &client, &url, &decl, "my-project", "my-assistant",
            Path::new("."), false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "existing-uuid");
        _resolve.assert_async().await;
        _update.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // A-001 AC: 401 on resolve → Authentication error
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_propagates_auth_error_from_resolve() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(401)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = assistant_decl("my-project", "my-assistant");

        let err = apply(
            &client, &url, &decl, "my-project", "my-assistant",
            Path::new("."), false,
        )
        .await
        .expect_err("401 must propagate as error");
        assert!(matches!(err, AppError::Authentication(_)));
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // A-001 AC: schema error when required field missing
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_schema_error_on_missing_required_field() {
        let mut server = mockito::Server::new_async().await;

        let _resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(404)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());

        // Declaration missing required `system_prompt`
        let decl = ParsedDeclaration {
            kind: EntityKind::Assistant,
            value: json!({
                "metadata": { "project": "my-project", "slug": "my-assistant" },
                "spec": {
                    "name": "Test",
                    "llm_model_type": "gpt-4",
                    "type": "codemie",
                    "context": [],
                    "toolkits": [],
                    "conversation_starters": [],
                    "shared": true,
                    "mcp_servers": [],
                    "enabled_builtin_subagents": [],
                    "categories": []
                    // system_prompt absent
                }
            }),
            source_path: PathBuf::from("test.yaml"),
        };

        let err = apply(
            &client, &url, &decl, "my-project", "my-assistant",
            Path::new("."), false,
        )
        .await
        .expect_err("missing required field must produce Schema error");
        assert!(matches!(err, AppError::Schema(_)));
        assert_eq!(err.exit_code(), 2);
    }
}
