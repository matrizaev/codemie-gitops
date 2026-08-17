# ADR-0011: Schema-aware canonical YAML serialization

## Status

Accepted (originally save ADR-015; current behavior).

## Context

General YAML serializers do not promise stable property order, quoting,
multiline chomping, numeric spelling, or cross-version byte identity. `save`
must produce byte-identical declarations for the same normalized state while
preserving authorable scalar types and ordered domain lists.

## Decision

- Reverse projection first produces a validated JSON-compatible declaration
  AST. A dedicated emitter serializes only that value domain and rejects YAML
  tags, anchors, aliases, merge keys, non-string mapping keys, non-finite
  numbers, and arbitrary extension nodes.
- Property order comes from an explicitly versioned schema-order table (from
  `contracts/declaration-v1alpha1.schema.json` as written). Free-form
  maps sort by Unicode scalar value; domain lists preserve server order;
  set-semantic lists use manifest-defined ordering and reject duplicates.
- Scalar, indentation, quoting, multiline, newline, and final-document rules
  are normative in `contracts/canonical-yaml-v1.md`. The emitter is
  accepted only through byte goldens covering all four kinds, every Datasource
  branch, scalar edge cases, Unicode ordering, and LF/trailing-newline behavior
  on every supported platform. Skill main content is serialized inline as
  `spec.content`; no sidecar exists.
- The round-trip invariant holds: `parse_safe_yaml(canonical_emit(V)) == V`,
  and canonical emission of the parsed result reproduces the exact bytes.

## Consequences

- Diffs are deterministic and independent of map insertion order; serializer
  changes become explicit contract changes.
- A small custom emitter and order table require maintenance; schema changes
  must update both projection and canonical-order fixtures.

## Alternatives considered

- Default library output: rejected (not a canonical contract).
- Emitting JSON because JSON is valid YAML: rejected (byte-stable but
  materially reduces reviewability of long prompts and nested declarations).

## References

- [canonical-yaml-v1.md](../../contracts/canonical-yaml-v1.md)
- `src/save/snapshot.rs` (serialization), `src/save/reverse.rs`
