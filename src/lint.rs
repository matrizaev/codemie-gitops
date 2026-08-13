//! Offline validation for exactly one selected declaration.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::domain::{InputFile, NaturalIdentity};
use crate::error::AppError;
use crate::output::{Action, Outcome, OutputMode};
use crate::parse::{EntityKind, ParsedDeclaration};
use crate::render::{Renderer, WarningCategory, WarningCode, WarningInput, WarningSource};

const INVOCATION_DEADLINE: Duration = Duration::from_secs(300);

/// Untrusted CLI input for one lint invocation.
pub(crate) struct RawLintCommand {
    pub(crate) file: PathBuf,
}

/// Validated lint command.
pub(crate) struct LintCommand {
    file: InputFile,
}

impl TryFrom<RawLintCommand> for LintCommand {
    type Error = AppError;

    fn try_from(raw: RawLintCommand) -> Result<Self, Self::Error> {
        Ok(Self {
            file: InputFile::try_from(raw.file)
                .map_err(|_| AppError::Usage("--file is required".into()))?,
        })
    }
}

/// Successful lint result with declaration-scoped warnings.
pub(crate) struct LintResult {
    outcome: Outcome,
    warnings: Vec<WarningInput>,
}

impl LintResult {
    pub(crate) fn write(self, mode: OutputMode) -> std::io::Result<()> {
        let mut renderer = Renderer::new(std::io::stdout(), std::io::stderr(), mode);
        for warning in &self.warnings {
            renderer.emit_warning(warning)?;
        }
        renderer.flush()?;
        self.outcome.write(mode)
    }
}

/// Validate one selected declaration without network access.
pub(crate) async fn lint(command: LintCommand) -> Result<LintResult, AppError> {
    let file = command.file.as_path().to_owned();
    tokio::time::timeout(
        INVOCATION_DEADLINE,
        tokio::task::spawn_blocking(move || {
            let loaded = crate::input::load(&file)?;
            let outcome = valid_outcome(&loaded.declaration)?;
            let warnings = collect_warnings(&loaded.declaration, &file)?;
            Ok::<_, AppError>(LintResult { outcome, warnings })
        }),
    )
    .await
    .map_err(|_| AppError::Timeout("lint exceeded the 300-second deadline".into()))?
    .map_err(|source| crate::error::ApplicationError::Worker {
        operation: "offline lint",
        source,
    })?
}

fn valid_outcome(declaration: &ParsedDeclaration) -> Result<Outcome, AppError> {
    let identity = NaturalIdentity::try_from(declaration)?;
    let project = identity.project().to_owned();
    Ok(match identity.kind() {
        EntityKind::Assistant => {
            Outcome::assistant(Action::Valid, project, identity.selector().to_owned())
        }
        EntityKind::Workflow => {
            Outcome::workflow(Action::Valid, project, identity.selector().to_owned())
        }
        EntityKind::Skill => {
            Outcome::new_skill(Action::Valid, project, identity.selector().to_owned())
        }
        EntityKind::Datasource => {
            Outcome::new_datasource(Action::Valid, project, identity.selector().to_owned())
        }
    })
}

fn collect_warnings(
    declaration: &ParsedDeclaration,
    selected: &Path,
) -> Result<Vec<WarningInput>, AppError> {
    let source_file = selected
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && name.len() <= 4096)
        .ok_or_else(|| AppError::Schema("selected filename cannot be represented safely".into()))?
        .to_owned();
    let mut warnings = Vec::new();
    if declaration.workflow_is_autonomous() {
        warnings.push(WarningInput::new(
            WarningCode::WDeprecatedValue,
            WarningCategory::DeprecatedValue,
            WarningSource::try_new(source_file.clone(), None, None, "spec.mode".to_owned())
                .map_err(crate::error::ApplicationError::from)?,
        ));
    }
    for field_path in declaration.suspicious_secret_paths() {
        warnings.push(WarningInput::new(
            WarningCode::WSuspectedPlaintextSecret,
            WarningCategory::SecretLikeField,
            WarningSource::try_new(source_file.clone(), None, None, field_path)
                .map_err(crate::error::ApplicationError::from)?,
        ));
    }
    warnings.sort_by(|left, right| {
        left.warning_code()
            .as_str()
            .cmp(right.warning_code().as_str())
            .then_with(|| left.field_path().cmp(right.field_path()))
    });
    Ok(warnings)
}
