# ADR-001: Embedded JSON Schema with a marked YAML front end

## Status

Proposed

## Context

FR-002 requires fully offline lint. FR-003/FR-022/FR-023 require bundled,
strict schemas, source-pinned presence/null classification, unknown-field rejection, and useful
file/line/field diagnostics. The implementation is a self-contained Rust
binary and cannot import server Pydantic models. YAML aliases must be handled
deterministically, while JSON Schema Draft 2020-12 is already used for workflow
execution configuration.

## Decision drivers

- Offline behavior
- Exact source diagnostics
- Closed schemas and conditional validation
- Schema reviewability and drift detection
- No Python/runtime dependency
- Safe YAML handling

## Options considered

### A. Hand-reviewed embedded JSON Schemas plus marked YAML AST

Parse YAML events with line/column markers, build a JSON-compatible tree and
pointer-to-marker index, then validate with embedded Draft 2020-12 schemas.
Validate against the checked-in source-pinned adapter manifest and drift tests.

### B. Rust structs generated directly from live or checked-in OpenAPI

Generate Serde models and validate through deserialization.

### C. Fetch OpenAPI and validate dynamically at lint time

Use the target server as the schema source on every invocation.

## Decision

Choose option A. The schema documents are the authoring contract; they combine
approved product strictness, natural references, and authoring-only sidecars
with a pinned server request contract. `yaml-rust2` marked events are the
recommended parser basis and `jsonschema` is the recommended validator basis.
All references are local/embedded and network retrieval is disabled.

Use narrow typed Rust models for the envelope, config, CLI arguments, successful
outcome, and safe diagnostic. Keep kind payloads as validated JSON-compatible
values until they enter a kind adapter, avoiding a second divergent schema
source in a large generated type graph. The exact architecture inputs are
`contracts/declaration-v1alpha1.schema.json` and
`contracts/adapter-manifest-v2.42.0.json`; implementation must not redesign
them.

The schema makes null-rejecting fields required and non-null. It permits
omission or YAML null only for `optionalNull` fields; the operation projector,
not JSON Schema defaults, materializes those omissions as explicit JSON null.
The adapter manifest records operation applicability and mixed/tool ownership
so a generic null filler cannot invent request members.

## Consequences

### Positive

- Lint is deterministic and offline.
- Schema errors can map to file, line, column, and field.
- Product-only constructs can differ safely from the server API.
- Schemas are independently reviewable and testable.
- New server fields cannot silently become accepted authoring fields.

### Negative

- Tool maintainers own a schema synchronization process.
- A marked AST and JSON Pointer index add parser implementation effort.
- JSON Schema plus adapter tests duplicate some server validation knowledge.

### Risks

- Manual field drift can reject valid server fields or accept obsolete ones.
- YAML scalar typing may differ from author expectations.
- Duplicate-key detection can be lost if implementation uses only a high-level
  loader; tests must enforce the marked event path.

## Follow-up actions

- Reproduce and verify the checked-in adapter manifest from pinned backend
  commit `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- Add schema-to-adapter and deployment-drift tests.
- Security-review YAML limits, alias expansion, and secret warning behavior.

## References

- Product specification v24: FR-002/003/014–017/021–023/025/035/036
- DR-001, DR-002, DR-012, IR-001, QR-006
- `contracts/declaration-v1alpha1.md`
- `contracts/declaration-v1alpha1.schema.json`
- `contracts/adapter-manifest-v2.42.0.json`
- `research.md`
