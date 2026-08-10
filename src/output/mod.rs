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
/// All field values must have passed schema validation before construction.
/// Only the applicable natural-key field for the kind is serialized; the
/// others are absent (additionalProperties: false per outcome.schema.json).
///
/// Security (SEC-005): field values must come from schema-validated fields only.
#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    action: Action,
    kind: String,
    project: String,
    /// Present for Assistant and Workflow; absent for Skill and Datasource.
    #[serde(skip_serializing_if = "Option::is_none")]
    slug: Option<String>,
    /// Present for Skill; absent for all other kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Present for Datasource; absent for all other kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_name: Option<String>,
}

impl Outcome {
    /// Construct an outcome for an Assistant or Workflow (natural key: `slug`).
    ///
    /// `kind`, `project`, and `slug` must be schema-validated values from
    /// the declaration AST — never from untrusted raw strings.
    pub fn new(action: Action, kind: String, project: String, slug: String) -> Self {
        Outcome {
            action,
            kind,
            project,
            slug: Some(slug),
            name: None,
            repo_name: None,
        }
    }

    /// Construct an outcome for a Skill (natural key: `name`).
    pub fn new_skill(action: Action, project: String, name: String) -> Self {
        Outcome {
            action,
            kind: "Skill".to_owned(),
            project,
            slug: None,
            name: Some(name),
            repo_name: None,
        }
    }

    /// Construct an outcome for a Datasource (natural key: `repo_name`).
    pub fn new_datasource(action: Action, project: String, repo_name: String) -> Self {
        Outcome {
            action,
            kind: "Datasource".to_owned(),
            project,
            slug: None,
            name: None,
            repo_name: Some(repo_name),
        }
    }

    /// Returns the natural key value for this outcome.
    fn key_value(&self) -> &str {
        if let Some(s) = &self.slug {
            return s;
        }
        if let Some(n) = &self.name {
            return n;
        }
        if let Some(r) = &self.repo_name {
            return r;
        }
        // Invariant: at least one key field must be set; unreachable via
        // the public constructors.
        ""
    }

    /// Write the outcome to stdout according to the selected mode.
    ///
    /// Text mode: fixed template `<action> <kind> <project>/<key>\n`
    /// where `key` is the kind's natural key.
    /// JSON mode: compact JSON object produced by serde_json, one line.
    ///
    /// Control characters and bidi characters are excluded from identity
    /// fields by schema validation before this point (SEC-005).
    pub fn write(&self, mode: OutputMode) {
        match mode {
            OutputMode::Text => {
                println!(
                    "{} {} {}/{}",
                    self.action.as_str(),
                    self.kind,
                    self.project,
                    self.key_value()
                );
            }
            OutputMode::Json => {
                // serde_json serializer ensures control characters are
                // JSON-escaped and skip_serializing_if keeps only the
                // applicable key field (SEC-005, outcome.schema.json).
                let json = serde_json::to_string(self)
                    .unwrap_or_else(|_| r#"{"error":"serialization failure"}"#.to_owned());
                println!("{json}");
            }
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
