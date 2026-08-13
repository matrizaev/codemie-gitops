# ADR-015: Use a schema-aware canonical YAML serializer

## Status

Accepted

## Context

General YAML serializers do not promise stable property order, quoting,
multiline chomping, numeric spelling, or cross-version byte identity. `save`
must produce byte-identical declarations for the same normalized state while
preserving authorable scalar types and ordered domain lists.

## Decision drivers

- FR-SAVE-024 and DR-SAVE-001/007
- Reviewable diffs
- Cross-platform byte stability
- Closed declaration schema
- Rejection of ambiguous or non-finite scalars

## Options considered

### A. Accept the default output of the current YAML library

Rejected. Library output is not a canonical contract.

### B. Emit JSON because JSON is valid YAML

Rejected. It is byte-stable but materially reduces reviewability of long
prompts and nested declarations.

### C. Implement a small schema-aware canonical emitter

Selected.

## Decision

Reverse projection first produces a validated JSON-compatible declaration AST.
A dedicated emitter serializes only that value domain. It does not accept YAML
tags, anchors, aliases, merge keys, non-string mapping keys, non-finite
numbers, or arbitrary extension nodes.

Property order comes from an explicitly versioned schema-order table. Free-form
maps are sorted by Unicode scalar value. Domain lists preserve server order;
set-semantic lists use manifest-defined ordering and reject duplicates.

Scalar, indentation, quoting, multiline, newline, and final-document rules are
normative in `contracts/canonical-yaml-v1.md`. The implementation is accepted
only through byte goldens covering all four kinds, every Datasource branch,
scalar edge cases, Unicode ordering, and LF/trailing-newline behavior on every
supported platform.

## Consequences

### Positive

- Diffs are deterministic and independent of map insertion order.
- Serializer changes become explicit contract changes.
- The emitted subset is straightforward to parse safely.

### Negative

- A small custom emitter and schema-order table require maintenance.
- Schema changes must update both projection and canonical-order fixtures.

### Risks

- Block-scalar chomping can alter content if not tested at byte level.
- Unicode comparison could accidentally use locale or UTF-16 order.

## Follow-up actions

- Add parse-round-trip and byte-golden tests.
- Run goldens on Linux and every additional supported platform.
- Fail CI when schema properties lack canonical-order entries.

## References

- Feature specification v2: FR-SAVE-024, DR-SAVE-001/003/006/007,
  QR-SAVE-001/007/009, AC-SAVE-019
- Parent ADR-001
- `../contracts/canonical-yaml-v1.md`
