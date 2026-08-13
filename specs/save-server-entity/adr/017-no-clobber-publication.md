# ADR-017: Publish complete artifacts with native no-replace operations

## Status

Accepted

## Context

`save` must never overwrite a path and must expose only complete files. A Skill
has two artifacts and its YAML may become visible only after its complete main
content sidecar. Ordinary check-then-write and replace-style rename APIs do not
satisfy the race contract. Save does not roll back a final path after it has
been published.

## Decision drivers

- FR-SAVE-025–027 and DR-SAVE-009
- QR-SAVE-004/005/007
- Publication-race safety
- Sensitive staging cleanup
- Deterministic failure injection

## Options considered

### A. Open final paths directly with create-new and stream bytes

Rejected. A reader can observe a truncated final file.

### B. Write temporary files and use replace-style rename

Rejected. A race-created target can be overwritten.

### C. Stage complete files and use native atomic no-replace publication

Selected.

## Decision

Path validation opens a repository-root-relative chain without following
symlinks and retains parent-directory handles. Every parent must already
exist. Initial collision checks happen before network access; publication
rechecks are enforced by the filesystem operation itself.

Complete bytes are first held in memory. Immediately before publication, each
artifact is written through an owner-only, random, create-new staging entry in
the destination directory, flushed, and synchronized. Final publication uses
an OS primitive with atomic no-replace semantics relative to the retained
directory handle: Linux `renameat2(RENAME_NOREPLACE)`, macOS
`renameatx_np(RENAME_EXCL)`, or a verified Windows rename primitive that fails
when the destination exists. Filesystems without verified same-directory
atomic no-replace support fail `E_OUTPUT_PATH` before server access; a
check-then-rename fallback is forbidden.

Single-file publication renames the YAML staging entry. Skill publication
renames the Markdown sidecar first and YAML second. Cancellation is deferred
from the first final rename until the publication attempt completes. If a
staging step fails, staging entries are removed. A race-created or replacement
final file is never unlinked.

The successful YAML no-replace rename is the publication commit point. No
fallible operation after that point may downgrade the invocation to a reported
failure; remaining staging cleanup is designed to be consumed by rename before
commit. Directory synchronization is completed before the success outcome
when supported, but a post-commit durability signal cannot convert a complete,
visible artifact set into a failure result. Crash recovery of a complete
sidecar published before YAML remains a deferred operational case; no
truncated final file or YAML-without-sidecar can result.

The exact state machine, path rules, no-rollback behavior, and failure matrix are
normative in `contracts/publication-v1.md`.

## Consequences

### Positive

- Existing paths cannot be replaced by a race.
- Every visible final file has complete bytes.
- Skill YAML is the atomic declaration-visibility marker.

### Negative

- The implementation requires small platform-specific filesystem adapters.
- There is no portable multi-file transaction; a crash can leave a complete
  sidecar without YAML.

### Risks

- Network filesystems may not implement advertised rename semantics correctly.
- A failed Skill YAML rename can leave a complete orphan sidecar.

## Follow-up actions

- Security-review directory-handle traversal, permissions, and cleanup.
- Run filesystem capability and race tests on every supported filesystem.
- Add deterministic fault injection at every stage and rename boundary.
- Document recovery for an orphan complete sidecar without broadening v1 UX.

## References

- Feature specification v2: FR-SAVE-025–027, DR-SAVE-009,
  QR-SAVE-004/005/007, VR-SAVE-004/012, AC-SAVE-009/016–018
- `../contracts/publication-v1.md`
