# Direct final-file write contract v1

Status: NORMATIVE ARCHITECTURE CONTRACT, revised by specification v3 and
ADR-018. The historical staged publication protocol is superseded.

## 1. Input

The writer accepts one validated canonical YAML byte sequence and one requested
final path. It accepts no sidecar, artifact set, repository root, staging path,
or rollback callback.

## 2. Preflight

Before network access, save rejects a detectably existing target, missing or
invalid parent, target directory/special file/symlink, and unsafe path form.
The diagnostic does not contain the path. Preflight is not the race boundary.

## 3. Direct write

After all server reads, reverse projection, confidentiality checks,
canonicalization, and generated-declaration validation pass, the writer opens
the requested final path directly with create-new/no-overwrite semantics and
writes the canonical bytes. It does not create a temporary/staging file, call a
rename operation, use `rustix`, or claim atomic visibility/durability.

If the final create observes a race-created target, it returns
`E_OUTPUT_EXISTS` without replacing/truncating it. If any operation after final
creation fails, it returns `E_OUTPUT_WRITE`; the incomplete final path may
remain and is not removed, renamed, or rolled back.

## 4. Commit and output

Completion of the ordinary final-file write is the success boundary. Only then
may the coordinator render `saved`. A failed/timeout/cancelled write leaves
stdout empty and produces exactly one safe diagnostic.

## 5. Failure-injection evidence

Tests must cover pre-existing target, create race, permission failure before
create, short write, disk-full/write error, close/finalization error where
observable, timeout during write, and process interruption. Assertions must
distinguish untouched existing paths from permitted incomplete newly created
paths and must prove no temporary/staging names or rename calls occur.
