# Generated single-declaration validation contract v1

Status: NORMATIVE ARCHITECTURE CONTRACT.

## Input

The validator receives exactly one generated declaration value and/or its
canonical in-memory YAML bytes. It receives no repository root, path enumerator,
sidecar resolver, configuration loader, or filesystem capability.

## Behavior

It applies the same duplicate-key-safe YAML, closed JSON Schema, semantic,
resource-budget, natural-reference-shape, and Workflow-local checks as lint.
It does not prove natural-reference existence and performs no network access.

Success returns an immutable validated declaration/byte sequence to the direct
writer. Failure occurs before the final output path is created and produces the
closed local/schema diagnostic.

## Evidence

- Generated positive/negative goldens match lint for equivalent bytes.
- Instrumented tests prove zero filesystem opens/enumeration by the validator.
- Missing neighboring declarations do not affect validation.
- Save-generated Skill output uses inline `content`; the shared declaration
  schema may also accept authored `contentFrom`, but save never generates it or
  invokes an auxiliary reader during generated-output validation.
