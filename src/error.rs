/// Top-level error enum for all application error categories.
///
/// Each variant maps to an exit code and error code per the CLI contract exit
/// and stream taxonomy (contracts/cli.md §7).
///
/// Variants whose exit code is 2 cover: usage errors, configuration errors,
/// schema/parse errors, auth errors, connectivity, API incompatibility, and
/// internal invariant violations.
///
/// Variants with exit code 1 cover reconciliation and server-rejection
/// conditions (ambiguity, adoption required, uncertain write, etc.).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// E_USAGE (exit 2): unrecognised or forbidden CLI flag/option.
    #[error("usage error: {0}")]
    Usage(String),

    /// E_CONFIGURATION (exit 2): invalid or missing configuration value,
    /// including URL validation failures.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// E_SCHEMA (exit 2): declaration fails closed schema or semantic rules.
    #[error("schema error: {0}")]
    Schema(String),

    /// E_YAML_PARSE (exit 2): YAML is not well-formed.
    #[error("YAML parse error: {0}")]
    YamlParse(String),

    /// E_AUTHENTICATION (exit 2): credential or auth-endpoint failure.
    #[error("authentication error: {0}")]
    Authentication(String),

    /// E_AUTHORIZATION (exit 2): principal lacks required permission.
    #[error("authorization error: {0}")]
    Authorization(String),

    /// E_VISIBILITY_UNPROVEN (exit 2): Datasource visibility preflight failed.
    #[error("visibility unproven: {0}")]
    VisibilityUnproven(String),

    /// E_CONNECTIVITY (exit 2): network unreachable, timeout, or TLS failure.
    #[error("connectivity error: {0}")]
    Connectivity(String),

    /// E_API_INCOMPATIBLE (exit 2): server response violates the pinned contract.
    #[error("API incompatible: {0}")]
    ApiIncompatible(String),

    /// E_INTERNAL (exit 2): local invariant violation.
    #[error("internal error: {0}")]
    Internal(String),

    /// E_RECONCILIATION (exit 1): ambiguous identity, adoption required,
    /// resolution instability, or uncertain write.
    #[error("reconciliation error: {0}")]
    Reconciliation(String),
}

impl AppError {
    /// Maps the error variant to the CLI contract exit code.
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Reconciliation(_) => 1,
            _ => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_exits_2() {
        assert_eq!(AppError::Usage("bad flag".into()).exit_code(), 2);
    }

    #[test]
    fn configuration_exits_2() {
        assert_eq!(AppError::Configuration("bad url".into()).exit_code(), 2);
    }

    #[test]
    fn reconciliation_exits_1() {
        assert_eq!(AppError::Reconciliation("ambiguous".into()).exit_code(), 1);
    }

    #[test]
    fn schema_exits_2() {
        assert_eq!(AppError::Schema("missing field".into()).exit_code(), 2);
    }

    #[test]
    fn internal_exits_2() {
        assert_eq!(AppError::Internal("invariant".into()).exit_code(), 2);
    }
}
