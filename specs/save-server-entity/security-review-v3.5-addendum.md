# Security review addendum: File Datasource placeholder export

## Status

Reviewed 2026-08-14 for specification v3.5 and ADR-020. This addendum does not
rewrite historical preimplementation reviews.

## Threat analysis

| Threat | Control | Residual risk |
|---|---|---|
| Server filename traversal | Preserve only exact safe basenames; replace separators, control characters, empty/dot names with deterministic generated names | Unicode-confusable names remain review-visible |
| Existing-file clobber | Directory and placeholder files use create-new behavior; collisions fail | Check/create races can leave partial new artifacts, never replace existing files |
| Secret/content disclosure | Placeholders are always zero bytes; bearer credentials and source response bodies are never written | Original source content cannot be recovered by save |
| Misleading completeness | YAML records `uploaded_files`; README and ADR require replacing empty bytes before apply | Local lint validates shape/existence, not semantic file content |
| Partial publication | Placeholders are created first and YAML last; success occurs only after YAML completes | Failure can leave orphan placeholder directory requiring manual removal |
| Resource exhaustion | Schema limits `spec.files` to ten; response/body/invocation limits remain active | A datasource exposing more than ten filenames cannot receive one placeholder per file |

## Findings

No high or critical issue found. Release documentation must retain the warning
that untouched zero-byte files should not be applied. Tests must continue to
cover lexical escape rejection, duplicate targets, create-new collisions,
zero-byte output, and credential canaries.
