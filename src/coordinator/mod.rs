/// Single-entity write-through coordinator (R-001).
///
/// One invocation follows the approved state machine:
/// local load/validation -> authentication -> compatibility/visibility
/// preflight -> kind adapter resolve/project/write -> post-write identity
/// verification -> typed success outcome.
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::adapters::{self, ApplyAction, ApplyResult};
use crate::cancellation::CancellationToken;
use crate::config::ValidatedUrl;
use crate::discovery::{discover_yaml_files, load_declaration_file_cancellable};
use crate::error::AppError;
use crate::http::{ApiClient, preflight_visibility};
use crate::output::{Action, Outcome};
use crate::parse::{
    EntityKind, MAX_YAML_FILE_BYTES, ParsedDeclaration, parse_and_validate_cancellable,
};
use crate::preflight::check_compatibility;
use crate::validate::{validate_graph, validate_natural};

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
            &file,
            &repo_root,
            follow_symlinks,
            default_project.as_deref(),
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

    // CompatibilityChecked: target identity version check plus the principal
    // capability proof required before resolution/write. Datasource owns the
    // same preflight inside its adapter, so avoid issuing it twice there.
    check_compatibility(&client, &command.base_url).await?;
    if !matches!(identity, TargetIdentity::Datasource { .. }) {
        preflight_visibility(&client, &command.base_url).await?;
    }

    let result = dispatch_adapter(
        &client,
        &command.base_url,
        &target,
        &identity,
        &command.repo_root,
        command.follow_symlinks,
        command.adopt_workflow_id.as_deref(),
        &cancellation,
    )
    .await?;

    verify_written_identity(&client, &command.base_url, &identity, &result.server_id)
        .await
        .map_err(classify_verification_failure)?;

    Ok(identity.success_outcome(result.action))
}

fn load_repository_declarations(
    repo_root: &Path,
    follow_symlinks: bool,
    default_project: Option<&str>,
    cancellation: &CancellationToken,
) -> Result<Vec<ParsedDeclaration>, AppError> {
    cancellation.checkpoint()?;
    let files = discover_yaml_files(repo_root, follow_symlinks)?;
    let mut declarations = Vec::with_capacity(files.len());
    for file in files {
        cancellation.checkpoint()?;
        if file.byte_len > MAX_YAML_FILE_BYTES as u64 {
            return Err(AppError::YamlParse(
                "declaration exceeds the 1 MiB byte limit".into(),
            ));
        }
        let raw = load_declaration_file_cancellable(
            &file.path,
            repo_root,
            follow_symlinks,
            MAX_YAML_FILE_BYTES as u64,
            cancellation,
        )?;
        let mut declaration = parse_and_validate_cancellable(
            &raw,
            &file.path,
            repo_root,
            follow_symlinks,
            cancellation,
        )?;
        materialize_effective_project(&mut declaration, default_project)?;
        validate_natural(&declaration)?;
        declarations.push(declaration);
    }
    cancellation.checkpoint()?;
    Ok(declarations)
}

fn load_target_declaration(
    file: &Path,
    repo_root: &Path,
    follow_symlinks: bool,
    default_project: Option<&str>,
    cancellation: &CancellationToken,
) -> Result<ParsedDeclaration, AppError> {
    cancellation.checkpoint()?;
    let target_path = std::fs::canonicalize(file)
        .map_err(|_| AppError::Schema("target declaration file is unavailable".into()))?;
    let mut declarations =
        load_repository_declarations(repo_root, follow_symlinks, default_project, cancellation)?;
    validate_graph(&declarations)?;
    cancellation.checkpoint()?;

    let target_index = declarations
        .iter()
        .position(|declaration| declaration.source_path == target_path)
        .ok_or_else(|| {
            AppError::Schema(
                "target file is not a discovered YAML declaration in the repository".into(),
            )
        })?;
    Ok(declarations.swap_remove(target_index))
}

fn materialize_effective_project(
    declaration: &mut ParsedDeclaration,
    default_project: Option<&str>,
) -> Result<(), AppError> {
    let metadata = declaration
        .value
        .get_mut("metadata")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| AppError::Schema("declaration metadata is required".into()))?;
    if metadata.get("project").is_none() {
        let project = default_project
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Schema(
                    "metadata.project is required when repository project is not configured".into(),
                )
            })?;
        metadata.insert(
            "project".to_owned(),
            serde_json::Value::String(project.to_owned()),
        );
    }
    Ok(())
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

async fn dispatch_adapter(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    declaration: &ParsedDeclaration,
    identity: &TargetIdentity,
    repo_root: &Path,
    follow_symlinks: bool,
    adopt_workflow_id: Option<&str>,
    cancellation: &CancellationToken,
) -> Result<ApplyResult, AppError> {
    match identity {
        TargetIdentity::Assistant { project, slug } => {
            adapters::assistant::apply(
                client,
                base_url,
                declaration,
                project,
                slug,
                repo_root,
                follow_symlinks,
            )
            .await
        }
        TargetIdentity::Workflow { project, slug } => {
            adapters::workflow::apply(
                client,
                base_url,
                declaration,
                project,
                slug,
                adopt_workflow_id,
                repo_root,
                follow_symlinks,
            )
            .await
        }
        TargetIdentity::Skill { project, name } => {
            adapters::skill::apply(
                client,
                base_url,
                declaration,
                project,
                name,
                repo_root,
                follow_symlinks,
            )
            .await
        }
        TargetIdentity::Datasource {
            project,
            repo_name,
            index_type,
        } => {
            adapters::datasource::apply_cancellable(
                client,
                base_url,
                declaration,
                project,
                repo_name,
                index_type,
                repo_root,
                follow_symlinks,
                cancellation,
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
"#
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
            .with_body(format!(
                r#"{{"version":"{}"}}"#,
                crate::preflight::EXPECTED_BACKEND_COMMIT
            ))
            .expect(1)
            .create_async()
            .await;
        let visibility = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"project-a","is_project_admin":true}]}"#)
            .expect(1)
            .create_async()
            .await;
        let resolve = server
            .mock("GET", "/v1/assistants/slug/assistant-a?project=project-a")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"assistant-server-id"}"#)
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
            .with_body(format!(
                r#"{{"version":"{}"}}"#,
                crate::preflight::EXPECTED_BACKEND_COMMIT
            ))
            .expect(1)
            .create_async()
            .await;
        let visibility = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"project-a","is_project_admin":true}]}"#)
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
            .with_body(r#"{"id":"assistant-server-id"}"#)
            .expect(1)
            .create_async()
            .await;
        let resolve_created = server
            .mock("GET", "/v1/assistants/slug/assistant-a?project=project-a")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"assistant-server-id"}"#)
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
            .expect(1)
            .create_async()
            .await;
        let visibility = server
            .mock("GET", "/v1/user")
            .expect(0)
            .create_async()
            .await;
        let resolve = server
            .mock("GET", "/v1/assistants/slug/assistant-a?project=project-a")
            .expect(0)
            .create_async()
            .await;
        let update = server
            .mock("PUT", "/v1/assistants/assistant-server-id")
            .expect(0)
            .create_async()
            .await;

        let error = apply(command(root.path(), &file, &server.url()))
            .await
            .expect_err("compatibility mismatch must stop before resolution/write");
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
        update.assert_async().await;
    }
}
