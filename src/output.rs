/// Output rendering boundary: typed stdout outcome and typed stderr diagnostic.
///
/// This module enforces the closed output contract from contracts/cli.md §7–§10
/// and contracts/outcome.schema.json / diagnostic.schema.json.
///
/// Security invariants (SEC-005):
/// - JSON output is produced by `serde_json`, never by string concatenation.
/// - No raw URL, body, server text, credential, declaration value, or exception
///   string may enter output.
/// - Each record produces exactly one physical output line.
/// - `action`, `kind`, `category`, `errorCode` come from fixed enums.
use std::io;

use serde::Serialize;

use crate::error::AppError;

/// Output mode selected by --output flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Text,
    Json,
}

/// Parse failure for `--output`.
#[derive(Debug)]
pub struct OutputModeParseError(String);

impl std::fmt::Display for OutputModeParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OutputModeParseError {}

impl std::str::FromStr for OutputMode {
    type Err = OutputModeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(OutputMode::Text),
            "json" => Ok(OutputMode::Json),
            other => Err(OutputModeParseError(format!(
                "unknown output mode '{other}'; expected 'text' or 'json'"
            ))),
        }
    }
}

/// The fixed set of success actions per outcome.schema.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Valid,
    Created,
    Updated,
    Saved,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Valid => "valid",
            Action::Created => "created",
            Action::Updated => "updated",
            Action::Saved => "saved",
        }
    }
}

/// A closed success outcome conforming to outcome.schema.json.
///
/// All field values must have passed schema validation before construction.
/// Only the applicable natural-key field for the kind is serialized; the
/// others are absent (additionalProperties: false per outcome.schema.json).
///
/// Security (SEC-005): field values must come from schema-validated fields only.
#[derive(Debug, Clone)]
pub struct Outcome {
    action: Action,
    identity: OutcomeIdentity,
    adoption_required: Option<bool>,
}

#[derive(Debug, Clone)]
enum OutcomeIdentity {
    Assistant { project: String, slug: String },
    Workflow { project: String, slug: String },
    Skill { project: String, name: String },
    Datasource { project: String, repo_name: String },
}

impl Serialize for Outcome {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct(
            "Outcome",
            if self.adoption_required.is_some() {
                5
            } else {
                4
            },
        )?;
        state.serialize_field("action", &self.action)?;
        match &self.identity {
            OutcomeIdentity::Assistant { project, slug } => {
                state.serialize_field("kind", "Assistant")?;
                state.serialize_field("project", project)?;
                state.serialize_field("slug", slug)?;
            }
            OutcomeIdentity::Workflow { project, slug } => {
                state.serialize_field("kind", "Workflow")?;
                state.serialize_field("project", project)?;
                state.serialize_field("slug", slug)?;
            }
            OutcomeIdentity::Skill { project, name } => {
                state.serialize_field("kind", "Skill")?;
                state.serialize_field("project", project)?;
                state.serialize_field("name", name)?;
            }
            OutcomeIdentity::Datasource { project, repo_name } => {
                state.serialize_field("kind", "Datasource")?;
                state.serialize_field("project", project)?;
                state.serialize_field("repo_name", repo_name)?;
            }
        }
        if let Some(adoption_required) = self.adoption_required {
            state.serialize_field("adoptionRequired", &adoption_required)?;
        }
        state.end()
    }
}

impl Outcome {
    /// Construct an Assistant outcome (natural key: `slug`).
    pub fn assistant(action: Action, project: String, slug: String) -> Self {
        Self {
            action,
            identity: OutcomeIdentity::Assistant { project, slug },
            adoption_required: None,
        }
    }

    /// Construct a Workflow outcome (natural key: `slug`).
    pub fn workflow(action: Action, project: String, slug: String) -> Self {
        Self {
            action,
            identity: OutcomeIdentity::Workflow { project, slug },
            adoption_required: None,
        }
    }

    /// Construct an outcome for a Skill (natural key: `name`).
    pub fn new_skill(action: Action, project: String, name: String) -> Self {
        Outcome {
            action,
            identity: OutcomeIdentity::Skill { project, name },
            adoption_required: None,
        }
    }

    /// Construct an outcome for a Datasource (natural key: `repo_name`).
    pub fn new_datasource(action: Action, project: String, repo_name: String) -> Self {
        Outcome {
            action,
            identity: OutcomeIdentity::Datasource { project, repo_name },
            adoption_required: None,
        }
    }

    pub fn saved_workflow(project: String, slug: String, adoption_required: bool) -> Self {
        Self {
            action: Action::Saved,
            identity: OutcomeIdentity::Workflow { project, slug },
            adoption_required: adoption_required.then_some(true),
        }
    }

    /// Write the outcome to stdout according to the selected mode.
    ///
    /// Text mode: fixed template `<action> <kind> <project>/<key>\n`
    /// where `key` is the kind's natural key.
    /// JSON mode: compact JSON object produced by serde_json, one line.
    ///
    /// Control characters and bidi characters are excluded from identity
    /// fields by schema validation before this point (SEC-005).
    pub fn write(&self, mode: OutputMode) -> io::Result<()> {
        let (kind, project, key) = self.render_identity();
        let mut renderer = crate::render::Renderer::new(io::stdout(), io::stderr(), mode);
        if self.adoption_required == Some(true) {
            renderer.emit_outcome_with_adoption(self.action, kind, project, &key, true)?;
        } else {
            renderer.emit_outcome(self.action, kind, project, &key)?;
        }
        renderer.flush()
    }

    fn render_identity(&self) -> (crate::render::EntityKind, &str, crate::render::EntityKey) {
        match &self.identity {
            OutcomeIdentity::Assistant { project, slug } => (
                crate::render::EntityKind::Assistant,
                project,
                crate::render::EntityKey::Slug(slug.clone()),
            ),
            OutcomeIdentity::Workflow { project, slug } => (
                crate::render::EntityKind::Workflow,
                project,
                crate::render::EntityKey::Slug(slug.clone()),
            ),
            OutcomeIdentity::Skill { project, name } => (
                crate::render::EntityKind::Skill,
                project,
                crate::render::EntityKey::Name(name.clone()),
            ),
            OutcomeIdentity::Datasource { project, repo_name } => (
                crate::render::EntityKind::Datasource,
                project,
                crate::render::EntityKey::RepoName(repo_name.clone()),
            ),
        }
    }
}

/// Write a failure diagnostic to stderr; stdout must remain empty.
///
/// Delegates to `render::write_app_error_to_stderr`. The error message
/// string is discarded — only the closed errorCode/category/exitCode are
/// emitted (SEC-005, F-007).
pub fn write_failure(error: &AppError, mode: OutputMode) {
    crate::render::write_app_error_to_stderr(error, mode);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_mode_from_str_text() {
        assert_eq!("text".parse::<OutputMode>().unwrap(), OutputMode::Text);
    }

    #[test]
    fn output_mode_from_str_json() {
        assert_eq!("json".parse::<OutputMode>().unwrap(), OutputMode::Json);
    }

    #[test]
    fn output_mode_from_str_unknown_errors() {
        assert!("xml".parse::<OutputMode>().is_err());
    }

    #[test]
    fn action_as_str() {
        assert_eq!(Action::Valid.as_str(), "valid");
        assert_eq!(Action::Created.as_str(), "created");
        assert_eq!(Action::Updated.as_str(), "updated");
    }

    #[test]
    fn outcome_new_constructs() {
        let o = Outcome::assistant(Action::Created, "my-project".into(), "my-slug".into());
        assert_eq!(o.action, Action::Created);
        assert!(matches!(o.identity, OutcomeIdentity::Assistant { .. }));
    }

    #[test]
    fn saved_workflow_serializes_adoption_only_when_required() {
        let ordinary = serde_json::to_value(Outcome::saved_workflow(
            "project".into(),
            "flow".into(),
            false,
        ))
        .unwrap();
        assert!(ordinary.get("adoptionRequired").is_none());

        let adoption = serde_json::to_value(Outcome::saved_workflow(
            "project".into(),
            "flow".into(),
            true,
        ))
        .unwrap();
        assert_eq!(adoption["adoptionRequired"], true);
        assert_eq!(adoption.as_object().unwrap().len(), 5);
    }
}
