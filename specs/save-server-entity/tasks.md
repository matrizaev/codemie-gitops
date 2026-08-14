# Implementation task breakdown: save server entity v3.5

Status: IMPLEMENTED — automated verification passed; live-server qualification remains.

## Q-SAVE-003 — Verify v3.4 architecture convergence

- References: spec v3 FR-SAVE-031–033; parent v33; ADR-018.
- Scope: independently check plan/data/contracts/schema/tasks/traceability.
- Evidence: no normative artifact requires repository discovery/config,
  closure, sidecars, staging/temp/rename/atomicity, or cancellation tokens.
- Completion: implementation and contract artifacts are aligned; automated
  checks pass, with live-server qualification remaining.

## R-SAVE-001 — Normalize OpenAPI responses into declarations

- References: spec v3.4 DR-SAVE-002–004; ADR-013; reverse manifest.
- Scope: project API context, categories, toolkits, tools, MCP servers, and
  integration settings into the closed declaration schema. Exclude API-only
  metadata and credential-bearing values; materialize only pinned API defaults
  required by the declaration schema.
- Evidence: reverse-projection regression tests and OpenAPI contract tests.
- Completion: implemented in `src/save/reverse.rs`; focused tests pass.

## S-SAVE-SEC-003 — Review direct-output security

- Scope: path validation, symlinks/special files, check/create race, partial
  secret-bearing file, no cleanup, error redaction, and timeout behavior.
- Evidence: threat model plus required negative/fault-injection tests.
- Completion: no unresolved high-severity issue.

## F-SAVE-004 — Narrow the save command boundary

- References: FR-SAVE-001/002/031; CLI save contract.
- Scope: require explicit project; remove repo root/follow-symlinks/config
  fallback; reject removed/secret-bearing flags before network.
- Expected components: CLI/config/save command types and tests.
- Completion: save command contains only selector, project, file, URL, output.

## Y-SAVE-002 — Emit one canonical inline declaration

- References: FR-SAVE-024/032; ADR-015/016; canonical/Skill contracts.
- Scope: map Skill main content to `spec.content`; remove `contentFrom`, sidecar
  path derivation, and multi-artifact builder; retain deterministic YAML and
  the current pinned non-exportable classification for File Datasource source
  bytes. Save must not invent paths or an alternate inline encoding.
- Evidence: four-kind goldens, repeated-run byte equality, no sidecar creation.
- Completion: reverse projection returns one canonical YAML byte sequence.

## F-SAVE-005 — Export File Datasource placeholders

- References: FR-SAVE-013/022/023/027/033; ADR-020.
- Scope: map `knowledge_base_file` to `file`, preserve `uploaded_files`, derive
  safe adjacent paths, and create zero-byte placeholders before YAML.
- Evidence: production qualification for all selected File Datasources,
  immediate lint success, path/collision tests, and zero-byte assertions.
- Completion: implemented; eleven production declarations and nineteen
  placeholders exported and linted successfully.

## V-SAVE-003 — Validate only generated declaration

- References: FR-SAVE-026/031; parent ADR-019.
- Dependencies: parent single-declaration validator and Y-SAVE-002.
- Scope: invoke validator in memory; remove overlay/repository enumeration and
  local reference-existence requirements.
- Evidence: invalid neighboring files/symlinks are never opened; missing local
  references do not block valid generated output.
- Completion: validated bytes are the sole writer input.

## P-SAVE-002 — Implement direct artifact writer

- References: FR-SAVE-025/027/033; ADR-018; publication contract.
- Dependencies: V-SAVE-003.
- Scope: preflight absence/path checks; after all gates, direct create-new and
  write final path; File Datasource writes placeholders then YAML; no
  temp/staging/rename/rustix/atomic protocol or cleanup of partial artifacts.
- Evidence: existing target before/racing create remains byte-identical;
  injected short-write/disk/flush failures produce exit 2, empty stdout,
  `E_OUTPUT_WRITE`, and may leave incomplete final YAML.
- Completion: only complete write renders `saved`.

## C-SAVE-002 — Integrate read-only coordinator

- Dependencies: existing strict read/reverse adapters plus F/Y/V/P tasks.
- Scope: order preflight, bounded server reads, projection/confidentiality,
  canonicalization, in-memory validation, direct write, and output.
- Evidence: every success/failure path sends zero mutations; one success record
  only after complete write.
- Completion: compiled-process acceptance suite covers all four kinds.

## O-SAVE-002 — Remove obsolete local-publication machinery

- Dependencies: C-SAVE-002 and parent v33 migration.
- Scope: remove overlay, sidecar, staging/publication state code and unused
  `tempfile`/`rustix`; retain no compatibility shim that can activate hidden
  reads or staged writes.
- Evidence: dependency audit, filesystem syscall/open trace, full tests,
  `make format`, and `make lint`.
- Completion: no obsolete production path remains.

## V-SAVE-004 — Post-implementation verification

- Evidence: spec-to-code trace; server-read fixtures; four-kind goldens;
  filesystem fault matrix; diagnostic schemas; no-mutation capture; cross-
  platform direct-write behavior without an atomicity claim.
- Completion: independent convergence report approves or blocks release.

## DOC-SAVE-002 — Refresh user-facing save documentation

- Scope: README/help/examples/runbooks/recovery guidance.
- Evidence: one declaration YAML, inline Skill content, File Datasource
  placeholders, required project, removed flags, and possible partial artifacts
  are accurately documented.
- Completion: no sidecar, staging, atomic/no-partial, repo-root, config-file, or
  prospective-repository instruction remains.

## L-SAVE-002 — Release readiness

- Dependencies: security and convergence reviews plus documentation.
- Evidence: same-artifact qualification, rollback, release notes, dependency
  removal/licensing, and partial-file support runbook.
- Completion: release assessment only; no release is authorized.
