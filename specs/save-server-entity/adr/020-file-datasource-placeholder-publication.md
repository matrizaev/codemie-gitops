# ADR-020: Publish File Datasource placeholders before YAML

## Status

Accepted by specification v3.5 (2026-08-14). This ADR narrowly supersedes
ADR-018's one-artifact rule for File Datasource save; ordinary kinds retain
ADR-018 direct single-YAML behavior.

## Context

CodeMie `knowledge_base_file` detail responses expose authorable metadata and
original filenames through `uploaded_files`, but do not expose source bytes.
The declaration language requires at least one explicit local path in
`spec.files`. Refusing export prevents users from moving UI-created file
Datasources into GitOps ownership.

## Decision

Reverse `knowledge_base_file` to `spec.index_type: file`. Preserve returned
filenames in `spec.uploaded_files`. Derive up to ten relative placeholder paths
beneath `<yaml-name>.files/`, preserving safe basenames and replacing unsafe or
duplicate basenames with deterministic `replace-content-N.txt` names.

After remote reads and in-memory declaration validation succeed, create the
directory and zero-byte placeholder files with no-replace semantics, then
write YAML last. Success requires all placeholders and YAML to complete.
Apply continues to read `spec.files` normally; users replace the empty bytes
before applying.

## Consequences

- File Datasources become reviewable and editable declarations.
- Missing server source bytes remain explicit rather than fabricated.
- `uploaded_files` lets update projection retain the current server filename
  set while edited placeholders are sent as multipart input.
- Publication is not atomic. Failure may leave an orphan placeholder directory
  without YAML; retry requires manual removal.
- Applying untouched zero-byte placeholders is permitted by local validation
  but is operationally discouraged and may be rejected by CodeMie based on
  file type/content.

## Security and safety

- Generated paths are relative and reject root/parent/prefix components.
- Basenames containing path separators or control characters are replaced.
- Existing files/directories are never overwritten.
- No server bytes, credentials, or tokens are written into placeholders.
- Diagnostics never include filenames or paths.

## References

- `../spec.md` v3.5, FR-SAVE-013/022/023/027/033
- `../contracts/publication-v1.md`
- `../contracts/save-read-reverse-v2.42.0-v1.json`
