/// Closed success and diagnostic renderers (F-007).
///
/// Implements the output boundary from contracts/cli.md §7–§10,
/// outcome.schema.json, diagnostic.schema.json, and warning.schema.json.
///
/// Security invariants (SEC-005, ADR-010):
/// - JSON output is produced by `serde_json`, never by string concatenation.
/// - `action`, `kind`, `category`, `errorCode`, `warningCode` come from fixed
///   enums only; no value is derived from untrusted input.
/// - Each record produces exactly one physical output line terminated by `\n`.
/// - `source.fieldPath` is a caller-provided canonical value, not echoed from
///   arbitrary user input.
/// - No raw URL, body, server text, credential, declaration value, or exception
///   string may enter output through this module.
/// - Stdout is only written to on success paths; stderr is only written to on
///   failure/diagnostic paths.
use std::io::{self, Write};

use serde::Serialize;

use crate::output::{Action, OutputMode};

// ---------------------------------------------------------------------------
// Kind and key
// ---------------------------------------------------------------------------

/// The fixed set of entity kinds per outcome.schema.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EntityKind {
    Assistant,
    Workflow,
    Skill,
    Datasource,
}

impl EntityKind {
    fn as_str(self) -> &'static str {
        match self {
            EntityKind::Assistant => "Assistant",
            EntityKind::Workflow => "Workflow",
            EntityKind::Skill => "Skill",
            EntityKind::Datasource => "Datasource",
        }
    }
}

/// The natural key for an entity: field name and value are kind-specific.
///
/// - Assistant / Workflow: `Slug`
/// - Skill: `Name`
/// - Datasource: `RepoName`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityKey {
    Slug(String),
    Name(String),
    RepoName(String),
}

impl EntityKey {
    /// Returns the JSON field name for this key.
    pub fn field_name(&self) -> &'static str {
        match self {
            EntityKey::Slug(_) => "slug",
            EntityKey::Name(_) => "name",
            EntityKey::RepoName(_) => "repo_name",
        }
    }

    /// Returns the key value.
    pub fn value(&self) -> &str {
        match self {
            EntityKey::Slug(v) => v,
            EntityKey::Name(v) => v,
            EntityKey::RepoName(v) => v,
        }
    }
}

// ---------------------------------------------------------------------------
// Error codes (diagnostic.schema.json)
// ---------------------------------------------------------------------------

/// Closed set of error codes per diagnostic.schema.json.
///
/// Variants are serialized as their exact schema string values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    clippy::enum_variant_names,
    reason = "the complete approved diagnostic.schema.json code set is retained at the typed output boundary"
)]
pub enum ErrorCode {
    // reconciliation (exit 1)
    EAmbiguousIdentity,
    EIdentityMarkerInvalid,
    EResolutionUnstable,
    EAdoptionRequired,
    EMissingReference,
    EWriteUncertain,
    EEntityNotFound,
    EEntityNotExportable,
    EOutputExists,
    EOutputPath,
    EOutputWrite,
    EWorkflowAlreadyMarked,
    // server-rejection (exit 1)
    EServerRejected,
    // usage (exit 2)
    EUsage,
    // local-input (exit 2)
    EYamlParse,
    ESchema,
    ESemantic,
    EReference,
    ESidecar,
    // configuration (exit 2)
    EConfiguration,
    // authentication (exit 2)
    EAuthentication,
    ETokenExpired,
    ELocalAuthUnavailable,
    ERateLimited,
    // authorization (exit 2)
    EAuthorization,
    EVisibilityUnproven,
    EWriteForbidden,
    // compatibility (exit 2)
    EApiIncompatible,
    EWriteVerificationIncompatible,
    // connectivity (exit 2)
    EConnectivity,
    EServerUnavailable,
    ETimeout,
    EWriteVerificationUnavailable,
    // internal (exit 2)
    EInternal,
}

impl ErrorCode {
    /// Returns the exact schema string value for this error code.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::EAmbiguousIdentity => "E_AMBIGUOUS_IDENTITY",
            ErrorCode::EIdentityMarkerInvalid => "E_IDENTITY_MARKER_INVALID",
            ErrorCode::EResolutionUnstable => "E_RESOLUTION_UNSTABLE",
            ErrorCode::EAdoptionRequired => "E_ADOPTION_REQUIRED",
            ErrorCode::EMissingReference => "E_MISSING_REFERENCE",
            ErrorCode::EWriteUncertain => "E_WRITE_UNCERTAIN",
            ErrorCode::EEntityNotFound => "E_ENTITY_NOT_FOUND",
            ErrorCode::EEntityNotExportable => "E_ENTITY_NOT_EXPORTABLE",
            ErrorCode::EOutputExists => "E_OUTPUT_EXISTS",
            ErrorCode::EOutputPath => "E_OUTPUT_PATH",
            ErrorCode::EOutputWrite => "E_OUTPUT_WRITE",
            ErrorCode::EWorkflowAlreadyMarked => "E_WORKFLOW_ALREADY_MARKED",
            ErrorCode::EServerRejected => "E_SERVER_REJECTED",
            ErrorCode::EUsage => "E_USAGE",
            ErrorCode::EYamlParse => "E_YAML_PARSE",
            ErrorCode::ESchema => "E_SCHEMA",
            ErrorCode::ESemantic => "E_SEMANTIC",
            ErrorCode::EReference => "E_REFERENCE",
            ErrorCode::ESidecar => "E_SIDECAR",
            ErrorCode::EConfiguration => "E_CONFIGURATION",
            ErrorCode::EAuthentication => "E_AUTHENTICATION",
            ErrorCode::ETokenExpired => "E_TOKEN_EXPIRED",
            ErrorCode::ELocalAuthUnavailable => "E_LOCAL_AUTH_UNAVAILABLE",
            ErrorCode::ERateLimited => "E_RATE_LIMITED",
            ErrorCode::EAuthorization => "E_AUTHORIZATION",
            ErrorCode::EVisibilityUnproven => "E_VISIBILITY_UNPROVEN",
            ErrorCode::EWriteForbidden => "E_WRITE_FORBIDDEN",
            ErrorCode::EApiIncompatible => "E_API_INCOMPATIBLE",
            ErrorCode::EWriteVerificationIncompatible => "E_WRITE_VERIFICATION_INCOMPATIBLE",
            ErrorCode::EConnectivity => "E_CONNECTIVITY",
            ErrorCode::EServerUnavailable => "E_SERVER_UNAVAILABLE",
            ErrorCode::ETimeout => "E_TIMEOUT",
            ErrorCode::EWriteVerificationUnavailable => "E_WRITE_VERIFICATION_UNAVAILABLE",
            ErrorCode::EInternal => "E_INTERNAL",
        }
    }
}

// ---------------------------------------------------------------------------
// Diagnostic category
// ---------------------------------------------------------------------------

/// Closed set of diagnostic categories per diagnostic.schema.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    Reconciliation,
    ServerRejection,
    Usage,
    LocalInput,
    Configuration,
    Authentication,
    Authorization,
    Compatibility,
    Connectivity,
    Internal,
}

impl DiagnosticCategory {
    /// Returns the exact schema string value for this category.
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCategory::Reconciliation => "reconciliation",
            DiagnosticCategory::ServerRejection => "server-rejection",
            DiagnosticCategory::Usage => "usage",
            DiagnosticCategory::LocalInput => "local-input",
            DiagnosticCategory::Configuration => "configuration",
            DiagnosticCategory::Authentication => "authentication",
            DiagnosticCategory::Authorization => "authorization",
            DiagnosticCategory::Compatibility => "compatibility",
            DiagnosticCategory::Connectivity => "connectivity",
            DiagnosticCategory::Internal => "internal",
        }
    }
}

// ---------------------------------------------------------------------------
// Warning codes and categories
// ---------------------------------------------------------------------------

/// Closed set of warning codes per warning.schema.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    WSuspectedPlaintextSecret,
    WDeprecatedValue,
}

impl WarningCode {
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCode::WSuspectedPlaintextSecret => "W_SUSPECTED_PLAINTEXT_SECRET",
            WarningCode::WDeprecatedValue => "W_DEPRECATED_VALUE",
        }
    }
}

/// Closed set of warning categories per warning.schema.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCategory {
    SecretLikeField,
    DeprecatedValue,
}

impl WarningCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCategory::SecretLikeField => "secret-like-field",
            WarningCategory::DeprecatedValue => "deprecated-value",
        }
    }
}

// ---------------------------------------------------------------------------
// Source location
// ---------------------------------------------------------------------------

/// A source coordinate attached to a diagnostic or warning.
///
/// `file` is bounded to 4,096 bytes (enforced by F-003 callers).
/// `field_path` must be generated canonically from the YAML AST, not echoed
/// from arbitrary input (SEC-005).
#[derive(Debug, Clone)]
pub struct SourceLocation {
    file: SourceFile,
    line: Option<u32>,
    column: Option<u32>,
    field_path: Option<FieldPath>,
}

#[derive(Debug, Clone)]
struct SourceFile(String);

impl TryFrom<String> for SourceFile {
    type Error = InvalidOutputField;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
            return Err(InvalidOutputField::SourceFile);
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone)]
struct FieldPath(String);

impl TryFrom<String> for FieldPath {
    type Error = InvalidOutputField;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
            return Err(InvalidOutputField::FieldPath);
        }
        Ok(Self(value))
    }
}

impl SourceLocation {
    #[allow(
        dead_code,
        reason = "diagnostic source context is part of the approved output contract but not emitted by current commands"
    )]
    pub fn try_new(
        file: String,
        line: Option<u32>,
        column: Option<u32>,
        field_path: Option<String>,
    ) -> Result<Self, InvalidOutputField> {
        Ok(Self {
            file: file.try_into()?,
            line,
            column,
            field_path: field_path.map(FieldPath::try_from).transpose()?,
        })
    }
}

// Serde representation mirrors diagnostic.schema.json "source" object.
#[derive(Serialize)]
struct SourceLocationJson<'a> {
    file: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<u32>,
    #[serde(rename = "fieldPath", skip_serializing_if = "Option::is_none")]
    field_path: Option<&'a str>,
}

impl<'a> From<&'a SourceLocation> for SourceLocationJson<'a> {
    fn from(s: &'a SourceLocation) -> Self {
        SourceLocationJson {
            file: &s.file.0,
            line: s.line,
            column: s.column,
            field_path: s.field_path.as_ref().map(|path| path.0.as_str()),
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP info
// ---------------------------------------------------------------------------

/// HTTP context attached to a diagnostic.
#[derive(Debug, Clone)]
pub struct HttpInfo {
    status: HttpStatus,
    method: HttpMethod,
    route_template: RouteTemplate,
}

#[derive(Debug, Clone, Copy)]
struct HttpStatus(u16);

impl TryFrom<u16> for HttpStatus {
    type Error = InvalidOutputField;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        (100..=599)
            .contains(&value)
            .then_some(Self(value))
            .ok_or(InvalidOutputField::HttpStatus)
    }
}

#[derive(Debug, Clone)]
struct RouteTemplate(String);

impl TryFrom<String> for RouteTemplate {
    type Error = InvalidOutputField;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if !value.starts_with('/')
            || value.contains('?')
            || value.contains('#')
            || value.chars().any(char::is_control)
        {
            return Err(InvalidOutputField::RouteTemplate);
        }
        Ok(Self(value))
    }
}

impl HttpInfo {
    #[allow(
        dead_code,
        reason = "typed HTTP context is part of the approved diagnostic contract but current safe diagnostics omit it"
    )]
    pub fn try_new(
        status: u16,
        method: HttpMethod,
        route_template: String,
    ) -> Result<Self, InvalidOutputField> {
        Ok(Self {
            status: status.try_into()?,
            method,
            route_template: route_template.try_into()?,
        })
    }
}

/// Closed set of HTTP methods per diagnostic.schema.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[allow(
    dead_code,
    reason = "diagnostic.schema.json permits typed HTTP context for all modifying and read methods"
)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
}

#[derive(Serialize)]
struct HttpInfoJson<'a> {
    status: u16,
    method: HttpMethod,
    #[serde(rename = "routeTemplate")]
    route_template: &'a str,
}

impl<'a> From<&'a HttpInfo> for HttpInfoJson<'a> {
    fn from(h: &'a HttpInfo) -> Self {
        HttpInfoJson {
            status: h.status.0,
            method: h.method,
            route_template: &h.route_template.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Diagnostic input
// ---------------------------------------------------------------------------

/// A fully typed diagnostic record ready for rendering.
///
/// Callers must supply only schema-allowlisted fields.
/// No raw URL, body, server text, credential, or declaration value.
pub struct DiagnosticInput {
    error_code: ErrorCode,
    category: DiagnosticCategory,
    /// Must be 1 or 2, per cli.md §7.
    exit_code: i32,
    source: Option<SourceLocation>,
    http: Option<HttpInfo>,
    /// Correlation ID matching `[A-Za-z0-9._:-]{1,128}`.
    request_id: Option<CorrelationId>,
    /// Server-provided correlation ID matching `[A-Za-z0-9._:-]{1,128}`.
    server_correlation_id: Option<CorrelationId>,
}

#[derive(Debug, Clone)]
struct CorrelationId(String);

impl TryFrom<String> for CorrelationId {
    type Error = InvalidOutputField;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
        {
            return Err(InvalidOutputField::CorrelationId);
        }
        Ok(Self(value))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidOutputField {
    #[error("source file is not safe for output")]
    SourceFile,
    #[error("field path is not canonical")]
    FieldPath,
    #[error("HTTP status is outside the valid range")]
    HttpStatus,
    #[error("route template is invalid")]
    RouteTemplate,
    #[error("correlation ID is invalid")]
    CorrelationId,
}

// Serde representation mirrors diagnostic.schema.json.
#[derive(Serialize)]
struct DiagnosticJson<'a> {
    #[serde(rename = "errorCode")]
    error_code: &'a str,
    category: &'a str,
    #[serde(rename = "exitCode")]
    exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<SourceLocationJson<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http: Option<HttpInfoJson<'a>>,
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    request_id: Option<&'a str>,
    #[serde(
        rename = "serverCorrelationId",
        skip_serializing_if = "Option::is_none"
    )]
    server_correlation_id: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// Warning source
// ---------------------------------------------------------------------------

/// Source location for a warning — `file` and `field_path` are required per
/// warning.schema.json.
#[derive(Debug, Clone)]
pub struct WarningSource {
    file: SourceFile,
    line: Option<u32>,
    column: Option<u32>,
    field_path: FieldPath,
}

impl WarningSource {
    pub fn try_new(
        file: String,
        line: Option<u32>,
        column: Option<u32>,
        field_path: String,
    ) -> Result<Self, InvalidOutputField> {
        Ok(Self {
            file: file.try_into()?,
            line,
            column,
            field_path: field_path.try_into()?,
        })
    }

    pub fn field_path(&self) -> &str {
        &self.field_path.0
    }
}

// Serde representation mirrors warning.schema.json "source" object.
#[derive(Serialize)]
struct WarningSourceJson<'a> {
    file: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<u32>,
    #[serde(rename = "fieldPath")]
    field_path: &'a str,
}

// ---------------------------------------------------------------------------
// Warning input
// ---------------------------------------------------------------------------

/// A fully typed warning record ready for rendering.
pub struct WarningInput {
    warning_code: WarningCode,
    category: WarningCategory,
    source: WarningSource,
}

impl WarningInput {
    pub fn new(
        warning_code: WarningCode,
        category: WarningCategory,
        source: WarningSource,
    ) -> Self {
        Self {
            warning_code,
            category,
            source,
        }
    }

    pub fn warning_code(&self) -> WarningCode {
        self.warning_code
    }

    pub fn field_path(&self) -> &str {
        self.source.field_path()
    }
}

// Serde representation mirrors warning.schema.json.
#[derive(Serialize)]
struct WarningJson<'a> {
    #[serde(rename = "warningCode")]
    warning_code: &'a str,
    category: &'a str,
    source: WarningSourceJson<'a>,
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// Generic renderer that writes outcomes to a stdout writer and diagnostics /
/// warnings to a stderr writer.
///
/// Parameterised over writers for testability: callers may pass `Vec<u8>` in
/// tests instead of `std::io::Stdout` / `std::io::Stderr`.
///
/// Invariants:
/// - `emit_outcome` writes only to `stdout`.
/// - `emit_diagnostic` and `emit_warning` write only to `stderr`.
/// - Every write produces exactly one newline-terminated physical line.
pub struct Renderer<W: Write, E: Write> {
    stdout: W,
    stderr: E,
    mode: OutputMode,
}

impl<W: Write, E: Write> Renderer<W, E> {
    /// Construct a renderer with the given writers and output mode.
    pub fn new(stdout: W, stderr: E, mode: OutputMode) -> Self {
        Renderer {
            stdout,
            stderr,
            mode,
        }
    }

    /// Emit a successful outcome record to stdout.
    ///
    /// Text mode: fixed template `<action> <kind> <project>/<key>\n`.
    /// JSON mode: compact JSON object produced by serde_json, one line.
    ///
    /// Values must come from schema-validated fields only (SEC-005).
    pub fn emit_outcome(
        &mut self,
        action: Action,
        kind: EntityKind,
        project: &str,
        key: &EntityKey,
    ) -> io::Result<()> {
        self.emit_outcome_with_adoption(action, kind, project, key, false)
    }

    /// Emit a successful outcome, including the save-only Workflow adoption
    /// marker when the selected unmarked Workflow was read by server ID.
    pub(crate) fn emit_outcome_with_adoption(
        &mut self,
        action: Action,
        kind: EntityKind,
        project: &str,
        key: &EntityKey,
        adoption_required: bool,
    ) -> io::Result<()> {
        match self.mode {
            OutputMode::Text => {
                // Fixed template: no untrusted input can inject record separators
                // because schema validation has already excluded C0/C1 controls
                // and bidi characters from these identifier fields.
                let adoption_suffix = if adoption_required {
                    " (adoption required on apply)"
                } else {
                    ""
                };
                writeln!(
                    self.stdout,
                    "{} {} {}/{}{}",
                    action.as_str(),
                    kind.as_str(),
                    project,
                    key.value(),
                    adoption_suffix
                )
            }
            OutputMode::Json => {
                // Build a serde_json::Value to produce the exact per-kind shape.
                // Only the applicable key field is included (no extra fields).
                let mut map = serde_json::Map::new();
                map.insert(
                    "action".to_owned(),
                    serde_json::Value::String(action.as_str().to_owned()),
                );
                map.insert(
                    "kind".to_owned(),
                    serde_json::Value::String(kind.as_str().to_owned()),
                );
                map.insert(
                    "project".to_owned(),
                    serde_json::Value::String(project.to_owned()),
                );
                map.insert(
                    key.field_name().to_owned(),
                    serde_json::Value::String(key.value().to_owned()),
                );
                if adoption_required {
                    map.insert("adoptionRequired".to_owned(), serde_json::Value::Bool(true));
                }
                let json = serde_json::to_string(&serde_json::Value::Object(map))
                    .expect("serde_json serialization of known-safe map must not fail");
                writeln!(self.stdout, "{json}")
            }
        }
    }

    /// Emit a diagnostic record to stderr; stdout is not touched.
    ///
    /// Text mode: fixed safe line `error: <errorCode>\n`.
    /// JSON mode: compact JSON object conforming to diagnostic.schema.json.
    pub fn emit_diagnostic(&mut self, diag: &DiagnosticInput) -> io::Result<()> {
        match self.mode {
            OutputMode::Text => {
                writeln!(self.stderr, "error: {}", diag.error_code.as_str())
            }
            OutputMode::Json => {
                let json_struct = DiagnosticJson {
                    error_code: diag.error_code.as_str(),
                    category: diag.category.as_str(),
                    exit_code: diag.exit_code,
                    source: diag.source.as_ref().map(SourceLocationJson::from),
                    http: diag.http.as_ref().map(HttpInfoJson::from),
                    request_id: diag.request_id.as_ref().map(|id| id.0.as_str()),
                    server_correlation_id: diag
                        .server_correlation_id
                        .as_ref()
                        .map(|id| id.0.as_str()),
                };
                let json = serde_json::to_string(&json_struct)
                    .expect("serde_json serialization of known-safe diagnostic must not fail");
                writeln!(self.stderr, "{json}")
            }
        }
    }

    /// Emit a warning record to stderr; stdout is not touched.
    ///
    /// Text mode: fixed phrase `warning: <warningCode>\n`.
    /// JSON mode: compact JSON object conforming to warning.schema.json.
    pub fn emit_warning(&mut self, warning: &WarningInput) -> io::Result<()> {
        match self.mode {
            OutputMode::Text => {
                writeln!(self.stderr, "warning: {}", warning.warning_code.as_str())
            }
            OutputMode::Json => {
                let json_struct = WarningJson {
                    warning_code: warning.warning_code.as_str(),
                    category: warning.category.as_str(),
                    source: WarningSourceJson {
                        file: &warning.source.file.0,
                        line: warning.source.line,
                        column: warning.source.column,
                        field_path: &warning.source.field_path.0,
                    },
                };
                let json = serde_json::to_string(&json_struct)
                    .expect("serde_json serialization of known-safe warning must not fail");
                writeln!(self.stderr, "{json}")
            }
        }
    }

    /// Flush both writers.
    pub fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()?;
        self.stderr.flush()
    }
}

// ---------------------------------------------------------------------------
// AppError → DiagnosticInput mapping
// ---------------------------------------------------------------------------

/// Map an `AppError` to a `DiagnosticInput` for rendering.
///
/// Error message strings from `AppError` are NOT forwarded to output; only
/// the closed `errorCode`, `category`, and `exitCode` are emitted (SEC-005).
pub fn diagnostic_from_app_error(error: &crate::error::AppError) -> DiagnosticInput {
    use crate::error::AppError;
    let (error_code, category) = match error {
        AppError::Usage(_) => (ErrorCode::EUsage, DiagnosticCategory::Usage),
        AppError::Configuration(_) | AppError::ConfigurationLayer(_) => {
            (ErrorCode::EConfiguration, DiagnosticCategory::Configuration)
        }
        AppError::InputLayer(_) => (ErrorCode::ESchema, DiagnosticCategory::LocalInput),
        AppError::Schema(_) => (ErrorCode::ESchema, DiagnosticCategory::LocalInput),
        AppError::ProjectionLayer(error) if error.is_compatibility() => (
            ErrorCode::EApiIncompatible,
            DiagnosticCategory::Compatibility,
        ),
        AppError::ProjectionLayer(_) => (ErrorCode::EInternal, DiagnosticCategory::Internal),
        AppError::ParseLayer(error) if error.is_yaml() => {
            (ErrorCode::EYamlParse, DiagnosticCategory::LocalInput)
        }
        AppError::ParseLayer(_) => (ErrorCode::ESchema, DiagnosticCategory::LocalInput),
        AppError::YamlParse(_) => (ErrorCode::EYamlParse, DiagnosticCategory::LocalInput),
        AppError::Authentication(_) => (
            ErrorCode::EAuthentication,
            DiagnosticCategory::Authentication,
        ),
        AppError::AuthLayer(error) if error.is_connectivity() => {
            (ErrorCode::EConnectivity, DiagnosticCategory::Connectivity)
        }
        AppError::AuthLayer(_) => (
            ErrorCode::EAuthentication,
            DiagnosticCategory::Authentication,
        ),
        AppError::Authorization(_) => {
            (ErrorCode::EAuthorization, DiagnosticCategory::Authorization)
        }
        AppError::VisibilityUnproven(_) => (
            ErrorCode::EVisibilityUnproven,
            DiagnosticCategory::Authorization,
        ),
        AppError::Connectivity(_) => (ErrorCode::EConnectivity, DiagnosticCategory::Connectivity),
        AppError::TransportLayer(error) if error.is_write_uncertain() => (
            ErrorCode::EWriteUncertain,
            DiagnosticCategory::Reconciliation,
        ),
        AppError::TransportLayer(error) if error.is_compatibility() => (
            ErrorCode::EApiIncompatible,
            DiagnosticCategory::Compatibility,
        ),
        AppError::TransportLayer(error) if error.is_internal() => {
            (ErrorCode::EInternal, DiagnosticCategory::Internal)
        }
        AppError::TransportLayer(_) => (ErrorCode::EConnectivity, DiagnosticCategory::Connectivity),
        AppError::ApiIncompatible(_) => (
            ErrorCode::EApiIncompatible,
            DiagnosticCategory::Compatibility,
        ),
        AppError::ServerRejected(_) => (
            ErrorCode::EServerRejected,
            DiagnosticCategory::ServerRejection,
        ),
        AppError::WriteUncertain(_) => (
            ErrorCode::EWriteUncertain,
            DiagnosticCategory::Reconciliation,
        ),
        AppError::WriteVerificationUnavailable(_) => (
            ErrorCode::EWriteVerificationUnavailable,
            DiagnosticCategory::Connectivity,
        ),
        AppError::WriteVerificationIncompatible(_) => (
            ErrorCode::EWriteVerificationIncompatible,
            DiagnosticCategory::Compatibility,
        ),
        // R-001 classifies whole-invocation deadline expiry at the external
        // boundary as E_CONNECTIVITY, while preserving a typed timeout inside
        // the application and worker layers.
        AppError::Timeout(_) => (ErrorCode::EConnectivity, DiagnosticCategory::Connectivity),
        AppError::Internal(_) => (ErrorCode::EInternal, DiagnosticCategory::Internal),
        AppError::Reconciliation(_) => {
            // Reconciliation errors map to exit 1. The specific reconciliation
            // sub-code (E_AMBIGUOUS_IDENTITY, E_ADOPTION_REQUIRED, etc.) is
            // determined by the adapter tasks (W-001, S-001, D-001) which will
            // produce richer error types. For now, AppError::Reconciliation
            // maps to E_AMBIGUOUS_IDENTITY as the generic reconciliation code.
            (
                ErrorCode::EAmbiguousIdentity,
                DiagnosticCategory::Reconciliation,
            )
        }
        AppError::EntityNotFound => (
            ErrorCode::EEntityNotFound,
            DiagnosticCategory::Reconciliation,
        ),
        AppError::EntityNotExportable => (
            ErrorCode::EEntityNotExportable,
            DiagnosticCategory::Reconciliation,
        ),
        AppError::SaveLayer(error) if error.is_output_exists() => {
            (ErrorCode::EOutputExists, DiagnosticCategory::LocalInput)
        }
        AppError::SaveLayer(error) if error.is_output_path() => {
            (ErrorCode::EOutputPath, DiagnosticCategory::LocalInput)
        }
        AppError::SaveLayer(error) if error.is_output_write() => {
            (ErrorCode::EOutputWrite, DiagnosticCategory::LocalInput)
        }
        AppError::SaveLayer(error) if error.is_compatibility() => (
            ErrorCode::EApiIncompatible,
            DiagnosticCategory::Compatibility,
        ),
        AppError::SaveLayer(_) => (ErrorCode::EInternal, DiagnosticCategory::Internal),
        AppError::ApplicationLayer(_) => (ErrorCode::EInternal, DiagnosticCategory::Internal),
        AppError::WorkflowAlreadyMarked => (
            ErrorCode::EWorkflowAlreadyMarked,
            DiagnosticCategory::Reconciliation,
        ),
    };
    DiagnosticInput {
        error_code,
        category,
        exit_code: error.exit_code(),
        source: None,
        http: None,
        request_id: None,
        server_correlation_id: None,
    }
}

// ---------------------------------------------------------------------------
// Convenience: render to real stdout/stderr
// ---------------------------------------------------------------------------

/// Emit a diagnostic from an `AppError` to real stderr with no stdout output.
///
/// The closed `errorCode`/`category`/`exitCode` diagnostic is always written
/// to stderr (SEC-005: no raw server text, credentials, or user input enters
/// the machine-readable output contract on stdout).
///
/// The full internal error chain is additionally emitted at `DEBUG` level via
/// the `tracing` subscriber. This does not weaken SEC-005: tracing output is
/// opt-in (`RUST_LOG=debug`), goes only to stderr, and is never part of the
/// machine-readable contract. It is safe to enable in development and CI
/// pipelines that do not parse stderr.
pub fn write_app_error_to_stderr(error: &crate::error::AppError, mode: OutputMode) {
    tracing::debug!(error = %error, "diagnostic detail");
    let diag = diagnostic_from_app_error(error);
    let mut renderer = Renderer::new(io::stdout(), io::stderr(), mode);
    // Ignore I/O error on stderr write: there is nothing meaningful to do if
    // stderr itself is broken during a failure path.
    let _ = renderer.emit_diagnostic(&diag);
    let _ = renderer.flush();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{Action, OutputMode};

    // --- F-007 AC-FR-026-01: success goes to stdout only ---

    #[test]
    fn text_outcome_goes_to_stdout_not_stderr() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_outcome(
            Action::Created,
            EntityKind::Assistant,
            "my-project",
            &EntityKey::Slug("my-slug".into()),
        )
        .unwrap();
        assert!(!r.stdout.is_empty(), "stdout must have output");
        assert!(r.stderr.is_empty(), "stderr must be empty on success");
    }

    #[test]
    fn json_outcome_goes_to_stdout_not_stderr() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_outcome(
            Action::Updated,
            EntityKind::Workflow,
            "proj",
            &EntityKey::Slug("flow".into()),
        )
        .unwrap();
        assert!(!r.stdout.is_empty(), "stdout must have output");
        assert!(r.stderr.is_empty(), "stderr must be empty on success");
    }

    // --- F-007 AC-FR-026-02: failure stdout is empty ---

    #[test]
    fn diagnostic_goes_to_stderr_not_stdout() {
        let diag = DiagnosticInput {
            error_code: ErrorCode::EUsage,
            category: DiagnosticCategory::Usage,
            exit_code: 2,
            source: None,
            http: None,
            request_id: None,
            server_correlation_id: None,
        };
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_diagnostic(&diag).unwrap();
        assert!(r.stdout.is_empty(), "stdout must be empty on failure");
        assert!(!r.stderr.is_empty(), "stderr must have output");
    }

    #[test]
    fn warning_goes_to_stderr_not_stdout() {
        let warn = WarningInput {
            warning_code: WarningCode::WSuspectedPlaintextSecret,
            category: WarningCategory::SecretLikeField,
            source: WarningSource::try_new(
                "decl.yaml".into(),
                Some(5),
                Some(1),
                "spec.apiKey".into(),
            )
            .unwrap(),
        };
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_warning(&warn).unwrap();
        assert!(r.stdout.is_empty(), "stdout must be empty on warning");
        assert!(!r.stderr.is_empty(), "stderr must have output");
    }

    // --- F-007: text success format exactly matches spec §8 ---

    #[test]
    fn text_outcome_format_assistant() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_outcome(
            Action::Updated,
            EntityKind::Workflow,
            "example",
            &EntityKey::Slug("flow".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        // Exact format from cli.md §8: "updated Workflow example/flow"
        assert_eq!(out.trim_end_matches('\n'), "updated Workflow example/flow");
    }

    #[test]
    fn text_outcome_format_skill_uses_name() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_outcome(
            Action::Created,
            EntityKind::Skill,
            "proj",
            &EntityKey::Name("my-skill".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        assert_eq!(out.trim_end_matches('\n'), "created Skill proj/my-skill");
    }

    #[test]
    fn text_outcome_format_datasource_uses_repo_name() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_outcome(
            Action::Created,
            EntityKind::Datasource,
            "proj",
            &EntityKey::RepoName("my-repo".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        assert_eq!(
            out.trim_end_matches('\n'),
            "created Datasource proj/my-repo"
        );
    }

    #[test]
    fn text_outcome_format_valid_action() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_outcome(
            Action::Valid,
            EntityKind::Assistant,
            "p",
            &EntityKey::Slug("s".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        assert_eq!(out.trim_end_matches('\n'), "valid Assistant p/s");
    }

    #[test]
    fn saved_workflow_text_marks_required_adoption() {
        let mut renderer = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        renderer
            .emit_outcome_with_adoption(
                Action::Saved,
                EntityKind::Workflow,
                "project",
                &EntityKey::Slug("flow".into()),
                true,
            )
            .unwrap();

        assert_eq!(
            String::from_utf8(renderer.stdout).unwrap(),
            "saved Workflow project/flow (adoption required on apply)\n"
        );
    }

    #[test]
    fn saved_workflow_json_marks_required_adoption() {
        let mut renderer = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        renderer
            .emit_outcome_with_adoption(
                Action::Saved,
                EntityKind::Workflow,
                "project",
                &EntityKey::Slug("flow".into()),
                true,
            )
            .unwrap();

        let value: serde_json::Value = serde_json::from_slice(renderer.stdout.as_slice()).unwrap();
        assert_eq!(value["adoptionRequired"], true);
        assert_eq!(value.as_object().unwrap().len(), 5);
    }

    // --- F-007: exactly one physical line per record ---

    #[test]
    fn text_outcome_produces_exactly_one_newline() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_outcome(
            Action::Created,
            EntityKind::Assistant,
            "p",
            &EntityKey::Slug("s".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        assert_eq!(out.chars().filter(|&c| c == '\n').count(), 1);
    }

    #[test]
    fn json_outcome_produces_exactly_one_newline() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_outcome(
            Action::Created,
            EntityKind::Workflow,
            "p",
            &EntityKey::Slug("s".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        assert_eq!(out.chars().filter(|&c| c == '\n').count(), 1);
    }

    // --- F-007 AC-FR-026-03: JSON success is valid parseable JSON ---

    #[test]
    fn json_outcome_assistant_is_valid_json_with_slug() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_outcome(
            Action::Updated,
            EntityKind::Workflow,
            "example",
            &EntityKey::Slug("flow".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
        assert_eq!(val["action"], "updated");
        assert_eq!(val["kind"], "Workflow");
        assert_eq!(val["project"], "example");
        assert_eq!(val["slug"], "flow");
        // No extra fields
        let obj = val.as_object().unwrap();
        assert_eq!(obj.len(), 4, "must have exactly 4 fields");
        assert!(
            !obj.contains_key("name"),
            "name must not appear for Workflow"
        );
        assert!(!obj.contains_key("repo_name"), "repo_name must not appear");
    }

    #[test]
    fn json_outcome_skill_uses_name_not_slug() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_outcome(
            Action::Created,
            EntityKind::Skill,
            "proj",
            &EntityKey::Name("my-skill".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(val["name"], "my-skill");
        assert!(val.get("slug").is_none(), "slug must not appear for Skill");
        assert!(val.get("repo_name").is_none());
    }

    #[test]
    fn json_outcome_datasource_uses_repo_name() {
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_outcome(
            Action::Created,
            EntityKind::Datasource,
            "proj",
            &EntityKey::RepoName("my-ds".into()),
        )
        .unwrap();
        let out = String::from_utf8(r.stdout).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(val["repo_name"], "my-ds");
        assert!(val.get("slug").is_none());
        assert!(val.get("name").is_none());
    }

    // --- F-007: JSON diagnostic is valid parseable JSON ---

    #[test]
    fn json_diagnostic_is_valid_json_with_required_fields() {
        let diag = DiagnosticInput {
            error_code: ErrorCode::ESchema,
            category: DiagnosticCategory::LocalInput,
            exit_code: 2,
            source: None,
            http: None,
            request_id: None,
            server_correlation_id: None,
        };
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_diagnostic(&diag).unwrap();
        let out = String::from_utf8(r.stderr).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
        assert_eq!(val["errorCode"], "E_SCHEMA");
        assert_eq!(val["category"], "local-input");
        assert_eq!(val["exitCode"], 2);
    }

    #[test]
    fn json_diagnostic_with_source_serializes_correctly() {
        let diag = DiagnosticInput {
            error_code: ErrorCode::ESchema,
            category: DiagnosticCategory::LocalInput,
            exit_code: 2,
            source: Some(
                SourceLocation::try_new(
                    "decl.yaml".into(),
                    Some(10),
                    Some(3),
                    Some("spec.slug".into()),
                )
                .unwrap(),
            ),
            http: None,
            request_id: None,
            server_correlation_id: None,
        };
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_diagnostic(&diag).unwrap();
        let out = String::from_utf8(r.stderr).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(val["source"]["file"], "decl.yaml");
        assert_eq!(val["source"]["line"], 10);
        assert_eq!(val["source"]["fieldPath"], "spec.slug");
    }

    #[test]
    fn json_diagnostic_reconciliation_exit_code_1() {
        let diag = DiagnosticInput {
            error_code: ErrorCode::EAmbiguousIdentity,
            category: DiagnosticCategory::Reconciliation,
            exit_code: 1,
            source: None,
            http: None,
            request_id: None,
            server_correlation_id: None,
        };
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_diagnostic(&diag).unwrap();
        let out = String::from_utf8(r.stderr).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(val["exitCode"], 1);
        assert_eq!(val["errorCode"], "E_AMBIGUOUS_IDENTITY");
        assert_eq!(val["category"], "reconciliation");
    }

    // --- F-007: text diagnostic format ---

    #[test]
    fn text_diagnostic_format_is_error_colon_code() {
        let diag = DiagnosticInput {
            error_code: ErrorCode::EUsage,
            category: DiagnosticCategory::Usage,
            exit_code: 2,
            source: None,
            http: None,
            request_id: None,
            server_correlation_id: None,
        };
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_diagnostic(&diag).unwrap();
        let out = String::from_utf8(r.stderr).unwrap();
        assert_eq!(out.trim_end_matches('\n'), "error: E_USAGE");
    }

    // --- F-007: JSON warning is valid JSON ---

    #[test]
    fn json_warning_is_valid_json() {
        let warn = WarningInput {
            warning_code: WarningCode::WSuspectedPlaintextSecret,
            category: WarningCategory::SecretLikeField,
            source: WarningSource::try_new(
                "decl.yaml".into(),
                Some(5),
                Some(1),
                "spec.apiKey".into(),
            )
            .unwrap(),
        };
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_warning(&warn).unwrap();
        let out = String::from_utf8(r.stderr).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).expect("must be valid JSON");
        assert_eq!(val["warningCode"], "W_SUSPECTED_PLAINTEXT_SECRET");
        assert_eq!(val["category"], "secret-like-field");
        assert_eq!(val["source"]["file"], "decl.yaml");
        assert_eq!(val["source"]["fieldPath"], "spec.apiKey");
    }

    // --- F-007: AppError mapping ---

    #[test]
    fn usage_error_maps_to_e_usage() {
        use crate::error::AppError;
        let d = diagnostic_from_app_error(&AppError::Usage("bad flag".into()));
        assert_eq!(d.error_code.as_str(), "E_USAGE");
        assert_eq!(d.category.as_str(), "usage");
        assert_eq!(d.exit_code, 2);
    }

    #[test]
    fn configuration_error_maps_to_e_configuration() {
        use crate::error::AppError;
        let d = diagnostic_from_app_error(&AppError::Configuration("bad url".into()));
        assert_eq!(d.error_code.as_str(), "E_CONFIGURATION");
        assert_eq!(d.category.as_str(), "configuration");
        assert_eq!(d.exit_code, 2);
    }

    #[test]
    fn schema_error_maps_to_e_schema() {
        use crate::error::AppError;
        let d = diagnostic_from_app_error(&AppError::Schema("missing field".into()));
        assert_eq!(d.error_code.as_str(), "E_SCHEMA");
        assert_eq!(d.category.as_str(), "local-input");
        assert_eq!(d.exit_code, 2);
    }

    #[test]
    fn reconciliation_error_maps_to_exit_1() {
        use crate::error::AppError;
        let d = diagnostic_from_app_error(&AppError::Reconciliation("ambiguous".into()));
        assert_eq!(d.exit_code, 1);
        assert_eq!(d.category.as_str(), "reconciliation");
    }

    #[test]
    fn authentication_error_maps_to_e_authentication() {
        use crate::error::AppError;
        let d = diagnostic_from_app_error(&AppError::Authentication("token expired".into()));
        assert_eq!(d.error_code.as_str(), "E_AUTHENTICATION");
        assert_eq!(d.exit_code, 2);
    }

    #[test]
    fn connectivity_error_maps_to_e_connectivity() {
        use crate::error::AppError;
        let d = diagnostic_from_app_error(&AppError::Connectivity("timeout".into()));
        assert_eq!(d.error_code.as_str(), "E_CONNECTIVITY");
        assert_eq!(d.exit_code, 2);
    }

    #[test]
    fn internal_error_maps_to_e_internal() {
        use crate::error::AppError;
        let d = diagnostic_from_app_error(&AppError::Internal("invariant".into()));
        assert_eq!(d.error_code.as_str(), "E_INTERNAL");
        assert_eq!(d.exit_code, 2);
    }

    // --- SEC-005: error code strings come from fixed enum, not user input ---

    #[test]
    fn error_code_strings_are_fixed_not_from_input() {
        // The error message "malicious\nnewline" must NOT appear in output.
        use crate::error::AppError;
        let malicious = AppError::Usage("malicious\nnewline injection".into());
        let d = diagnostic_from_app_error(&malicious);
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Text);
        r.emit_diagnostic(&d).unwrap();
        let out = String::from_utf8(r.stderr).unwrap();
        // Must be exactly "error: E_USAGE\n" — no injected content.
        assert_eq!(out, "error: E_USAGE\n");
    }

    #[test]
    fn json_error_code_is_not_user_supplied_value() {
        use crate::error::AppError;
        let malicious = AppError::Configuration("http://evil.com\"}extra".into());
        let d = diagnostic_from_app_error(&malicious);
        let mut r = Renderer::new(Vec::<u8>::new(), Vec::<u8>::new(), OutputMode::Json);
        r.emit_diagnostic(&d).unwrap();
        let out = String::from_utf8(r.stderr).unwrap();
        let val: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(val["errorCode"], "E_CONFIGURATION");
        // The malicious input string must not appear anywhere in output.
        assert!(
            !out.contains("http://evil.com"),
            "user input must not appear in output"
        );
    }

    // --- Entity key helpers ---

    #[test]
    fn entity_key_slug_field_name() {
        assert_eq!(EntityKey::Slug("x".into()).field_name(), "slug");
    }

    #[test]
    fn entity_key_name_field_name() {
        assert_eq!(EntityKey::Name("x".into()).field_name(), "name");
    }

    #[test]
    fn entity_key_repo_name_field_name() {
        assert_eq!(EntityKey::RepoName("x".into()).field_name(), "repo_name");
    }

    // --- Error code completeness ---

    #[test]
    fn all_error_codes_have_nonempty_str() {
        let codes = [
            ErrorCode::EAmbiguousIdentity,
            ErrorCode::EIdentityMarkerInvalid,
            ErrorCode::EResolutionUnstable,
            ErrorCode::EAdoptionRequired,
            ErrorCode::EMissingReference,
            ErrorCode::EWriteUncertain,
            ErrorCode::EServerRejected,
            ErrorCode::EUsage,
            ErrorCode::EYamlParse,
            ErrorCode::ESchema,
            ErrorCode::ESemantic,
            ErrorCode::EReference,
            ErrorCode::ESidecar,
            ErrorCode::EConfiguration,
            ErrorCode::EAuthentication,
            ErrorCode::ETokenExpired,
            ErrorCode::ELocalAuthUnavailable,
            ErrorCode::ERateLimited,
            ErrorCode::EAuthorization,
            ErrorCode::EVisibilityUnproven,
            ErrorCode::EWriteForbidden,
            ErrorCode::EApiIncompatible,
            ErrorCode::EWriteVerificationIncompatible,
            ErrorCode::EConnectivity,
            ErrorCode::EServerUnavailable,
            ErrorCode::ETimeout,
            ErrorCode::EWriteVerificationUnavailable,
            ErrorCode::EInternal,
        ];
        for code in &codes {
            let s = code.as_str();
            assert!(!s.is_empty(), "error code must have non-empty string");
            assert!(s.starts_with("E_"), "error code must start with E_: {s}");
        }
    }
}
