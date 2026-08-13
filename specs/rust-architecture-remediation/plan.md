# Rust Architecture Remediation Plan: v33/v3 convergence

## Status

Architecture status: READY FOR PRE-IMPLEMENTATION VERIFICATION

The previous remediation plan treated repository discovery, closure validation,
cooperative cancellation, and staged native publication as behavior
to preserve. Product specifications v33 and save v3 supersede those behaviors.
They are now removal/migration targets. Server reconciliation, strict DTOs,
typed errors, bounded HTTP, confidentiality, and canonical declaration
semantics remain preservation targets.

## Objective

Converge the current Rust product on a small modular single-binary architecture
whose local boundary is one declaration and whose save boundary is one direct
final-file write, while retaining approved server and output behavior.

## Authoritative inputs

1. `specs/codemie-cicd-tool.md` v33.
2. `specs/save-server-entity/spec.md` v3.
3. Revised plans/contracts/ADRs/tasks under those feature directories.
4. Pinned OpenAPI, adapter manifest, and reference-only CodeMie 2.42.0 evidence.
5. Current implementation as migration evidence, not target behavior.

## Current architecture and gaps

The code has a useful library facade, typed domain/config/auth elements, strict
adapter/HTTP logic, reverse save projection, canonical rendering, and typed
errors. It also currently:

- exposes repository/symlink flags and loads `.codemie/config.yaml`;
- discovers/sorts repository YAML and builds disk/overlay views;
- resolves explicit Skill/File inputs through repository-root machinery and
  validates cross-file graph closure;
- threads `CancellationToken` through blocking local APIs;
- saves a Skill YAML/Markdown artifact set through `tempfile` and `rustix`.

Those local/filesystem paths conflict with the approved specs.

## Target module boundaries

```text
cli            exact argument DTOs and dispatch
config         flags/environment endpoint/auth selectors only
input          bounded one-file open/read and marked parse
declaration    generated closed schema types and semantic/reference-shape rules
application    lint/apply/save commands and invocation deadline
adapters       kind-specific online target/reference/read/reverse/write policy
http           bounded transport, compatibility, auth, strict DTO decoding
output         closed success/warning/diagnostic rendering
filesystem     direct create-new save writer only
```

Repository walking/views/overlays and the staged publisher are not target
production modules. Narrow direct-read helpers remain for explicitly authored
Skill/File paths. A cancellation primitive may remain inside
runtime orchestration only if useful; it must not shape domain interfaces.

## Domain rules

- Raw CLI/Serde structs convert immediately through `TryFrom` into validated
  commands/newtypes.
- `InputFile`, `ProjectName`, URLs, selectors, natural references, byte budgets,
  deadlines, and output paths carry invariants.
- Layer-owned `thiserror` enums convert with `From`; only the output boundary
  maps to closed safe diagnostics.
- Apply adapters own online natural-reference resolution and preserve exact
  authorization/race/write behavior.
- Save reverse projection produces one `GeneratedDeclaration`; the writer
  accepts only validated canonical bytes and an absent final target.

## Delivery phases

### Phase 0 — Rebaseline contracts and tests

- Update embedded schema and CLI snapshots for removed flags/config/closure.
- Convert old repository-walking/staging tests from preservation evidence into
  deletion or negative-surface evidence.
- Add syscall/open tracing and request-capture harnesses.
- Gate: independent pre-implementation verification and security review.

### Phase 1 — Narrow CLI and configuration

- Remove repository-root and follow-symlinks fields/options.
- Remove filesystem configuration/root discovery and explicit fallbacks.
- Keep environment-only secrets and validated endpoint policy.
- Gate: removed flags fail before local/network access.

### Phase 2 — Establish the one-file declaration pipeline

- Add/narrow typed bounded loader and marked parser.
- Retain inline Skill or explicit `contentFrom`; narrow auxiliary reads to
  declaration-relative no-symlink bounded paths.
- Split semantic/reference-shape checks from online existence resolution.
- Switch lint to exactly one declaration.
- Gate: filesystem-open evidence shows only `--file`.

### Phase 3 — Migrate apply reference resolution

- Feed one typed declaration into the coordinator.
- Direct-read validated File Datasource paths under per-file/aggregate bounds
  and construct multipart parts without enumeration or temporary copies.
- Resolve all natural references online through adapters before mutation.
- Retain compatibility, authorization, pagination, race, preservation, and
  exactly-one-write rules.
- Gate: missing/ambiguous/unauthorized evidence yields zero writes.

### Phase 4 — Migrate save local output

- Render Skill main content inline and one canonical YAML.
- Validate generated declaration in memory through the one-file rules.
- Replace overlay/staging publisher with direct create-new final write.
- Model and test `FailedPartial`; never clean it up or report success.
- Gate: zero modifying HTTP and no temp/staging/rename operations.

### Phase 5 — Remove obsolete code/dependencies

- Delete unused repository view/walking/overlay/staging modules and
  their APIs.
- Remove `config` YAML loading, `tempfile`, `rustix`, `tokio-util`, or walker
  dependencies only when repository-wide usage proves them unnecessary.
- Preserve test-only dependencies only with a concrete remaining consumer.
- Gate: no dead code; `make format`, `make lint`, full tests.

### Phase 6 — Independent convergence and release evidence

- Execute v33/v3 verification/security matrices and user-doc refresh.
- Verify same artifact across target environments and rollback instructions.
- Release remains a separate explicitly authorized action.

## Migration and rollback

Keep each phase compiling and tested. Introduce the new path, switch callers,
then delete the old path; never maintain runtime dual behavior. The embedded
schema, CLI, and implementation ship in one artifact, so mixed-version local
contracts are unsupported. Before release, rollback is source/artifact rollback.
After a failed save direct write, rollback is manual local file remediation,
not CLI cleanup.

## Quality and completeness gates

- Exactly one local input is opened for lint/apply.
- Offline lint validates reference shape, not existence.
- Apply resolves every reference online before exactly one authorized write.
- Save validates/writes one inline YAML and diagnoses partial final files.
- No repository config/root/walk/order/closure/implicit inputs/removed flags survive.
- No staging/temp/rename/atomic save guarantee survives.
- Timeouts and budgets remain without a required cancellation-token API.
- Typed errors and closed safe output remain intact.

Specification v33.3 retains explicit Skill/File paths under a narrow bounded
direct-read contract; no product conflict remains.

## Handoff

Implementation is bounded by the task lists in the two feature directories.
Verification must rebaseline the preservation inventory before code movement.
Security review must assess narrowed reads, online reference resolution, direct
write races/partial files, and diagnostic confidentiality.
