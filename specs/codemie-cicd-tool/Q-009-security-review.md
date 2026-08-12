# Security review: v31 personal-project owner qualification

## Status

```text
Security review status: APPROVED WITH REQUIRED REMEDIATION
Threat-model status: ACCEPTABLE WITH REQUIRED CONTROLS
```

The v31 authorization design may proceed to T-004 implementation. The product
and refreshed architecture resolve SEC-Q009-001 by requiring exact project
detail to contain exactly one total member and requiring that sole member to be
the authenticated `/v1/user.user_id`. The prior v30 `NOT APPROVED` verdict is
superseded.

No critical or high finding is open. One medium lifecycle-documentation finding
remains: several active labels still say `RESET BY V30` or `pre-v30`, which can
mislead an operator about whether v30 live evidence remains usable. This does
not make the T-004 design unsafe because V-000B depends on the refreshed v31
V-000A and V-003 reruns the complete same-capability qualification. The labels
must nevertheless be corrected before V-000B or V-003 evidence is accepted.

Approval is for the next implementation stage only. It does not approve the
existing pre-v31 implementation, live qualification, enterprise writes,
release, or deployment.

## Review mode and scope

- Review mode: Mode B — architecture threat review, pre-implementation.
- Lifecycle task: Q-009 security delta review before T-004.
- Review date: 2026-08-12.
- Product source: `specs/codemie-cicd-tool.md` v31.
- Repository baseline: `f850a76b514db14dd75e2423e193e873f3810f41`
  plus the uncommitted v31 product/architecture delta reviewed in place.
- Pinned reference baseline: CodeMie backend `2.42.0` at
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- In scope: sole-member ownership evidence; ownership spoofing; shared-project
  creator/member escalation; duplicate memberships, members, and JSON keys;
  exact `user_id`/project equality; endpoint/origin/token/session binding;
  structural route encoding; final authorization revalidation; Assistant least
  privilege; separate write ability; state ordering; zero writes; and response,
  diagnostic, and V-000 evidence privacy.
- Explicit exclusions: implementation convergence; live target access;
  credentials and `.env`; release/deployment authorization; server changes;
  and modification of reference-only `codemie/` or `codemie-ui/`.

No Jira or Confluence material was provided or available locally.

## Sources of truth

- Product: SC-022, FR-033/037, DR-013, IR-013, BR-007, PA-005/008,
  VR-017, and AC-FR-037-01/02/03 in `specs/codemie-cicd-tool.md` v31.
- Architecture: `plan.md`, `data-model.md`, ADR-013, and amendments to
  ADR-007/008/009/012.
- Contracts: `contracts/http-adapter.md`,
  `contracts/adapter-manifest-v2.42.0.json`, and
  `contracts/source-baseline.md`.
- Tasks: Q-009, T-004, O-002A, V-000A/B, V-003, and the final trace matrices in
  `tasks.md`.
- Independent artifact analysis: `Q-009-verification-report.md`, status
  `READY WITH NON-BLOCKING FINDINGS`.
- Prior security evidence: this report's superseded v30 review,
  `Q-007-security-review.md`, and
  `O-002-preimplementation-security-review.md`.
- Pinned reference evidence: personal-project creation/repair and assignment
  restrictions; authenticated user/project-detail responses; and
  Workflow/Skill/Datasource visibility filters under `codemie/`.
- Security policy: no separate repository security policy was located;
  `AGENTS.md` and the approved product security requirements govern the review.

## Executive assessment

The v31 fallback is sufficiently narrow and testable:

- It derives the actor only from authenticated, non-empty `user_id` and never
  from email, display name, project naming, declaration content, a project-list
  row, or an entity creator.
- The existing admin branch remains global administrator/maintainer or exact
  effective-project administrator. The owner fallback begins only if that
  branch does not qualify.
- The fallback requires exactly one `/v1/user.projects[]` match, exact
  visibility-filtered detail name, `project_type == "personal"`,
  `created_by == user_id`, and exactly one total detail member whose `user_id`
  is the authenticated actor. A false boolean `is_project_admin` is accepted
  without coercion.
- Empty, multiple, duplicate-owner, owner-plus-other, and sole-mismatched
  member arrays fail as visibility-unproven. This closes the hidden-namespace
  path demonstrated in SEC-Q009-001.
- Duplicate JSON object keys are rejected before DTO conversion; only unique
  additive unconsumed members are ignored.
- One opaque invocation capability owns the API client, validated target
  origin, bearer credential, and internal session through qualification,
  exhaustive resolution, operation-specific write evidence, final visibility
  revalidation, sealing, and dispatch. Dispatch cannot substitute a client,
  base URL/origin, token, or session.
- Exact project detail appends the project with a structural URL path-segment
  API. Query encoding and string concatenation are prohibited for this route.
- Assistant remains excluded from both complete-visibility reads and retains
  its exact direct lookup and operation-specific evidence.
- Ownership supplies visibility only. It cannot synthesize row write ability.
- Every qualification failure occurs before entity resolution and before the
  modifying boundary; safe-output and zero-write matrices are assigned to both
  T-004 and V-000A.

The architecture cannot atomically freeze membership or role state across a
multi-page scan and write because the pinned server exposes no snapshot-bound
authorization primitive. Final same-capability visibility revalidation,
server-side write authorization, serialized writers, and post-write identity
verification are proportionate controls. A privileged authorization change in
the remaining interval is a documented low residual risk, not evidence that
the prior v30 gap remains open.

## Assets, actors, and trust boundaries

| Asset | Required property | Actors / boundary |
|---|---|---|
| Workflow/Skill/Datasource natural-key namespace | Complete visibility and integrity | Authenticated owner, other member, platform administrator, entity APIs |
| Actor and effective-project identity | Authenticity and exact binding | Bearer/session -> `/v1/user` -> exact project detail |
| Entity write capability | Independent authorization | Visibility proof -> resolver -> row ability -> sealed dispatch |
| Assistant operations | Least privilege | Declaration -> exact Assistant lookup/write evidence |
| Bearer credential and HTTP bodies | Confidentiality | Environment -> opaque API capability -> validated origin |
| CLI/V-000 records | Privacy and integrity | Runtime evidence -> closed output/evidence schemas |

Relevant hostile or faulty actors include an authenticated ordinary member, a
shared-project creator, another personal-project member in legacy/drifted
state, an administrator changing membership concurrently, a compromised or
misdirected target, a network attacker, and malformed target responses.

## Threat-model assessment

| Threat | Required control in v31 | Status |
|---|---|---|
| Email/project-name or declaration ownership spoofing | Authenticated `user_id`; exact comparisons; prohibited inference sources | SATISFIED IN DESIGN |
| Shared-project creator/member escalates to complete visibility | Exact `project_type == personal`, creator equality, exclusive member | SATISFIED IN DESIGN |
| Another user's personal project qualifies | Exact project membership, detail, creator, and sole-member binding | SATISFIED IN DESIGN |
| Owner plus another member hides private namespace | `members.len() == 1` and sole member is authenticated owner | SATISFIED — SEC-Q009-001 resolved |
| Duplicate membership/member is treated as proof | One exact user-project membership; exactly one total detail member | SATISFIED IN DESIGN |
| Duplicate JSON key overwrites security evidence | Duplicate-aware object parsing before DTO conversion | SATISFIED IN DESIGN |
| Additive response data widens authority | Ignore only unique additive unconsumed fields | SATISFIED IN DESIGN |
| Project value changes route/origin/query | One structural encoded path segment; adversarial corpus | SATISFIED IN DESIGN |
| Proof is replayed across client/origin/token/session | Opaque invocation capability carried by `PreparedWrite`; no dispatch replacements | SATISFIED IN DESIGN |
| Authorization changes after initial proof | Final same-capability revalidation immediately before seal | SATISFIED WITH RESIDUAL TOCTOU RISK |
| Ownership becomes write permission | Independent operation-specific row ability remains mandatory | SATISFIED IN DESIGN |
| Assistant is over-constrained or elevated | No `/v1/user` or project-detail preflight; exact direct path | SATISFIED IN DESIGN |
| Failed proof reaches a modifying call | Ordered state machine; only sealed `PreparedWrite` can dispatch | SATISFIED IN DESIGN; IMPLEMENTATION EVIDENCE PENDING |
| Body/token/identity/origin leaks through diagnostics | FR-016 allowlist; process-local evidence; closed V-000 record | SATISFIED IN DESIGN; IMPLEMENTATION EVIDENCE PENDING |

## Finding SEC-Q009-001

Finding ID: SEC-Q009-001  
Severity: MEDIUM  
Confidence: CONFIRMED  
Status: RESOLVED

Title: Personal-owner predicate did not prove exclusive membership or complete
entity visibility

Original issue:

v30 required exactly one detail member matching the authenticated owner but
allowed additional members. A non-admin owner could therefore miss another
member's private or creator-owned Workflow, Skill, or Datasource rows and make
identity decisions over an incomplete namespace.

Resolution evidence:

- FR-037 now requires `members[]` to contain exactly one total entry and binds
  that sole entry to authenticated `user_id`.
- DR-013 and IR-013 classify empty, multiple, duplicate-owner,
  owner-plus-other, and sole-mismatched arrays as
  `E_VISIBILITY_UNPROVEN`, exit 2, before resolution/write.
- PA-008 and VR-017 repeat the exclusive-member invariant and preserve
  separate write ability and Assistant exclusion.
- ADR-013, `PersonalOwnerScope.total_detail_member_count: 1`, the HTTP
  contract, adapter manifest predicate, plan, source baseline, T-004, and
  V-000A carry the same rule.
- AC-FR-037-01/02 require the single-owner positive and cardinality negatives.
- Both Rust and Python task matrices require zero entity-resolution and zero
  modifying calls for qualification negatives.

Verification required after implementation:

- Exact sole non-admin owner succeeds for Workflow, Skill, and Datasource.
- Empty, duplicate-owner, owner-plus-another-user, other-only, and sole
  mismatched arrays fail visibility qualification.
- Every negative yields exit 2, empty stdout, one allowlisted diagnostic, zero
  entity resolver calls, and zero POST/PUT/PATCH/DELETE calls.
- Unique additive unconsumed fields remain compatible.

Owner: product-spec-owner and solution-architect (completed design change);
implementation-engineer and verification-engineer (pending implementation and
independent retest).

## Finding SEC-Q009-002

Finding ID: SEC-Q009-002  
Severity: MEDIUM  
Confidence: CONFIRMED  
Status: OPEN

Title: Active lifecycle labels can be read as permitting superseded v30 live
evidence

Affected components:

- `tasks.md` V-000B and V-003 lifecycle labels and invalid-record sentence.
- ADR-012's amendment-version label.
- The final product readiness narrative.

Evidence:

- V-000B/V-003 still say `RESET BY V30`, and the invalidation sentence names
  `pre-v30` rather than every pre-v31 record.
- ADR-012 still says it was amended for product v30.
- The product readiness paragraph still says v30 requires the downstream
  refresh.
- `Q-009-verification-report.md` independently records the same issue as
  Q009-VER-005.

Attack/failure scenario:

1. An operator sees an apparently successful v30 qualification record.
2. A stale lifecycle label is interpreted as invalidating only pre-v30 data.
3. The operator attempts to use that record without the v31 sole-member,
   duplicate-key, invocation-capability, path-segment, and final-revalidation
   proof.

Impact:

Lifecycle evidence can be misclassified. Current task dependencies prevent a
conforming execution from accepting the stale record, so this does not block
T-004 implementation. It blocks acceptance of V-000B/V-003 evidence until the
labels are corrected.

Likelihood:

Low to medium. The substantive v31 task bodies are clear, but lifecycle labels
are commonly used in operational review.

Required remediation:

- Solution architect: replace applicable `RESET BY V30`, `pre-v30`, and stale
  amendment/readiness labels with v31 or `pre-v31`, explicitly invalidating
  every pre-v31 V-000B/V-003 record.
- Verification engineer: repeat the active-artifact stale-version search.

Owner: solution-architect; verification-engineer for retest.

Verification:

- Active lifecycle statements consistently identify v31.
- Every pre-v31, differently principaled, differently digested, stale, or
  differently scoped V-000B/V-003 record is explicitly unusable.

## Required T-004 and V-000A controls

These controls are mandatory implementation acceptance criteria, not optional
hardening:

1. Convert `/v1/user` and project detail through duplicate-aware JSON object
   construction before typed DTO conversion. Reject duplicates even when values
   agree. Reject all missing/null/empty/wrong-type consumed fields. Tolerate
   only unique additive unconsumed fields.
2. Require one exact effective-project membership, exact personal detail, and
   exactly one total member: authenticated `user_id`.
3. Use byte-exact decoded equality without trimming, case folding, Unicode or
   email normalization, substring matching, or display-name substitution.
4. Use one opaque invocation capability owning the `ApiClient`, validated
   scheme/host/effective port, bearer credential, and internal session.
5. Make `PreparedWrite` carry that capability plus kind, effective project,
   operation, completed resolution, selected server ID where applicable,
   separate write-ability evidence, final visibility proof, and projected
   request. Dispatch accepts no independent base URL/client/token/session.
6. Construct exact project detail by appending one structural URL path segment,
   never string concatenation or query encoding. Cover `/`, `%2f`, `%252f`,
   `?`, `#`, space, Unicode, `.`, and `..` without origin/base/query/fragment or
   segment-count change.
7. Revalidate the chosen admin or owner proof with the same capability after
   resolution, write evidence, and projection, immediately before sealing.
8. Keep Assistant on its direct exact path with no complete-visibility GET.
9. Never derive write ability from ownership. Existing update/adoption requires
   source-pinned operation evidence.
10. Test every field, duplicate, cardinality, mismatch, 401/403/404, redirect,
    timeout, size/depth, path, capability-substitution, evidence-change, and
    write-ability fault for correct taxonomy and zero modification.
11. Keep token, origin, internal session, actor ID, member list, raw project
    value, bodies, arbitrary headers, payloads, and exceptions out of CLI and
    durable V-000 evidence. Output only fixed categories and already-approved
    safe identifiers/request IDs.

## Security requirement coverage

| Requirement/property | Status | Evidence |
|---|---|---|
| Sole total member / complete namespace | SATISFIED IN DESIGN | FR-037, DR/IR-013, PA-008, VR-017, ADR/data model/contracts/tasks |
| Ownership and project spoofing resistance | SATISFIED IN DESIGN | Authenticated `user_id`, exact detail/type/creator/member equality |
| Duplicate-aware strict JSON | SATISFIED IN DESIGN | DR-013, manifest, HTTP contract, both task matrices |
| Additive-unconsumed compatibility | SATISFIED IN DESIGN | IR-012/013 and duplicate-aware contract |
| Same client/origin/token/session | SATISFIED IN DESIGN | `InvocationApiCapability`, sealed dispatch, substitution tests |
| Single-segment encoding | SATISFIED IN DESIGN | ADR/plan/data model/manifest/HTTP/task adversarial corpus |
| Final pre-seal revalidation | SATISFIED IN DESIGN | State model, HTTP contract, manifest ordering, task mutation |
| Separate write ability | SATISFIED IN DESIGN | FR-033/037, PA-008, sealed evidence, negative tests |
| Assistant least privilege | SATISFIED IN DESIGN | Applicability matrix and Assistant-no-preflight tests |
| Zero writes on failure | SATISFIED IN DESIGN | Ordered seal and Rust/Python request-count matrices |
| Diagnostic/evidence privacy | SATISFIED IN DESIGN | FR-016, DR-013, V-000 fixed evidence contract |
| Live-evidence version labels | PARTIALLY SATISFIED | SEC-Q009-002 |

## Validation performed

Local checks actually run:

```text
git status --short
git diff -- <v31 product/architecture/contract/task artifacts>
git diff --check
git rev-parse HEAD
git -C codemie rev-parse HEAD
git -C codemie status --short
rg -n <v31 authorization/session/encoding/JSON/privacy/version terms> <active artifacts>
sed -n / nl -ba over the relevant product, architecture, contracts, tasks,
verification report, prior security evidence, and pinned reference source
python3 duplicate-aware parse and required-v31 assertion of
contracts/adapter-manifest-v2.42.0.json
```

Observed results:

- Workspace baseline was `f850a76b514db14dd75e2423e193e873f3810f41`
  with the reviewed v31 artifacts uncommitted.
- The adapter manifest parsed without duplicate keys and asserted the sole
  member predicate, opaque invocation binding, one-segment route construction,
  duplicate-key rejection, final revalidation, Assistant exclusion, and
  separate write evidence.
- `git diff --check` passed before this report update.
- The stale-version search confirmed SEC-Q009-002.
- No network access, credential read, `.env` inspection, intrusive/live test,
  implementation edit, reference edit, or remote mutation occurred.

## Unverified areas and residual risk

- T-004 and the refreshed V-000A are not implemented. This review assesses
  design sufficiency, not runtime convergence.
- No live target was queried. V-000B/V-003 require fresh v31 evidence and
  explicit write authorization.
- Server-side authorization/membership changes are not snapshot-bound across
  the exhaustive scan and write. Final revalidation narrows but cannot remove
  that interval.
- O-001/O-002B provider activation, V-000B/V-003, V-001/V-002, release review,
  and deployment authorization remain incomplete.
- No regulatory/compliance conclusion or risk acceptance is made.

## Remediation and next action

| Finding/control | Owner | Required action | Gate | Status |
|---|---|---|---|---|
| SEC-Q009-001 | implementation-engineer / verification-engineer | Implement and independently retest v31 sole-member proof | T-004/V-000A completion | RESOLVED IN DESIGN; IMPLEMENTATION PENDING |
| SEC-Q009-002 | solution-architect | Correct stale v30/pre-v30 lifecycle labels | Before accepting V-000B/V-003 evidence | OPEN |
| Mandatory security controls | implementation-engineer | Implement all eleven controls above | V-001/V-002 | PENDING |
| Attack-path retest | verification-engineer / security-reviewer | Retest exact negatives and equivalent paths | Before release review | PENDING |

Next action: the implementation engineer may begin T-004 and refreshed V-000A.
The solution architect must close SEC-Q009-002 before any live v31
qualification or enterprise-smoke evidence is accepted. Post-implementation
verification and security review must prove the controls before release
progression.

## Accepted risks

None. The residual authorization TOCTOU interval is documented for downstream
validation and operations; this report does not accept risk on behalf of an
owner.
