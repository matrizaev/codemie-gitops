//! One-declaration apply orchestration.

use std::path::PathBuf;
use std::time::Duration;

use secrecy::SecretString;

use crate::adapters::{self, ApplyResult};
use crate::config::ValidatedUrl;
use crate::domain::{InputFile, NaturalIdentity, WorkflowId};
use crate::error::AppError;
use crate::http::ApiClient;
use crate::input::FilePart;
use crate::output::Outcome;
use crate::parse::{EntityKind, ParsedDeclaration};

/// Whole-invocation deadline from the approved resource budget.
pub const INVOCATION_DEADLINE: Duration = Duration::from_secs(300);

/// Untrusted apply input assembled at the CLI boundary.
pub struct RawApplyCommand {
    pub file: PathBuf,
    pub base_url: ValidatedUrl,
    pub token: String,
    pub adopt_workflow_id: Option<String>,
}

/// Validated inputs for one apply invocation.
pub struct ApplyCommand {
    file: InputFile,
    base_url: ValidatedUrl,
    token: SecretString,
    adopt_workflow_id: Option<WorkflowId>,
}

impl TryFrom<RawApplyCommand> for ApplyCommand {
    type Error = AppError;

    fn try_from(raw: RawApplyCommand) -> Result<Self, Self::Error> {
        if raw.token.is_empty() {
            return Err(AppError::Authentication(
                "CODEMIE_TOKEN is required for apply".into(),
            ));
        }
        Ok(Self {
            file: InputFile::try_from(raw.file)
                .map_err(|_| AppError::Usage("--file is required".into()))?,
            base_url: raw.base_url,
            token: raw.token.into(),
            adopt_workflow_id: raw
                .adopt_workflow_id
                .as_deref()
                .map(str::parse::<WorkflowId>)
                .transpose()
                .map_err(|_| {
                    AppError::Usage("--adopt-workflow-id must be a canonical UUID".into())
                })?,
        })
    }
}

/// Apply exactly one selected declaration under the five-minute deadline.
pub async fn apply(command: ApplyCommand) -> Result<Outcome, AppError> {
    tokio::time::timeout(INVOCATION_DEADLINE, apply_inner(command))
        .await
        .map_err(|_| {
            AppError::Timeout(
                "apply exceeded the 300-second deadline; a write may have committed".into(),
            )
        })?
}

async fn apply_inner(command: ApplyCommand) -> Result<Outcome, AppError> {
    let file = command.file.as_path().to_owned();
    let loaded = tokio::task::spawn_blocking(move || crate::input::load(&file))
        .await
        .map_err(|source| crate::error::ApplicationError::Worker {
            operation: "local validation",
            source,
        })??;
    let declaration = loaded.declaration;
    let identity = NaturalIdentity::try_from(&declaration)?;
    validate_adoption_selector(&identity, command.adopt_workflow_id.as_ref())?;
    let client = ApiClient::new(command.base_url.clone(), command.token)?;
    let adopt_workflow_id = command.adopt_workflow_id.map(|id| id.to_string());
    let result = dispatch_adapter(
        &client,
        &declaration,
        &identity,
        adopt_workflow_id.as_deref(),
        loaded.file_parts,
    )
    .await?;
    verify_written_identity(&client, &identity, result.server_id())
        .await
        .map_err(classify_verification_failure)?;
    Ok(identity.success_outcome(result.action()))
}

fn validate_adoption_selector(
    identity: &NaturalIdentity,
    selector: Option<&WorkflowId>,
) -> Result<(), AppError> {
    if selector.is_some() && identity.kind() != EntityKind::Workflow {
        return Err(AppError::Usage(
            "--adopt-workflow-id is valid only for Workflow declarations".into(),
        ));
    }
    Ok(())
}

async fn dispatch_adapter(
    client: &ApiClient,
    declaration: &ParsedDeclaration,
    identity: &NaturalIdentity,
    adopt_workflow_id: Option<&str>,
    file_parts: Vec<FilePart>,
) -> Result<ApplyResult, AppError> {
    match identity.kind() {
        EntityKind::Assistant => {
            adapters::assistant::apply(client, declaration, identity.project(), identity.selector())
                .await
        }
        EntityKind::Workflow => {
            adapters::workflow::apply(
                client,
                adapters::workflow::ApplyRequest {
                    declaration,
                    project_name: identity.project(),
                    slug: identity.selector(),
                    adopt_workflow_id,
                },
            )
            .await
        }
        EntityKind::Skill => {
            adapters::skill::apply(client, declaration, identity.project(), identity.selector())
                .await
        }
        EntityKind::Datasource => {
            let index_type = identity.datasource_index_type().ok_or_else(|| {
                AppError::Internal("Datasource identity is missing its validated index type".into())
            })?;
            adapters::datasource::apply(
                client,
                adapters::datasource::ApplyRequest {
                    declaration,
                    project_name: identity.project(),
                    repo_name: identity.selector(),
                    index_type,
                    file_parts,
                },
            )
            .await
        }
    }
}

async fn verify_written_identity(
    client: &ApiClient,
    identity: &NaturalIdentity,
    expected_server_id: &str,
) -> Result<(), AppError> {
    match identity.kind() {
        EntityKind::Assistant => {
            adapters::assistant::verify_identity(
                client,
                identity.project(),
                identity.selector(),
                expected_server_id,
            )
            .await
        }
        EntityKind::Workflow => {
            adapters::workflow::verify_identity(
                client,
                identity.project(),
                identity.selector(),
                expected_server_id,
            )
            .await
        }
        EntityKind::Skill => {
            adapters::skill::verify_identity(
                client,
                identity.project(),
                identity.selector(),
                expected_server_id,
            )
            .await
        }
        EntityKind::Datasource => {
            let index_type = identity.datasource_index_type().ok_or_else(|| {
                AppError::Internal("Datasource identity is missing its validated index type".into())
            })?;
            adapters::datasource::verify_identity(
                client,
                identity.project(),
                identity.selector(),
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
        AppError::Reconciliation(_) | AppError::ResolutionUnstable(_) => AppError::WriteUncertain(
            "write may have committed; identity did not verify exactly once".into(),
        ),
        // Verification runs strictly after the write dispatched, so any
        // failure here means the write may have committed even when the error
        // is an unrelated class (authorization change, entity not found, etc.).
        other => AppError::WriteVerificationUnavailable(format!(
            "write may have committed; verification failed: {other}"
        )),
    }
}
