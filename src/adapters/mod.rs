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

// Re-exports are added in F-002 when the CLI dispatch uses them.
