# Implementation task breakdown: save server entity

Status: architecture task plan for approved feature specification v2.

Application work begins only after Q-SAVE-001 and S-SAVE-SEC-001 converge.
All tasks preserve unrelated workspace changes and never modify `codemie/` or
`codemie-ui/`. After Rust changes, run `make format` and `make lint` before
completion.

## 1. Dependency and evidence policy

Normative inputs:

- [`spec.md`](spec.md), approved v2;
- [`plan.md`](plan.md) and [`data-model.md`](data-model.md);
- ADR-013–017;
- every file under [`contracts/`](contracts/);
- inherited parent declaration, HTTP, CLI, identity, output, and security
  contracts.

No task may weaken an inherited contract. A field/route mismatch, unsupported
filesystem primitive, or validator divergence is an architecture/security
finding, not permission to infer or fall back.

## 2. Pre-implementation gates

### Q-SAVE-001 — Verify pinned save-read and reverse contract

- Objective: independently compare the reverse manifest to approved spec v2,
  declaration schema, and pinned reference-only source.
- Requirements: C-SAVE-005; FR-SAVE-007–023/030; DR-SAVE-002–008;
  IR-SAVE-001–006; VR-SAVE-006–010/013; AC-SAVE-001–015/020/021/025/026.
- Architecture: ADR-013, reverse manifest/schema, plan §§3/8/13/23.
- Scope: every route, alias, response field, nested object, reference route,
  pagination invariant, secret/exclusion, exportability predicate, and
  Datasource discriminator.
- Expected artifacts: `specs/save-server-entity/verification-preimplementation.md`.
- Dependencies: none.
- Acceptance evidence:
  - checkout tag/commit equality;
  - field inventory has one and only one DR-SAVE-002 class;
  - `vcs_type` and `index_type` never enter Datasource natural filtering;
  - positive/negative fixture matrix is enumerated;
  - no reference-only edit is proposed.
- Validation: JSON/schema validation commands from Q-SAVE-002 plus source-line
  cross-checks recorded reproducibly.
- Risk: an enriched response may serialize a nested object differently from
  its persistence model.
- Completion: no unresolved contract/source conflict; otherwise architecture
  returns to NOT READY and implementation does not begin.

### Q-SAVE-002 — Validate architecture artifacts and closed schemas

- Objective: prove every checked-in JSON/Schema/Markdown contract is
  well-formed, linked, closed, and internally consistent.
- Requirements: FR-SAVE-028–030; QR-SAVE-009.
- Architecture: ADR-013–017; all feature contracts.
- Scope: JSON parse, Draft 2020-12 meta-schema, positive/negative instances,
  relative links, duplicate JSON keys, whitespace, and ID traceability.
- Expected artifacts: evidence section in pre-implementation verification.
- Dependencies: Q-SAVE-001 may run in parallel.
- Acceptance evidence:
  - every existing v1 outcome/diagnostic instance remains valid in v2;
  - `saved` and adoption-required positives pass;
  - forbidden adoption combinations and every wrong code/category/exit pair
    fail;
  - manifest validates and missing/extra fields fail its schema.
- Validation: repository-supported JSON Schema validator plus `jq`, link
  checker, and `git diff --check`.
- Risk: checking JSON syntax without meta-schema/instance probes gives false
  confidence.
- Completion: all checks reproducibly pass.

### S-SAVE-SEC-001 — Pre-implementation security review

- Objective: independently approve the save trust boundaries before Rust work.
- Requirements: FR-SAVE-004/015/016/022/023/025–030;
  DR-SAVE-002/006/008/009; QR-SAVE-003–007; AC-SAVE-013–018/021–024/026.
- Architecture: ADR-013/016/017, reverse manifest, Skill snapshot, publication,
  v2 diagnostic contract.
- Scope: malicious response DTOs, prompt/content canaries, nested credentials,
  masks, managed IDs, logging/panic behavior, path traversal/symlink/reparse,
  hard-link/rename races, staging visibility, cancellation, orphan-sidecar
  residue, filesystem support.
- Expected artifacts: `specs/save-server-entity/security-review-preimplementation.md`.
- Dependencies: Q-SAVE-001/002.
- Acceptance evidence: threat model, reproducible finding evidence, severity,
  explicit disposition, and no unresolved high/critical finding.
- Validation: schema probes and proposed failure/canary matrix review.
- Risk: publication correctness depends on OS/filesystem semantics.
- Completion: APPROVED or APPROVED WITH non-blocking findings; otherwise code
  remains blocked.

## 3. Foundations

### F-SAVE-001 — Add typed CLI command and v2 output vocabulary

- Objective: add exact save parsing/domain types and closed result taxonomy
  without performing reads or writes.
- Requirements: FR-SAVE-001/002/005/006/028/029;
  VR-SAVE-001–004; AC-SAVE-003/004/009/016/023/024.
- Architecture: CLI save contract, outcome/diagnostic v2 schemas,
  data model §§2/10.
- Scope: clap command/argument groups, effective project/config resolution,
  validated selectors/new paths, `Action::Saved`, `adoptionRequired`, new safe
  error codes/category, renderer fixtures.
- Expected files/components: `src/cli/mod.rs`, `src/config/mod.rs`,
  `src/output/mod.rs`, `src/render/mod.rs`, `src/error.rs`, new `src/save/` domain
  module, CLI/output tests.
- Dependencies: Q-SAVE-001/002, S-SAVE-SEC-001.
- Acceptance evidence:
  - every exact valid command parses;
  - every cross-kind/missing/repeated/secret flag fails before network;
  - invalid explicit project does not fall back;
  - v1 render fixtures remain byte-identical;
  - save outcomes/diagnostics validate against v2.
- Validation: targeted Rust unit/integration tests, schema instance tests,
  `make format`, `make lint`.
- Risk: clap may exit outside the typed renderer; integration tests must verify
  selected-mode behavior for runtime validation and E_USAGE contract.
- Completion: typed command can be constructed, but no server/filesystem
  mutation path is wired.

### F-SAVE-002 — Add read-only HTTP capability and save deadline

- Objective: make it type-impossible for save adapters/coordinator to dispatch
  a modifying request.
- Requirements: FR-SAVE-004/006/030; IR-SAVE-001/004–006;
  QR-SAVE-003/006; AC-SAVE-020–022.
- Architecture: plan §§11/13, parent HTTP contract, ADR-013.
- Scope: narrow `ReadApi` interface/facade exposing bounded GET, optional GET,
  query encoding, and visibility; shared cancellation/deadline; safe route
  context. Existing modifying functions remain private to prepared apply writes.
- Expected files/components: `src/http/mod.rs`, `src/cancellation.rs`,
  `src/save/mod.rs`, transport compile-surface and instrumented tests.
- Dependencies: F-SAVE-001.
- Acceptance evidence:
  - save production graph exposes no POST/PUT/PATCH/DELETE function;
  - 60-second requests/300-second invocation, body/header/depth/page budgets,
    retries, TLS, redirects, token input remain inherited;
  - exact pinned clone reporting info version 0.16.0 is not rejected merely by
    that value;
  - raw body/error never reaches output or tracing.
- Validation: compile-surface assertions, fake-server method journal for every
  error class, timeout/retry tests.
- Risk: sharing `ApiClient` directly could accidentally expose internal write
  methods; use a capability wrapper or sealed trait.
- Completion: read-only integration boundary independently testable.

### F-SAVE-003 — Implement strict reverse DTO framework

- Objective: establish manifest-aligned boundary decoding and normalized
  snapshot primitives shared by all four adapters.
- Requirements: FR-SAVE-013–016/023/030; DR-SAVE-002/003;
  IR-SAVE-006; QR-SAVE-005/009; VR-SAVE-009/010.
- Architecture: ADR-013, reverse manifest, data model §§3/4.
- Scope: `deny_unknown_fields` contracted DTOs or equivalent duplicate/unknown
  enforcement, ignored non-retaining types, strict JSON/YAML decode helpers,
  managed ID newtype, non-exportable/incompatible errors, field-mutation fixture
  harness.
- Expected files/components: new `src/save/read.rs`, `src/save/snapshot.rs`,
  adapter test support.
- Dependencies: F-SAVE-002.
- Acceptance evidence: deleting/mutating every consumed field fails before
  artifacts; extra contracted fields fail; ignored secret fields cannot be
  debug/serialize formatted; no managed-ID serialization path.
- Validation: generated mutation matrix and compile-time trait assertions.
- Risk: serde flatten/default can weaken missing/unknown checks.
- Completion: adapter authors cannot bypass a classified snapshot constructor.

## 4. Entity read/reverse adapters

### A-SAVE-001 — Implement Assistant save adapter

- Objective: save one exact codemie Assistant and recover all natural refs.
- Requirements: FR-SAVE-003/007/013–018/023/030;
  DR-SAVE-001–004; IR-SAVE-001/003/006; AC-SAVE-001/010/015.
- Architecture: reverse manifest `Assistant`; data model §§4/6.
- Scope: direct slug detail, exact project/slug, author fields, category ID
  projection, Settings/MCP safe projection, same-project context resolution,
  sub-Assistant/Skill reference recovery, mask/non-codemie rejection.
- Expected files/components: `src/adapters/assistant.rs` or new
  `src/save/adapters/assistant.rs`, fixtures/tests.
- Dependencies: F-SAVE-003; S/D reference helpers may be introduced narrowly.
- Acceptance evidence: full field positive golden, one negative per field
  class/reference failure, mask canary, managed-ID recursive scan, zero writes.
- Validation: fake-server route/method journal and projector fixture tests.
- Risk: enriched category/Settings DTOs contain extra audit/credential fields.
- Completion: returns only typed `AssistantSnapshot` and natural map.

### W-SAVE-001 — Implement Workflow save adapter

- Objective: support marked natural selection and unmarked exact-ID conversion
  without marker mutation.
- Requirements: FR-SAVE-008–010/013–018/021/030;
  DR-SAVE-001–005; IR-SAVE-001–003/006;
  VR-SAVE-006–009; AC-SAVE-002–005/010.
- Architecture: parent ADR-008, reverse manifest `Workflow`.
- Scope: two exhaustive zero-based passes, strict reserved marker classifier,
  ID candidate checks, strict `meta_config` JSON and `yaml_config` YAML,
  remove reserved member only, recover persisted/inline refs, preserve graph IDs,
  set typed adoption-required only after all checks.
- Expected files/components: `src/adapters/workflow.rs` refactor/shared scan or
  `src/save/adapters/workflow.rs`, decoder/projector tests.
- Dependencies: F-SAVE-003 and A/S/D reference resolvers.
- Acceptance evidence: marked/unmarked/malformed/conflict/churn matrix,
  display-name irrelevance, UUID absent from all bytes/logs, no server mutation.
- Validation: page origin/fingerprint tests, strict duplicate-key tests,
  reference golden and method journal.
- Risk: reusing apply adoption code could accidentally seal a prepared write;
  share pure scan/classification only.
- Completion: typed Workflow snapshot/projected identity with no write ability.

### S-SAVE-001 — Implement Skill resolution and observed-stable snapshot

- Objective: read a unique Skill, exact main content, and complete companion
  payloads without mixing detected revisions.
- Requirements: FR-SAVE-011/019/020/030; DR-SAVE-006;
  IR-SAVE-001/002/006; QR-SAVE-005/006; AC-SAVE-006–009.
- Architecture: ADR-016, Skill snapshot contract, reverse manifest `Skill`.
- Scope: complete visibility/list, detail A/B/C, payload A/B in normalized order,
  content/path/encoding/base64/size/budget validation, safe toolkit/MCP
  projection, exact main sidecar bytes.
- Expected files/components: `src/adapters/skill.rs` shared scan or
  `src/save/adapters/skill.rs`, snapshot/content fixtures.
- Dependencies: F-SAVE-003.
- Acceptance evidence: churn injected between every observation, duplicate and
  unsafe paths, metadata/content mismatch, binary/text companion cases, exact
  sidecar bytes, no payload/path/ID logs.
- Validation: fake server sequence assertions, body limits, cancellation,
  snapshot goldens.
- Risk: doubled payload reads approach deadline; enforce sequential budgets and
  no unbounded buffering beyond approved totals.
- Completion: returns `SkillObservedStableSnapshot` only after exact equality.

### D-SAVE-001 — Implement Datasource natural resolution and reverse union

- Objective: resolve project+repo_name only and project the exact supported
  persisted discriminator combination.
- Requirements: FR-SAVE-012–016/022/023/030; DR-SAVE-003/008;
  IR-SAVE-001–003/006; VR-SAVE-006/008–010/013;
  AC-SAVE-013/014/025/026.
- Architecture: v2 spec correction, reverse manifest `Datasource`, data model §5.
- Scope: complete list visibility, exact two-field filtering, detail,
  `index_type`+`vcs_type` composite, every supported non-File branch, alias
  mapping, scheduling/guardrails, SharePoint secret exclusion, typed
  File/provider/Bedrock non-exportability.
- Expected files/components: `src/adapters/datasource.rs` shared scan or
  `src/save/adapters/datasource.rs`, per-branch fixtures.
- Dependencies: F-SAVE-003.
- Acceptance evidence:
  - duplicates remain ambiguous regardless of discriminator;
  - full 3x2 code strategy/VCS matrix;
  - Autonomous and deprecated Workflow states return
    `E_ENTITY_NOT_EXPORTABLE` with no publication;
  - missing/wrong/legacy/unknown combinations E_API_INCOMPATIBLE;
  - all nine supported declaration branches positive;
  - File/provider/Bedrock exit 1;
  - access_token/expires_at/provider OTP/settings canaries absent everywhere.
- Validation: schema projection fixtures, response mutation tests, method
  journal proving no `/settings` or `/export` calls.
- Risk: apply resolver currently filters by kind; save must not reuse that
  filtered entry point.
- Completion: every selected detail produces exactly one supported union,
  incompatible error, or approved non-exportable error.

### R-SAVE-001 — Implement pure reverse projector and managed-ID stripping

- Objective: convert normalized snapshots/reference maps into exactly one
  closed declaration AST without I/O/defaulting.
- Requirements: FR-SAVE-013–017/021–023; DR-SAVE-001–005/008;
  VR-SAVE-008–010/013; AC-SAVE-001–003/010/014/015/025.
- Architecture: ADR-013, reverse manifest, data model §§6/7.
- Scope: envelope/metadata, all field aliases, explicit null/current values,
  Workflow reserved removal, reference substitution, local/opaque ID
  preservation, recursive managed-ID and secret canary defense.
- Expected files/components: new `src/save/projection.rs`, per-kind normalized
  JSON fixtures and property tests.
- Dependencies: A/W/S/D-SAVE-001.
- Acceptance evidence: AST validates against current schema; forward apply
  projection accepts it; no server managed ID; current null/empty/defaulted
  server values appear exactly; unsupported state cannot construct output.
- Validation: JSON Schema validation, recursive canary probes, snapshot-to-AST
  goldens, no-I/O compile/module tests.
- Risk: free-form maps may contain UUID-like authored values; tests distinguish
  classified managed positions from content and do not use unreliable regex
  stripping as the implementation rule.
- Completion: only validated closed `ProjectedDeclaration` leaves the module.

## 5. Local artifact construction and validation

### Y-SAVE-001 — Implement canonical YAML and Skill artifact builder

- Objective: create byte-identical immutable artifacts from a projected AST.
- Requirements: FR-SAVE-019/024; DR-SAVE-001/006/007;
  QR-SAVE-001/007/009; AC-SAVE-007/019.
- Architecture: ADR-015, canonical YAML contract.
- Scope: schema-order table, Unicode free-map ordering, array policy, scalar
  rules, literal blocks, LF/final newline, sidecar basename/exact bytes,
  `ArtifactSet` constructor.
- Expected files/components: new `src/save/yaml.rs`, `src/save/artifact.rs`,
  `tests/goldens/save/canonical/**`.
- Dependencies: R-SAVE-001.
- Acceptance evidence: every kind/branch/scalar fixture exact; parse-round-trip
  and re-emission; randomized map insertion order; Linux and supported platform
  equality; Skill sidecar byte identity.
- Validation: golden test command, schema-order coverage test, forbidden YAML
  token/CR/BOM/trailing-space scan.
- Risk: number or block-chomping edge can alter values.
- Completion: immutable artifact bytes meet the canonical contract.

### V-SAVE-001 — Extract shared repository view and overlay validation

- Objective: run exactly the existing lint closure against disk plus generated
  bytes before publication.
- Requirements: FR-SAVE-018/026; VR-SAVE-005/011;
  QR-SAVE-002; AC-SAVE-011/012.
- Architecture: ADR-014, prospective validation contract.
- Scope: `RepositoryView`, disk implementation preserving current behavior,
  overlay non-shadowing implementation, shared validation pipeline, generated
  Skill sidecar resolution, typed target result. Lint warning behavior remains
  outside and unchanged.
- Expected files/components: `src/repository.rs`, `discovery/mod.rs`,
  `parse/mod.rs`, `lint.rs`, new repository view tests.
- Dependencies: Y-SAVE-001 (interface can begin earlier, integration depends on it).
- Acceptance evidence: all current lint/apply tests byte/outcome equivalent;
  overlay versus materialized disk equivalence; duplicate/missing ref/sidecar
  failure; no write/open of proposed finals.
- Validation: full existing test suite plus differential/property tests.
- Risk: synthetic path diagnostics or warning timing can regress parent
  behavior.
- Completion: prospective validation returns success iff immediate disk lint
  after equivalent publication would return 0.

## 6. Publication and coordinator

### P-SAVE-001 — Implement secure no-replace artifact publisher

- Objective: publish one YAML or Skill sidecar+YAML with no clobber and clean
  reported failure.
- Requirements: FR-SAVE-025–027; DR-SAVE-009; QR-SAVE-004/005/007;
  VR-SAVE-004/012; AC-SAVE-009/016–018.
- Architecture: ADR-017, publication contract, data model §9.
- Scope: typed paths, directory-handle no-follow traversal, capability probe,
  owner-only same-dir staging, complete writes/sync, native no-replace adapter,
  YAML-last state machine, cancellation deferral, orphan-sidecar handling,
  typed faults.
- Expected files/components: new `src/save/publication.rs` plus small OS-specific
  submodules and process-level race/fault tests.
- Dependencies: V-SAVE-001 and security gate.
- Acceptance evidence: every pre-publication failure removes staging;
  post-sidecar/pre-YAML failure leaves at most a complete orphan sidecar;
  race-created files unchanged; final bytes complete; YAML never without
  sidecar; unsupported filesystem refused before network.
- Validation: deterministic fault harness, independent-process rename races,
  cancellation matrix, qualified filesystem suite.
- Risk: filesystem semantic mismatch is release-blocking; no fallback.
- Completion: at least one declared target platform passes the full contract.

### C-SAVE-001 — Implement read-only save coordinator and end-to-end command

- Objective: connect validated command, adapters, projector, artifact builder,
  prospective validator, publisher, and one outcome under one deadline.
- Requirements: all FR-SAVE-001–030, IR-SAVE-001–006, and AC-SAVE-001–026.
- Architecture: plan §§11–17 and data model lifecycle.
- Scope: phase ordering/checkpoints, kind dispatch, reference reads, adoption
  flag construction, cleanup/drop boundaries, exact output/exit behavior. No
  write dispatcher, automatic lint subprocess, or Git action.
- Expected files/components: `src/save/mod.rs`, `src/cli/mod.rs`, `src/main.rs`,
  end-to-end CLI tests.
- Dependencies: F-SAVE-001/002, all adapters, R/Y/V/P-SAVE-001.
- Acceptance evidence: executable AC-SAVE-001–026; exact HTTP method/request
  sequence; pre-network collision; immediate lint; output schema/bytes;
  cancellation at every phase; no retained ID/secret/staging.
- Validation: full Rust tests, acceptance fake server, instrumented filesystem,
  `make format`, `make lint`.
- Risk: late error after commit could falsely report failure; coordinator treats
  publisher `Committed` as irreversible success.
- Completion: all approved behavior passes with no unrelated command regression.

## 7. Security, verification, documentation, and release

### S-SAVE-SEC-002 — Post-implementation security review

- Objective: verify controls are implemented, not merely designed.
- Requirements: QR-SAVE-003–007 and all security-sensitive FR/AC.
- Architecture: pre-review findings and ADR-013/016/017.
- Scope: code/dependencies/config/tests/logging/panic/output, response canaries,
  ID stripping, path/race/failure injection, artifact/staging permissions.
- Expected artifact: `specs/save-server-entity/security-review-postimplementation.md`.
- Dependencies: C-SAVE-001.
- Acceptance evidence: independent reproduction and no unresolved high/critical
  finding.
- Validation: targeted exploit/race/canary commands recorded.
- Risk: tests that mock away OS primitives cannot establish production safety.
- Completion: security readiness verdict.

### V-SAVE-002 — Post-implementation convergence verification

- Objective: prove spec, architecture, code, tests, and behavior converge.
- Requirements: every FR/DR/IR/QR/VR/AC.
- Architecture: plan §23 and all tasks.
- Scope: trace audit, contract/source DTO comparison, test execution, code
  inspection, negative surface, zero-write evidence, platform evidence.
- Expected artifact: `specs/save-server-entity/verification-postimplementation.md`.
- Dependencies: C-SAVE-001, S-SAVE-SEC-002 may run independently.
- Acceptance evidence: explicit PASS/FAIL per AC-SAVE-001–026 and requirement
  family; implementer claims alone are not evidence.
- Validation: full project checks, JSON/schema/link/whitespace, acceptance and
  failure-injection suites.
- Risk: platform tests on only tmpfs do not qualify production filesystem.
- Completion: READY or NOT READY for release engineering.

### O-SAVE-001 — Document author and operator workflow

- Objective: explain dependency-first save, review, later Workflow adoption,
  non-exportability, collision, and crash-orphan recovery without claiming
  ownership transfer.
- Requirements: BR-SAVE-001–009; FR-SAVE-010/018/022/025–029.
- Architecture: CLI, publication, existing Workflow adoption runbook.
- Scope: README/help/examples/runbook outside reference-only trees; no live
  credentials or server IDs in examples.
- Expected files/components: README/help and `ops/` documentation selected by
  implementation plan.
- Dependencies: C-SAVE-001 behavior stable; can be drafted earlier.
- Acceptance evidence: commands match CLI contract; unmarked Workflow UUID is
  explicitly kept out of repository/output; orphan sidecar recovery is manual
  and narrow; File/provider/Bedrock behavior accurate.
- Validation: help snapshots, Markdown links, example offline lint.
- Risk: wording “own” could imply authorization/marker transfer.
- Completion: independent reviewer can follow safe onboarding end to end.

### L-SAVE-001 — Assess release readiness and filesystem qualification

- Objective: decide whether the implemented binary is releasable on explicitly
  named OS/filesystem pairs.
- Requirements: QR-SAVE-001–009; AC-SAVE-016–022/024.
- Architecture: plan §16, publication §8.
- Scope: CI/package/supply chain, same artifact promotion, support matrix,
  rollback, release notes, operational docs; no release without user authority.
- Expected artifact: release-readiness report/release notes prepared by release
  engineer.
- Dependencies: V-SAVE-002 and S-SAVE-SEC-002 ready verdicts, O-SAVE-001.
- Acceptance evidence: full checks, checksum/SBOM, no secrets, qualified
  process-race tests on each claimed filesystem, binary rollback procedure.
- Validation: repository release workflow plus platform qualification commands.
- Risk: advertising untested network/container filesystems violates ADR-017.
- Completion: READY/NOT READY recommendation only; no tag/publish/deploy without
  explicit authorization.

## 8. Completion audit

| Requirement family | Implementation owners | Independent gates |
|---|---|---|
| FR-SAVE-001–006 | F-SAVE-001/002, C-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| FR-SAVE-007–012 | A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-013–023 | F-SAVE-003, adapters, R-SAVE-001 | Q-SAVE-001, S-SAVE-SEC-001/002, V-SAVE-002 |
| FR-SAVE-024 | Y-SAVE-001 | V-SAVE-002, L-SAVE-001 |
| FR-SAVE-025–027 | P-SAVE-001 | S-SAVE-SEC-001/002, V-SAVE-002, L-SAVE-001 |
| FR-SAVE-028–030 | F-SAVE-001/002, C-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| DR-SAVE-001–009 | R/Y/V/P-SAVE-001 plus adapters | Q-SAVE-001/002, V-SAVE-002 |
| IR-SAVE-001–006 | F-SAVE-002 plus adapters | Q-SAVE-001, V-SAVE-002 |
| QR-SAVE-001–009 | Y/V/P/C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002, L-SAVE-001 |
| VR-SAVE-001–013 | Foundations, adapters, V/P-SAVE-001 | V-SAVE-002 |
| AC-SAVE-001–026 | C-SAVE-001 and primary component tasks in plan §23 | V-SAVE-002 |

No generic task may be marked complete in place of the explicit security,
observability, documentation, platform qualification, rollback, or cleanup
work above.
