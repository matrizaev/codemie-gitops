/// Single-entity write-through coordinator (R-001).
///
/// One invocation follows the approved state machine:
/// local load/validation -> authentication -> kind-specific operation preflight
/// -> adapter resolve/project/sealed-write -> post-write identity
/// verification -> typed success outcome.
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::adapters::{self, ApplyAction, ApplyResult};
use crate::cancellation::CancellationToken;
use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::ApiClient;
use crate::output::{Action, Outcome};
use crate::parse::{EntityKind, ParsedDeclaration};
use crate::repository::{TargetLoadRequest, load_target_declaration};

/// Whole-invocation deadline from the approved resource budget.
pub const INVOCATION_DEADLINE: Duration = Duration::from_secs(300);

/// Validated inputs required to run one apply invocation.
pub struct ApplyCommand {
    pub file: PathBuf,
    pub repo_root: PathBuf,
    pub base_url: ValidatedUrl,
    pub token: String,
    pub default_project: Option<String>,
    pub follow_symlinks: bool,
    pub adopt_workflow_id: Option<String>,
}

#[derive(Debug, Clone)]
enum TargetIdentity {
    Assistant {
        project: String,
        slug: String,
    },
    Workflow {
        project: String,
        slug: String,
    },
    Skill {
        project: String,
        name: String,
    },
    Datasource {
        project: String,
        repo_name: String,
        index_type: String,
    },
}

impl TargetIdentity {
    fn success_outcome(&self, action: ApplyAction) -> Outcome {
        let action = match action {
            ApplyAction::Created => Action::Created,
            ApplyAction::Updated => Action::Updated,
        };
        match self {
            TargetIdentity::Assistant { project, slug } => Outcome::new(
                action,
                "Assistant".to_owned(),
                project.clone(),
                slug.clone(),
            ),
            TargetIdentity::Workflow { project, slug } => {
                Outcome::new(action, "Workflow".to_owned(), project.clone(), slug.clone())
            }
            TargetIdentity::Skill { project, name } => {
                Outcome::new_skill(action, project.clone(), name.clone())
            }
            TargetIdentity::Datasource {
                project, repo_name, ..
            } => Outcome::new_datasource(action, project.clone(), repo_name.clone()),
        }
    }
}

/// Execute one apply under the normative five-minute invocation deadline.
pub async fn apply(command: ApplyCommand) -> Result<Outcome, AppError> {
    apply_with_deadline(command, INVOCATION_DEADLINE).await
}

async fn apply_with_deadline(
    command: ApplyCommand,
    deadline: Duration,
) -> Result<Outcome, AppError> {
    let cancellation = CancellationToken::default();
    tokio::select! {
        result = apply_inner(command, cancellation.clone()) => result,
        _ = tokio::time::sleep(deadline) => {
            cancellation.cancel();
            Err(AppError::Timeout("apply exceeded the 300-second deadline".into()))
        }
    }
}

async fn apply_inner(
    command: ApplyCommand,
    cancellation: CancellationToken,
) -> Result<Outcome, AppError> {
    // LocalValidated: discovery, parsing, effective-project materialization,
    // semantic validation, and repository graph closure all precede network.
    let file = command.file.clone();
    let repo_root = command.repo_root.clone();
    let default_project = command.default_project.clone();
    let follow_symlinks = command.follow_symlinks;
    let local_cancellation = cancellation.clone();
    let target = tokio::task::spawn_blocking(move || {
        load_target_declaration(
            TargetLoadRequest {
                file: &file,
                repo_root: &repo_root,
                default_project: default_project.as_deref(),
                follow_symlinks,
            },
            &local_cancellation,
        )
    })
    .await
    .map_err(|_| AppError::Internal("local validation worker failed".into()))??;
    cancellation.checkpoint()?;
    let identity = target_identity(&target)?;
    validate_adoption_selector(&identity, command.adopt_workflow_id.as_deref())?;

    // Authenticated: ApiClient construction occurs only after all locally
    // determinable failures above have been exhausted.
    if command.token.is_empty() {
        return Err(AppError::Authentication(
            "CODEMIE_TOKEN is required for apply".into(),
        ));
    }
    let client = ApiClient::new(command.base_url.clone(), command.token)?;

    // Each adapter establishes its operation-applicable non-mutating evidence
    // and seals it with the projected request before its modifying dispatcher
    // is reachable. `/v1/info` is observability only and is never an apply gate.
    let result = dispatch_adapter(
        &client,
        &command.base_url,
        DispatchRequest {
            declaration: &target,
            identity: &identity,
            repo_root: &command.repo_root,
            follow_symlinks: command.follow_symlinks,
            adopt_workflow_id: command.adopt_workflow_id.as_deref(),
            cancellation: &cancellation,
        },
    )
    .await?;

    verify_written_identity(&client, &command.base_url, &identity, &result.server_id)
        .await
        .map_err(classify_verification_failure)?;

    Ok(identity.success_outcome(result.action))
}

fn target_identity(declaration: &ParsedDeclaration) -> Result<TargetIdentity, AppError> {
    let project = required_string(&declaration.value, "/metadata/project")?;
    match declaration.kind {
        EntityKind::Assistant => Ok(TargetIdentity::Assistant {
            project,
            slug: required_string(&declaration.value, "/metadata/slug")?,
        }),
        EntityKind::Workflow => Ok(TargetIdentity::Workflow {
            project,
            slug: required_string(&declaration.value, "/metadata/slug")?,
        }),
        EntityKind::Skill => Ok(TargetIdentity::Skill {
            project,
            name: required_string(&declaration.value, "/metadata/name")?,
        }),
        EntityKind::Datasource => Ok(TargetIdentity::Datasource {
            project,
            repo_name: required_string(&declaration.value, "/metadata/repo_name")?,
            index_type: required_string(&declaration.value, "/spec/index_type")?,
        }),
    }
}

fn required_string(value: &serde_json::Value, pointer: &str) -> Result<String, AppError> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| AppError::Schema(format!("required field {pointer} is absent")))
}

fn validate_adoption_selector(
    identity: &TargetIdentity,
    selector: Option<&str>,
) -> Result<(), AppError> {
    let Some(selector) = selector else {
        return Ok(());
    };
    if !matches!(identity, TargetIdentity::Workflow { .. }) {
        return Err(AppError::Usage(
            "--adopt-workflow-id is valid only for Workflow declarations".into(),
        ));
    }
    if !is_uuid(selector) {
        return Err(AppError::Usage("--adopt-workflow-id must be a UUID".into()));
    }
    Ok(())
}

fn is_uuid(value: &str) -> bool {
    let groups = [8usize, 4, 4, 4, 12];
    let mut parts = value.split('-');
    for expected in groups {
        let Some(part) = parts.next() else {
            return false;
        };
        if part.len() != expected || !part.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return false;
        }
    }
    parts.next().is_none()
}

struct DispatchRequest<'a> {
    declaration: &'a ParsedDeclaration,
    identity: &'a TargetIdentity,
    repo_root: &'a Path,
    follow_symlinks: bool,
    adopt_workflow_id: Option<&'a str>,
    cancellation: &'a CancellationToken,
}

async fn dispatch_adapter(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    request: DispatchRequest<'_>,
) -> Result<ApplyResult, AppError> {
    match request.identity {
        TargetIdentity::Assistant { project, slug } => {
            adapters::assistant::apply(
                client,
                base_url,
                request.declaration,
                project,
                slug,
                request.repo_root,
                request.follow_symlinks,
            )
            .await
        }
        TargetIdentity::Workflow { project, slug } => {
            adapters::workflow::apply(
                client,
                base_url,
                adapters::workflow::ApplyRequest {
                    declaration: request.declaration,
                    project_name: project,
                    slug,
                    adopt_workflow_id: request.adopt_workflow_id,
                    repo_root: request.repo_root,
                    follow_symlinks: request.follow_symlinks,
                },
            )
            .await
        }
        TargetIdentity::Skill { project, name } => {
            adapters::skill::apply(
                client,
                base_url,
                request.declaration,
                project,
                name,
                request.repo_root,
                request.follow_symlinks,
            )
            .await
        }
        TargetIdentity::Datasource {
            project,
            repo_name,
            index_type,
        } => {
            adapters::datasource::apply(
                client,
                base_url,
                adapters::datasource::ApplyRequest {
                    declaration: request.declaration,
                    project_name: project,
                    repo_name,
                    index_type,
                    repo_root: request.repo_root,
                    follow_symlinks: request.follow_symlinks,
                },
                request.cancellation,
            )
            .await
        }
    }
}

async fn verify_written_identity(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    identity: &TargetIdentity,
    expected_server_id: &str,
) -> Result<(), AppError> {
    match identity {
        TargetIdentity::Assistant { project, slug } => {
            adapters::assistant::verify_identity(
                client,
                base_url,
                project,
                slug,
                expected_server_id,
            )
            .await
        }
        TargetIdentity::Workflow { project, slug } => {
            adapters::workflow::verify_identity(client, base_url, project, slug, expected_server_id)
                .await
        }
        TargetIdentity::Skill { project, name } => {
            adapters::skill::verify_identity(client, base_url, project, name, expected_server_id)
                .await
        }
        TargetIdentity::Datasource {
            project,
            repo_name,
            index_type,
        } => {
            adapters::datasource::verify_identity(
                client,
                base_url,
                project,
                repo_name,
                index_type,
                expected_server_id,
            )
            .await
        }
    }
}

fn classify_verification_failure(error: AppError) -> AppError {
    match error {
        AppError::Connectivity(_) | AppError::Timeout(_) => AppError::WriteVerificationUnavailable(
            "write may have committed; verification could not complete".into(),
        ),
        AppError::ApiIncompatible(_) => AppError::WriteVerificationIncompatible(
            "write may have committed; verification response was incompatible".into(),
        ),
        AppError::Reconciliation(_) => AppError::WriteUncertain(
            "write may have committed; identity did not verify exactly once".into(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::output::OutputMode;
    use crate::render::{Renderer, diagnostic_from_app_error};

    fn assistant_yaml() -> &'static str {
        r#"apiVersion: codemie.epam.com/v1alpha1
kind: Assistant
metadata:
  project: project-a
  slug: assistant-a
spec:
  name: Assistant A
  system_prompt: Helpful
  llm_model_type: gpt
  type: codemie
  context: []
  toolkits: []
  conversation_starters: []
  shared: false
  mcp_servers: []
  sub_assistants: []
  enabled_builtin_subagents: []
  skills: []
  categories: []
  prompt_variables: []
"#
    }

    fn workflow_yaml() -> &'static str {
        r#"apiVersion: codemie.epam.com/v1alpha1
kind: Workflow
metadata:
  project: project-a
  slug: workflow-a
spec:
  name: Workflow A
  description: A valid coordinator workflow
  mode: Sequential
  shared: false
  execution_config:
    messages_limit_before_summarization: 10
    tokens_limit_before_summarization: 1000
    type: default
    enable_summarization_node: false
    recursion_limit: 10
    max_concurrency: 1
    verbose: false
    max_iteration_key_output_limit: 100
    assistants: []
    tools: []
    custom_nodes: []
    states: []
    retry_policy:
      initial_interval: 1000
      backoff_factor: 2
      max_interval: 60000
      max_attempts: 3
"#
    }

    fn workflow_page(page: u32) -> String {
        let marker = serde_json::json!({
            "codemie.epam.com/gitops/workflow-identity": {
                "version":2,"creator_user_id":"user-1",
                "project": "project-a",
                "slug": "workflow-a"
            }
        })
        .to_string();
        serde_json::json!({
            "data": [{
                "id": "workflow-server-id",
                "project": "project-a",
                "name": "Workflow A",
                "meta_config": marker,
                "created_by":{"id":"user-1"},"user_abilities": ["read", "write"]
            }],
            "pagination": {"page": page, "per_page": 100, "total": 1, "pages": 1}
        })
        .to_string()
    }

    fn empty_workflow_page() -> &'static str {
        r#"{"data":[],"pagination":{"page":0,"per_page":100,"total":0,"pages":0}}"#
    }

    fn command(root: &Path, file: &Path, base_url: &str) -> ApplyCommand {
        ApplyCommand {
            file: file.to_owned(),
            repo_root: root.to_owned(),
            base_url: ValidatedUrl::try_from(base_url).unwrap(),
            token: "test-token".into(),
            default_project: None,
            follow_symlinks: false,
            adopt_workflow_id: None,
        }
    }

    #[test]
    fn uuid_selector_accepts_canonical_uuid() {
        assert!(is_uuid("123e4567-e89b-12d3-a456-426614174000"));
    }

    #[test]
    fn uuid_selector_rejects_non_uuid() {
        assert!(!is_uuid("workflow-id"));
    }

    #[test]
    fn verification_reconciliation_becomes_write_uncertain() {
        let error = classify_verification_failure(AppError::Reconciliation("ambiguous".into()));
        assert!(matches!(error, AppError::WriteUncertain(_)));
        assert_eq!(error.exit_code(), 1);
    }

    #[tokio::test]
    async fn deadline_expiry_has_timeout_taxonomy() {
        let result: Result<(), AppError> = tokio::time::timeout(
            Duration::from_millis(1),
            tokio::time::sleep(Duration::from_millis(20)),
        )
        .await
        .map_err(|_| AppError::Timeout("deadline".into()));
        let error = result.expect_err("deadline must expire");
        assert!(matches!(error, AppError::Timeout(_)));
        assert_eq!(error.exit_code(), 2);
        assert_eq!(
            diagnostic_from_app_error(&error).error_code,
            crate::render::ErrorCode::EConnectivity
        );
    }

    #[tokio::test]
    async fn cancellation_propagates_to_blocking_workers() {
        let cancellation = CancellationToken::default();
        let worker_token = cancellation.clone();
        let observed = Arc::new(AtomicBool::new(false));
        let worker_observed = Arc::clone(&observed);
        let worker = tokio::task::spawn_blocking(move || {
            loop {
                if worker_token.checkpoint().is_err() {
                    worker_observed.store(true, Ordering::Release);
                    break;
                }
                std::thread::yield_now();
            }
        });

        tokio::time::sleep(Duration::from_millis(1)).await;
        cancellation.cancel();
        tokio::time::timeout(Duration::from_secs(1), worker)
            .await
            .expect("blocking worker must observe cancellation")
            .expect("blocking worker must exit normally");
        assert!(observed.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn existing_entity_is_written_once_then_verified() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        let file = root.path().join("assistant.yaml");
        fs::write(&file, assistant_yaml()).unwrap();

        let mut server = mockito::Server::new_async().await;
        let info = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"version":"0.16.0"}"#)
            .expect(0)
            .create_async()
            .await;
        let visibility = server
            .mock("GET", "/v1/user")
            .expect_at_least(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"project-a","is_project_admin":true}]}"#)
            .expect(1)
            .create_async()
            .await;
        let resolve = server
            .mock("GET", "/v1/assistants/slug/assistant-a?project=project-a")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"id":"assistant-server-id","user_abilities":["read","write"],"future_field":true}"#,
            )
            .expect(2)
            .create_async()
            .await;
        let update = server
            .mock("PUT", "/v1/assistants/assistant-server-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"assistant-server-id"}"#)
            .expect(1)
            .create_async()
            .await;

        let outcome = apply(command(root.path(), &file, &server.url()))
            .await
            .expect("coordinator apply must succeed");
        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            serde_json::json!({
                "action": "updated",
                "kind": "Assistant",
                "project": "project-a",
                "slug": "assistant-a"
            })
        );
        info.assert_async().await;
        visibility.assert_async().await;
        resolve.assert_async().await;
        update.assert_async().await;
    }

    #[tokio::test]
    async fn absent_entity_is_created_once_then_verified() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        let file = root.path().join("assistant.yaml");
        fs::write(&file, assistant_yaml()).unwrap();

        let mut server = mockito::Server::new_async().await;
        let info = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"version":"0.16.0"}"#)
            .expect(0)
            .create_async()
            .await;
        let visibility = server
            .mock("GET", "/v1/user")
            .expect_at_least(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"project-a","is_project_admin":true}]}"#)
            .expect(1)
            .create_async()
            .await;
        let resolve_absent = server
            .mock("GET", "/v1/assistants/slug/assistant-a?project=project-a")
            .with_status(404)
            .expect(1)
            .create_async()
            .await;
        let create = server
            .mock("POST", "/v1/assistants")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"assistant-server-id","user_abilities":["write"]}"#)
            .expect(1)
            .create_async()
            .await;
        let resolve_created = server
            .mock("GET", "/v1/assistants/slug/assistant-a?project=project-a")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"assistant-server-id","user_abilities":["write"]}"#)
            .expect(1)
            .create_async()
            .await;
        let update = server
            .mock("PUT", "/v1/assistants/assistant-server-id")
            .expect(0)
            .create_async()
            .await;
        let delete = server
            .mock("DELETE", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let outcome = apply(command(root.path(), &file, &server.url()))
            .await
            .expect("coordinator apply must create the absent identity");
        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            serde_json::json!({
                "action": "created",
                "kind": "Assistant",
                "project": "project-a",
                "slug": "assistant-a"
            })
        );
        info.assert_async().await;
        visibility.assert_async().await;
        resolve_absent.assert_async().await;
        create.assert_async().await;
        resolve_created.assert_async().await;
        update.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn workflow_update_uses_page_zero_twice_without_info_contact() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        let file = root.path().join("workflow.yaml");
        fs::write(&file, workflow_yaml()).unwrap();

        let mut server = mockito::Server::new_async().await;
        let info = server
            .mock("GET", "/v1/info")
            .expect(0)
            .create_async()
            .await;
        let user = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"project-a","is_project_admin":true}]}"#)
            .expect(3)
            .create_async()
            .await;
        let project_page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(workflow_page(0))
            .expect(1)
            .create_async()
            .await;
        let marketplace_page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                        .to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_workflow_page())
            .expect(1)
            .create_async()
            .await;
        let page_one = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/workflows\?.*page=1.*$".to_owned()),
            )
            .expect(0)
            .create_async()
            .await;
        let detail = server
            .mock("GET", "/v1/workflows/id/workflow-server-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "id": "workflow-server-id",
                    "project": "project-a",
                    "name": "Workflow A",
                    "meta_config": serde_json::json!({
                        "codemie.epam.com/gitops/workflow-identity": {
                            "version":2,"creator_user_id":"user-1",
                            "project": "project-a",
                            "slug": "workflow-a"
                        }
                    }).to_string(),
                    "created_by": {"id": "user-1"},
                    "user_abilities": ["read", "write"]
                })
                .to_string(),
            )
            .expect(2)
            .create_async()
            .await;
        let update = server
            .mock("PUT", "/v1/workflows/workflow-server-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"workflow-server-id"}"#)
            .expect(1)
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

        let outcome = apply(command(root.path(), &file, &server.url()))
            .await
            .expect("Workflow coordinator update and post-write scan must succeed");
        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            serde_json::json!({
                "action": "updated",
                "kind": "Workflow",
                "project": "project-a",
                "slug": "workflow-a"
            })
        );
        info.assert_async().await;
        user.assert_async().await;
        project_page_zero.assert_async().await;
        marketplace_page_zero.assert_async().await;
        page_one.assert_async().await;
        detail.assert_async().await;
        update.assert_async().await;
        post.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn workflow_postwrite_bad_origin_is_classified_without_info_or_second_write() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        let file = root.path().join("workflow.yaml");
        fs::write(&file, workflow_yaml()).unwrap();

        let mut server = mockito::Server::new_async().await;
        let info = server
            .mock("GET", "/v1/info")
            .expect(0)
            .create_async()
            .await;
        let user = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"project-a","is_project_admin":true}]}"#)
            .expect(3)
            .create_async()
            .await;
        let initial_project = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(workflow_page(0))
            .expect(1)
            .create_async()
            .await;
        let initial_marketplace = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                        .to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_workflow_page())
            .expect(1)
            .create_async()
            .await;
        let postwrite_project = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(workflow_page(1))
            .expect(0)
            .create_async()
            .await;
        let postwrite_marketplace = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/workflows\?.*scope=marketplace$".to_owned()),
            )
            .expect(0)
            .create_async()
            .await;
        let detail_ok = server
            .mock("GET", "/v1/workflows/id/workflow-server-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "id": "workflow-server-id",
                    "project": "project-a",
                    "name": "Workflow A",
                    "meta_config": serde_json::json!({
                        "codemie.epam.com/gitops/workflow-identity": {
                            "version":2,"creator_user_id":"user-1",
                            "project": "project-a",
                            "slug": "workflow-a"
                        }
                    }).to_string(),
                    "created_by": {"id": "user-1"},
                    "user_abilities": ["write"]
                })
                .to_string(),
            )
            .expect(2)
            .create_async()
            .await;
        let update = server
            .mock("PUT", "/v1/workflows/workflow-server-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"workflow-server-id"}"#)
            .expect(1)
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

        let outcome = apply(command(root.path(), &file, &server.url()))
            .await
            .expect("detail verification should succeed despite list shape");
        assert_eq!(
            serde_json::to_value(outcome).unwrap()["action"],
            serde_json::json!("updated")
        );

        info.assert_async().await;
        user.assert_async().await;
        initial_project.assert_async().await;
        initial_marketplace.assert_async().await;
        postwrite_project.assert_async().await;
        postwrite_marketplace.assert_async().await;
        detail_ok.assert_async().await;
        update.assert_async().await;
        post.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }

    #[tokio::test]
    async fn local_graph_failure_stops_before_any_server_call() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        let file = root.path().join("assistant.yaml");
        let broken = assistant_yaml().replace(
            "  skills: []",
            "  skills:\n    - project: project-a\n      name: missing-skill",
        );
        fs::write(&file, broken).unwrap();

        let error = apply(command(root.path(), &file, "http://127.0.0.1:9"))
            .await
            .expect_err("unresolved repository reference must fail locally");
        assert!(matches!(error, AppError::Schema(_)), "got {error:?}");
        assert_eq!(error.exit_code(), 2);
    }

    #[tokio::test]
    async fn compatibility_failure_has_exact_prewrite_count_and_empty_stdout() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        let file = root.path().join("assistant.yaml");
        fs::write(&file, assistant_yaml()).unwrap();

        let mut server = mockito::Server::new_async().await;
        let info = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"version":"wrong-version"}"#)
            .expect(0)
            .create_async()
            .await;
        let visibility = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_body(r#"{"user_id":"user-1","projects":[{"name":"project-a"}]}"#)
            .expect(1)
            .create_async()
            .await;
        let resolve = server
            .mock("GET", "/v1/assistants/slug/assistant-a?project=project-a")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"assistant-server-id"}"#)
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

        let error = apply(command(root.path(), &file, &server.url()))
            .await
            .expect_err("invalid direct-lookup evidence must stop before write");
        assert!(matches!(error, AppError::ApiIncompatible(_)));

        let mut renderer = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        renderer
            .emit_diagnostic(&diagnostic_from_app_error(&error))
            .unwrap();
        let (stdout, stderr) = renderer.into_writers();
        assert!(stdout.is_empty(), "failure path must leave stdout empty");
        assert!(!stderr.is_empty(), "failure path must emit one diagnostic");

        info.assert_async().await;
        visibility.assert_async().await;
        resolve.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
        patch.assert_async().await;
        delete.assert_async().await;
    }
}
