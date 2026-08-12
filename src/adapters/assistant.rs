/// Assistant entity adapter — A-001.
///
/// Exact `(project, slug)` resolution via `GET /v1/assistants/slug/{slug}?project={project}`.
/// Absent identity → POST/created; present identity → unconditional PUT/updated.
/// Assistant intentionally has no `/v1/user` admin preflight. Its direct lookup
/// and existing-row write ability are sealed with the projected request.
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
use crate::http::{ApiClient, encode_query_value, preflight_visibility};
use crate::parse::ParsedDeclaration;
use crate::projection::{ExistingEntity, project};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, ResolutionTarget, WriteAbilityEvidence,
    decode_write_response, prove_write,
};

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

/// Fields consumed from the direct Assistant identity response.
#[derive(Deserialize)]
struct AssistantLookupResponse {
    id: String,
    user_abilities: Vec<String>,
}

/// Minimal fields consumed from a successful modifying response.
#[derive(Deserialize)]
struct AssistantWriteResponse {
    #[serde(rename = "assistantId", alias = "id")]
    id: Option<String>,
}

/// Actual strict direct-lookup evidence required by the Assistant seal.
#[derive(Debug)]
pub(super) struct CompletedResolution {
    effective_project: String,
    _slug: String,
    target: ResolutionTarget,
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
    let visibility = preflight_visibility(client, base_url, project_name).await?;
    // Step 1: Resolve identity.
    let resolve_path = format!(
        "/v1/assistants/slug/{}?project={}",
        encode_query_value(slug),
        encode_query_value(project_name)
    );
    let existing: Option<AssistantLookupResponse> =
        client.get_optional(base_url, &resolve_path).await?;

    let resolution = match &existing {
        Some(existing) => CompletedResolution {
            effective_project: project_name.to_owned(),
            _slug: slug.to_owned(),
            target: ResolutionTarget::Update {
                server_id: existing.id.clone(),
            },
            _write_ability: Some(prove_write(&existing.user_abilities, "Assistant")?),
        },
        None => CompletedResolution {
            effective_project: project_name.to_owned(),
            _slug: slug.to_owned(),
            target: ResolutionTarget::Create,
            _write_ability: None,
        },
    };

    let existing_entity = existing.as_ref().map(|e| ExistingEntity {
        server_id: e.id.clone(),
        meta_config: None,
    });

    // Step 2: Project.
    let plan = project(
        decl,
        existing_entity.as_ref(),
        None,
        repo_root,
        follow_symlinks,
    )?;

    // Step 3: seal the completed direct-lookup evidence with projection. The
    // modifying dispatcher accepts no raw or partial-evidence `WritePlan`.
    let prepared = PreparedWrite::assistant(client, visibility, resolution, plan)?;
    dispatch(prepared).await
}

async fn dispatch(prepared: PreparedWrite<'_>) -> Result<ApplyResult, AppError> {
    let (action, resolved_update_id) = match prepared.target() {
        ResolutionTarget::Create => (ApplyAction::Created, None),
        ResolutionTarget::Update { server_id } => (ApplyAction::Updated, Some(server_id.clone())),
    };
    let response = ApiClient::dispatch_prepared(prepared).await?;
    let response: AssistantWriteResponse = decode_write_response(response)?.ok_or_else(|| {
        AppError::Internal("Assistant modifying request cannot return a conflict signal".into())
    })?;
    let server_id = match (response.id, resolved_update_id) {
        (Some(id), _) => id,
        (None, Some(id)) => id,
        (None, None) => {
            return Err(AppError::Internal(
                "Assistant update path missing resolved server identity".into(),
            ));
        }
    };
    Ok(ApplyResult { action, server_id })
}

/// Resolve an Assistant natural reference without writing it (DR-003/W-002).
pub async fn resolve_reference(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
) -> Result<String, AppError> {
    let path = format!(
        "/v1/assistants/slug/{}?project={}",
        encode_query_value(slug),
        encode_query_value(project_name)
    );
    client
        .get_optional::<AssistantLookupResponse>(base_url, &path)
        .await?
        .map(|item| item.id)
        .ok_or_else(|| {
            AppError::Reconciliation("referenced Assistant is missing on the target server".into())
        })
}

/// Verify that the authored identity resolves to the route ID returned by the
/// modifying request. This is a read-only post-write check (R-001).
pub async fn verify_identity(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let actual = resolve_reference(client, base_url, project_name, slug).await?;
    if actual == expected_server_id {
        Ok(())
    } else {
        Err(AppError::Reconciliation(
            "Assistant write may have committed but identity verification did not match".into(),
        ))
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

    async fn membership(server: &mut mockito::ServerGuard) -> mockito::Mock {
        server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_body(r#"{"user_id":"user-1","projects":[{"name":"my-project"}]}"#)
            .create_async()
            .await
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
                    "prompt_variables": [],
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
        let _membership = membership(&mut server).await;

        let _resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(404)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/v1/assistants")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"assistantId":"uuid-1"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = assistant_decl("my-project", "my-assistant");

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "uuid-1");
        _resolve.assert_async().await;
        _create.assert_async().await;
    }

    #[tokio::test]
    async fn apply_creates_when_response_uses_assistant_id() {
        let mut server = mockito::Server::new_async().await;
        let _membership = membership(&mut server).await;

        let _resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(404)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/v1/assistants")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"Specified assistant saved","assistantId":"uuid-2","validation":null}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = assistant_decl("my-project", "my-assistant");

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "uuid-2");
        _resolve.assert_async().await;
        _create.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // A-001 AC: 200 → PUT/updated
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_updates_when_found() {
        let mut server = mockito::Server::new_async().await;
        let _membership = membership(&mut server).await;

        let _resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"existing-uuid","user_abilities":["read","write"]}"#)
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
            &client,
            &url,
            &decl,
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "existing-uuid");
        _resolve.assert_async().await;
        _update.assert_async().await;
    }

    #[tokio::test]
    async fn apply_updates_when_response_omits_id() {
        let mut server = mockito::Server::new_async().await;
        let _membership = membership(&mut server).await;

        let _resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"existing-uuid","user_abilities":["read","write"]}"#)
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/assistants/existing-uuid")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"Specified assistant updated","validation":null}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = assistant_decl("my-project", "my-assistant");

        let result = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "existing-uuid");
        _resolve.assert_async().await;
        _update.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_missing_lookup_abilities_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let _membership = membership(&mut server).await;
        let resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"existing-uuid"}"#)
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

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            &assistant_decl("my-project", "my-assistant"),
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
        )
        .await
        .expect_err("missing direct-lookup ability evidence must fail");

        assert!(matches!(error, AppError::ApiIncompatible(_)));
        resolve.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
    }

    #[tokio::test]
    async fn lookup_field_type_matrix_fails_before_every_modifying_route() {
        for response in [
            r#"{"user_abilities":["write"]}"#,
            r#"{"id":1,"user_abilities":["write"]}"#,
            r#"{"id":null,"user_abilities":["write"]}"#,
            r#"{"id":"assistant"}"#,
            r#"{"id":"assistant","user_abilities":"write"}"#,
            r#"{"id":"assistant","user_abilities":[1]}"#,
        ] {
            let mut server = mockito::Server::new_async().await;
            let _membership = membership(&mut server).await;
            let lookup = server
                .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(response)
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
                &assistant_decl("my-project", "my-assistant"),
                "my-project",
                "my-assistant",
                Path::new("."),
                false,
            )
            .await
            .expect_err("invalid direct-lookup field must fail compatibility");

            assert!(matches!(error, AppError::ApiIncompatible(_)));
            lookup.assert_async().await;
            post.assert_async().await;
            put.assert_async().await;
            patch.assert_async().await;
            delete.assert_async().await;
        }
    }

    #[tokio::test]
    async fn apply_rejects_nonwriting_lookup_abilities_before_put() {
        let mut server = mockito::Server::new_async().await;
        let _membership = membership(&mut server).await;
        let resolve = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"existing-uuid","user_abilities":["read"]}"#)
            .expect(1)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            &assistant_decl("my-project", "my-assistant"),
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
        )
        .await
        .expect_err("a valid non-writing ability list must stop before PUT");

        assert!(matches!(error, AppError::Authorization(_)));
        resolve.assert_async().await;
        put.assert_async().await;
    }

    #[test]
    fn assistant_lookup_requires_consumed_fields_and_allows_additions() {
        let baseline = serde_json::from_value::<AssistantLookupResponse>(json!({
            "id":"assistant",
            "user_abilities":["write"]
        }))
        .expect("baseline lookup must decode");
        let additive = serde_json::from_value::<AssistantLookupResponse>(json!({
            "id":"assistant",
            "user_abilities":["write"],
            "future_field":{"nested":true}
        }))
        .expect("additive unconsumed field must decode");
        let decl = assistant_decl("my-project", "my-assistant");
        let projected = |lookup: &AssistantLookupResponse| {
            project(
                &decl,
                Some(&ExistingEntity {
                    server_id: lookup.id.clone(),
                    meta_config: None,
                }),
                None,
                Path::new("."),
                false,
            )
            .expect("lookup must project")
        };
        let crate::projection::WritePlan::Update {
            request: crate::projection::RequestBody::Json(baseline_body),
            ..
        } = projected(&baseline)
        else {
            panic!("existing Assistant must project an update JSON body")
        };
        let crate::projection::WritePlan::Update {
            request: crate::projection::RequestBody::Json(additive_body),
            ..
        } = projected(&additive)
        else {
            panic!("existing Assistant must project an update JSON body")
        };
        assert_eq!(baseline_body, additive_body);

        for response in [
            json!({"user_abilities":["write"]}),
            json!({"id":"assistant"}),
            json!({"id":1,"user_abilities":["write"]}),
            json!({"id":"assistant","user_abilities":[1]}),
        ] {
            assert!(serde_json::from_value::<AssistantLookupResponse>(response).is_err());
        }
    }

    // -----------------------------------------------------------------------
    // A-001 AC: 401 on resolve → Authentication error
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_propagates_auth_error_from_resolve() {
        let mut server = mockito::Server::new_async().await;
        let _membership = membership(&mut server).await;

        let _mock = server
            .mock("GET", "/v1/assistants/slug/my-assistant?project=my-project")
            .with_status(401)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = assistant_decl("my-project", "my-assistant");

        let err = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
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
        let _membership = membership(&mut server).await;

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
                    "prompt_variables": [],
                    "categories": []
                    // system_prompt absent
                }
            }),
            source_path: PathBuf::from("test.yaml"),
        };

        let err = apply(
            &client,
            &url,
            &decl,
            "my-project",
            "my-assistant",
            Path::new("."),
            false,
        )
        .await
        .expect_err("missing required field must produce Schema error");
        assert!(matches!(err, AppError::Schema(_)));
        assert_eq!(err.exit_code(), 2);
    }
}
