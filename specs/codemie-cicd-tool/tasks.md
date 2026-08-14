# Completed implementation task breakdown: v33.3 single-file processing

Status: IMPLEMENTED — automated verification and unit/integration tests pass.

This file is completion history, not the current source/module inventory. Use
`../../docs/implementation-reference.md` to reconstruct the implementation.

All tasks were derived from `../codemie-cicd-tool.md` v33–v33.3 and ADR-019. They do
not authorize changes in reference-only `codemie/` or `codemie-ui/`.

## Q-011 — Verify v33 architecture convergence

- Objective: independently confirm spec, CLI/declaration contracts, save v3,
  ADRs, schemas, and this task list agree.
- Dependencies: architecture artifact refresh complete.
- Evidence: trace every v33 acceptance criterion; prove no contract requires a
  repository root, walking, closure, config file, implicit input, removed flag, or
  cancellation-token API.
- Completion: verification status permits implementation or reports exact
  upstream conflicts.

## F-011 — Implement explicit bounded auxiliary inputs

- Requirements: FR-025/038–041, DR-014, VR-018.
- Dependencies: F-009/F-010.
- Scope: resolve Skill `contentFrom` and 1–10 File Datasource paths relative to
  the selected declaration parent; reject escape/symlink/non-regular/unreadable
  and duplicate targets; enforce 128 KiB Skill, 32 MiB/file, 128 MiB aggregate;
  inline Skill content and stream exact File multipart parts with safe basename.
- Evidence: path/size/UTF-8/content/basename negatives, exact multipart capture,
  and filesystem traces proving no enumeration/unlisted/temp/staging access.
- Completion: only explicitly authored auxiliary inputs can be opened.

## SEC-011 — Review the narrowed filesystem boundary

- Objective: review one-file reads, direct save writes, partial-output failure,
  path safety, diagnostics, and timeout behavior.
- Evidence: threat model and tests for symlink/special-file/race/oversize/error
  paths without path/content leakage.
- Completion: no unresolved high-severity issue.

## F-008 — Replace CLI and configuration surface

- Requirements: v33 command surface; FR-001/004/005; ADR-011/019.
- Scope: remove `--repo-root` and `--follow-symlinks` from lint/apply; reject
  them as unknown; remove `.codemie/config.yaml` and nearest-Git-root lookup;
  use flags/environment for URL/auth URL and authored project.
- Expected components: `src/cli`, `src/config`, CLI/config tests.
- Evidence: help snapshots and negative tests prove removed inputs fail before
  filesystem/network activity.
- Completion: no production config-file/root resolution remains.

## F-009 — Implement bounded single-declaration loading

- Requirements: v33 single-file boundary and resource budgets.
- Scope: ordinary bounded read of only `--file`; marked YAML parse; closed schema
  and semantic validation; timeout integration without a required token API.
- Expected components: `src/parse`, new/narrow loader module, typed errors.
- Evidence: instrumented tests prove no parent enumeration or neighbor opens;
  oversize, non-regular, invalid YAML, and timeout failures are closed/safe.
- Completion: loader returns one typed effective declaration.

## F-010 — Remove repository-closure authoring

- Requirements: v33; declaration contract.
- Dependencies: F-009.
- Scope: retain exactly-one-of Skill `content`/`contentFrom`; delete production
  walking/repository-view/graph-closure paths; retain reference-shape and
  Workflow-local semantic checks.
- Migration components: declaration schema/build embedding, removed
  `src/discovery` and `src/repository.rs`, retained `src/validate` and
  `src/domain`, plus tests.
- Evidence: schema/path goldens accept safe `contentFrom`; missing neighboring
  declarations do not fail lint; wrong-shaped references do fail.
- Completion: lint reads exactly one file and performs zero network access.

## R-002 — Resolve every apply reference online

- Requirements: FR-006/008/009/022 and v33 online existence rule.
- Dependencies: F-009/F-010.
- Scope: pass typed natural references from the one declaration to existing
  kind adapters; perform bounded server reads and exact authorization before
  one write; preserve creator/project identity and race policies.
- Expected components: coordinator and Assistant/Workflow/Skill/Datasource
  adapters.
- Evidence: missing/ambiguous/unauthorized reference tests send no mutation;
  valid references map to the exact source-pinned request.
- Completion: no local declaration is required to resolve a server reference.

## O-003 — Remove obsolete dependencies and dead abstractions

- Dependencies: F-008–R-002 and save v3 implementation tasks.
- Scope: remove unused repository config, walker, overlay,
  cancellation, `tempfile`, and `rustix` production dependencies only after
  call-site evidence proves them unused.
- Evidence: dependency/build checks, dead-code-free compilation, full tests,
  `make format`, and `make lint`.
- Completion: no obsolete product boundary survives under a generic name.

## V-004 — Post-implementation convergence verification

- Dependencies: all implementation tasks.
- Evidence: compiled-process tests for exactly-one-file access, removed flags,
  offline shape-only lint, online apply resolution, timeout/resource budgets,
  output streams, zero unauthorized writes, and save v3 interaction.
- Completion: code/tests/docs converge on v33.

## DOC-003 — Refresh user-facing documentation

- Dependencies: implementation convergence.
- Scope: README, examples, CI snippets, operator/author runbooks, and help text.
- Evidence: no documented repository config, repo-root, symlink-following,
  directory-wide lint/apply, implicit inputs, staged/atomic save, or no-partial-
  output promise remains.
- Completion: examples invoke one declaration and use only documented explicit
  Skill/File auxiliary paths.

## L-002 — Release readiness

- Dependencies: independent verification/security reviews and DOC-003.
- Evidence: same-artifact qualification, rollback instructions, dependency and
  license review, and release notes identifying the intentionally breaking CLI
  boundary.
- Completion: release engineer reports readiness; no release is authorized by
  this task.
