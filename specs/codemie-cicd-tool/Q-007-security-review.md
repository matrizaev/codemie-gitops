# Security review: v28 compatibility correction

## Status

```text
Security review status: APPROVED FOR NEXT STAGE
Threat-model status: ACCEPTABLE WITH REQUIRED CONTROLS
```

The decision to remove `GET /v1/info.version` as an unconditional compatibility
gate is security-sound. That value is a self-asserted semantic application
version, not authenticated build provenance, and comparing it with the pinned
Git SHA rejects the exact reviewed clone without preventing a malicious or
misdirected target from claiming any value.

The final post-remediation review confirms the semantic-version gate was
removed without weakening exact-project authorization, Assistant least
privilege, strict consumed-field decoding, row write-ability, page-0 exhaustive
resolution, diagnostic confidentiality, or fail-before-write behavior.

SEC-Q007-002, SEC-Q007-005, and SEC-Q007-006 are resolved. The sole production
CodeMie modifying transport boundary consumes a non-forgeable `PreparedWrite`
that owns kind-specific resolution evidence and the linked projected request.
Workflow adoption now exhausts both scopes before candidate lookup, requires
zero exact markers and a strictly unmarked exact-project writable candidate,
and never seals a write on a failed gate. Workflow marker parsing now has four
states and rejects malformed, duplicate-key, conflicting, or wrong-shaped
reserved records before modification.

SEC-Q007-007 is also resolved. Ordinary Workflow resolution now derives the
schema-validated authored `spec.name`, passes it separately from
`metadata.slug` through both exhaustive scopes, and uses it only for the
nonselecting unmarked-row adoption guard. Regression probes with differing
name/slug values prove adoption-required failure and zero modifying calls in
both scopes, while explicit by-ID adoption remains candidate-only. No open
HIGH or CRITICAL finding remains in the reviewed scope.

## Review mode and scope

- Review mode: Mode C — implementation security review, retaining the earlier
  Mode B threat-review record below.
- Feature: v28 source-derived, operation-applicable compatibility gate.
- Baseline: backend tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- In scope: ADR-004, adapter manifest, HTTP contract, plan, data model, tasks,
  SC-021, IR-011/012, and their acceptance criteria.
- Runtime implementation, tests, and the current working-tree delta were
  inspected directly. Focused and full local tests were run.
- Excluded: release authorization, deployment qualification, intrusive target
  testing, and changes under the reference-only `codemie/` and `codemie-ui/`
  trees.
- Jira and Confluence: not consulted; no content was provided locally for this
  delta.

## Sources of truth

- Product: `specs/codemie-cicd-tool.md` v28, especially IR-011/012 and
  AC-IR-011-01/AC-IR-012-01.
- Architecture: `adr/004-openapi-subset-compatibility-gate.md`, `plan.md`, and
  `data-model.md`.
- Contracts: `contracts/adapter-manifest-v2.42.0.json`,
  `contracts/http-adapter.md`, and `contracts/source-baseline.md`.
- Tasks: Q-007, T-003, R-001, and D-001 in `tasks.md`.
- Independent pre-remediation implementation verification:
  `Q-007-post-implementation-verification.md` (created concurrently with the
  earlier review; its then-current sealed-boundary finding corroborated the
  original SEC-Q007-002 evidence but is not final-remediation evidence).
- Supporting implementation evidence: current `src/http/mod.rs`,
  `src/coordinator/mod.rs`, and all four `src/adapters/*.rs`; deleted historical
  `src/preflight/mod.rs` was reviewed through the working-tree diff.

## Threat assessment

### Downgrade and target spoofing

Removing the semantic-version comparison does not remove a trustworthy
anti-downgrade control: `/v1/info.version` is supplied by the same target whose
behavior is being assessed and can be changed independently of its source. A
compromised target could return the pinned SHA just as easily as `0.16.0`.
ADR-004 instead fixes the client contract at build time, strictly validates the
operation-applicable response subset, forbids runtime contract widening, and
assigns broader semantic drift to deployment/release qualification. This is a
reasonable division of controls.

Residual risk remains: an older or modified deployment that reproduces the
consumed subset may pass runtime probes. Runtime response conformance is not
build attestation. TLS/validated target selection, immutable client contracts,
and V-000/release contract tests are therefore required; `/v1/info` must not be
reintroduced as a compensating control.

### Confused deputy

The bearer credential is sent to an explicitly configured, validated target and
redirect controls prevent cross-origin forwarding. The relevant deputy risk is
project authorization: a project-admin result is sufficient only when the
`projects[]` entry names the exact effective project. Accepting project-admin in
any project falsely proves complete visibility for another project.

### Fail-open behavior

The implemented rule is correct: every required, consumed field and shape is
strict; missing, wrong-type, unstable, truncated, or over-budget evidence fails
before modification, while genuinely additional unconsumed fields are ignored.
The write state and manifest must make that rule mechanically unambiguous. No
Serde defaults, `Option`, fallback counts, truthy coercion, partial-page success,
or ignored adapter error may construct a write-capable state.

## Findings

### Finding ID: SEC-Q007-001

Severity: MEDIUM
Confidence: CONFIRMED
Status: RESOLVED

Title: Capability proof is not consistently scoped to the exact effective project or to Datasource

Affected assets:
- Cross-project entity integrity and identity uniqueness
- Workflow, Skill, and Datasource complete-visibility guarantees

Affected components:
- `contracts/adapter-manifest-v2.42.0.json:52-55`
- `contracts/http-adapter.md:365-368`
- `tasks.md:405-435`
- Existing evidence: `src/http/mod.rs:67-91,612-634`

Evidence:
- The HTTP contract requires project-admin for the exact project, but its text
  names only Workflow and Skill even though ADR-012 and D-001 also require the
  control for Datasource.
- The manifest consumes `projects[].name` but records no exact-effective-project
  predicate.
- T-003 requires missing/invalid field tests but no wrong-project-admin negative
  test.
- The current DTO omits `projects[].name`, defaults missing security fields, and
  accepts `is_project_admin=true` for any project. This demonstrates that the
  ambiguity has already produced an unsafe interpretation.

Attack scenario:
1. A principal is project-admin for project A but applies a declaration for project B.
2. `/v1/user` returns the truthful project-A admin entry.
3. A non-project-bound preflight treats this as complete visibility for B.
4. Resolution may operate over an incomplete B view and select/create against
   unsafe identity evidence; server-side write authorization may constrain the
   final impact but does not repair the false completeness proof.

Expected control:
Global admin/maintainer may satisfy the preflight globally. Otherwise exactly
one decoded `projects[]` entry must have `name == effective_project` and
`is_project_admin == true`. The same rule applies before Workflow, Skill, and
Datasource resolution.

Actual behavior:
The exact-project rule exists in one contract sentence but is not consistently
represented in the manifest/task evidence, and the current implementation
accepts project-admin for an unrelated project.

Impact:
False visibility proof can cause cross-project confused-deputy behavior,
duplicate natural identities, unsafe target selection, or an unauthorized write
attempt.

Likelihood:
Medium. Multi-project service principals are expected, and the current code
already implements the unsafe interpretation. Server authorization is a
compensating boundary, so confirmed cross-project write capability was not
established by this architecture review.

Required remediation:
- Solution architect: make Datasource explicit in the HTTP contract and record
  the exact-effective-project predicate in the authoritative manifest/tasks.
- Implementation engineer: strictly decode every consumed role/project field,
  remove missing-field defaults, and compare the project name before treating
  project-admin as sufficient.

Owner: solution-architect (artifact correction), then implementation-engineer

Verification:
- Exact target-project admin passes.
- Admin of only another project fails `E_VISIBILITY_UNPROVEN`, exit 2, with zero
  POST/PUT/DELETE.
- Missing or wrong-type `is_admin`, `is_maintainer`, `projects`, `name`, or
  `is_project_admin` fails `E_API_INCOMPATIBLE`, exit 2, with zero modification.
- The matrix is exercised for Workflow, Skill, and Datasource create/update;
  global admin and maintainer alternatives remain positive cases.

References:
- IR-009/012, QR-009, ADR-012, D-001, T-003, R-001

Resolution evidence (focused re-review):
- The manifest now records the exact-effective-project predicate and distinct
  compatibility versus visibility outcomes.
- ADR-012, the HTTP contract, source baseline, plan, data model, and tasks now
  include Datasource and the exact same-entry project/name/admin rule.
- T-003/R-001 require wrong-project, missing-field, strict-decoding, and
  zero-modifying-request tests.

### Finding ID: SEC-Q007-002

Severity: MEDIUM
Confidence: CONFIRMED
Status: RESOLVED

Title: PreparedWrite did not make completed resolution evidence mandatory at the modifying boundary

Affected assets:
- Integrity of all four entity write operations

Affected components:
- `src/adapters/mod.rs:29-125`
- `src/http/mod.rs:313-365,443-535`
- Adapter modifying dispatchers

Original evidence (before final remediation):
- `NonMutatingResolutionComplete` is a zero-sized marker created inside
  `PreparedWrite::seal`; the constructor accepts only kind/project, optional
  visibility, and `WritePlan`, not completed typed read evidence.
- `ApiClient::post`, `put`, `post_or_conflict`, `post_multipart`, and
  `put_multipart` remain raw modifying entry points that accept path/body data
  with no `PreparedWrite`.
- No compile-time boundary test or complete state-machine fault matrix proves
  that a partial read state cannot reach those methods.

Attack scenario:
1. An adapter branch omits or mishandles one required identity, reference,
   detail, preservation, permission, or pagination read.
2. It still calls `PreparedWrite::assistant` or `PreparedWrite::visible` with a
   projected plan; the constructor manufactures the completed-resolution
   marker.
3. The dispatcher unwraps the plan and calls a raw modifying transport method.
4. POST/PUT occurs without the evidence the type claims to seal.

Expected control:
A write-capable plan/state cannot exist until the selected adapter has returned
strictly validated identity, reference, detail/preservation, permission, and
pagination evidence applicable to that specific create/update path.

Original behavior:
The new type checks entity-kind/project preflight consistency, but it neither
contains nor requires completed operation-specific read evidence. The actual
transport methods remain reachable without the type.

Impact:
The central fail-before-write guarantee remains a coding convention. An omitted
or misclassified pre-write fault can become a modifying request; SEC-Q007-005
demonstrates such a current branch.

Likelihood:
Medium. Most current ordinary branches order checks correctly, but raw methods
are used throughout every adapter and Workflow adoption already seals partial
evidence.

Required remediation:
Represent completed kind-specific read evidence with non-forgeable types, make
it an input to `PreparedWrite`, and make production modifying transport methods
accept only that sealed value (or move the raw send primitives behind a private
dispatcher that does). Partial evidence must not be convertible into a write
plan accepted by the modifying boundary.

Owner: implementation-engineer

Verification:
- Tests inject one failure at each generic and adapter-specific evidence step
  and assert no modifying route is called.
- A write entry point cannot be called with partial evidence (compile-time type
  test where practical, otherwise a focused unit test over the state machine).
- Create, update, Workflow adoption, and Skill 409 re-resolution are covered;
  post-write verification remains explicitly separate because the write may
  already have committed.

References:
- IR-012, AC-IR-012-01, data model section 7, HTTP contract section 1, R-001

Architecture resolution evidence (focused re-review):
- The state order is now visibility proof, operation-specific non-mutating
  resolution, projection, sealed `PreparedWrite`, and only then write.
- The HTTP contract and R-001 require the modifying dispatcher to accept only
  the sealed type and require fault injection at every earlier read boundary.

Superseded implementation evidence (before final remediation):
- `src/adapters/mod.rs:29-31,85-103` defines the completed-resolution evidence
  as a zero-sized marker and constructs it inside `PreparedWrite::seal`; no
  result from identity, reference, detail, preservation, or pagination reads is
  required to construct the marker.
- `src/http/mod.rs:313-365,443-535` leaves JSON, conflict-aware, and multipart
  modifying methods callable with raw path/body inputs and no `PreparedWrite`.
  The transport boundary therefore does not enforce the approved seal.
- `src/adapters/workflow.rs:102-150` demonstrates the practical partial-evidence
  path: adoption skips exhaustive identity enumeration, constructs
  `PreparedWrite`, and reaches PUT after only the candidate detail read.
- No compile-time or focused state-machine test proves that partial evidence
  cannot reach the raw modifying methods. The existing test at
  `src/adapters/workflow.rs:922-973` instead asserts that adoption skips the
  required enumeration and succeeds.

Final implementation resolution evidence:
- `src/adapters/mod.rs` now defines non-zero, kind-specific
  `CompletedResolution` values for Assistant, Workflow, Skill, and Datasource.
  They retain the applicable effective project, resolved create/update target,
  scan/detail/reference evidence, and write-ability proof before the private
  `PreparedWrite` constructors can seal the projected request.
- `PreparedWrite::seal` and `PreparedWrite::into_request` independently require
  that entity kind, exact-project preflight, completed resolution target, and
  projected create/update target agree. Multipart file bytes are read before
  sealing and are owned by the prepared value.
- The raw CodeMie `post`, `put`, conflict-aware POST, and multipart methods in
  `src/http/mod.rs` are private. The only production-visible modifier is
  `ApiClient::dispatch_prepared`, which consumes `PreparedWrite`; all adapter
  modifying paths use it and no adapter contains a raw modifying call.
- `completed_resolution_evidence_is_kind_specific_and_not_zero_sized` and
  `production_http_api_exposes_only_the_prepared_modifying_boundary` pass.
  Adapter fault tests additionally demonstrate that incompatible, unstable,
  unauthorized, and invalid-marker states do not reach a modifying route.

### Finding ID: SEC-Q007-003

Severity: LOW
Confidence: CONFIRMED
Status: RESOLVED

Title: Manifest wording says an unknown consumed response field is ignored

Affected assets:
- Fail-closed response compatibility policy

Affected components:
- `contracts/adapter-manifest-v2.42.0.json:15-20`

Evidence:
- The manifest says `"unknownConsumedResponseField": "ignore"`.
- v28 permits only *additional unconsumed* response fields to be ignored and
  requires every consumed field to be strict.

Attack scenario:
1. An implementation or generator interprets the manifest key literally.
2. A newly encountered member inside a consumed response structure is treated
   as ignorable without deciding whether the client consumes it.
3. Contract drift is silently accepted and may influence identity or permission
   behavior outside the typed subset.

Expected control:
Only additional fields that are explicitly unconsumed by the immutable client
contract are ignored. Required consumed members and their containers, types,
nullability, and operation applicability are strict.

Actual behavior:
The machine-readable key conflates “unknown” with “unconsumed,” while the prose
artifacts use the correct narrower rule.

Impact:
Implementation ambiguity can weaken fail-closed response decoding.

Likelihood:
Low because the product, plan, tasks, and HTTP prose consistently state the
correct rule, but the manifest is declared authoritative.

Required remediation:
Rename or replace this policy with an unambiguous additional-unconsumed-field
rule, and ensure the manifest or an explicitly referenced typed contract records
requiredness/type/nullability/applicability for every consumed response member.

Owner: solution-architect

Verification:
Manifest validation plus paired fixtures: extra unconsumed members pass; missing,
wrong-type, wrong-nullability, or structurally misplaced consumed members fail
before modification.

References:
- IR-012, AC-IR-012-01, ADR-004

Resolution evidence (focused re-review):
- The manifest policy value is now
  `ignore-only-when-additive-and-unconsumed`.
- The HTTP contract, data model, plan, and T-003 paired fixtures consistently
  limit tolerance to additive fields the selected operation does not consume.

### Finding ID: SEC-Q007-004

Severity: MEDIUM
Confidence: HIGH
Status: RESOLVED

Title: Remediation expands project-admin privilege to Assistant without product approval

Affected assets:
- CI service-account privilege and blast radius
- Assistant apply availability under the approved least-privilege model

Affected components:
- `adr/012-datasource-visibility.md:27-32,81-87`
- `contracts/adapter-manifest-v2.42.0.json:52-61`
- `contracts/http-adapter.md:377-386`
- `contracts/source-baseline.md:50-56`
- `data-model.md:213-216`
- `tasks.md:429-445,568-572`

Evidence:
- Product FR-033 and PA-005 require complete-visibility privilege for Workflow
  and Skill, while ADR-012 supplies the separately approved Datasource rule.
- PA-003 requires CI service accounts to hold only the read/create/update
  permissions needed for in-scope entity types.
- The pinned source baseline describes Assistant as a direct exact
  slug/project lookup with partial uniqueness rather than an exhaustive,
  visibility-sensitive resolver.
- The remediated artifacts nevertheless require global admin/maintainer or
  exact-project admin before Assistant resolution and make all-kind tests an
  implementation acceptance criterion.

Attack scenario:
1. An adopting team grants project-admin solely because the CLI now requires it
   for Assistant apply.
2. The CI credential is exposed or misused.
3. The attacker receives the broader project-admin capabilities rather than the
   narrower Assistant read/create/update permissions authorized by PA-003.
4. Impact extends to other project resources and administrative operations
   allowed by that role.

Expected control:
Workflow, Skill, and Datasource use the exact-project/global privilege predicate
because their client-side exhaustive resolution needs complete visibility.
Assistant uses its approved direct exact lookup and server-enforced
read/create/update authorization unless the product-spec-owner explicitly adds
an Assistant complete-visibility/admin prerequisite.

Actual behavior:
The architecture and tasks apply the higher privilege predicate to all four
entity kinds without a corresponding product requirement or decision.

Impact:
The change can force unnecessary standing administrative privilege into CI,
increase credential-compromise blast radius, and reject otherwise valid
least-privilege Assistant automation.

Likelihood:
Medium. The all-four-kind requirement is explicit in normative artifacts and
would be implemented by T-003/R-001. Exact project binding constrains scope but
does not remove the excess privilege.

Required remediation:
Either remove Assistant from the capability-preflight applicability set, state
machine, source-baseline claim, and all-kind test matrix, or obtain an explicit
product-spec-owner decision that updates FR-033/PA-003/PA-005 and documents why
Assistant requires project-admin rather than narrower permissions. The security
reviewer cannot make that product choice.

Owner: product-spec-owner (behavior decision), then solution-architect

Verification:
- If Assistant is removed: a least-privilege Assistant principal can pass the
  direct exact lookup/write flow while Workflow, Skill, and Datasource retain
  exact-project/global capability tests and zero-write failures.
- If Assistant is approved: updated product requirements and acceptance
  criteria explicitly trace the privilege prerequisite, and the risk of broader
  CI credentials is assigned to an authorized owner.

References:
- FR-033, PA-003, PA-005, ADR-002, ADR-012, T-003, R-001

Resolution evidence (focused re-review):
- The manifest limits `GET /v1/user` applicability to Workflow, Datasource, and
  Skill.
- ADR-012, the HTTP/source contracts, plan, data model, and tasks explicitly
  exclude Assistant from the admin predicate under PA-003.
- The data model represents `AssistantAdminPreflightNotRequired` separately and
  requires strict direct `(project, slug)` resolution in
  `NonMutatingResolutionComplete` before sealing `PreparedWrite`.
- T-003/R-001 require tests proving Assistant makes no `/v1/user` call yet
  cannot write without valid direct-lookup evidence.

Implementation retest result:
- RESOLVED. `src/adapters/assistant.rs:67-95` performs no `/v1/user` request,
  strictly decodes the direct lookup, proves existing-row write ability, and
  seals the projected request. Coordinator tests assert zero `/v1/user` and
  zero `/v1/info` contacts on Assistant create and update.

### Finding ID: SEC-Q007-005

Severity: MEDIUM
Confidence: CONFIRMED
Status: RESOLVED

Title: Workflow adoption sealed a write without proving zero existing identity markers

Affected assets:
- Workflow natural-key integrity
- Reserved GitOps identity records

Affected components:
- `src/adapters/workflow.rs:94-150,922-973`
- `src/projection/mod.rs:606-676`

Original evidence (before final remediation):
- The adoption branch fetches only `GET /v1/workflows/id/{adopt_id}` and skips
  both exhaustive Workflow scopes. It therefore cannot establish that zero
  exact `(project, slug)` markers already exist.
- The branch checks candidate project and row write ability, but does not
  require the candidate to be unmarked. Projection then installs the requested
  reserved identity record over any prior reserved value.
- `apply_adopt_id_skips_enumeration` passes while serving a candidate that is
  already marked and observing a PUT. The full suite reports this test as
  passing.

Attack scenario:
1. A project-authorized principal supplies an adoption UUID while another
   Workflow already carries the declaration's exact reserved identity.
2. The adapter skips the complete two-scope identity scan and reads only the
   supplied candidate.
3. The candidate's existing marker is not required to be absent, and the
   projector replaces it with the declaration's marker.
4. PUT commits a duplicate identity or reassigns a marked Workflow; post-write
   ambiguity detection occurs only after integrity has already changed.

Expected control:
Explicit adoption first proves zero existing exact identity markers, then
requires the selected candidate to be exact-project, writable, unmarked, and
safely mergeable before sealing a prepared write.

Original behavior:
The zero-existing-marker and unmarked-candidate evidence is absent, but
`PreparedWrite` and PUT remain reachable.

Impact:
An authorized apply can corrupt Workflow identity uniqueness or retarget a
previously marked Workflow. Detection after PUT cannot prevent the mutation.

Likelihood:
Medium. Adoption is explicit and requires project/write access, but it is the
approved recovery path for legacy Workflows and the current test suite codifies
the unsafe shortcut.

Required remediation:
Enumerate both required Workflow scopes before the candidate detail read,
reject any existing exact marker, strictly require the candidate's reserved
member to be absent, and preserve non-reserved metadata. Do not construct
`PreparedWrite` on any failed check.

Owner: implementation-engineer

Verification:
- Existing exact marker plus adoption selector produces exit 1 and zero PUT.
- Marked candidate, malformed candidate metadata, wrong project, or absent
  write ability produces the specified safe error and zero modifying requests.
- Only an unmarked, exact-project, writable candidate with zero existing exact
  markers reaches one PUT and post-write verification.

References:
- FR-029, AC-FR-029-02, ADR-008, `http-adapter.md` section 5.2, R-001

Final implementation resolution evidence:
- `src/adapters/workflow.rs` calls `enumerate_all` before branching on
  `--adopt-workflow-id`; `enumerate_all` exhausts independent project and
  marketplace page-0 scans and returns their concrete evidence.
- Adoption refuses any existing exact marker before candidate lookup. The
  by-ID candidate must echo the supplied ID, belong to the exact effective
  project, classify strictly as `Unmarked`, and expose `write`; only then can a
  Workflow `CompletedResolution` be sealed for one PUT.
- `apply_adopt_id_requires_zero_marker_scan_and_unmarked_candidate`,
  `adoption_existing_exact_marker_stops_before_candidate_read_and_write`, and
  `adoption_candidate_fault_matrix_stops_before_every_modifying_route` pass.
  The negative matrix asserts zero POST, PUT, PATCH, and DELETE for marked,
  malformed, wrong-project, and non-writing candidates.

### Finding ID: SEC-Q007-006

Severity: MEDIUM
Confidence: CONFIRMED
Status: RESOLVED

Title: Invalid Workflow identity metadata was treated as absence and could reach create

Affected assets:
- Workflow natural-key uniqueness and reconciliation integrity

Affected components:
- `src/adapters/workflow.rs:341-408,455-475,997-1009`

Original evidence (before final remediation):
- `matches_identity` returns `false` for malformed JSON, a missing reserved key,
  and every wrong-type or conflicting reserved record; it cannot distinguish an
  ordinary unmarked row from invalid identity evidence.
- `enumerate_pass` filters only boolean matches. When both exhaustive scopes
  return no boolean match, `apply` selects the create path and seals a POST.
- The unit tests explicitly assert that invalid JSON and missing reserved
  content are non-matches; no negative apply test asserts zero POST for invalid
  target-project identity metadata.

Attack scenario:
1. A visible Workflow in the effective project contains a malformed or
   conflicting reserved GitOps identity member for the declaration's identity.
2. Exhaustive pagination retrieves the row successfully.
3. The semantic decoder silently converts the invalid evidence to `false`.
4. Resolution concludes the identity is absent and POST can create a duplicate
   instead of failing closed.

Expected control:
The resolver distinguishes unmarked metadata from malformed/conflicting
reserved metadata and returns `E_IDENTITY_MARKER_INVALID`, exit 1, with zero
modifying requests for invalid target-project evidence.

Original behavior:
Invalid reserved identity content is indistinguishable from absence at the
resolution boundary.

Impact:
Duplicate Workflow identities can be created and later reconciliation becomes
ambiguous; automated recovery cannot safely select or roll back either row.

Likelihood:
Medium. The condition requires pre-existing malformed or conflicting metadata,
but migration, manual API/UI edits, or a prior client can create it, and the
current logic deterministically fails open.

Required remediation:
Replace the boolean helper with a strict typed classification such as
`Unmarked | Exact | OtherValid | Invalid`, propagate invalid target-project
records as the approved reconciliation error, and seal no POST/PUT on that
path.

Owner: implementation-engineer

Verification:
Inject malformed JSON, non-object roots, wrong-type `version/project/slug`, and
conflicting duplicate reserved records on both Workflow scopes; each applicable
case must produce the specified safe diagnostic and exact zero POST/PUT/DELETE.

References:
- FR-028/030/032, AC-FR-028-01, ADR-008, IR-012, R-001

Final implementation resolution evidence:
- `classify_marker` now returns the closed states `Unmarked`, `Exact`,
  `OtherValid`, and `Invalid`. A custom recursive Serde visitor rejects
  duplicate object members at every depth and trailing JSON; the reserved
  identity is a closed three-member object with version `1`, non-empty string
  project/slug, and project equality with the server row.
- An invalid target-project marker in either enumeration scope returns
  reconciliation failure immediately. It cannot be collapsed into the
  zero-match create path. Ordinary updates revalidate the detail marker as
  `Exact` before projection.
- `marker_classification_invalid_shapes_fail_closed` and
  `invalid_target_project_markers_in_either_scope_fail_closed_with_no_modification`
  pass, including malformed JSON, non-object roots, wrong types, conflicting
  row projects, and duplicate reserved members with zero modifying calls.

### Finding ID: SEC-Q007-007

Severity: HIGH
Confidence: CONFIRMED
Status: RESOLVED

Title: Workflow legacy adoption guard compares server display name to slug and can permit duplicate creation

Affected assets:
- Workflow natural-key integrity and legacy-record safety
- Correct enforcement of explicit adoption

Affected components:
- `src/adapters/workflow.rs:152-175,448-500,544-560`
- Workflow ordinary zero-marker resolution

Pre-remediation evidence:
- `ApplyRequest` supplies only `project_name` and `slug` to `enumerate_all`.
  Neither `enumerate_all` nor `enumerate_pass` receives authored `spec.name`.
- For an unmarked exact-project row, `enumerate_pass` records the adoption guard
  only when `item.name == slug`.
- AC-FR-030-02 and ADR-008 require an unmarked row with the same Workflow
  display name to produce adoption-required failure. Workflow display name is
  authored `spec.name`; `metadata.slug` is the distinct persisted natural-key
  component and need not equal it.
- Existing positive tests use values that do not exercise a differing
  `spec.name`/`metadata.slug` legacy row, so the full green suite does not catch
  this branch.

Attack scenario:
1. A project-authorized principal applies a Workflow whose `spec.name` differs
   from `metadata.slug`, which the declaration contract permits.
2. An existing exact-project legacy Workflow has that authored display name and
   no reserved identity marker.
3. Exhaustive page-0 scans retrieve the row, but the guard compares its name to
   the slug and discards it.
4. Resolution incorrectly concludes that no adoption guard exists and POST can
   create another Workflow instead of requiring explicit reviewed adoption.

Expected control:
Ordinary zero-marker resolution compares an unmarked exact-project row's
server `name` with the schema-validated authored `/spec/name`. Any exact match
causes adoption-required failure and exact zero modifying requests. The value
must remain a nonselecting guard and must not affect explicit by-ID adoption.

Pre-remediation behavior:
The guard compares server `name` with `/metadata/slug`. A valid declaration
whose name and slug differ can bypass the guard and reach the create seal.

Impact:
The CLI can create a duplicate/misleading Workflow beside the intended legacy
row, defeating the explicit-adoption integrity boundary. Post-write marker
verification cannot discover the unmarked legacy row because it verifies only
the newly persisted reserved identity.

Likelihood:
Medium to high in legacy/UI-authored estates: human-readable display names and
machine slugs commonly differ. Exploitation requires an authorized apply
principal, but no malformed input or race is required.

Required remediation:
Derive the exact authored Workflow display name from the validated declaration,
pass it independently from `slug` through both exhaustive scopes, and use it
only for the unmarked nonselecting adoption guard. Add a regression fixture in
which `spec.name != metadata.slug`; the matching unmarked row must return the
approved exit-1 adoption error with zero POST/PUT/PATCH/DELETE. Preserve the
explicit-adoption rule that another same-display-name unmarked row does not
veto the supplied by-ID candidate.

Owner: implementation-engineer

Verification:
- Ordinary apply, differing name/slug, matching unmarked row: adoption-required
  and exact zero modifying calls.
- Ordinary apply, differing name/slug, no matching unmarked row: ordinary
  create remains eligible after all other evidence passes.
- Explicit adoption with another unmarked same-display-name row: candidate-only
  semantics remain unchanged.
- Both project and marketplace scopes exercise the corrected guard.

References:
- FR-030, AC-FR-030-02, ADR-008, W-001, R-001

Final implementation resolution evidence:
- `workflow::apply` now obtains the non-empty authored display name from
  `/spec/name` after declaration parsing/validation and passes it independently
  of the slug to `enumerate_all` (`src/adapters/workflow.rs:158-174`).
- Both project-visible and marketplace passes forward that value, and the
  unmarked-row guard compares `item.name` with the authored display name
  (`src/adapters/workflow.rs:448-470,544-555`). Post-write identity verification
  passes no display-name guard, so it remains marker-only.
- `unmarked_authored_display_name_blocks_create_when_name_differs_from_slug`
  passes for both scopes and asserts exit 1 plus exact zero POST, PUT, PATCH,
  and DELETE calls. `unmarked_same_display_name_row_does_not_veto_explicit_adoption`
  passes for both scopes and proves explicit by-ID adoption still updates only
  the selected strict unmarked candidate.

## Security regression expectations

The following controls remain required regression coverage for the reviewed
boundary:

1. Assert `apply` makes zero requests to `/v1/info`. Independently serve SHA,
   `0.16.0`, another value, missing/invalid JSON, 404, 5xx, and timeout behavior;
   none may override valid or invalid operation evidence.
2. For each entity and applicable create/update/adoption/reference path, corrupt
   one required consumed field or container at a time and assert
   `E_API_INCOMPATIBLE`, exit 2, empty stdout, safe stderr, and exact zero counts
   across every POST/PUT/DELETE route.
3. Exercise the exact-project capability matrix in SEC-Q007-001 for Workflow,
   Skill, and Datasource. Prove Assistant makes no `/v1/user` call and still
   requires strict direct-lookup evidence before its prepared write is sealed.
4. For every paginated resolver, test missing/wrong-type page members, wrong
   page origin, changing `per_page/total/pages`, early truncation, repeated page
   or ID, cycles, and page/item budget exhaustion. Each pre-write case must fail
   with zero modification.
5. Test row-level write evidence independently: absent, wrong-type, or
   non-writing `user_abilities` cannot update; extra unconsumed row fields do not
   widen requests.
6. Capture outbound requests for the positive additive-field fixtures and prove
   no new declaration or request property is introduced.

## Validation performed

Commands run from the repository root:

```text
cargo test --locked adapters:: -- --nocapture
cargo test --locked adapters::workflow::tests -- --nocapture
cargo test --locked http::tests::preflight_visibility -- --nocapture
cargo test --locked coordinator::tests -- --nocapture
cargo test --locked --all-targets
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_o001_*.py'
git diff --check
git diff --name-only -- codemie codemie-ui
rg -n '/v1/info|EXPECTED_BACKEND_COMMIT|check_compatibility|preflight' src --glob '*.rs'
rg -n 'pub(\(crate\))? async fn (post|put|post_or_conflict|post_multipart|put_multipart)|dispatch_prepared' \
  src/http/mod.rs src/adapters --glob '*.rs'
```

Observed results:
- Adapter-focused tests: 66 passed, 0 failed; the Workflow-only subset passed
  23 tests, including both SEC-Q007-007 regressions.
- Visibility-focused tests: 9 passed, 0 failed.
- Coordinator-focused tests: 11 passed, 0 failed, including page-0 post-write
  resolution and zero-`/v1/info`/zero-second-write fault classification.
- Full Rust suite: 343 unit tests and 9 CLI integration tests passed; 0 failed.
- O-001 control suite: 16 passed, 0 failed. Formatting, Clippy with warnings
  denied, and `git diff --check` also passed.
- Production Rust contains no `/v1/info`, `EXPECTED_BACKEND_COMMIT`, or
  `check_compatibility` runtime gate. The only `/v1/info` occurrences are one
  non-gating comment and coordinator fixtures that require zero contacts.
- The manifest conformance unit test passes for `infoEndpointIsIdentity=false`,
  the additive-unconsumed policy, exact Workflow/Datasource/Skill
  applicability, exact-project matching, and page-0 origins.
- Strict DTO tests pass for every currently represented consumed response
  member. Exact-project negative tests and row-level non-writing-ability tests
  assert zero modification for Workflow, Skill, and Datasource; Assistant tests
  assert no admin preflight and zero modification on invalid/non-writing direct
  lookup evidence.
- Pagination validators require the zero-based origin, page size, count
  formula, stable page/total/count fingerprints, advertised-total completeness,
  unique IDs per scan, and page/item budgets. Page-origin/shape matrices,
  multi-page page-0 traversal, and repeated-ID zero-write probes pass; compatible
  snapshot churn is correctly classified as reconciliation rather than
  compatibility failure.
- Diagnostic mapping discards all `AppError` message strings and renders only
  closed codes/categories/exits; the coordinator fault test confirms empty
  stdout and non-empty closed stderr.
- Static write-entry search confirms raw CodeMie modifiers are private to the
  HTTP module and only `dispatch_prepared` is exposed to production callers.
  Every adapter dispatches through that consuming boundary.
- Workflow adoption and marker tests now prove both-scope zero-marker evidence,
  the strict unmarked candidate gate, recursive duplicate rejection, four-state
  classification, the authored-display-name guard, and zero modifying calls on
  each exercised fault.
- `git diff --name-only -- codemie codemie-ui` is empty: the reference-only
  trees were not modified.
- An initial unprivileged Workflow test run could not bind Mockito's loopback
  socket (`Operation not permitted`); the same command passed after granting
  local test-server permission. An interim formatting check observed concurrent
  test edits; the stable-snapshot `cargo fmt --all -- --check` rerun passed.

## Remediation plan

| Finding | Owner | Required action | Verification | Status |
|---|---|---|---|---|
| SEC-Q007-001 | implementation-engineer | exact-project, all-applicable-entity capability proof; strict DTO | project matrix and zero-write tests | RESOLVED |
| SEC-Q007-002 | implementation-engineer | make actual completed read evidence mandatory and close raw transport write reachability | boundary test plus adapter fault matrix | RESOLVED |
| SEC-Q007-003 | solution-architect / implementation-engineer | preserve additive-unconsumed-only policy | manifest validation and paired fixtures | RESOLVED |
| SEC-Q007-004 | product-spec-owner / solution-architect / implementation-engineer | retain Assistant least privilege and strict direct evidence | no-admin and invalid-lookup zero-write tests | RESOLVED |
| SEC-Q007-005 | implementation-engineer | complete adoption identity scan and reject marked candidates before seal | adoption negative matrix with zero writes | RESOLVED |
| SEC-Q007-006 | implementation-engineer | strictly classify malformed/conflicting Workflow markers | marker fault matrix with zero writes | RESOLVED |
| SEC-Q007-007 | implementation-engineer | compare unmarked server display name with authored `spec.name`, independently of slug | differing-name/slug, both-scope zero-write guard matrix | RESOLVED |

## Accepted risks and unverified areas

- No risk was accepted by this reviewer.
- Runtime consumed-subset conformance is not source attestation; this is a
  documented residual design limitation requiring V-000/release qualification.
- Actual deployment compatibility, server authorization behavior, and build
  provenance were not verified in this local implementation review.
- Not every consumed-field and page-budget permutation is exercised through the
  top-level coordinator. Strict DTO/invariant unit matrices, adapter-level
  zero-write probes, and the evidence-bearing modifying boundary provide the
  security evidence for this approval; independent convergence verification
  still owns exhaustive acceptance-test completeness.

## Next action

The verification engineer should perform final independent convergence
verification on this stable snapshot, including the resolved SEC-Q007-007
regressions. Release readiness assessment may follow only if verification also
passes. This approval is for the next lifecycle stage and does not authorize
release or deployment.
