# ADR-013: Qualify exact personal-project ownership as complete visibility

## Status

Superseded by product specification v32 and ADR-014 through ADR-018.

This proposed v31 design was never accepted. Its administrator/personal-owner
branches, project-detail dependency, exclusive-member test, and
`E_VISIBILITY_UNPROVEN` creation gate MUST NOT be implemented or used as
qualification evidence. Historical text below is retained only as decision
history.

## Context

ADR-012's accepted admin predicate remains safe for shared projects, but the
pinned `2.42.0` server deliberately creates a personal-project owner with
`is_project_admin=false`. Product specification v31 FR-037 approves one narrow
alternate proof for Workflow, Skill, and Datasource. Assistant least privilege
and operation-specific write authorization are unchanged.

Pinned reference-only evidence shows that personal-project creation stores
`project_type=personal`, `created_by=user_id`, and a non-admin owner membership;
exact project detail is visibility-filtered and exposes the fields needed to
bind project, owner, and current member.

## Decision drivers

- Admit the server-defined personal owner without widening shared-project access.
- Preserve complete identity visibility before client-side exhaustive resolution.
- Keep malformed contract evidence distinct from valid authorization mismatch.
- Make zero writes reachable from every failed proof branch.
- Preserve Assistant's direct path and separate existing-target write checks.

## Options considered

1. Keep admin-only qualification. Rejected: contradicts approved FR-037 and the
   pinned personal-project model.
2. Infer ownership from email/project name or creator fields elsewhere.
   Rejected: weak binding and explicitly prohibited by v31.
3. Use strict same-session `/v1/user` plus visibility-filtered exact project
   detail. Selected: supplies conjunctive actor, project, type, owner, and
   membership evidence without a write or server change.

## Decision

For Workflow, Skill, and Datasource only:

1. Strictly decode same-session `GET /v1/user`: non-empty string `user_id`,
   booleans `is_admin`/`is_maintainer`, array `projects`, and every entry's
   non-empty string `name` plus boolean `is_project_admin`. Reject duplicate
   keys in every consumed JSON object before DTO conversion.
2. Preserve ADR-012's global admin/maintainer or exact-project admin predicate.
3. Only if it fails, require exactly one membership with name exactly equal to
   the effective project, then GET
   `/v1/projects/{percent-encoded-effective-project}` with the same session.
4. Strictly decode non-empty strings `name`, `project_type`, `created_by`, array
   `members`, and every member's non-empty string `user_id` plus boolean
   `is_project_admin`.
5. Qualify only when name is exact, type is `personal`, creator equals the
   authenticated `user_id`, and `members` has exactly one total entry whose
   `user_id` equals the authenticated user.
   `is_project_admin=false` is accepted and never converted to true.
6. One opaque invocation-scoped `ApiClient` capability owns the validated
   origin, bearer token, and session for qualification, exhaustive resolution,
   write ability, final visibility revalidation, sealing, and dispatch.
7. Proceed to exhaustive resolution, independently prove operation-specific
   write ability, revalidate visibility with that capability, and seal a
   project/kind/client-bound `PreparedWrite` only after all evidence passes.

The detail URL appends the encoded effective project as exactly one structural
path segment. `/`, `%2f`, `%252f`, `?`, `#`, space, Unicode, `.`, and `..` must
not alter origin, base path, query, fragment, or segment count.

Missing, null, empty, wrong-type, or duplicate-key consumed fields are
`E_API_INCOMPATIBLE`, exit 2. Well-typed absent, duplicate, inaccessible, or
cardinality/mismatched proof—including empty, multiple, duplicate-owner, or
sole-mismatched members—is `E_VISIBILITY_UNPROVEN`, exit 2. Both leave stdout empty,
emit only safe diagnostics, and issue zero modifying requests. Additive
unconsumed fields remain compatible.

## Consequences

### Positive

- Exact non-admin personal owners can safely use exhaustive resolvers.
- Shared/global rules and the write boundary remain unchanged.
- The decision is source-pinned, testable, and requires no server change.

### Negative

- Adds one conditional GET and strict decoder to the non-admin W/S/D path.
- Existing admin-only implementation and live evidence must be refreshed.

### Risks

- Session mixing or normalization would weaken actor/project binding; typed
  session-bound evidence and exact equality are required.
- Treating ownership as write permission would widen authorization; the
  `PreparedWrite` seal carries separate visibility and write evidence.

## Follow-up actions

- Q-009: independent pre-implementation architecture/security-boundary review.
- T-004: Rust preflight/PreparedWrite implementation and fake-server matrix.
- V-000A: Python GET-only harness and mutation matrix refresh.
- O-002A: guidance/checker refresh; V-000B and V-003 fresh live reset.

## References

- Product specification v31: SC-022, FR-033/037, DR-013, IR-013, PA-005/008,
  VR-017, AC-FR-037-01/02/03.
- ADR-012 (amended, not superseded); ADR-007/008/009.
- `contracts/http-adapter.md`; `contracts/adapter-manifest-v2.42.0.json`.
