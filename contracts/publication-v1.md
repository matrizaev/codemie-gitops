# Direct artifact write contract v1

Status: NORMATIVE ARCHITECTURE CONTRACT, revised by specification v3.5,
ADR-018, and ADR-020. The historical staged publication protocol is superseded.

## 1. Input

The writer accepts one validated canonical YAML byte sequence, one requested
final path, and for File Datasource only, generated relative zero-byte
placeholder paths. It accepts no repository root, staging path, or rollback
callback.

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

For File Datasource, the writer first creates the deterministic adjacent
placeholder directory and every placeholder with create-new semantics, then
creates YAML last. It never overwrites an existing directory or file and never
writes server source content into a placeholder.

If the final create observes a race-created target, it returns
`E_OUTPUT_EXISTS` without replacing/truncating it. If any operation after final
creation fails, it returns `E_OUTPUT_WRITE`; the incomplete final path may
remain and is not removed, renamed, or rolled back.

A File Datasource failure may leave a complete or partial orphan placeholder
directory without YAML. It is not cleaned up automatically.

## 4. Commit and output

Completion of YAML after every required placeholder is the success boundary. Only then
may the coordinator render `saved`. A failed/timeout/cancelled write leaves
stdout empty and produces exactly one safe diagnostic.

## 5. Failure-injection evidence

Tests must cover pre-existing target, create race, permission failure before
create, short write, disk-full/write error, close/finalization error where
observable, timeout during write, and process interruption. Assertions must
distinguish untouched existing paths from permitted incomplete newly created
paths and must prove no temporary/staging names or rename calls occur.
File Datasource tests additionally cover safe/unsafe basenames, duplicates,
directory collision, placeholder failure, YAML-last ordering, orphan recovery,
and exact zero-byte content.
