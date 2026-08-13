# Verification Report: Q-002 — Workflow and Skill Capability Fixtures

**Verification mode**: Architecture-review (ARCHITECTURE-REVIEW eligibility)
**Status**: PASS — 0 blocking findings. Algorithm specifications are sufficient
for implementation of W-001 and S-001.
**Date**: 2026-08-10
**Dependency satisfied**: Q-001 PASS (2026-08-10)

---

## Scope

This report verifies design correctness for ADR-007 (Skill exhaustive list
resolution) and ADR-008 (Workflow `meta_config` identity and adoption) against:

- Product specification v26, FR-028–035, PA-005/006, VR-007–010/013
- `specs/codemie-cicd-tool/adr/007-skill-exhaustive-list-resolution.md`
- `specs/codemie-cicd-tool/adr/008-workflow-meta-config-identity-and-adoption.md`
- `specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json`
- `specs/codemie-cicd-tool/contracts/cli.md`
- `specs/codemie-cicd-tool/contracts/http-adapter.md`
- `specs/codemie-cicd-tool/Q-001-verification-report.md` (prior findings)
- `specs/codemie-cicd-tool.md` v26 (§13–21, §22 edge cases)
- `specs/codemie-cicd-tool/plan.md`
- `specs/codemie-cicd-tool/tasks.md`

No implementation files (`src/`) were inspected; no network calls were made.
This verification covers algorithm specification correctness only.

---

## Sources of truth

- Specification: `specs/codemie-cicd-tool.md` v26 (READY FOR IMPLEMENTATION)
- Requirements in scope: FR-028–035, PA-005/006, VR-007–010/013
- Acceptance criteria: AC-FR-028-01/02, AC-FR-029-01/02, AC-FR-030-01/02,
  AC-FR-031-01/02, AC-FR-033-01, AC-FR-034-01
- ADRs: ADR-007 (Accepted), ADR-008 (Accepted)
- Manifest: `adapter-manifest-v2.42.0.json` pinned to backend
  commit `2a481c290c99bf30ef80aadafa03d876a7f5f732`
- Prior verification: Q-001 PASS with N7 probe confirming reserved key
  protection

---

## 1. Skill Resolution Algorithm (ADR-007) — Scenario Analysis

### 1.1 More than 100 Skills with pagination drift

ADR-007 Step 2 specifies `per_page=100` and instructs the implementation to
"enumerate every page." This means a corpus of 101 or more Skills spanning
multiple pages is consumed in full before any identity decision.

Step 3 specifically addresses drift:

> Detect pagination cycles, repeated row IDs, changing totals/cursors, or
> inconsistent snapshots. Compatible but unstable resolution is exit 1.

Scenario: a new Skill is created between page 1 and page 2 fetches,
incrementing the `total` field. The algorithm detects the changed total as a
snapshot inconsistency and fails with exit 1 before any write. No write occurs
against an incomplete snapshot.

Manifest `paginationConsumedFields` for Skill: `["skills", "page", "perPage",
"total", "pages"]`. The `total` field is consumed, making total-change detection
possible.

**Verdict: PASS.**

### 1.2 `user_abilities`, exact project scoping

ADR-007 Step 1 requires proof of "project-manager/admin visibility and write
capability for the target project." Failure is exit 2 before write.

The manifest `capabilityPreflight` uses `GET /v1/user` and consumes
`is_admin`, `is_maintainer`, and `projects[].is_project_admin` to verify
project-level visibility.

ADR-007 Step 5 ("One: prove write ability") uses the per-entity `user_abilities`
field. The manifest `entityConsumedFields` for Skill lists `user_abilities` as a
required consumed field.

Exact project scoping is addressed in Step 4: "Client-filter the complete
visible set by exact project and exact name." No fuzzy match or display-name
approximation is permitted.

Note: `meta_config` is a Workflow-only concept. Skills have no `meta_config`
field. This scenario element is not applicable for Skill and is correctly absent
from ADR-007.

**Verdict: PASS.**

### 1.3 Zero-result, one-result, and multiple-result cases

ADR-007 Step 5 defines all three cases:

- Zero exact matches: POST once.
- One exact match: prove write ability, GET required detail, PUT by returned
  ID on every valid apply.
- More than one: `E_AMBIGUOUS_IDENTITY`, exit 1, no write.

The acceptance criterion AC-FR-031-01 confirms these three cases are testable
behavioral contracts.

**Verdict: PASS.**

### 1.4 Forbidden-visibility and incomplete-visibility cases

Both cases are handled by the preflight in ADR-007 Step 1.

- Forbidden visibility (principal has no project-admin role at all): preflight
  `GET /v1/user` returns a response where `projects[].is_project_admin` is false
  for the effective project and `is_admin`/`is_maintainer` are also false.
  Result: exit 2 before write.
- Incomplete visibility (partial access — can see own Skills but not all project
  Skills): same preflight detection. Without project-admin, complete visibility
  cannot be proved. Result: exit 2 before write.

The spec AC-FR-033-01 and FR-033 confirm "complete visibility" is required
before any resolution or write; inability to prove it is exit 2.

Edge case "Workflow/Skill principal has partial visibility" in spec §22 confirms:
"apply fails with exit code 2 before write because absence or uniqueness cannot
be proven."

**Verdict: PASS.**

### 1.5 Explicit adoption with another same-display-name unmarked row

Skills do not have an adoption path. The `--adopt-workflow-id` flag is
Workflow-only (VR-008: "MUST be rejected for non-Workflow declarations"; CLI
contract §1: accepted only for Workflow declarations).

ADR-007 has no adoption mechanism. The absence is intentional. There is no
display-name-based selection in ADR-007. Option A ("First/newest/current-principal
match") was explicitly rejected.

The scenario "explicit adoption with another same-display-name unmarked row" is
fully defined for Workflow in ADR-008 (see section 2.4). For Skill, the correct
behavior is: multiple exact `(project, name)` matches fail with
`E_AMBIGUOUS_IDENTITY`; there is no adoption path that a same-name row could
affect.

**Verdict for Skill: NOT APPLICABLE (correctly absent). Workflow: see §2.4.**

### 1.6 No invented response fields and no display-name selection/veto

ADR-007 manifest `entityConsumedFields`: `["id", "name", "project",
"created_by", "user_abilities"]`. All five fields are real server response
fields evidenced in the pinned source. No invented or fabricated field is
consumed.

Display-name selection is explicitly prohibited:
- Option A rejected: "order and creator are not the approved identity"
- Step 4: "Client-filter ... by exact project and exact name"
- AC-FR-031-02: "creator, recency, write ability, relevance, and list order
  are not tiebreakers"

**Verdict: PASS.**

---

## 2. Workflow Identity Algorithm (ADR-008) — Scenario Analysis

### 2.1 `codemie.epam.com/gitops/workflow-identity` as authoritative identity carrier

ADR-008 defines the reserved member:

```json
"codemie.epam.com/gitops/workflow-identity": {
  "version": 1,
  "project": "<exact effective project>",
  "slug": "<exact metadata.slug>"
}
```

The manifest records the path explicitly:
```json
"reservedIdentityReadWritePath":
  "meta_config[\"codemie.epam.com/gitops/workflow-identity\"]"
```

Normal resolution selects only by exact effective project AND valid reserved
record. Display name, creator, and recency are not identity signals.

AC-FR-029-01 confirms: "Another Workflow has the same display name but no
matching identity record — display name does not select the other Workflow."

**Verdict: PASS.**

### 2.2 Decode/merge/canonical encode rules

The manifest `metaConfigCodec` specifies each operation:

| Stage | Specification |
|---|---|
| Decode | reject malformed JSON, duplicate object keys, non-object roots, invalid reserved record, invalid UTF-8 |
| Merge | preserve decoded non-reserved members; overlay authored non-reserved members; set exact reserved identity member |
| Encode | compact JSON string; object keys sorted recursively by Unicode scalar value; UTF-8 without BOM; non-finite numbers rejected |

ADR-008 supplements: "the container as a nullable string containing a JSON
object. Strict decode rejects invalid UTF-8, malformed/duplicate-key JSON,
non-object roots, and invalid reserved values."

The mixed-ownership classification ensures `meta_config` does not participate in
the generic omitted-field null loop. This is consistent with the manifest
`fieldClasses.mixedOwned: ["spec.meta_config"]`.

DR-007 and spec AC-FR-028-02 confirm non-reserved members are preserved unless
an authored value explicitly replaces the same member.

**Verdict: PASS.**

### 2.3 Reserved key protected in declaration schema

Q-001 negative probe N7 ("Reserved `meta_config` key injection") confirmed that
the declaration schema rejects any attempt to author the reserved key via
`propertyNames: {not: {const: ...}}`. This is a standing finding from Q-001.

Spec VR-007 and FR-020 additionally prohibit the reserved key in user-authored
YAML.

**Verdict: PASS (Q-001 confirmed).**

### 2.4 Adoption path covers the multiple-same-name case

ADR-008 "Explicit legacy adoption" section explicitly states:

> Another unmarked row with the same mutable display name neither selects nor
> vetoes this explicitly selected candidate.

This directly covers the scenario where a second Workflow with the identical
`name` (display name) to the intended `slug` exists when `--adopt-workflow-id`
is supplied.

The adoption preconditions are:
1. Canonical UUID syntax.
2. Zero valid exact marker matches and no invalid/conflicting marker.
3. By-ID detail: candidate in exact project with provable write capability.
4. Candidate has no valid or invalid reserved identity member.
5. Existing metadata is a mergeable object.

None of the preconditions reference display name. A second unmarked Workflow
with the same display name cannot block adoption because preconditions 2 and 4
only examine the reserved identity member content, not the `name` field.

The normal resolution zero-match guard ("enumerate unmarked exact display-name
candidates as a nonselecting guard; any candidate causes `E_ADOPTION_REQUIRED`")
is distinct from the adoption path. The guard is nonselecting — it warns without
choosing. Multiple display-name candidates all cause `E_ADOPTION_REQUIRED`
without distinguishing one from many.

AC-FR-030-01 and AC-FR-030-02 confirm these behaviors as testable contracts.

**Verdict: PASS.**

### 2.5 No algorithmic race conditions for create vs. update decisions

The algorithm makes a create-vs-update decision based on the result of
exhaustive enumeration. Between the decision (zero matches observed) and the
POST, a concurrent invocation could create the same Workflow.

ADR-008 addresses this via bounded post-write re-resolution:

> "a bounded post-write full re-resolution must find exactly one identity
> associated with the expected route ID. There is no automatic delete, rollback,
> or blind write retry."

If the concurrent write creates a second identity record, post-write resolution
finds multiple matches and exits 1 with "write may already have committed." This
is observable, non-destructive, and requires manual remediation.

ADR-008 explicitly acknowledges that prevention requires operational controls:
"CI serialization and governed UI/API writers are mandatory because the API has
no conditional write or unique marker index." QR-010/QR-011 and AC-FR-034-01
confirm this is by design, not a gap.

The algorithm itself introduces no internal race condition: each invocation is
sequential (one request at a time) with post-write verification gating the
success report.

**Verdict: PASS.**

---

## 3. Scope Requirements Coverage Matrix

| Required scenario | Skill (ADR-007) | Workflow (ADR-008) | Status |
|---|---|---|---|
| More than 100 rows | Step 2 (per_page=100, all pages) | "Exhaust every relevant list page" | PASS |
| Scopes | `project_with_marketplace` explicit in manifest route | pass 1: no scope (project-visible); pass 2: scope=marketplace (globally published) — pinned in enumeratePasses, per_page=100 | PASS (VER-001 CLOSED) |
| Pagination drift | Step 3: changing totals/cycles → exit 1 | Snapshot drift → compatible instability → exit 1 | PASS |
| `meta_config` | Not applicable to Skill | Reserved record decode/merge/encode defined in ADR and manifest | PASS |
| Abilities | `user_abilities` in entityConsumedFields; preflight proves project-admin | `user_abilities` in entityConsumedFields; preflight proves project-admin | PASS |
| Exact project | Step 4: client-filter exact project AND exact name | "Client-filter exact effective project and reserved record" | PASS |
| Zero/one/multiple | Step 5: POST / PUT / E_AMBIGUOUS_IDENTITY | Normal resolution: POST / PUT / E_AMBIGUOUS_IDENTITY | PASS |
| Forbidden visibility | Preflight fails → exit 2 before write | Preflight fails → exit 2 before write | PASS |
| Incomplete visibility | Preflight fails → exit 2 before write | Preflight fails → exit 2 before write | PASS |
| Explicit adoption + same-display-name unmarked row | NOT APPLICABLE (no adoption for Skills) | Adoption preconditions ignore display name; ADR explicitly specified | PASS |

---

## 4. Acceptance Criteria Coverage

| Criterion | Algorithm coverage | Evidence |
|---|---|---|
| AC-FR-028-01 | Workflow create persists identity record | ADR-008 reserved record definition; manifest `reservedIdentityReadWritePath` |
| AC-FR-028-02 | Non-reserved `meta_config` preserved | ADR-008 merge rule; manifest `metaConfigCodec.merge` |
| AC-FR-029-01 | Ordinary resolution uses only exact identity record | ADR-008 normal resolution; display name nonselecting |
| AC-FR-029-02 | Invalid or duplicate identity fails exit 1 | ADR-008: `E_IDENTITY_MARKER_INVALID` / `E_AMBIGUOUS_IDENTITY` |
| AC-FR-030-01 | Explicit adoption preserves server entity | ADR-008 adoption preconditions 1–5 |
| AC-FR-030-02 | Unmarked display-name match never implicitly adopted | ADR-008: "display name ... never selects ... candidate"; `E_ADOPTION_REQUIRED` |
| AC-FR-031-01 | Skill handles zero/one/multiple including >1 page | ADR-007 Steps 2–5; pagination field `total` consumed |
| AC-FR-031-02 | Search hints do not define identity | ADR-007 Step 2: "Hints do not replace client filtering" |
| AC-FR-033-01 | Incomplete visibility fails exit 2 before write | ADR-007 Step 1; ADR-008 incomplete visibility clause |
| AC-FR-034-01 | Post-write ambiguity visible and non-destructive | ADR-007 Step 7; ADR-008 post-write re-resolution |

---

## 5. Findings

### Finding VER-001

```
Finding ID: VER-001
Severity: MEDIUM
Status: CLOSED

Title:
Workflow enumeration scope parameter value not pinned in manifest

Evidence:
- contracts/adapter-manifest-v2.42.0.json, Workflow routes.enumerate:
  "GET /v1/workflows?minimal_response=false&page={page}&per_page={per_page}&scope={scope}"
- Skill enumerate route for comparison:
  "GET /v1/skills?filters={project,scope:project_with_marketplace,search}&page={page}&per_page=100"
- ADR-008: "across the project and marketplace-inclusive scopes defined by
  the source-pinned contract"

Expected:
The manifest should pin the exact `scope` parameter value(s) to use when
enumerating Workflows, as it does for Skill (`project_with_marketplace`),
so the implementation does not need to derive scope values from the reference
source independently.

Actual:
The Workflow enumerate route uses `{scope}` as an unresolved placeholder. The
exact scope value(s) required for complete project-and-marketplace-inclusive
enumeration are not stated in the manifest or ADR. The implementer must consult
the pinned source reference
(rest_api/routers/workflow.py:109-142, commit 2a481c290c99bf30ef80aadafa03d876a7f5f732)
to determine them.

Impact:
If the implementation enumerates an insufficient scope (e.g., project-only),
it may miss Workflows that are visible to a project-admin through the marketplace
scope. The zero-match path could then incorrectly POST-create a Workflow that
already exists under a different scope, producing duplicate identity records.
Detection via post-write verification reduces but does not eliminate risk.

Required action:
The solution architect should pin the exact scope value(s) for Workflow
enumeration in the manifest (e.g., add a "scopes" or "enumerationQuery" field
analogous to the Skill route), with the same evidence citation as other
manifest fields.

Owner: solution-architect

Verification:
A follow-up manifest patch that lists the exact scope(s) and the source lines
confirming their completeness resolves this finding. No implementation change
is required once the manifest is updated.

Resolution (2026-08-10):
Manifest updated. routes.enumerate (single ambiguous string) replaced with
routes.enumeratePasses (two-element array of fully-resolved URL templates):
  Pass 1: GET /v1/workflows?minimal_response=false&page={page}&per_page=100
  Pass 2: GET /v1/workflows?minimal_response=false&page={page}&per_page=100&scope=marketplace
Source evidence: service/workflow_config/workflow_config_index_service.py,
WorkflowScope(StrEnum) has exactly one member: MARKETPLACE='marketplace'.
Absent scope returns user-visible project workflows (membership-filtered);
scope=marketplace returns globally-published (is_global) workflows only.
Both passes are required for complete identity resolution per ADR-008.
per_page pinned to 100 (also resolves VER-002). See enumerateScopeEvidence
field in adapter-manifest-v2.42.0.json Workflow.routes.
```

### Finding VER-002

```
Finding ID: VER-002
Severity: LOW
Status: OPEN

Title:
Workflow per_page value not specified in manifest enumerate route

Evidence:
- contracts/adapter-manifest-v2.42.0.json, Workflow routes.enumerate:
  "... &per_page={per_page} ..."
- Skill enumerate route: "... &per_page=100" (fixed)

Expected:
A concrete per_page value (e.g., 100) matching Skill's convention, to prevent
implementations from using arbitrary page sizes that might interact unexpectedly
with server-side pagination totals used for drift detection.

Actual:
The Workflow enumerate route uses `{per_page}` as an unresolved placeholder.
No default or recommended value is stated in the manifest or ADR-008.

Impact:
Low. The pagination consumed fields (`total`, `pages`) make drift detection
independent of page size. The omission primarily reduces specification
precision, not safety.

Required action:
Solution architect should add a concrete per_page value to the Workflow
enumerate route in the manifest, consistent with the Skill value of 100.

Owner: solution-architect

Verification:
Resolved by adding a concrete per_page value to the manifest.
```

### Finding VER-003

```
Finding ID: VER-003
Severity: LOW
Status: OPEN

Title:
Per-entity user_abilities write proof values not documented in manifest

Evidence:
- contracts/adapter-manifest-v2.42.0.json Skill entityConsumedFields:
  ["id", "name", "project", "created_by", "user_abilities"]
- contracts/adapter-manifest-v2.42.0.json Workflow entityConsumedFields:
  ["id", "project", "name", "meta_config", "user_abilities"]
- ADR-007 Step 5: "prove write ability" — no definition of which
  user_abilities values constitute proof
- ADR-008 normal resolution: "prove project/write ability" — same gap
- Compare: Assistant manifest has "writeEvidence":
  "detail/list user_abilities and project authorization"

Expected:
The manifest or ADR should specify which field(s) within `user_abilities` prove
write permission for a Skill or Workflow entity, analogous to how the
capabilityPreflight specifies `projects[].is_project_admin` for project-level
visibility.

Actual:
The interpretation of `user_abilities` values for per-entity write permission
is not documented in either ADR or manifest for Skill and Workflow. It is
traceable from the pinned source evidence references, but the implementation
must independently read those source files to determine the exact semantics.

Impact:
Low. The source is pinned at an exact commit SHA, so the implementation can
derive the correct behavior. The gap increases implementation risk but is not
blocking given the source evidence references.

Required action:
Solution architect should add a `writeEvidence` or `writeAbilityField`
annotation to the Skill and Workflow manifest entries, citing the specific
user_abilities field path and value that proves write permission, consistent
with the Assistant entry's pattern.

Owner: solution-architect

Verification:
Resolved by adding the write-ability annotation to the manifest.
```

### Finding VER-004

```
Finding ID: VER-004
Severity: NOTE
Status: CLOSED (by design)

Title:
Q-002 task item 5 under "Skill capability" describes a Workflow-only scenario

Evidence:
- tasks.md Q-002 Skill capability item 5: "Explicit adoption with another
  same-display-name unmarked row case is handled"
- ADR-007 contains no adoption path (correct)
- ADR-008 explicitly covers this scenario

Expected:
The task item likely refers to verifying the Workflow adoption scenario
(ADR-008) and confirming that ADR-007 has no analogous implicit adoption
mechanism.

Actual:
ADR-007 correctly has no adoption path. Skills cannot be adopted via
`--adopt-workflow-id`; VR-008 rejects the flag for non-Workflow declarations.
The adoption-with-same-display-name scenario is fully specified in ADR-008.

Impact:
None. Both algorithms behave correctly. The task description wording is
ambiguous but leads to the same verification outcome.

Required action:
None. This is a documentation observation for future task authors.

Owner: N/A
```

---

## 6. Unverified Areas

The following areas were not verified in this review and remain open for
downstream tasks:

1. **Live API behavior**: No calls were made to a running CodeMie instance. The
   exact response shape and field semantics for `user_abilities`, scope filter
   behavior, and `meta_config` preservation are verified by contract reference
   only. V-000 (Deployment Verification) must confirm behavioral conformance
   against a live target.

2. **Workflow scope parameter values**: RESOLVED (VER-001 CLOSED 2026-08-10).
   The manifest now pins both scope values via enumeratePasses: absent scope
   (user-visible project workflows) and scope=marketplace (globally-published
   workflows). Source confirmed from pinned reference
   service/workflow_config/workflow_config_index_service.py WorkflowScope enum.

3. **Pagination stability guarantees beyond total-change detection**: The ADR
   detects total-field drift and cycle/repeated-ID drift. The scenario where
   new items cause rows to shift between pages without changing the total is not
   separately addressed in the algorithm. This is a platform API constraint
   (server-side keyset vs. offset pagination) outside the tool's control; the
   fail-closed behavior is correct.

---

## 7. Non-Blocking Observations from Q-001 Relevant to This Report

From Q-001 (for reference only, already recorded):

- **OBS-002**: `manifest.clientConfigurationContract.authUrlRequiredFor` names
  only `keycloak_client_credentials`; Mode (c) ROPC also requires `auth_url`.
  No correctness gap — cli.md §6 is normative. Not re-raised here.

---

## 8. Algorithm Correctness Summary

### ADR-007 (Skill)

| Property | Verdict |
|---|---|
| Exhaustive pagination covering >100 items | PASS |
| Pagination drift detection (changing totals, repeated IDs, cycles) | PASS |
| Scope: project_with_marketplace | PASS |
| Exact project and name client filtering | PASS |
| Preflight visibility proof (project-admin) | PASS |
| Zero match → POST | PASS |
| One match → PUT (unconditional on every valid apply) | PASS |
| Multiple matches → E_AMBIGUOUS_IDENTITY, exit 1, no write | PASS |
| Post-write re-resolution and race visibility | PASS |
| 409 re-resolution (bounded, POST not repeated) | PASS |
| No display-name selection or veto | PASS |
| No invented response fields | PASS |
| No adoption path (correctly absent) | PASS |

### ADR-008 (Workflow)

| Property | Verdict |
|---|---|
| Reserved identity key as sole resolution criterion | PASS |
| Strict decode rules for meta_config string | PASS |
| Non-reserved member preservation in merge | PASS |
| Canonical encode (sorted keys, compact, UTF-8, no BOM) | PASS |
| Reserved key protected in declaration schema (Q-001) | PASS |
| Exhaustive pagination covering >100 items | PASS |
| Pagination drift detection | PASS |
| Scope: marketplace-inclusive (pinned: no-scope + scope=marketplace) | PASS — VER-001 CLOSED |
| Exact project client filter | PASS |
| Zero match → adoption guard → POST / E_ADOPTION_REQUIRED | PASS |
| One match → PUT (unconditional) | PASS |
| Multiple matches → E_AMBIGUOUS_IDENTITY, exit 1 | PASS |
| Post-write re-resolution and race visibility | PASS |
| Adoption: UUID-only selection | PASS |
| Adoption: same-display-name row does not select or veto | PASS |
| Adoption: preconditions 1–5 complete | PASS |
| No display-name selection or veto in normal resolution | PASS |
| No invented response fields | PASS |

---

## 9. Verdict

```
Artifact analysis status: READY WITH NON-BLOCKING FINDINGS
```

All required scenarios (>100 rows, scopes, pagination drift, `meta_config`,
abilities, exact project, zero/one/multiple, forbidden/incomplete visibility,
explicit adoption with another same-display-name unmarked row) are covered by
the ADR-007 and ADR-008 algorithm specifications and the adapter manifest.

No scenario is algorithmically undefined. The algorithms produce no invented
response fields and no display-name selection or veto path in either Skill or
Workflow resolution.

Three non-blocking findings were identified:
- VER-001 (MEDIUM): Workflow scope parameter value not pinned in manifest.
  CLOSED 2026-08-10: manifest updated with enumeratePasses array (two concrete
  URL templates) and enumerateScopeEvidence annotation. Source:
  service/workflow_config/workflow_config_index_service.py WorkflowScope enum.
- VER-002 (LOW): Workflow per_page not specified; recommend adding a concrete
  value for implementation clarity. CLOSED 2026-08-10: per_page=100 pinned in
  both enumeratePasses entries (resolved together with VER-001).
- VER-003 (LOW): Per-entity user_abilities write proof values not documented
  for Skill or Workflow; traceable from pinned source evidence but not
  explicit in the manifest.

VER-001 and VER-002 are closed. VER-003 remains open (LOW, non-blocking).
None of the findings block implementation of W-001, S-001, or their dependents.
