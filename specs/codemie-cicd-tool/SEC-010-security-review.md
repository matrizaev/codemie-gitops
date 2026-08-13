# Security review report: v32.2 member creation and creator-scoped reconciliation

## Status

```text
Security review status: APPROVED FOR NEXT STAGE
Threat-model status: ACCEPTABLE WITH REQUIRED CONTROLS
```

This approval permits progression to release review for the bounded T-005
implementation. It does not approve a release, deployment, live mutation, or
migration of v30/v31 evidence.

## Review mode

- Implementation security review (final post-implementation SEC-010 retest),
  retaining the architecture threat model below.

## Scope

- Feature: v32.2 member-qualified creation, exact-row mutation authorization,
  and creator-scoped Workflow and Skill reconciliation.
- Version: repository `f850a76b514db14dd75e2423e193e873f3810f41` plus the
  working-tree v32.2 specification, implementation, and refreshed artifacts reviewed
  on 2026-08-12.
- Components: product requirements, plan, data model, Q-010, task gates,
  HTTP/declaration/diagnostic/manifest contracts, ADR-013 through ADR-018, and
  the pinned backend source baseline.
- Actors: authenticated project member, project/global administrator, another
  project member, same-principal concurrent writer, governed external writer,
  compromised or misconfigured target, and a caller attempting to substitute
  identifiers, credentials, origin, or response evidence.
- Assets: entity integrity, object-level authorization, tenant/project
  isolation, creator namespaces, bearer credentials, server IDs, request and
  response bodies, and diagnostic privacy.
- Trust boundaries: CLI input to validated domain types; authenticated
  `GET /v1/user` to membership/creator evidence; visible lists/details to
  identity and ability evidence; sealed evidence to the modifying dispatcher;
  server collision/write result to post-write verification and diagnostics.
- Explicit exclusions: live/network behavior, deployment configuration,
  release authorization, `.env`, and modifications to
  the reference-only `codemie/` and `codemie-ui/` trees.

## Sources of truth

- Specification: `specs/codemie-cicd-tool.md`, stable v32.2,
  notably FR-028 through FR-034, FR-037, DR-007/013, IR-013, PA-005/008,
  VR-011/017, and their acceptance criteria.
- Architecture: `plan.md`, `data-model.md`, ADR-014 through ADR-018.
- Superseded decision: ADR-013, whose status explicitly prohibits the v31
  administrator/personal-owner/project-detail gate.
- Contracts: `contracts/http-adapter.md`,
  `contracts/adapter-manifest-v2.42.0.json`, declaration and diagnostic
  contracts, and `contracts/source-baseline.md`.
- Independent evidence: `Q-010-verification-report.md` and
  `Q-010-postimplementation-verification.md`, the latter VERIFIED — READY FOR
  RELEASE REVIEW with Q010-POST-001 and Q010-POST-002 closed.
- Pinned reference-only evidence: backend tag 2.42.0 at
  `2a481c290c99bf30ef80aadafa03d876a7f5f732` and UI tag 2.42.0 at
  `55945d075d82e771c4a2f4238afec1eb4c79d1e1`.
- Jira/Confluence: none supplied.

## Executive assessment

The v32.2 authorization boundary is implemented and suitable for release
review. Exact project
membership is a create qualification, not an inferred object-write privilege;
every existing-row update and Workflow adoption separately requires the exact
server-returned `write` ability. Workflow and Skill selection is bound to the
authenticated `user_id`, foreign creators cannot become targets or leaked
references, and Assistant remains on its exact direct lookup rather than the
creator-enumeration paths. Datasource list absence is treated as partial
visibility: it permits at most one create, while HTTP 409 terminates without a
guessed read, retry, or update.

The architecture closes the earlier v31 privilege model rather than preserving
an exception: role flags, ownership, project type, project-detail membership
cardinality, and administration cannot qualify or deny creation. ADR-013 and
all v31 implementation/evidence are explicitly superseded.

No open critical, high, or medium implementation finding was identified. The absence of
atomic Workflow-marker uniqueness is a documented residual race, not an
authorization bypass: writer serialization reduces likelihood and mandatory
post-write re-resolution exposes ambiguity without delete, rollback, or a
second mutation. The controls below are mandatory T-005 acceptance gates.

## Threat model and required controls

| Threat | Boundary and impact | Required preventive/detective control | Assessment |
|---|---|---|---|
| Membership or creator spoofing | Auth response to authorization; cross-user or cross-project write | Strictly decode non-empty `/v1/user.user_id`, the projects array, and every consumed non-empty project name; require an exact effective-project membership; derive creator only from this response; prohibit authored/email/role/row-owner inference | Adequately specified |
| Membership treated as update authority | Resolution to PUT/adoption; unauthorized mutation | Require exact string `write` in the selected row's strictly decoded `user_abilities`; creator, membership, visibility, role, detail success, and other strings never substitute | Adequately specified |
| Hidden Workflow collision or legacy takeover | Visible-list boundary; cross-user overwrite | Match only exact same-principal v2 `{version:2, project, creator_user_id, slug}` records. V1/unmarked records never ordinary-match. Adoption requires explicit canonical UUID, same project and creator, zero existing v2 match, exact `write`, mergeable metadata, and one PUT | Adequately specified |
| Hidden/foreign Skill selection or ID disclosure | Skill list/reference boundary; cross-user write or information disclosure | Filter exact `(project, authenticated_user_id, name)` after strict creator decoding. Exclude foreign rows from target ambiguity. Inline references use the same scope, are read-only, return only the current creator's invocation-local ID, and never require or perform a Skill write | Adequately specified |
| Same-creator duplicate selection | List ordering to target selection; arbitrary write | Zero/one/multiple matrices are mandatory; multiple exact same-creator rows fail with zero mutations. Never select first/newest/list-order | Adequately specified |
| Datasource hidden-row takeover | Partial list to mutation; guessed object write | Visible exact one may update only with `write`; visible ambiguity fails. A miss permits exactly one create. HTTP 409 is terminal exit 1 with no retry, `find_id`, guessed GET, PUT, or body disclosure | Adequately specified; target qualification must verify 409 semantics |
| Capability swapping or replay across evidence | Authentication/read-to-write boundary; confused deputy | One opaque, non-serializable invocation capability owns validated scheme/host/effective port, bearer token, principal/session, project, kind, identity, and operation. `PreparedWrite` carries it and dispatch accepts no separate client, URL, token, or session | Adequately specified |
| Malformed or additive response changes authorization | JSON boundary; fail-open or parser differential | Detect duplicate keys before DTO conversion in every consumed object; reject missing/null/empty/wrong-type consumed evidence. Tolerate only unique additive unconsumed members, which cannot satisfy evidence or enter requests | Adequately specified |
| Project/ID structural injection | URL/request boundary; route or origin substitution | Build route and query values with the validated URL encoder; any path parameter is one encoded segment. Server IDs must be canonical typed UUIDs obtained from validated responses or the explicit Workflow adoption selector; creator IDs never enter route construction by string concatenation | Adequately specified |
| Time-of-check/write race | Non-atomic read/write boundary; duplicate or uncertain state | Complete all applicable reads and ability proof before the final seal; same capability performs write and bounded post-write exact re-resolution. Serialize same-principal writers. Ambiguity/uncertainty fails visibly, reports possible commit, and performs no rollback/delete/retry | Residual risk documented and bounded |
| Diagnostic exfiltration | Target response to stdout/stderr/logs; token, PII, server-ID or payload leak | Failure stdout is empty. Synthesize diagnostics only from the closed allowlist; omit request/response bodies, arbitrary headers, URLs/origins, tokens, credentials, creator IDs, server IDs, declaration values, and exception dumps. Accept correlation headers only under VR-011 | Adequately specified |
| Assistant privilege widening | Shared coordinator to Assistant adapter | Assistant still requires exact membership, but uses only direct exact `(project,slug)` lookup; it does not use creator-list, project-detail, admin, personal-owner, Workflow adoption, or Skill/Datasource collision logic. Existing Assistant update still requires exact `write` | Adequately specified |
| Reintroduction of v31 exception | Historical artifacts to implementation; privilege inconsistency | T-005 must remove project-detail/admin/personal-owner gates and reject all v31 qualification and marker evidence. ADR-013 is decision history only | Adequately specified |

## Authorization and zero-write matrix

| Operation | Required evidence | Failure mutation count |
|---|---|---|
| Any create | Exact membership, completed kind-specific absence/miss resolution, same sealed capability | Zero before seal; at most one POST after seal |
| Any existing-row update | Create evidence plus one exact target and exact `write` | Zero when ability is absent/malformed or identity is ambiguous |
| Workflow adoption | Explicit UUID, same project/current creator, v1 or unmarked, zero current-creator v2 match, mergeable metadata, exact `write` | Zero on every failed predicate; exactly one PUT after seal |
| Workflow inline Skill reference | Exact membership and one current-creator Skill under the same capability | Zero Skill mutations in all outcomes |
| Datasource miss/collision | Exact membership and visible miss; one POST maximum | After 409: no further mutation or guessed lookup |

## Final implementation reassessment

All ten mandatory T-005 verification controls below are now represented by
implementation and executable negative/request-count evidence. In particular:

- `PreparedWrite` privately owns the same `ApiClient` reference, authorization
  proof, resolution, request plan, and (for File Datasource) multipart bytes;
  modifying dispatch accepts only that aggregate.
- Skill target and inline-reference creator matrices cover foreign-only,
  foreign-plus-current, current-only, and duplicate-current cases. They prove
  exact selected IDs/actions and zero unintended POST/PUT/PATCH/DELETE.
- Stable v32.2 Skill 409 handling sends one POST, then exactly one bounded
  page-0-origin read-only same-creator re-resolution. Zero, one, multiple, and
  incompatible results are terminal for that invocation; there is no second
  POST or conversion to PUT.
- JSON and File multipart Datasource collision tests prove exactly one POST,
  body discard, no multipart replay, and no PUT/PATCH/DELETE.
- Exact membership, exact case-sensitive `write`, strict duplicate-aware
  decoding, Workflow v2/adoption, Assistant direct lookup, post-write
  uncertainty, and safe diagnostic boundaries have negative tests and passed
  the independently coordinated suite.

## Findings

No open security findings. No risk was accepted on behalf of the user.

### Residual risk SEC010-R01 — Workflow same-principal race

- Classification: NOTE; documented architectural limitation.
- Impact: concurrent creates/adoptions can leave duplicate v2 markers or an
  uncertain committed write because the pinned API offers no uniqueness or
  conditional-write primitive.
- Controls: protected-environment serialization, external-writer freeze for
  adoption, exactly one modifying request, post-write exact re-resolution, no
  automatic rollback/delete, and manual duplicate remediation.
- Owner: implementation engineer for enforcement/tests; release engineer for
  operational evidence. A future server-side uniqueness/conditional-write
  control would further reduce residual risk but does not block release review.

## Required T-005 verification — SATISFIED

The implementation provides negative fake-server tests proving:

1. No membership, malformed/duplicate membership JSON, identity ambiguity,
   missing exact `write`, creator mismatch, capability/origin/token/session
   substitution, or failed final seal produces POST/PUT.
2. `write`, `Write`, creator ownership, administrator role, and successful
   detail/list responses are not interchangeable authorization evidence.
3. Workflow v2, v1/unmarked adoption, foreign-creator, same-creator duplicate,
   and concurrent/post-write ambiguity branches have exact request counts.
4. The complete inline `skillRefs` matrix emits no foreign creator ID and never
   mutates a Skill.
5. Datasource 409 performs one POST total and no retry, fallback lookup, or PUT.
6. Duplicate keys fail before DTO conversion while unique additive unconsumed
   fields succeed without widening authorization or request projection.
7. Project/slug/name/UUID edge inputs cannot change origin, base path, segment
   count, query, fragment, principal, or selected entity.
8. Failure snapshots and tracing contain no credential, token, authorization or
   cookie value, origin/URL, request/response body, server text, creator/server
   ID, declaration value, or exception dump.
9. Assistant uses no project-detail/admin/creator-list/adoption path, yet still
   requires membership for create and exact row `write` for update.
10. Post-write zero/multiple/mismatched-ID results report possible commit and
    cause no compensating mutation.

Independent post-implementation verification exercised these paths and found
no open blocking, high, or medium finding.

## Validation performed

- `git rev-parse HEAD`; `git -C codemie rev-parse HEAD`; and
  `git -C codemie-ui rev-parse HEAD` — confirmed the reviewed repository and
  both pinned reference revisions.
- Focused `sed`, `rg`, `find`, and `jq` inspection of the specification,
  refreshed architecture/contracts/tasks, Q-010, and pinned source — confirmed
  membership response shape, creator/ability fields, Skill creator uniqueness,
  Workflow own-row visibility, and source 409 paths without modifying source.
- `git diff --check` — passed before this report was added.
- Independently coordinated gates recorded in
  `Q-010-postimplementation-verification.md`: `cargo test --locked` passed 348
  unit plus 9 integration tests; Python V-000/O-002 passed 49/O-002; focused
  `tests.test_v000_target` passed 27; `make format`, `make lint`, and
  `git diff --check` passed. The verifier also transparently records a later
  mock-server rerun blocked by sandbox socket permission, with no product
  assertion failure.
- No network, live API, `.env`, mutation test, implementation edit, reference
  edit, or commit was performed.

## Unverified areas

- No live target was queried, so deployment preservation of Datasource 409,
  response shapes, abilities, and pagination remains a later qualification
  gate.
- Protected CI serialization and external-writer freeze require fresh v32
  operational evidence; prior v31 evidence cannot be relabeled.

## Next action

The release engineer may assess the bounded v32.2 T-005 implementation while
preserving V-000C, fresh live V-000B/V-003, protected-writer serialization, and
explicit deployment authorization as later blocking gates.
