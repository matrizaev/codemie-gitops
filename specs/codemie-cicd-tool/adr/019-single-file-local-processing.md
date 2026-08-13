# ADR-019: Use a strict single-file local-processing boundary

## Status

Accepted by the explicit product decision in specification v33 (2026-08-13).

## Context

Earlier architecture treated a Git repository as the local unit: commands
discovered declarations, loaded config and Skill sidecars, built a graph
closure, and exposed root/symlink controls. V33 instead defines one `--file` as
the entire local input to lint/apply.

## Decision drivers

- Exact product boundary and predictable filesystem access
- Offline lint without hidden dependencies
- Online server authority for reference existence
- Simpler resource and timeout behavior
- Removal of obsolete configuration, implicit-read, and repository-wide attack surfaces

## Options considered

1. Keep repository abstractions but enumerate only the target.
2. Introduce a new one-file loader alongside the repository engine.
3. Replace the production repository engine with a one-file loader.

## Decision

Choose option 3, using option 2 only as an incremental migration state.
Lint/apply open and validate exactly the selected declaration plus only the
explicit Skill `contentFrom` and File Datasource `spec.files[]` inputs named by
it. Offline natural references are shape-checked; apply resolves them online.
There is no root, walking, closure, ordering, config-file, implicit input,
`--repo-root`, or `--follow-symlinks` contract. Auxiliary paths resolve from the
declaration parent with containment/no-symlink/regular/bounded rules. Bounded
reads and the command deadline are required; a cancellation-token API is not.

## Consequences

### Positive

- One explicit input determines all local behavior.
- Neighboring invalid or malicious files cannot affect the invocation.
- Local validation and server authority have clear boundaries.

### Negative

- CI must invoke the command once per desired declaration.
- Lint cannot detect a missing server reference offline.
- Existing repository-oriented code and tests require deletion or replacement.

### Risks

- Leaving dormant fallbacks could reintroduce hidden reads.
- Online reference resolution must remain complete and pre-write.

## Follow-up actions

- Implement tasks F-008–R-002.
- Independently verify filesystem-open traces and zero unauthorized writes.
- Refresh user-facing examples/runbooks after implementation.

## References

- `../../codemie-cicd-tool.md` v33
- `../plan.md`
- `../contracts/cli.md`
- `../contracts/declaration-v1alpha1.md`
