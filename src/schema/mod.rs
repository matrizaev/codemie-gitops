/// Declaration schema validation.
///
/// The closed v1alpha1 JSON Schema is bundled at compile time so that lint and
/// schema validation are entirely offline. The schema is authoritative; no live
/// schema discovery or default insertion occurs (ADR-001; contracts/cli.md §4).
///
/// YAML resource budgets enforced before AST allocation (SEC-003, F-004 scope):
/// - Per-file byte limit: 1 MiB before parsing.
/// - Nesting depth: 32 levels.
/// - Alias/anchor expanded node limit: 1,000.
/// - Scalar length: 128 KiB.
/// - Collection member limit: 10,000 per array/object.
use crate::error::AppError;

/// The canonical v1alpha1 declaration JSON Schema, embedded at compile time
/// from the checked-in contract artifact.
///
/// The path is relative to this source file's location inside the workspace.
pub const DECLARATION_SCHEMA_JSON: &str =
    include_str!("../../specs/codemie-cicd-tool/contracts/declaration-v1alpha1.schema.json");

/// Validate a declaration value against the bundled v1alpha1 schema.
///
/// Compiles the bundled schema on first call, validates `value`, and collects
/// all validation errors. Returns `Ok(())` when the value is fully conformant
/// with the closed v1alpha1 schema. Returns `AppError::Schema` (exit 2) when
/// any schema violation is found. Returns `AppError::Internal` when the bundled
/// schema fails to compile (indicating a defect in the schema artifact).
///
/// Note: the full YAML safety pipeline (injection scan, resource budgets,
/// sidecar expansion) is implemented in `crate::parse`. Call
/// `parse::parse_and_validate` in preference to this function when the input
/// arrives as raw YAML.
pub fn validate_declaration(value: &serde_json::Value) -> Result<(), AppError> {
    let schema_json: serde_json::Value =
        serde_json::from_str(DECLARATION_SCHEMA_JSON).map_err(|e| {
            AppError::Internal(format!("bundled declaration schema is not valid JSON: {e}"))
        })?;

    let validator = jsonschema::validator_for(&schema_json).map_err(|e| {
        AppError::Internal(format!("bundled declaration schema failed to compile: {e}"))
    })?;

    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::Schema(errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_schema_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(DECLARATION_SCHEMA_JSON)
            .expect("embedded declaration schema must be valid JSON");
        // Verify the top-level structure matches the known schema shape.
        assert!(
            parsed.get("$schema").is_some(),
            "schema must contain a '$schema' key"
        );
        assert!(
            parsed.get("oneOf").is_some() || parsed.get("$defs").is_some(),
            "schema must contain 'oneOf' or '$defs'"
        );
    }

    #[test]
    fn embedded_schema_has_correct_id() {
        let parsed: serde_json::Value = serde_json::from_str(DECLARATION_SCHEMA_JSON).unwrap();
        let id = parsed["$id"].as_str().unwrap_or("");
        assert!(
            id.contains("codemie"),
            "schema $id should reference codemie, got: {id}"
        );
    }
}
