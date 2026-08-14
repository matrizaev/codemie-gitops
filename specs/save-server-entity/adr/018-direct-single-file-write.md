# ADR-018: Validate one inline declaration and write its final path directly

## Status

Accepted by the explicit product decision in specification v3 (2026-08-13).
Superseded only for File Datasource placeholder publication by ADR-020.

## Context

The prior design created a Skill sidecar, validated a prospective repository,
staged complete artifacts, and atomically renamed them without replacement.
The approved product boundary now requires one YAML, inline Skill content,
single-declaration validation, and a direct final-path write with observable
partial-file failures.

## Decision drivers

- Exact v3 product behavior
- One intentional local artifact
- No hidden repository reads
- Existing-target protection
- Honest failure semantics without an atomicity promise

## Options considered

1. Retain staged no-replace publication.
2. Use a temporary file and replace/no-replace rename.
3. Open the final path create-new and write it directly.

## Decision

Choose option 3. All remote reads, reverse projection, confidentiality checks,
canonicalization, and in-memory single-declaration validation finish first.
Then save opens the final path with create-new/no-overwrite semantics and writes
one YAML. It creates no sidecar, staging/temp file, or rename transaction. A
failure after final creation may leave incomplete YAML, returns
`E_OUTPUT_WRITE`, emits empty stdout, and triggers no cleanup/rollback.

## Consequences

### Positive

- Implementation matches the intentionally simple one-file contract.
- Existing paths are never replaced.
- No temporary secret-bearing artifact or multi-file ordering exists.

### Negative

- Readers can observe or later encounter an incomplete failed output.
- The operator must remediate that path manually before retrying.
- No atomic visibility or crash-consistency guarantee is provided.

### Risks

- A generic write helper might silently reintroduce truncation or temp/rename.
- Failure handling might accidentally report success after a short write.

## Follow-up actions

- Implement P-SAVE-002 and its fault-injection matrix.
- Security-review direct-path races, symlinks, partial-file confidentiality,
  and safe diagnostics.
- Remove unused overlay/staging dependencies after call-site migration.

## References

- `../spec.md` v3, FR-SAVE-031–033
- `../contracts/single-declaration-validation-v1.md`
- `../contracts/publication-v1.md`
