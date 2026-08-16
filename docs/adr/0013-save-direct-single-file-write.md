# ADR-0013: Validate one inline declaration and write its final path directly

## Status

Accepted (originally save ADR-018 and ADR-020; current behavior).

## Context

The prior design created a Skill sidecar, validated a prospective repository,
staged complete artifacts, and atomically renamed them without replacement.
The approved product boundary requires one YAML, inline Skill content,
single-declaration validation, and a direct final-path write with observable
partial-file failures. File Datasource save additionally faces server
`knowledge_base_file` responses that expose filenames but not source
bytes.

## Decision

- **Single artifact**: `save` performs only GET requests, then renders
  and validates one canonical inline declaration in memory, and writes it
  directly to the create-new final path. No sidecar, staging/temp file,
  rename, `rustix`, rollback, or atomicity promise exists.
- **Create-new semantics**: an existing target (or race-created target) is
  never replaced or truncated (`E_OUTPUT_EXISTS`). A failure after final
  creation is `E_OUTPUT_WRITE` with empty stdout; the incomplete new path
  may remain and is not removed, renamed, or rolled back. Success (`saved`)
  requires the complete write.
- **File Datasource placeholders** (narrow exception to the one-artifact
  rule): reverse `knowledge_base_file` to `spec.index_type: file`,
  preserve returned filenames in `spec.uploaded_files`, derive up to ten
  safe relative placeholder paths beneath `<yaml-name>.files/`
  (unsafe/duplicate basenames become deterministic
  `replace-content-N.txt` names), create the directory and zero-byte
  placeholders with no-replace semantics, then write YAML last. Failure may
  leave an orphan placeholder directory without YAML; retry requires manual
  removal. Operators replace the empty bytes before `apply`.
- Preflight rejects a detectably existing target, missing/invalid parent,
  symlink/special-file target, and unsafe path form before network access.
  Server reads, projection, confidentiality checks, canonicalization, and
  in-memory validation all precede the final create.

## Consequences

- Implementation matches the intentionally simple one-file contract; existing
  paths are never replaced.
- Readers can observe or later encounter an incomplete failed output; operator
  recovery is manual.
- File Datasources become reviewable, editable declarations with explicit
  missing-bytes state rather than fabricated content.

## Alternatives considered

- Retaining staged no-replace publication: rejected by specification v3.
- Temporary file plus rename: rejected (same).
- Refusing File Datasource export: rejected (blocks moving UI-created file
  Datasources into GitOps ownership).

## References

- [cli-save-v1.md](../../contracts/cli-save-v1.md)
- [publication-v1.md](../../contracts/publication-v1.md)
- [single-declaration-validation-v1.md](../../contracts/single-declaration-validation-v1.md)
- `src/save/mod.rs`, `src/save/publication.rs`
