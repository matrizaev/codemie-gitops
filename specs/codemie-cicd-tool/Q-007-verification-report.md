# Artifact analysis report: codemie-gitops — Q-007 v28 compatibility correction

## Status

```text
Artifact analysis status: READY WITH NON-BLOCKING FINDINGS
```

Lifecycle status: **READY FOR IMPLEMENTATION**.
`Q-007-security-review.md` now records `APPROVED FOR NEXT STAGE`, with
SEC-Q007-001 through SEC-Q007-004 resolved and no unresolved MEDIUM-or-higher
security finding.

Q-007 focused re-verification result: **PASS**. Product specification v28,
ADR-004, the source baseline, adapter manifest, HTTP/CLI contracts, data model,
plan, and reopened tasks converge on one target-compatibility rule: the pinned
source-derived consumed contract is authoritative; semantic
`GET /v1/info.version` is observability only and cannot accept or reject
`apply`.

T-003 and R-001 may enter implementation. Their completion must cover the full
operation-applicable strict-decoding correction described below, not only
removal of the coordinator's `/v1/info` call.

## Scope

- Feature: v28 `E_API_INCOMPATIBLE` compatibility correction
- Specification: `specs/codemie-cicd-tool.md` v28
- Scenarios and requirements: SC-021, IR-011, IR-012
- Acceptance criteria: AC-IR-011-01, AC-IR-012-01
- Architecture: `plan.md`, `data-model.md` section 7, ADR-004
- Contracts: `source-baseline.md`, `http-adapter.md`,
  `adapter-manifest-v2.42.0.json`, `cli.md`
- Tasks: Q-007, reopened T-003 and R-001, downstream V-000/V-001
- Current implementation inspected: `src/preflight/mod.rs`,
  `src/coordinator/mod.rs`, `src/http/mod.rs`, and all four entity adapters
- Provided Jira material: none
- Provided Confluence material: none
- Exclusions: implementation changes, deployment qualification, release
  readiness, and reference-only source modifications

The unrelated working-tree changes in `Makefile`, `ops/dev/`, and
`scripts/wait-for-dev-dependencies.sh` were not assessed and were not modified.
The reference-only `codemie/` and `codemie-ui/` trees were inspected only as
already-pinned evidence and were not modified.

## Executive assessment

The compatibility identity conflict is resolved without weakening the
fail-before-write boundary:

1. Backend tag `2.42.0`, commit
   `2a481c290c99bf30ef80aadafa03d876a7f5f732`, remains the immutable source
   baseline.
2. The exact source reports semantic `APP_VERSION=0.16.0`; every normative
   architecture artifact now states that this value is not a Git/source/API
   identity.
3. `/v1/info` is not a required `apply` request and its value, shape,
   availability, or status cannot determine compatibility.
4. Compatibility is established by strictly decoding the selected operation's
   required non-mutating capability, identity/reference/detail, preservation,
   permission, and pagination responses.
5. Missing or invalid required consumed evidence remains exit-2
   `E_API_INCOMPATIBLE`, with empty stdout, safe stderr, and no POST, PUT,
   DELETE, or other modifying request.
6. Additional unconsumed response fields remain tolerated and cannot widen the
   authoring or outbound request contracts.

No unresolved product or architecture decision blocks implementation. The
current Rust implementation is intentionally not converged on v28; the gaps
are reproducible and assigned to reopened T-003/R-001.

## Evidence consulted

### Repository artifacts

- `specs/codemie-cicd-tool.md`
- `specs/codemie-cicd-tool/plan.md`
- `specs/codemie-cicd-tool/data-model.md`
- `specs/codemie-cicd-tool/tasks.md`
- `specs/codemie-cicd-tool/adr/004-openapi-subset-compatibility-gate.md`
- `specs/codemie-cicd-tool/contracts/source-baseline.md`
- `specs/codemie-cicd-tool/contracts/http-adapter.md`
- `specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json`
- `specs/codemie-cicd-tool/contracts/cli.md`
- `specs/codemie-cicd-tool/Q-001-verification-report.md`
- `specs/codemie-cicd-tool/Q-002-verification-report.md`
- `specs/codemie-cicd-tool/Q-006-verification-report.md`
- `specs/codemie-cicd-tool/Q-007-security-review.md`
- `src/preflight/mod.rs`
- `src/coordinator/mod.rs`
- `src/http/mod.rs`
- `src/adapters/assistant.rs`
- `src/adapters/workflow.rs`
- `src/adapters/skill.rs`
- `src/adapters/datasource.rs`

### Validation performed

```text
cargo test --locked preflight -- --nocapture
  PASS — 14 passed; 0 failed; 296 filtered out

cargo test --locked coordinator::tests -- --nocapture
  PASS — 9 passed; 0 failed; 301 filtered out

git diff --check
  PASS

jq -e '<exact applicability/predicate/additive-policy assertions>' \
  contracts/adapter-manifest-v2.42.0.json
  PASS — applicability is exactly Workflow/Datasource/Skill; exact-project and
  additive-unconsumed policies are present

rg -n '<Assistant admin-preflight conflict patterns>' <remediated artifacts>
  PASS — matches only explicit Assistant exclusion/no-call statements; no
  remaining Assistant admin prerequisite found

rg -n '<sealed state and dispatcher patterns>' data-model.md http-adapter.md tasks.md
  PASS — operation preflight -> completed reads -> projection -> sealed write
  ordering and dispatcher-only acceptance are present

Q-007-security-review.md status/finding check
  PASS — APPROVED FOR NEXT STAGE; SEC-Q007-001 through SEC-Q007-004 RESOLVED
```

The focused Rust tests are evidence of the current gap, not v28 conformance:
they still encode a successful SHA-valued `/v1/info.version`, reject a semantic
version mismatch, accept missing `/v1/user` role fields by defaulting them, and
require the coordinator's unconditional compatibility call.

## Traceability coverage

| Requirement / criterion | Architecture and contract evidence | Implementation task and required test evidence | Status |
|---|---|---|---|
| SC-021 | ADR-004 selects the source-derived manifest and rejects `/v1/info` as identity; source baseline records `APP_VERSION=0.16.0` | T-003/R-001 exact-pinned-source success fixture or asserted zero `/v1/info` contacts | COVERED |
| IR-011 | Manifest `infoEndpointIsIdentity: false`; plan sections 5, 6, 10, 13; data-model `PrewriteEvidenceEstablished` | Remove runtime Git-SHA comparison and unconditional coordinator call | COVERED |
| AC-IR-011-01 | Spec requires ordinary flow when `0.16.0` is observed and all applicable evidence passes | Coordinator observes exactly one selected POST/PUT only after all reads pass; `/v1/info` is absent or ignored | TESTABLE |
| IR-012 | Manifest enumerates capability, entity, pagination, request, and response fields; HTTP contract requires strict consumed-field decoding | T-003 repairs strict DTOs; R-001 invalidates required fields one at a time and asserts zero modifying requests | COVERED |
| AC-IR-012-01 failure case | Data-model failure transition and CLI exit/stream contract | `E_API_INCOMPATIBLE`, exit 2, empty stdout, safe stderr, zero POST/PUT/DELETE for each required-field/shape/pagination fault | TESTABLE |
| AC-IR-012-01 additive case | Manifest ignores unknown consumed-response additions and rejects unknown request/declaration fields | Additive response fixtures pass without changing accepted declarations or captured outbound request fields | TESTABLE |

## Focused remediation re-verification

| Security finding | Remediated evidence | Assessment |
|---|---|---|
| SEC-Q007-001 — exact-project capability proof | Manifest applicability is exactly Workflow/Skill/Datasource; its predicate joins `projects[].name == effectiveProject` and `is_project_admin == true` on the same entry. HTTP contract, source baseline, ADR-012, plan, data model, T-003, D-001, and R-001 agree. Missing/invalid fields are incompatible; a valid false predicate is visibility-unproven. | RESOLVED IN ARCHITECTURE |
| SEC-Q007-002 — premature write-capable state | State order is operation preflight, completed non-mutating resolution, projection, sealed prepared write, then write. HTTP accepts only the sealed value; R-001 requires dispatcher-boundary and injected-read-failure tests. | RESOLVED IN ARCHITECTURE |
| SEC-Q007-003 — additive-field ambiguity | Manifest policy now says `ignore-only-when-additive-and-unconsumed`; HTTP/data-model/plan/tasks require paired additive and missing/wrong-type fixtures. | RESOLVED IN ARCHITECTURE |
| SEC-Q007-004 — unapproved Assistant admin prerequisite | Assistant is absent from `capabilityPreflight.appliesToEntityKinds`, does not call `/v1/user`, and seals its prepared write from strict direct `(effective_project, slug)` resolution plus applicable write evidence. Workflow/Skill/Datasource retain exact-project visibility. | RESOLVED; SECURITY APPROVED FOR NEXT STAGE |

### Sealed `PreparedWrite` feasibility

The control is feasible as a Rust type-state boundary without production
behavior invention. A private/non-forgeable `PreparedWrite` aggregate can own:

- the entity kind and validated effective-project domain values;
- the kind-specific `OperationPreflight` result;
- completed operation-specific resolution/reference/detail/pagination evidence;
  and
- the already projected `CreateRequest` or `UpdateRequest`.

Only the coordinator-owned constructor may assemble that aggregate after all
fallible reads and projection return successfully, and only the modifying
dispatcher accepts it. Adapters therefore cannot call POST/PUT from a partial or
error state. Create, update, Workflow adoption, and the pre-write part of Skill
create all fit this sequence. Skill 409 re-resolution and ordinary identity
verification remain explicitly post-write because the first POST may already
have committed.

The data-model pseudocode calls the closed aggregate `PrewriteEvidence` while
the prose/tasks call it `PreparedWrite`; this is a naming inversion, not an
unresolved control choice. R-001 unambiguously defines the required aggregate
and boundary. See Q7-VER-003.

## Consistency analysis

### Compatibility identity

The following are mutually consistent:

- ADR-004 rejects `/v1/info` as a reliable source or contract identity.
- `source-baseline.md` records backend tag/commit and separately marks package
  and application versions informational.
- The manifest pins the backend commit and sets
  `compatibilityPolicy.infoEndpointIsIdentity` to `false`.
- `http-adapter.md` treats `/v1/info` as observability and requires strict
  decoding of manifest-consumed fields.
- `data-model.md` replaces the generic `CompatibilityChecked` state with the
  operation-specific `PrewriteEvidenceEstablished` state.
- `plan.md` and `tasks.md` remove any required `/v1/info` call and retain the
  source-derived non-mutating read boundary.
- `cli.md` already assigns compatibility failures to exit 2 and requires empty
  stdout plus a safe stderr diagnostic.

No architecture artifact was found that still treats
`/v1/info.version` as Git/source/API identity or a required acceptance gate.

### Operation-applicable evidence

The manifest is the field-level source of truth and makes the correction
bounded and testable. It enumerates:

- `GET /v1/user` consumed fields and its Workflow/Skill/Datasource-only
  exact-effective-project predicate;
- Assistant strict direct `(project, slug)` identity/write evidence without an
  admin preflight;
- Workflow and Skill exhaustive entity and pagination fields;
- Datasource exhaustive entity and pagination fields;
- operation-specific routes and request/response boundaries.

The plan and data model require those fields and shapes to be decoded before
the selected write. T-003 requires missing/invalid-field and additive-field
tests; R-001 adds coordinator-level request counts and output/diagnostic
assertions. This is sufficient evidence design for the v28 correction.

The operation-specific ordering is also safe: local validation and
authentication precede non-mutating evidence; kind-applicable visibility and
identity/detail reads precede projection and the sealed `PreparedWrite`;
POST/PUT accepts only that sealed value. Post-write re-resolution remains
post-write evidence and is not incorrectly represented as a pre-write check.

## Current implementation gap

The following observations are expected pre-implementation failures, not proof
against the approved architecture:

- `src/preflight/mod.rs` decodes `/v1/info.version`, compares it to
  `EXPECTED_BACKEND_COMMIT`, and rejects `0.16.0`.
- `src/coordinator/mod.rs` unconditionally calls `check_compatibility` before
  dispatch and currently calls `/v1/user` for Assistant even though the
  remediated architecture excludes Assistant from the admin prerequisite.
- `src/http/mod.rs` places `#[serde(default)]` on required `GET /v1/user`
  fields and does not decode `projects[].name`. Missing required evidence can
  therefore become `E_VISIBILITY_UNPROVEN` instead of
  `E_API_INCOMPATIBLE`, and project-admin evidence is not tied to the exact
  effective project for Workflow, Skill, or Datasource.
- Workflow currently decodes only `pagination.pages` and only `id` plus
  `meta_config` from rows, rather than every manifest-consumed pagination and
  entity field.
- Skill currently decodes only `skills` plus `pages` and omits manifest-required
  page metadata, `created_by`, and `user_abilities`.
- Datasource currently decodes only `pagination.pages` and omits other
  manifest-required page metadata and row `user_abilities`.
- Assistant decodes only `id`; its manifest-required write evidence is not
  established in the adapter.

Accordingly, an implementation that only removes `check_compatibility` is
insufficient. T-003's strict-decoding scope and R-001's field-by-field,
zero-write acceptance tests are mandatory completion evidence.

## Required decoder and test inventory for T-003/R-001

This table is the concrete implementation scope derived from the checked-in
manifest. Field spelling follows the wire contract exactly.

The obsolete runtime surface is also exact: `src/preflight/mod.rs`
`InfoResponse`, `CompatibilityResult`, and `check_compatibility` must no longer
participate in `apply`; `EXPECTED_BACKEND_COMMIT` may remain only as checked-in
manifest/build provenance. `src/coordinator/mod.rs` must remove the import and
unconditional `check_compatibility` call and rename the stale
`CompatibilityChecked` state/comment to the operation-specific
`PrewriteEvidenceEstablished` concept. No replacement target-version probe is
authorized.

| File and current Rust type | Required fields for the applicable read | Current gap |
|---|---|---|
| `src/http/mod.rs` — `UserResponse` (Workflow/Skill/Datasource only) | `is_admin`, `is_maintainer`, `projects` | All three currently use `#[serde(default)]`; omission does not fail compatibility. This response must not be required for Assistant. |
| `src/http/mod.rs` — `UserProject` (Workflow/Skill/Datasource only) | `name`, `is_project_admin` | `name` is not decoded and `is_project_admin` defaults when absent. Project-admin proof is therefore not tied to the exact effective project. |
| `src/adapters/assistant.rs` — `AssistantIdResponse` or a new resolve/detail DTO | Strict direct `(effective_project, slug)` response evidence: `id` plus the manifest's applicable existing-target `user_abilities`; no `/v1/user` admin proof | Only `id` is decoded, while the coordinator unnecessarily requires `/v1/user`. An existing Assistant can reach PUT without per-row write evidence. Create/write result responses need only the fields consumed on that path; server-owned fields that are not used must not be made artificial requirements. |
| `src/adapters/workflow.rs` — `WorkflowPage` | `data`, `pagination` | Container fields are required today, but field-level omission/type tests are absent. |
| `src/adapters/workflow.rs` — `WorkflowPagination` | `page`, `pages`, `total`, `per_page` | Only `pages` is decoded. |
| `src/adapters/workflow.rs` — `WorkflowItem` | `id`, `project`, `name`, `meta_config`, `user_abilities` | Only `id` and `meta_config` are decoded. `meta_config` is nullable but required as a response member; missing and explicit JSON `null` must not be conflated. Existing-target `user_abilities` must prove `write`. |
| `src/adapters/skill.rs` — `SkillPage` | `skills`, `page`, `perPage`, `total`, `pages` | Only `skills` and `pages` are decoded. Note the pinned camel-case `perPage`. |
| `src/adapters/skill.rs` — `SkillItem` | `id`, `name`, `project`, `created_by`, `user_abilities` | `created_by` and `user_abilities` are not decoded. Existing-target `user_abilities` must prove `write`. |
| `src/adapters/datasource.rs` — `DatasourcePage` | `data`, `pagination` | Container fields are required today, but field-level omission/type tests are absent. |
| `src/adapters/datasource.rs` — `DatasourcePagination` | `page`, `per_page`, `total`, `pages` | Only `pages` is decoded. Datasource pagination is zero-indexed. |
| `src/adapters/datasource.rs` — `DatasourceItem` | `id`, `repo_name`, `project_name`, `index_type`, `user_abilities` | `user_abilities` is not decoded. An existing Datasource must not reach PUT without applicable write proof. |

The same strict DTOs must be used for ordinary resolution, Workflow reference
resolution, explicit Workflow adoption/detail, Skill and Datasource reference
resolution, create-409 re-resolution, and pre-write update/detail reads wherever
those paths consume the listed contract fields. A parallel permissive DTO on a
secondary path would leave IR-012 unsatisfied.

`user_abilities` at the pinned source is an action list and `write` is the
relevant action. Classification must distinguish contract evidence from
authorization evidence:

- missing field, wrong JSON type, or invalid element type is
  `E_API_INCOMPATIBLE`, exit 2;
- a structurally valid ability list that does not contain `write` is an
  authorization/write-proof failure, exit 2;
- neither case may issue a modifying request.

Required focused tests:

1. Delete each required field above, one fixture at a time, and assert
   `E_API_INCOMPATIBLE`, exit 2, empty stdout, one safe stderr diagnostic, and
   zero modifying requests.
2. Replace each required field with an incompatible JSON type, one at a time,
   with the same assertions. Include invalid nested `projects[]` entries and
   invalid ability-list elements.
3. For nullable-but-required Workflow `meta_config`, prove explicit `null`
   follows the permitted nullable path while an omitted member fails
   compatibility.
4. For Workflow, Skill, and Datasource `/v1/user`, prove global
   admin/maintainer succeeds only with a fully decodable response; prove
   project-admin succeeds only for a
   `projects[].name` exactly equal to the effective project; present-but-false
   or wrong-project roles produce the authorization/visibility failure rather
   than compatibility failure. Separately prove Assistant performs no
   `/v1/user` request and reaches the seal only through strict direct lookup.
5. For each existing entity kind, prove a valid `user_abilities` list
   containing `write` permits the update flow and a valid list without `write`
   stops before PUT with the authorization classification.
6. Exercise pagination invariants, not just deserialization: requested/returned
   page agreement, pinned page size, stable `pages`/`total`, exhaustive terminal
   traversal, repeated-page/repeated-ID detection, and page/item caps. Every
   compatibility drift detected before selection must have zero modifying
   requests.
7. Add unrelated response members at the top level, pagination level, entity
   level, and `/v1/user` project-entry level; prove they are ignored and do not
   change the captured outbound request.
8. Replace the old SHA-valued preflight/coordinator success fixtures. The
   positive coordinator test must assert zero `/v1/info` requests (a configured
   `0.16.0` endpoint may exist but must remain uncalled) and exactly one selected
   POST or PUT after all applicable evidence passes.
9. Configure `/v1/info` as matching, non-matching, missing, malformed, 404,
   500, and unreachable in separate tests—or prove it is never contacted—and
   verify none of those states overrides either a valid operation or a failure
   in required operation evidence.
10. Count all modifying methods/routes exposed by the fake server, not only the
    expected entity route. Every pre-write failure must observe zero POST, PUT,
    PATCH, and DELETE requests.

The test matrix should be generated or table-driven where practical so adding
a required manifest field cannot silently omit its negative case. Mutation
reasoning is mandatory: removing a DTO field, restoring `#[serde(default)]`,
skipping a pagination check, or bypassing the `write`-ability check must make at
least one test fail.

## Security, migration, and operations review

- Security: no new credential or authorization surface is introduced. Removing
  the unreliable semantic-version request is safe only with the documented
  strict capability/permission/identity evidence. Safe diagnostic and no-body
  output boundaries remain unchanged.
- Migration: not applicable; no persisted client or server schema changes.
- Compatibility: additive target response fields remain compatible; missing or
  invalid consumed fields remain fail-closed.
- Operations: deployment qualification remains assigned to V-000 and release
  review. `/v1/info` may still be observed operationally but cannot be used as
  deployment/source identity.
- Rollback: the v28 correction does not authorize deployment. O-001 and its
  downstream chain remain paused until implementation and independent
  convergence verification pass.

## Findings

### Finding ID: Q7-VER-001

Severity: LOW
Status: OPEN

Title:
Architecture text says strict adapter evidence is preserved although current
decoders do not establish the full manifest subset

Evidence:
- `plan.md` section 5 says operation-applicable evidence already comes from
  strict adapter reads.
- T-003 says to "preserve" strict decoding.
- Current Workflow, Skill, Datasource, Assistant, and `/v1/user` DTOs omit or
  default manifest-required consumed fields, as listed in this report.
- R-001 nevertheless requires one-field-at-a-time incompatibility tests and
  exact zero-modifying-request counts.

Expected:
The implementation handoff should make clear that full manifest-subset strict
decoding must be restored or added, not merely retained while `/v1/info` is
removed.

Actual:
The normative task acceptance evidence is sufficient, but the summary wording
understates the amount of existing decoder work.

Impact:
An implementer reading only the summary could remove the obsolete gate while
leaving fail-open response DTOs. Reading the complete T-003/R-001 scope prevents
that mistake, so this does not block implementation.

Required action:
The implementation engineer must treat the manifest field inventory and
T-003/R-001 field-by-field tests as required scope. The solution architect
should replace "preserve" with "restore and preserve" in the next artifact
maintenance pass.

Owner:
implementation-engineer (code/tests); solution-architect (wording)

Verification:
Mutation-oriented tests must fail when each required consumed field is removed,
has the wrong type, or has invalid pagination behavior; every pre-write failure
must observe zero modifying requests.

### Finding ID: Q7-VER-002

Severity: LOW
Status: OPEN

Title:
Some normative compatibility references retain stale source-version metadata

Evidence:
- ADR-004 references product specification v24 and IR-002/003/005/008-010.
- `source-baseline.md` still attributes explicit auth endpoint selection to
  product specification v24.
- `http-adapter.md` has been refreshed to product specification v28 and
  IR-001-012.
- The substantive bodies of these artifacts already match v28 and are cited by
  Q-007, IR-011, and IR-012.

Expected:
Normative artifact headers and references identify v28 and IR-011/012 so future
traceability checks do not mistake aligned content for an older contract.

Actual:
The compatibility rules are converged and the HTTP contract metadata is fixed,
but ADR-004 and one source-baseline sentence remain stale.

Impact:
Traceability/documentation weakness only; no behavior or implementation choice
is ambiguous.

Required action:
Refresh ADR-004 and `source-baseline.md` source/version/reference metadata in
the next architecture maintenance pass without changing the approved decision.

Owner:
solution-architect

Verification:
Headers and references cite product specification v28 and IR-011/012; the
substantive compatibility decision remains unchanged.

### Finding ID: Q7-VER-003

Severity: LOW
Status: OPEN

Title:
The sealed-write aggregate has inverted names in data-model pseudocode

Evidence:
- `data-model.md` defines `PrewriteEvidence = closed { ..., prepared_write:
  CreateRequest | UpdateRequest }`.
- The following sentence says the HTTP boundary accepts only `PreparedWrite`
  carrying that `PrewriteEvidence`.
- `http-adapter.md` and R-001 consistently require one sealed prepared-write
  aggregate containing evidence plus the projected request.

Expected:
One non-recursive name and ownership relationship describes the aggregate
accepted by the modifying dispatcher.

Actual:
The control invariant and construction order are unambiguous, but a literal
reading makes each name appear to contain the other.

Impact:
Minor implementation-handoff friction. A private `PreparedWrite { evidence,
request }` or a single aggregate with those fields satisfies the same approved
boundary; no product or architecture choice changes.

Required action:
Normalize the pseudocode names in the next architecture maintenance pass. Do
not weaken the private construction or dispatcher-only acceptance rule.

Owner:
solution-architect

Verification:
The data model defines one non-recursive sealed aggregate, and R-001's negative
state-transition tests remain unchanged.

## Known prior non-blocking finding

Q-002 VER-003 remains open: the manifest names `user_abilities` as consumed
write evidence but does not fully annotate its nested shape/value semantics for
Workflow and Skill. `http-adapter.md` requires the value to contain `write`, and
the manifest pins the exact source locations, so T-003/R-001 can be implemented
without a new product decision. The solution architect should still add the
field/value annotation before post-implementation convergence verification.

## Blocking decisions

None. No product, architecture, verification, or pre-implementation security
decision remains open for T-003/R-001.

## Files changed during verification

- `specs/codemie-cicd-tool/Q-007-verification-report.md` — added this report

No production code, architecture source, contract, reference-only file, or
unrelated working-tree file was modified.

## Recommended next action

Proceed to the implementation engineer for reopened T-003 and R-001 only.
Independent post-change convergence verification must confirm:

1. no runtime target comparison with the pinned Git SHA;
2. no required `/v1/info` request;
3. exact pinned-source success with all applicable reads valid;
4. strict field/shape/pagination decoding for every manifest-consumed read;
5. correct authorization versus compatibility classification;
6. empty stdout, safe stderr, exit 2, and zero modifying requests for every
   pre-write incompatibility; and
7. additive response fields do not alter accepted declarations or captured
   outbound requests;
8. Workflow/Skill/Datasource require exact-effective-project visibility, while
   Assistant performs no `/v1/user` call and seals from strict direct lookup;
   and
9. the modifying dispatcher is unreachable without the sealed `PreparedWrite`.
