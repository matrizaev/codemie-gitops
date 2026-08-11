//! Offline lint coordinator (F-005).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cancellation::CancellationToken;
use crate::discovery::MAX_SOURCE_FILE_PATH_BYTES;
use crate::error::AppError;
use crate::output::{Action, Outcome, OutputMode};
use crate::parse::{EntityKind, ParsedDeclaration};
use crate::render::{Renderer, WarningCategory, WarningCode, WarningInput, WarningSource};
use crate::repository::{TargetLoadRequest, load_target_declaration};

const INVOCATION_DEADLINE: Duration = Duration::from_secs(300);
const MAX_WARNING_FIELD_PATH_LENGTH: usize = 1024;

/// Validated inputs for one offline lint invocation.
pub struct LintCommand {
    pub file: PathBuf,
    pub repo_root: PathBuf,
    pub default_project: Option<String>,
    pub follow_symlinks: bool,
}

/// Successful lint result: one closed outcome plus zero or more safe warnings.
pub struct LintResult {
    outcome: Outcome,
    warnings: Vec<WarningInput>,
}

impl LintResult {
    pub fn write(self, mode: OutputMode) {
        let mut renderer = Renderer::new(std::io::stdout(), std::io::stderr(), mode);
        for warning in &self.warnings {
            let _ = renderer.emit_warning(warning);
        }
        let _ = renderer.flush();
        self.outcome.write(mode);
    }
}

/// Validate the target and complete repository closure without network access.
pub async fn lint(command: LintCommand) -> Result<LintResult, AppError> {
    let cancellation = CancellationToken::default();
    let worker_cancellation = cancellation.clone();
    let repo_root = command.repo_root.clone();
    let worker = tokio::task::spawn_blocking(move || {
        let declaration = load_target_declaration(
            TargetLoadRequest {
                file: &command.file,
                repo_root: &command.repo_root,
                default_project: command.default_project.as_deref(),
                follow_symlinks: command.follow_symlinks,
            },
            &worker_cancellation,
        )?;
        let outcome = valid_outcome(&declaration)?;
        // v27: the complete discovered repository is validation/reference
        // context, while non-fatal warnings belong only to the selected target.
        // Loading has already completed graph closure, so a closure failure can
        // never be mixed with warning output.
        let warnings = collect_declaration_warnings(&declaration, &repo_root)?;
        Ok::<_, AppError>(LintResult { outcome, warnings })
    });

    tokio::select! {
        result = worker => result
            .map_err(|_| AppError::Internal("offline lint worker failed".into()))?,
        _ = tokio::time::sleep(INVOCATION_DEADLINE) => {
            cancellation.cancel();
            Err(AppError::Timeout("lint exceeded the 300-second deadline".into()))
        }
    }
}

fn valid_outcome(declaration: &ParsedDeclaration) -> Result<Outcome, AppError> {
    let project = required_string(&declaration.value, "/metadata/project")?;
    match declaration.kind {
        EntityKind::Assistant => Ok(Outcome::new(
            Action::Valid,
            "Assistant".to_owned(),
            project,
            required_string(&declaration.value, "/metadata/slug")?,
        )),
        EntityKind::Workflow => Ok(Outcome::new(
            Action::Valid,
            "Workflow".to_owned(),
            project,
            required_string(&declaration.value, "/metadata/slug")?,
        )),
        EntityKind::Skill => Ok(Outcome::new_skill(
            Action::Valid,
            project,
            required_string(&declaration.value, "/metadata/name")?,
        )),
        EntityKind::Datasource => Ok(Outcome::new_datasource(
            Action::Valid,
            project,
            required_string(&declaration.value, "/metadata/repo_name")?,
        )),
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

// Warnings are intentionally declaration-scoped: v27 emits them only for the
// `--file` target, never for declarations loaded solely as graph context.
fn collect_declaration_warnings(
    declaration: &ParsedDeclaration,
    repo_root: &Path,
) -> Result<Vec<WarningInput>, AppError> {
    let source_file = safe_source_file(&declaration.source_path, repo_root)?;
    let mut warnings = Vec::new();

    if declaration.kind == EntityKind::Workflow
        && declaration
            .value
            .pointer("/spec/mode")
            .and_then(serde_json::Value::as_str)
            == Some("Autonomous")
    {
        warnings.push(WarningInput {
            warning_code: WarningCode::WDeprecatedValue,
            category: WarningCategory::DeprecatedValue,
            source: WarningSource {
                file: source_file.clone(),
                line: None,
                column: None,
                field_path: "spec.mode".to_owned(),
            },
        });
    }

    let mut suspicious_paths = BTreeSet::new();
    collect_suspicious_paths(
        &declaration.value,
        &CanonicalWarningPath::root(),
        &mut suspicious_paths,
    );
    warnings.extend(suspicious_paths.into_iter().map(|field_path| WarningInput {
        warning_code: WarningCode::WSuspectedPlaintextSecret,
        category: WarningCategory::SecretLikeField,
        source: WarningSource {
            file: source_file.clone(),
            line: None,
            column: None,
            field_path,
        },
    }));

    warnings.sort_by(|left, right| {
        left.warning_code
            .as_str()
            .as_bytes()
            .cmp(right.warning_code.as_str().as_bytes())
            .then_with(|| {
                left.source
                    .field_path
                    .as_bytes()
                    .cmp(right.source.field_path.as_bytes())
            })
    });

    Ok(warnings)
}

fn safe_source_file(source_path: &Path, repo_root: &Path) -> Result<String, AppError> {
    let abbreviated = source_path.strip_prefix(repo_root).unwrap_or(source_path);
    let candidate = abbreviated.to_str().ok_or_else(|| {
        AppError::Schema("source file path is not valid UTF-8 for safe output".into())
    })?;
    if !candidate.is_empty() && candidate.len() <= MAX_SOURCE_FILE_PATH_BYTES {
        return Ok(candidate.to_owned());
    }
    let basename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::Schema("source file path cannot be represented safely".into()))?;
    if basename.is_empty() || basename.len() > MAX_SOURCE_FILE_PATH_BYTES {
        return Err(AppError::Schema(
            "source file path exceeds the safe output limit".into(),
        ));
    }
    Ok(basename.to_owned())
}

#[derive(Clone)]
struct CanonicalWarningPath {
    rendered: String,
    extendable: bool,
}

impl CanonicalWarningPath {
    fn root() -> Self {
        Self {
            rendered: String::new(),
            extendable: true,
        }
    }

    fn child_key(&self, key: &str) -> Self {
        if !self.extendable || !safe_field_segment(key) {
            return Self {
                rendered: self.rendered.clone(),
                extendable: false,
            };
        }
        let rendered = if self.rendered.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.rendered)
        };
        if rendered.len() > MAX_WARNING_FIELD_PATH_LENGTH {
            return Self {
                rendered: self.rendered.clone(),
                extendable: false,
            };
        }
        Self {
            rendered,
            extendable: true,
        }
    }

    fn child_index(&self, index: usize) -> Self {
        if !self.extendable {
            return self.clone();
        }
        let rendered = format!("{}[{index}]", self.rendered);
        if rendered.len() > MAX_WARNING_FIELD_PATH_LENGTH {
            return Self {
                rendered: self.rendered.clone(),
                extendable: false,
            };
        }
        Self {
            rendered,
            extendable: true,
        }
    }

    fn warning_path(&self) -> Option<String> {
        (!self.rendered.is_empty()).then(|| self.rendered.clone())
    }
}

fn safe_field_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn collect_suspicious_paths(
    value: &serde_json::Value,
    path: &CanonicalWarningPath,
    output: &mut BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort_unstable();
            for key in keys {
                let child_path = path.child_key(key);
                let child = &object[key];
                if credential_field_name(key)
                    && child.as_str().is_some_and(resembles_plaintext_secret)
                    && let Some(warning_path) = child_path.warning_path()
                {
                    output.insert(warning_path);
                }
                collect_suspicious_paths(child, &child_path, output);
            }
        }
        serde_json::Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                collect_suspicious_paths(child, &path.child_index(index), output);
            }
        }
        _ => {}
    }
}

fn credential_field_name(field: &str) -> bool {
    let normalized: String = field
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    ["token", "secret", "password", "apikey", "credential"]
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn resembles_plaintext_secret(value: &str) -> bool {
    if value.len() < 20 {
        return false;
    }
    let mut classes = [false; 4];
    let mut distinct = BTreeSet::new();
    for character in value.chars() {
        distinct.insert(character);
        match character {
            value if value.is_ascii_lowercase() => classes[0] = true,
            value if value.is_ascii_uppercase() => classes[1] = true,
            value if value.is_ascii_digit() => classes[2] = true,
            _ => classes[3] = true,
        }
    }
    distinct.len() >= 12 && classes.into_iter().filter(|present| *present).count() >= 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_entropy_credential_value_is_detected_without_retaining_value() {
        let secret = "A9!vK2@qP7#xR4$mN8%z";
        assert!(resembles_plaintext_secret(secret));
        let mut paths = BTreeSet::new();
        collect_suspicious_paths(
            &serde_json::json!({"spec": {"api_key": secret}}),
            &CanonicalWarningPath::root(),
            &mut paths,
        );
        assert_eq!(paths.into_iter().collect::<Vec<_>>(), ["spec.api_key"]);
    }

    #[test]
    fn ordinary_long_text_is_not_treated_as_a_credential() {
        let mut paths = BTreeSet::new();
        collect_suspicious_paths(
            &serde_json::json!({"spec": {"description": "A9!vK2@qP7#xR4$mN8%z"}}),
            &CanonicalWarningPath::root(),
            &mut paths,
        );
        assert!(paths.is_empty());
    }

    #[test]
    fn unsafe_dynamic_key_collapses_to_safe_structural_ancestor() {
        let secret = "A9!vK2@qP7#xR4$mN8%z";
        let long_key = format!("api_key{}", "x".repeat(MAX_WARNING_FIELD_PATH_LENGTH));
        for key in [
            long_key,
            "api\u{0001}key".to_owned(),
            "api\u{202e}key".to_owned(),
        ] {
            let mut paths = BTreeSet::new();
            collect_suspicious_paths(
                &serde_json::json!({"spec": {"custom_metadata": {key: secret}}}),
                &CanonicalWarningPath::root(),
                &mut paths,
            );
            assert_eq!(
                paths.into_iter().collect::<Vec<_>>(),
                ["spec.custom_metadata"]
            );
        }
    }
}
