# Secure local artifact publication contract v1

Status: NORMATIVE ARCHITECTURE CONTRACT.

Source: approved feature specification v2, FR-SAVE-025–027,
DR-SAVE-006/009, QR-SAVE-004/005/007, VR-SAVE-004/012, and
AC-SAVE-009/016–018.

## 1. Artifact sets

Publication accepts only an immutable, prospectively validated artifact set:

```text
Single = { yaml: CompleteArtifact }
Skill  = { sidecar: CompleteArtifact, yaml: CompleteArtifact }

CompleteArtifact = {
  final_relative_path,
  complete_bytes,
  byte_length,
  content_kind: declaration_yaml | skill_main_markdown
}
```

No response DTO, selector UUID, server ID, URL, token, diagnostic value, or
mutable AST reaches this boundary.

## 2. Initial path validation

Before network access:

1. Resolve the repository root using existing behavior and retain its stable
   directory identity.
2. Convert the requested YAML path to a normalized repository-relative path.
   Reject absolute, empty, `.`/`..`, NUL/control, invalid UTF-8, and root escape.
3. Derive the Skill sidecar basename exactly under DR-SAVE-006.
4. Require final paths to be distinct under the destination filesystem's name
   comparison.
5. Walk each existing parent component relative to an open repository
   directory handle without following symlinks. Every parent must be a
   directory and already exist.
6. Reject either final leaf if any directory entry exists, regardless of type,
   contents, symlink target, or file identity.
7. Verify the destination filesystem supports the selected same-directory
   atomic no-replace primitive. Unsupported semantics are `E_OUTPUT_PATH`.

`--follow-symlinks` does not modify these rules. Path diagnostics do not emit
the rejected path.

## 3. Staging

Staging begins only after server resolution, reverse projection, compatibility,
secret/exportability checks, canonical rendering, and prospective validation
all pass.

For each artifact, create an unpredictable hidden entry in its final parent
using a directory-relative create-new call with owner read/write permissions.
Open and validate the same handle, write all bytes with bounded short-write
handling, flush, synchronize the file, and verify final length. The entry must
remain on the same filesystem and in the same directory as its final leaf.

Staging names, handles, and file identities are internal and never rendered or
logged. Stage order for Skill is sidecar then YAML. Any staging failure removes
all invocation-owned staging entries and returns `E_OUTPUT_WRITE` with no final
entry.

## 4. Publication state machine

```text
Prepared
  -> StagedAll
  -> SidecarPublished       (Skill only)
  -> Committed              (YAML published)

Prepared | StagedAll | SidecarPublished
  -> Failed
```

Transitions:

- `StagedAll -> Committed` for non-Skill: atomic no-replace rename YAML.
- `StagedAll -> SidecarPublished` for Skill: atomic no-replace rename sidecar.
- `SidecarPublished -> Committed`: atomic no-replace rename YAML.

Final rename is relative to the retained parent handle and uses only a native
operation whose verified contract is “rename this staging entry only if the
final name is absent.” A destination-exists race maps to `E_OUTPUT_EXISTS`.
Replace-style rename and check-then-rename are forbidden.

The YAML rename is the commit point. Thus a visible YAML always references an
already complete visible Skill sidecar. Each final file is either absent or
the complete staged byte sequence.

## 5. Cancellation and partial publication

Before the first final rename, cancellation removes staging entries normally.
After a final rename, cancellation is recorded and cannot undo the completed
publication. Save has no rollback operation and never removes a final path,
because it cannot prove that a path was not replaced by another process.

For a Skill, the sidecar is published before YAML. If the YAML no-replace
operation fails, the command reports the failure and the complete sidecar may
remain as an orphan. The orphan is not a declaration because no YAML references
it; recovery is manual removal after review. A crash can produce the same
complete orphan sidecar. A truncated final file and YAML without sidecar remain
impossible.

Once YAML commits, the invocation returns success. No fallible cleanup remains:
the no-replace rename consumes the final staging entry. A late cancellation
cannot convert a complete committed artifact set into a reported failure.

## 6. Crash boundary

A process or host crash can occur between the two Skill renames. The only
possible final artifact from this interval is a complete owner-only Markdown
sidecar without YAML. A truncated final file and YAML without sidecar are not
possible. An operator may remove a verified orphan sidecar manually before
retrying.

## 7. Failure-injection matrix

Tests inject faults at least at:

- each parent component open and metadata check;
- filesystem-capability check;
- each stage create, write chunk, flush, sync, and length check;
- initial and publication-time destination collision;
- sidecar no-replace rename;
- YAML no-replace rename;
- cancellation after sidecar publication and before YAML publication;
- cancellation before staging, between every stage, after sidecar publication,
  and immediately before YAML commit; and
- directory synchronization.

For every failure before the first final rename, assertions are:

- stdout empty and one typed diagnostic;
- no invocation-created final or staging entry;
- pre-existing/race-created entries byte-identical;
- no target URL, path, ID, token, content, or raw OS error in output/logs; and
- zero modifying server requests.

For a failure after Skill sidecar publication and before YAML publication, the
complete sidecar may remain, stdout remains empty, one typed diagnostic is
emitted, and no existing or race-created path is changed. The test must verify
that no YAML is visible without its sidecar.

Race tests use independent processes, not only threads, and cover final paths
created after initial validation.

## 8. Platform qualification

Each supported OS/filesystem pair must contract-test its primitive:

- Linux: `renameat2` with `RENAME_NOREPLACE` relative to retained directory
  descriptors.
- macOS: `renameatx_np` with `RENAME_EXCL` and equivalent directory safety.
- Windows: a handle-relative or fully verified same-directory atomic move that
  fails on existing destination and does not follow a reparse-point parent.

The initial implementation may support a subset, but help/release artifacts
must state the qualified set. Network or userspace filesystems are unsupported
until the same tests pass. Silent fallback is forbidden.
