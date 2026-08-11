/// Entity adapter module: one adapter per entity kind.
///
/// All adapters share the same write policy (ADR-002):
/// - Absent identity → POST/`created`.
/// - Present identity → unconditional PUT/`updated`.
/// - No field-state-dependent write suppression.
/// - Server IDs are invocation-local only and never output.
pub mod assistant;
pub mod datasource;
pub mod skill;
pub mod workflow;

/// What a successful apply operation did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyAction {
    Created,
    Updated,
}

/// The result of a successful single-entity apply.
///
/// `server_id` is the server-assigned UUID. It is invocation-local: callers
/// may forward it to the render layer for outcome reporting but MUST NOT
/// write it to persistent state or expose it in user-facing output.
#[derive(Debug)]
pub struct ApplyResult {
    /// Whether this was a create or an update.
    pub action: ApplyAction,
    /// Server UUID (never forwarded to logs or user-visible output, SEC-005).
    pub server_id: String,
}
