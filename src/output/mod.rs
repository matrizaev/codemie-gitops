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
use serde::Serialize;

use crate::error::AppError;

/// Output mode selected by --output flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Text,
    Json,
}

impl std::str::FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(OutputMode::Text),
            "json" => Ok(OutputMode::Json),
            other => Err(format!("unknown output mode '{other}'; expected 'text' or 'json'")),
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
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Valid => "valid",
            Action::Created => "created",
            Action::Updated => "updated",
        }
    }
}

/// A closed success outcome conforming to outcome.schema.json.
///
/// All field values must have passed schema validation before construction;
/// the struct is intentionally not `pub` constructable by fields — use
/// `Outcome::new`.
#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    action: Action,
    kind: String,
    project: String,
    slug: String,
}

impl Outcome {
    /// Construct a validated outcome record.
    ///
    /// `kind`, `project`, and `slug` must be schema-validated values from
    /// the declaration AST — never from untrusted raw strings.
    pub fn new(action: Action, kind: String, project: String, slug: String) -> Self {
        Outcome { action, kind, project, slug }
    }

    /// Write the outcome to stdout according to the selected mode.
    ///
    /// Text mode: fixed template `<action> <kind> <project>/<slug>\n`.
    /// JSON mode: compact JSON object, one line.
    pub fn write(&self, mode: OutputMode) {
        match mode {
            OutputMode::Text => {
                println!("{} {} {}/{}", self.action.as_str(), self.kind, self.project, self.slug);
            }
            OutputMode::Json => {
                // Serializer ensures control characters are JSON-escaped.
                let json = serde_json::to_string(self)
                    .unwrap_or_else(|_| r#"{"error":"serialization failure"}"#.to_owned());
                println!("{json}");
            }
        }
    }
}

/// Write a failure diagnostic to stderr; stdout must remain empty.
///
/// This is a stub: the full closed diagnostic schema (diagnostic.schema.json)
/// is implemented in F-007.
pub fn write_failure(_error: &AppError, _mode: OutputMode) {
    todo!("diagnostic rendering implemented in F-007")
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
        let o = Outcome::new(
            Action::Created,
            "Assistant".into(),
            "my-project".into(),
            "my-slug".into(),
        );
        assert_eq!(o.action, Action::Created);
        assert_eq!(o.kind, "Assistant");
    }
}
