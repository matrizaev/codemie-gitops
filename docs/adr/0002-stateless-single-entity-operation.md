# ADR-0002: Stateless, single-entity, single-file, single-binary operation

## Status

Accepted (originally ADR-003 core, ADR-005, and ADR-019; current behavior).

## Context

The tool runs locally and in CI, is distributed as native binaries, and has no
requirement for a daemon, database, plugin host, or shared network service.
Earlier designs treated a Git repository as the local unit: commands discovered
declarations, loaded a config file and Skill sidecars, built a graph closure,
and exposed root/symlink controls.

## Decision

- **Stateless**: each invocation is independent. There is no local state
  database, token/cookie cache, plan, delete, batch operation, or automatic
  rollback. Credentials are passed per invocation and never stored.
- **Single entity**: one command invocation processes exactly one declaration.
- **Single file**: `lint`/`apply` open exactly the selected `--file` plus
  only the explicit auxiliary inputs named by it (Skill `contentFrom`, File
  Datasource `spec.files[]`). There is no repository root, walking, closure,
  ordering, config-file, implicit input, `--repo-root`, or
  `--follow-symlinks` contract. Auxiliary paths resolve from the declaration
  parent under containment/no-symlink/regular/bounded rules. Bounded reads and
  the command deadline are required; a cancellation-token API is not.
- **Single binary**: one Rust binary with internal modules for CLI/config,
  parsing, schemas, HTTP, kind adapters, projection, and output. No dynamic
  plugin ABI. Release binaries target Linux x86_64/aarch64 GNU, macOS aarch64,
  and Windows x86_64 MSVC; Rustls avoids an OpenSSL runtime dependency.

## Consequences

- CI must invoke the command once per desired declaration; a neighbor's invalid
  file can never affect an invocation.
- Lint cannot detect a missing server reference offline; apply resolves
  references online (see ADR-0003).
- Any kind update releases the whole binary; shared validation/output/security
  behavior cannot drift between kinds.
- Dormant fallbacks must not reintroduce hidden reads; instrumented
  filesystem-open tests guard the boundary.

## Alternatives considered

- Repository-wide processing: rejected (superseded by the one-file product
  decision).
- Separate per-kind executables or a persistent reconciliation service:
  rejected (no operational need; one artifact is simpler to version and scan).
- Session/token cache: rejected (secret lifecycle/state outside the contract).

## References

- [cli.md](../../contracts/cli.md)
- [implementation-reference.md](../implementation-reference.md)
- `src/input.rs`, `src/config.rs`, `src/cli.rs`
