# ADR-0001: Embedded closed JSON Schema with a marked YAML front end

## Status

Accepted (originally ADR-001 in the feature spec; current behavior).

## Context

`lint` must work fully offline. The tool is a self-contained Rust binary that
cannot import server Pydantic models, and it must reject unknown fields,
report useful file/line/field diagnostics, and stay synchronized with a pinned
server contract. YAML aliases/tags must be handled deterministically. JSON
Schema Draft 2020-12 is already used for workflow execution configuration.

## Decision

Embed hand-reviewed JSON Schemas in the binary and validate against them using
a marked YAML AST:

- Parse YAML events with line/column markers, build a JSON-compatible tree and
  pointer-to-marker index, then validate with embedded Draft 2020-12 schemas.
- The checked-in `contracts/declaration-v1alpha1.schema.json` is the authoring
  contract; the checked-in `contracts/adapter-manifest-v2.42.0.json` pins the
  server request projection. Neither is redesigned by implementation.
- All schema references are local/embedded; network schema retrieval is
  disabled. `build.rs` generates Rust DTOs from the same schema via `typify`,
  but the generated DTOs do not widen what the embedded schema accepts.
- The schema makes null-rejecting fields required and non-null, and permits
  omission or YAML null only for explicitly optional-null fields. The operation
  projector, not JSON Schema defaults, materializes omissions as JSON null.

## Consequences

- Lint is deterministic and offline; schema errors map to file/line/column.
- New server fields cannot silently become accepted authoring fields.
- Maintainers own a schema synchronization process; manual drift can reject
  valid fields or accept obsolete ones.
- Duplicate-key detection requires the marked-event path; tests must enforce it.

## Alternatives considered

- Rust structs generated directly from OpenAPI: rejected (validation only
  through deserialization, divergent schema source).
- Fetching OpenAPI and validating at lint time: rejected (not offline, and live
  OpenAPI cannot establish behavioral semantics).

## References

- [declaration-v1alpha1.schema.json](../../contracts/declaration-v1alpha1.schema.json)
- [declaration-v1alpha1.md](../../contracts/declaration-v1alpha1.md)
- [adapter-manifest-v2.42.0.json](../../contracts/adapter-manifest-v2.42.0.json)
- `src/schema.rs`, `src/parse.rs`, `build.rs`
