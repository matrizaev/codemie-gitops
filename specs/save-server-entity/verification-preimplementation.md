# Artifact analysis report: save server entity

## Status

**Artifact analysis status: READY FOR IMPLEMENTATION**

Review date: 2026-08-12.

This is an independent Mode A review. It does not approve release. The current
normative artifacts incorporate the pinned-server corrections and the approved
product decisions: deprecated Autonomous Workflows are non-exportable, present
nulls are preserved as YAML null, save never adopts, validation follows the
current lint order, and publication never rolls back final paths. The security
review is approved with non-blocking findings. Implementation may begin from
the accepted ADRs and contracts; implementation tests remain required.

The detailed findings below preserve the original review evidence for audit.
They are historical unless explicitly marked CURRENT in the status table that
follows.

### Current implementation addendum — 2026-08-14

The implementation has since converged on the v3.4 artifacts. The reverse
projector now normalizes OpenAPI response objects into declaration fields for
Assistant and Skill context, categories, toolkits, MCP servers, and integration
settings. Automated evidence includes 210 Rust unit tests, 6 CLI tests, 4
OpenAPI contract tests, `cargo check --workspace --all-targets --locked`,
`make format`, and `make lint`. Live-server qualification remains a release
readiness activity and is not claimed by this historical pre-implementation
report.

## Current gate summary

| Gate | Status | Evidence |
|---|---|---|
| Pinned reverse contract | PASS | Corrected manifest validates against its closed schema; Workflow paths, visibility routes, Datasource filter, MCP predicate, and Autonomous exclusion are explicit. |
| Architecture artifacts | PASS | JSON parsing, schema validation, and whitespace checks pass. |
| Security review | PASS WITH NON-BLOCKING FINDINGS | `security-review-preimplementation.md` covers response, secret, ID, path, race, staging, orphan-sidecar, cancellation, and diagnostic controls. |
| ADR-013 through ADR-017 | ACCEPTED | All five ADR status fields are `Accepted`. |
| Implementation readiness | READY | Autonomous rejection and nine-branch Datasource coverage are explicit implementation acceptance obligations. |

No current pre-implementation blocker remains in the specification or
architecture artifacts. Platform qualification and implementation evidence are
still required before release.

## Scope

- Feature: save one server-side Assistant, Workflow, Skill, or Datasource as a
  local declaration without modifying the server.
- Authoritative product source: [`spec.md`](spec.md), status APPROVED, version 2.
- Architecture: [`plan.md`](plan.md), [`data-model.md`](data-model.md),
  [`tasks.md`](tasks.md), [`traceability.md`](traceability.md).
- Contracts: every Markdown and JSON artifact under [`contracts/`](contracts/).
- ADRs: [`ADR-013`](adr/013-versioned-save-read-reverse-contract.md) through
  [`ADR-017`](adr/017-no-clobber-publication.md).
- Parent sources: the current artifacts under `specs/codemie-cicd-tool/`,
  including the v1alpha1 declaration schema and apply adapter/read contracts.
- External evidence: reference-only `codemie/` tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- Excluded: implementation and post-implementation verification. No Jira or
  Confluence content was supplied or claimed as reviewed.

## Executive assessment

The command surface, configuration precedence, four-kind task decomposition,
canonical YAML design, Skill A/B/C observation sequence, corrected Datasource
`index_type`/`vcs_type` mapping, GET-only boundary, and v2 output/diagnostic
schema compatibility are substantially specified and traceable. Every
FR/DR/IR/QR/VR/AC identifier has an architecture row, task owner, and evidence
gate.

The artifacts nevertheless do not converge on executable behavior:

1. The pinned Workflow list implementation excludes Autonomous rows from both
   enumeration passes, but the approved specification and reverse manifest call
   some Autonomous workflows exportable.
2. The Workflow reference paths in the normative reverse manifest have an
   extra `execution_config` segment and therefore do not address the stored
   payload.
3. The normative manifest omits the required visibility preflight and does not
   provide exact, independently classifiable reference-read contracts.
4. Valid server states containing required custom MCP configuration or nullable
   required authoring values lack complete non-exportability predicates.
5. The text success contract hides the mandatory unmarked-Workflow adoption
   result.
6. The prospective validator contract changes the observable sidecar-validation
   order that it says it preserves.
7. Publication introduces an undefined fail-stop terminal outside the declared
   lifecycle and one-diagnostic contract.

The ADRs remain `Proposed`; their own traceability gate says they must be
accepted before dependent work starts. Acceptance alone would not cure the
technical findings below.

## Evidence consulted

### Repository artifacts

- Feature specification, plan, data model, task list, traceability matrix,
  ADR-013..017, and all feature contracts/schemas.
- Parent specification, plan, data model, tasks, ADR-007/008/009/012,
  `contracts/adapter-manifest-v2.42.0.json`, `contracts/http-adapter.md`,
  `contracts/declaration-v1alpha1.schema.json`, and source baseline.
- Current shared validation implementation: `src/repository.rs` and
  `src/parse/mod.rs`.
- Pinned backend routing/models/services for Assistant, Workflow, Skill,
  Datasource, filters, user visibility, MCP storage, scheduler enrichment, and
  reference resolution. The checkout identity was independently confirmed by:

```text
git -C codemie rev-parse HEAD
2a481c290c99bf30ef80aadafa03d876a7f5f732

git -C codemie describe --tags --exact-match HEAD
2.42.0
```

The `codemie/` and `codemie-ui/` trees were not modified.

## Validation performed

| Check | Observed result |
|---|---|
| Duplicate-key JSON parsing, using Python `json.loads(..., object_pairs_hook=...)`, across the three feature JSON contracts and inherited parent schemas they reference | PASS: 10 files |
| `jsonschema.Draft202012Validator.check_schema` on `save-read-reverse-v1.schema.json`, `outcome-v2.schema.json`, and `diagnostic-v2.schema.json` | PASS |
| Reverse-manifest instance validation | PASS |
| Reverse-manifest negative probes: missing `entities`, extra root property, modifying method, wrong pinned commit | PASS: all rejected |
| Outcome-v2 positive/negative instance probes | PASS: 5 positive, 5 negative |
| Diagnostic-v2 code/category/exit closure | PASS: all 34 approved triples accepted; 68 wrong category/exit mutations rejected |
| Parent outcome-v1 instances against outcome-v2 | PASS: 12/12 |
| Every parent diagnostic-v1 instance against diagnostic-v2 | PASS |
| Markdown local-link resolution | PASS: no missing targets |
| Stable-ID traceability count | PASS: FR 30/30, DR 9/9, IR 6/6, QR 9/9, VR 13/13, AC 26/26 |
| `jq empty` on feature and parent JSON contracts | PASS |
| `git diff --check -- specs/save-server-entity specs/codemie-cicd-tool` | PASS |
| Trailing-whitespace scan over reviewed Markdown/JSON | PASS |

These structural passes do not prove semantic correctness; findings VER-SAVE-001
through VER-SAVE-004 are examples of errors accepted by the schemas.

## Contract-correction addendum

After the server-code fact review, the normative reverse manifest was corrected
without changing the product scope:

- VER-SAVE-002 is corrected in the manifest: Workflow reference paths now use
  the root `yaml_config.assistants[]` and `yaml_config.states[]` positions.
- VER-SAVE-003b is corrected in the manifest: Datasource filters are encoded
  as JSON using the server's `project` key. The missing exact response-field
  inventories and independent semantic route checks remain open.
- The manifest now records `/v1/user` visibility evidence for Workflow, Skill,
  and Datasource resolution and splits reference list/detail routes. This
  narrows VER-SAVE-003 but does not close it until the exact pinned response
  fields and predicates are independently verified.
- VER-SAVE-004a is addressed by an explicit non-exportability predicate for
  persisted MCP custom configuration or auth tokens. VER-SAVE-004b remains open
  for nullable required authoring fields.

The addendum above is superseded by the current gate summary. The historical
findings below are retained for audit and are not current blockers.

## Traceability coverage and convergence

`traceability.md` supplies an owner and independent gate for every identifier.
The table below records artifact convergence, not implementation completion.
`CONVERGED` means no pre-implementation contradiction was found in that group;
it is not post-implementation satisfaction.

| IDs | Status | Finding/evidence |
|---|---|---|
| FR-SAVE-001–006 | CONVERGED | Exact CLI surface/precedence and GET-only boundary are assigned to F/C work packages. |
| FR-SAVE-007, 011–13, 19–20, 24–25, 28–29 | CONVERGED | Selection, Skill snapshot/content, declaration, canonical rendering, no-overwrite precheck, and safe output have contracts/tasks. |
| FR-SAVE-008–009 | NOT READY | VER-SAVE-001; enumeration cannot cover every specified Workflow state. |
| FR-SAVE-010 | NOT READY | VER-SAVE-005. |
| FR-SAVE-014–17, 21–23, 30 | NOT READY | VER-SAVE-002/003/004. |
| FR-SAVE-018, 026 | NOT READY | VER-SAVE-006. |
| FR-SAVE-027 | NOT READY | VER-SAVE-007. |
| DR-SAVE-001, 006–008 | CONVERGED | Closed declaration, Skill content, canonical YAML, and corrected composite Datasource mapping are covered. |
| DR-SAVE-002–005 | NOT READY | VER-SAVE-002/003/004. |
| DR-SAVE-009 | NOT READY | VER-SAVE-007. |
| IR-SAVE-004–005 | CONVERGED | Inherited transport/config and pinned backend identity are explicit. |
| IR-SAVE-001–003, 006 | NOT READY | VER-SAVE-001/002/003. |
| QR-SAVE-001, 003, 006, 008 | CONVERGED | Determinism, read-only boundary, budgets/deadline, and output closure are tasked. |
| QR-SAVE-002 | NOT READY | VER-SAVE-006. |
| QR-SAVE-004 | NOT READY | VER-SAVE-007. |
| QR-SAVE-005, 009 | NOT READY | VER-SAVE-003/004 and semantic gaps not caught by schema. |
| QR-SAVE-007 | PARTIAL | Overlay/canonical portability is covered; publication terminal behavior is not (VER-SAVE-007). |
| VR-SAVE-001–005, 007, 013 | CONVERGED | Local validation/identity/marker/discriminator rules are present; Datasource mapping is correct. |
| VR-SAVE-006 | NOT READY | VER-SAVE-001/003. |
| VR-SAVE-008–010 | NOT READY | VER-SAVE-002/003/004. |
| VR-SAVE-011 | NOT READY | VER-SAVE-006. |
| VR-SAVE-012 | PARTIAL | No-replace race behavior is defined, but rollback has an undefined terminal (VER-SAVE-007). |
| AC-SAVE-001, 005–009, 013–16, 19–20, 22–26 | CONVERGED | Scenario-level contract/task coverage exists; AC-025/026 prove the corrected Git/SVN strategy mapping. |
| AC-SAVE-002, 010 | NOT READY | VER-SAVE-002/003. |
| AC-SAVE-003–004 | NOT READY | VER-SAVE-001/005. |
| AC-SAVE-011–012 | NOT READY | VER-SAVE-006. |
| AC-SAVE-017–018 | NOT READY | VER-SAVE-007. |
| AC-SAVE-021 | NOT READY | VER-SAVE-003/004. |

## Findings

### Finding ID: VER-SAVE-001

Severity: **BLOCKER**

Status: **SUPERSEDED**

Title: Workflow enumeration cannot observe the Autonomous rows that the feature
calls exportable

Evidence:

- `spec.md:1164` permits an Autonomous/deprecated Workflow state that remains
  schema-valid to be saved.
- `contracts/save-read-reverse-v2.42.0-v1.json:355` includes “Sequential or
  schema-valid Autonomous Workflow” in supported exportability.
- Pinned `codemie/src/codemie/service/workflow_config/workflow_config_index_service.py:75-80`
  defines the Sequential-only modifier; lines 107-116 apply it to both project
  and marketplace enumeration.

Expected: FR-SAVE-008/009 and VR-SAVE-006 require exhaustive selection and a
complete marker-conflict proof for every saveable Workflow state.

Actual: both approved list passes hide every Autonomous row. Natural selection
cannot find one; ID selection cannot prove absence of a hidden valid or invalid
marker conflict.

Impact: a required supported state is unreachable, and ID-based adoption can be
accepted without the specified complete conflict proof.

Required action: the product specification owner must decide whether Autonomous
Workflow is non-exportable in v1 or authorize another read surface. The solution
architect must then make selection, manifest routes, compatibility behavior,
tests, and source evidence agree.

Owner: product-spec-owner, then solution-architect

Verification: test marked and unmarked Autonomous rows, including hidden
valid/invalid marker conflicts, against the exact pinned API; prove either safe
selection or the approved non-exportability result.

### Finding ID: VER-SAVE-002

Severity: **BLOCKER**

Status: **SUPERSEDED**

Title: Workflow managed-reference source paths address a nonexistent nested
object

Evidence:

- Pinned `codemie/src/codemie/core/workflow_models/workflow_config.py:180-200`
  extracts declaration `execution_config` and serializes that object directly
  into `WorkflowConfig.yaml_config`; `parse_execution_config` reads
  `yaml_data.assistants` directly at lines 221-234.
- The normative manifest uses
  `yaml_config.execution_config.assistants[...]` and
  `yaml_config.execution_config.states[...]` at lines 319-322 and 350-352.
- The manifest's own `workflowExecutionConfig` shared component correctly lists
  `assistants`, `tools`, `custom_nodes`, and `states` at its root.

Expected: FR-SAVE-016/017/021 and DR-SAVE-004/005 require every managed ID to be
reversed from the exact persisted position, with Workflow-local IDs preserved.

Actual: the normative paths contain one extra `execution_config` segment.

Impact: Workflow references can be missed, leak managed IDs, or cause valid
Workflows to be projected without required natural references.

Required action: correct every Workflow field/reference source path and add
positive/negative fixtures built from the pinned detail response, including
persisted and inline actors, Skill/Datasource arrays, and local state IDs.

Owner: solution-architect

Verification: mutation tests must fail when a managed-ID path is skipped or when
a local graph ID is stripped.

### Finding ID: VER-SAVE-003

Severity: **HIGH**

Status: **SUPERSEDED**

Title: The normative read manifest is not an exact, complete read/field contract

Evidence:

- `spec.md:812-818` and `contracts/skill-snapshot-v1.md:10-16` require
  `GET /v1/user` for Workflow, Skill, and Datasource complete visibility. The
  reverse manifest contains no `/v1/user` route, fields, or predicate. The parent
  adapter manifest provides the missing route and exact consumed fields at
  `specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json:52-61`.
- Assistant and Workflow reference reads are pseudo-routes such as
  `"/v1/skills and /v1/skills/{id}"` and
  `"/v1/index and /v1/index/{id}"` (manifest lines 149-163 and 290-303),
  without separate list/detail pagination, query encoding, response fields, or
  visibility requirements.
- Datasource uses `filters={project_name}` at line 453, while the pinned route
  JSON-decodes `filters` (`routers/index.py:195-218`) and its accepted exact
  project filter key is `project` (`service/filter/filter_services.py:23-35`).
- The schema accepts any route string beginning `/v1/` and cannot detect these
  omissions. Entity `fieldRules` are not route-specific consumed-field
  inventories, so DR-SAVE-002's exactly-one classification is not demonstrable.

Expected: DR-SAVE-002, IR-SAVE-001–003/006, FR-SAVE-030, and Q-SAVE-001 require
exact routes, query shapes, pagination, response-field inventories, visibility,
and closed compatibility handling for every consumed read.

Actual: essential reads are absent or collapsed into descriptive placeholders.

Impact: implementations can conform to the manifest while using incomplete
visibility, a wrong filter, or permissive/inconsistent DTOs.

Required action: split every reference list/detail/content call into an exact
route record; add `/v1/user`; define percent-encoded query JSON, applicability,
all consumed response/pagination fields, unknown/additive classification, and
source evidence. Add a semantic contract validator for route-name references
and exactly-one field classification.

Owner: solution-architect

Verification: Q-SAVE-001 must compare every route/field to pinned source and run
missing/wrong/unknown-field and pagination mutation fixtures per read.

### Finding ID: VER-SAVE-004

Severity: **HIGH**

Status: **SUPERSEDED**

Title: Exportability predicates do not cover valid secret-dependent and nullable
server states

Evidence:

- The manifest labels MCP `config` and `mcp_connect_auth_token` secret/masked but
  provides no whole-entity predicate when either is required. Pinned
  `MCPAccessControlService` preserves inline config on save
  (`service/mcp/access_control.py:118-131`) and uses it when
  `use_custom_config=true` (lines 159-181). The declaration MCP schema cannot
  represent `config` or the auth token.
- Pinned source types `Assistant.llm_model_type`, `Workflow.yaml_config`, code
  Datasource `link`/`branch`, and several `setting_id` fields as optional, while
  corresponding declaration branches require non-null authoring values. The
  manifest calls these direct-authorable or bounded-transform without explicit
  present-null non-exportability rules.
- Workflow `meta_config` is a valid nullable persisted string, but its rule only
  says strict JSON-object decode and does not define the unmarked/null branch.

Expected: FR-SAVE-014/023 and DR-SAVE-002/003 require each valid server state to
produce a complete faithful declaration, an approved non-exportable result, or
a compatibility result for an actually incompatible response shape.

Actual: state-dependent required-secret and required-null cases are not closed.

Impact: an implementation may silently discard required custom behavior,
misclassify a valid server state as API drift, invent defaults, or emit a
schema-invalid/semantically incomplete declaration.

Required action: audit every branch-required field against pinned nullability;
define explicit complete predicates and exact result codes. At minimum cover MCP
custom config/auth token, Assistant required model, Workflow yaml/meta nulls,
and Datasource required link/branch/settings fields.

Owner: solution-architect; product-spec-owner for any change to supported states

Verification: per-kind fixtures for absent, explicit null, masked, secret,
encrypted, and complete values must prove exactly one of projection,
`E_ENTITY_NOT_EXPORTABLE`, or `E_API_INCOMPATIBLE`, with no partial artifacts.

### Finding ID: VER-SAVE-005

Severity: **HIGH**

Status: **SUPERSEDED**

Title: Text output does not communicate the mandatory Workflow adoption result

Evidence:

- FR-SAVE-010 says a successful ID-selected Workflow outcome must include
  `adoptionRequired: true`.
- `contracts/cli-save-v1.md:86-96` fixes text output to
  `saved Workflow <project>/<slug>` and explicitly adds no text-mode indication,
  relying on external documentation. JSON does include the property.

Expected: the selected output mode's sole success outcome communicates the
required adoption state without leaking the UUID.

Actual: text mode is indistinguishable from a fully adopted/naturally selected
Workflow save.

Impact: the default human-facing mode can imply ownership is complete when a
separate reviewed adoption action is still required.

Required action: either add a safe adoption-required token to the one text line,
or obtain an explicit product-spec amendment limiting FR-SAVE-010 to JSON and
defining the human-mode behavior.

Owner: product-spec-owner, then solution-architect

Verification: render tests for marked natural selection and unmarked ID
selection in both text and JSON modes.

### Finding ID: VER-SAVE-006

Severity: **HIGH**

Status: **SUPERSEDED**

Title: Prospective validation order contradicts the current shared lint engine

Evidence:

- `contracts/prospective-validation-v1.md:10-18` says observable order remains
  schema, effective project, natural semantics, then sidecar expansion.
- Current `src/repository.rs:69-78` calls `parse_and_validate_cancellable` before
  effective-project materialization and natural validation.
- Current `src/parse/mod.rs:179-208` validates schema and expands/revalidates the
  Skill sidecar inside that call. Therefore sidecar expansion occurs before
  effective-project and natural validation.
- ADR-014 and task V-SAVE-001 require current lint diagnostic behavior to remain
  a regression constraint.

Expected: FR-SAVE-018/026, QR-SAVE-002, and VR-SAVE-011 require the same complete
offline closure as lint, including observable failure precedence and
cancellation behavior.

Actual: the normative target order would change which error occurs first when
sidecar and project/natural failures coexist.

Impact: implementation following the contract can regress lint/apply behavior;
implementation preserving current behavior would violate the contract.

Required action: revise the contract/ADR to the verified current order or
explicitly approve and specify a product-visible diagnostic-order migration.

Owner: solution-architect; product-spec-owner if observable order is changed

Verification: run all existing lint fixtures plus combined-failure and
cancellation-order fixtures through DiskRepositoryView and overlay views.

### Finding ID: VER-SAVE-007

Severity: **HIGH**

Status: **SUPERSEDED**

Title: Publication fail-stop has no defined command terminal or output behavior

Evidence:

- `contracts/publication-v1.md:100-112` says unverifiable rollback identity
  enters a “fail-stop internal state” and must not claim clean output failure.
- `data-model.md:278-306` defines only `Committed` success or `FailedClean` with
  one diagnostic; no fail-stop state or transition exists.
- FR-SAVE-027/028 and publication lines 127-128 require every reported failure
  to be clean and to emit exactly one diagnostic. The crash exception applies
  only when no command result is reported.

Expected: all ordinary failure, cancellation, and race paths have a bounded,
safe terminal compatible with the 300-second invocation and output contracts.

Actual: it is undefined whether fail-stop aborts, hangs, crashes, exceeds the
deadline, or emits a different diagnostic; tasks do not define evidence for
that terminal.

Impact: race/fault handling can violate all-or-none reporting, deadline, and
exact-output guarantees or leave operators without a defined recovery signal.

Required action: define the terminal state, process/result semantics, deadline
interaction, and supported-filesystem prequalification that makes it reachable
only under an explicitly classified crash/unsupported condition. Align the data
model, publication contract, CLI diagnostics, tasks, and failure injection.

Owner: solution-architect with security-reviewer

Verification: independent-process replacement/race, rollback-identity failure,
cancellation, and deadline injection must prove the chosen terminal and that no
foreign entry is removed.

### Finding ID: VER-SAVE-008

Severity: **MEDIUM**

Status: **SUPERSEDED**

Title: Datasource test count and pre-security traceability are ambiguous

Evidence:

- Task D-SAVE-001 says “all eight supported branches positive” while its scope
  describes every non-File branch. The closed declaration union has nine
  exportable branches: Git, SVN, Confluence, Jira, Xray, Azure Wiki, Azure Work
  Item, SharePoint, and Google. The separate 3x2 code matrix does not resolve
  which count the phrase intends.
- `traceability.md` individual evidence rows generally reference
  S-SAVE-SEC-002, while task-plan family rows and the lifecycle gate require
  S-SAVE-SEC-001 before implementation.

Expected: task evidence enumerates every supported branch and traceability
makes the pre-implementation security gate visible at the affected requirement
rows.

Actual: the count can permit one missing branch, and the detailed matrix
understates the pre-security dependency.

Impact: a work package can be considered complete with incomplete union tests,
and readers can miss the required security gate.

Required action: enumerate all nine branch names and their discriminator
fixtures in D-SAVE-001; consistently attach S-SAVE-SEC-001 to security-sensitive
rows or document why the task-level gate is the sole authoritative linkage.

Owner: solution-architect

Verification: task/traceability lint proves every declared exportable branch has
a named positive fixture and both security review stages are linked.

## Confirmed convergent decisions

- **Datasource discriminator correction:** manifest lines 568-579 correctly use
  persisted `vcs_type` (`git|svn`) for declaration `spec.index_type` and
  persisted code-strategy `index_type` (`code|summary|chunk-summary`) for
  declaration `spec.indexType`. Neither participates in natural identity.
- **Skill snapshot/content:** resolution is complete-visibility and exhaustive;
  detail A, payload A, detail B, payload B, detail C are sequential; payload
  equality covers metadata and decoded bytes; main content is exact UTF-8 in a
  deterministic adjacent sidecar and companions remain inline/sorted.
- **Canonical YAML:** the contract fixes field ordering, scalar styles, LF/final
  newline, JSON-number constraints, and exact sidecar bytes with determinism
  evidence tasks.
- **GET-only behavior:** adapter type separation plus method-journal tests cover
  POST/PUT/PATCH/DELETE prohibition. No architecture task targets reference-only
  backend code.
- **Output compatibility:** outcome-v2 and diagnostic-v2 are closed and accept
  all tested v1 instances while rejecting wrong save-specific action/field and
  code/category/exit combinations.
- **Task structure:** implementation is ordered behind Q-SAVE-001/Q-SAVE-002 and
  S-SAVE-SEC-001, then adapters/projector/render/validation/publication/coordinator,
  post-security, independent verification, documentation, and release review.

## Security, migration, and operations review

- Security is represented by pre/post security tasks, secret classifications,
  safe diagnostics/log fields, bounded reads, GET-only adapters, no raw-body
  retention, prospective in-memory validation, and native no-replace publication.
  VER-SAVE-004 and VER-SAVE-007 prevent a positive security gate today.
- No server schema/data migration is required. Local config/state is unchanged;
  output/diagnostic schemas are additive v2 replacements with verified v1
  instance compatibility.
- Observability is bounded to safe enum/count/duration fields. Platform
  qualification and operator documentation are assigned to O-SAVE-001 and
  L-SAVE-001. Release support matrices remain correctly deferred, but the
  publication terminal must first be designed.

## Blocking decisions

1. Product owner: resolve Autonomous Workflow support versus the pinned API.
2. Product owner: resolve whether text mode must expose adoption-required state.
3. Solution architect: correct Workflow source paths and rebuild the exact read
   manifest/field inventories.
4. Solution architect: close exportability predicates and validation order.
5. Solution architect/security reviewer: define the publication fail-stop
   terminal.
6. Authorized review process: accept ADR-013..017 only after corrections and
   repeat reviews; all five currently say `Proposed`.

## Recommended next action

Return VER-SAVE-001 and VER-SAVE-005 to the product specification owner for
explicit decisions. Return all other open findings to the solution architect,
then repeat Q-SAVE-001/Q-SAVE-002, pre-implementation verification, and the
pre-implementation security review. Do not hand the current task list to the
implementation engineer.
