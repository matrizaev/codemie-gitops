#[derive(Debug, thiserror::Error)]
pub(crate) enum ApplicationError {
    #[error("{operation} worker failed")]
    Worker {
        operation: &'static str,
        #[source]
        source: tokio::task::JoinError,
    },
    #[error("validated output context could not be constructed")]
    OutputField(#[from] crate::render::InvalidOutputField),
    #[error("{operation} JSON encoding failed")]
    JsonEncoding {
        operation: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("{operation} JSON decoding failed")]
    JsonDecoding {
        operation: &'static str,
        #[source]
        source: serde_json::Error,
    },
}

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
    /// Typed configuration-source or validated-origin failure.
    #[error("configuration error: {0}")]
    ConfigurationLayer(#[from] crate::config::ConfigError),

    /// Typed parser/DTO conversion failure with its source retained.
    #[error("schema error: {0}")]
    ParseLayer(#[from] crate::parse::ParseError),

    /// Typed HTTP transport/response failure with its source retained.
    #[error("transport error: {0}")]
    TransportLayer(#[from] crate::http::TransportError),

    /// Typed authentication transport/decoding failure.
    #[error("authentication error: {0}")]
    AuthLayer(#[from] crate::auth::AuthError),

    /// Typed save projection and direct-output failure with its source retained.
    #[error("save error: {0}")]
    SaveLayer(#[from] crate::save::publication::SaveError),

    /// Typed request-projection failure with its source retained.
    #[error("projection error: {0}")]
    ProjectionLayer(#[from] crate::projection::ProjectionError),

    /// Typed explicit-input failure with its source retained.
    #[error("input error: {0}")]
    InputLayer(#[from] crate::input::InputError),

    /// Typed orchestration failure with worker/encoding sources retained.
    #[error("application error: {0}")]
    ApplicationLayer(#[from] ApplicationError),

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

    /// E_SERVER_REJECTED (exit 1): the server rejected a modifying request
    /// after local input had passed validation.
    #[error("server rejected write: {0}")]
    ServerRejected(String),

    /// E_WRITE_UNCERTAIN (exit 1): a modifying request may have committed but
    /// the invocation cannot safely report success.
    #[error("write result uncertain: {0}")]
    WriteUncertain(String),

    /// E_WRITE_VERIFICATION_UNAVAILABLE (exit 2): post-write verification
    /// could not complete because connectivity/server availability was lost.
    #[error("write verification unavailable: {0}")]
    WriteVerificationUnavailable(String),

    /// E_WRITE_VERIFICATION_INCOMPATIBLE (exit 2): post-write verification
    /// received a response outside the pinned contract.
    #[error("write verification incompatible: {0}")]
    WriteVerificationIncompatible(String),

    /// E_TIMEOUT (exit 2): the whole invocation exceeded its deadline.
    #[error("invocation timeout: {0}")]
    Timeout(String),

    /// E_INTERNAL (exit 2): local invariant violation.
    #[error("internal error: {0}")]
    Internal(String),

    /// E_RECONCILIATION (exit 1): ambiguous identity, adoption required,
    /// resolution instability, or uncertain write.
    #[error("reconciliation error: {0}")]
    Reconciliation(String),

    /// E_ENTITY_NOT_FOUND (exit 1): the selected server entity does not exist.
    #[error("selected entity was not found")]
    EntityNotFound,

    /// E_ENTITY_NOT_EXPORTABLE (exit 1): server state cannot be reconstructed safely.
    #[error("selected entity is not exportable")]
    EntityNotExportable,

    /// E_WORKFLOW_ALREADY_MARKED (exit 1): explicit ID selection found a marker.
    #[error("workflow is already marked; use natural-key selection")]
    WorkflowAlreadyMarked,
}

impl AppError {
    /// Maps the error variant to the CLI contract exit code.
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::TransportLayer(error) if error.is_write_uncertain() => 1,
            AppError::Reconciliation(_)
            | AppError::EntityNotFound
            | AppError::EntityNotExportable
            | AppError::WorkflowAlreadyMarked
            | AppError::ServerRejected(_)
            | AppError::WriteUncertain(_) => 1,
            _ => 2,
        }
    }

    #[cfg(test)]
    pub(crate) fn is_api_incompatible(&self) -> bool {
        matches!(self, Self::ApiIncompatible(_))
            || matches!(self, Self::TransportLayer(error) if error.is_compatibility())
    }

    #[cfg(test)]
    pub(crate) fn is_connectivity(&self) -> bool {
        matches!(self, Self::Connectivity(_))
            || matches!(self, Self::AuthLayer(error) if error.is_connectivity())
            || matches!(self, Self::TransportLayer(error) if !error.is_compatibility() && !error.is_write_uncertain() && !error.is_internal())
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
