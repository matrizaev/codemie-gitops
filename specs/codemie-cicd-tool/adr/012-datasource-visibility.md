# ADR-012: Datasource complete-visibility boundary (options analysis)

## Status

Accepted — Option A selected, 2026-08-10

**Platform owner prerequisite resolved from source evidence.** The critical
prerequisite (which role grants complete `GET /v1/index` visibility) is
answered by `index_service.py:276-282`:

```python
return or_(
    and_(IndexInfo.project_name.in_(user.project_names), IndexInfo.project_space_visible),
    IndexInfo.project_name.in_(user.admin_project_names),   # no project_space_visible guard
    IndexInfo.created_by['id'].astext == user.id,
)
```

`admin_project_names` carries no `project_space_visible` filter, so a
**project-admin** sees every Datasource row in their project. Global-admin
(`user.is_admin`) bypasses the filter entirely but is not required. Option A
is viable at project-admin scope.

D-001 implementation is authorized once all A-001/W-001/S-001 peers are
complete (see tasks.md D-001 dependencies).

**Security clarification (2026-08-11, v28):** project-admin evidence is scoped,
not global. The accepted predicate is global admin, global maintainer, or one
`projects[]` entry whose `name` equals the declaration's exact effective project
and whose `is_project_admin` value is true. An admin entry for another project
does not authorize visibility for the target project. This predicate applies to
Datasource, Workflow, and Skill pre-write resolution. Assistant is excluded:
its source-pinned exact `(project, slug)` lookup follows PA-003 least privilege
and does not require complete-list visibility proof.

## Context

The pre-implementation security review (SEC-004) found that the Datasource
exhaustive resolver cannot prove complete identity visibility for a regular
project member. The current ADR-009 decision that "one exact list match
proves visibility/write capability" holds for the principal's own visible set,
but that visible set may be incomplete.

**Pinned server evidence** (read-only reference,
`codemie/src/codemie/service/index/index_service.py:162-218,275-282`):

- `GET /v1/index` applies a visibility filter that returns rows visible to the
  requesting principal: project-visible, project-admin-visible, and
  creator-owned rows
- A regular project member may receive a consistent, complete pagination
  envelope over an incomplete identity set (hidden rows exist outside their
  visibility scope)
- No database uniqueness constraint exists on `(project, repo_name, kind)`;
  `find_id` returns the first matching row
- No atomic natural-key upsert or uniqueness-enforcement endpoint exists at
  the pinned version

**Comparison with Workflow/Skill**:
- Workflow and Skill: FR-033 requires privileged visibility scope; the HTTP
  adapter preflight (`GET /v1/user`) verifies global-admin or project-admin
  access before resolution
- Datasource: no equivalent completeness-proof precondition; exhaustive
  resolution proceeds without visibility verification

**Consequence**: A CLI that does not hold a privileged Datasource view may
create a duplicate (the hidden row exists but is invisible) or update one
visible row while a hidden duplicate exists. Later privileged resolution
becomes ambiguous and requires manual repair.

## Decision drivers

- Zero/one/multiple natural-key decisions must operate over a demonstrably
  complete identity set, or the server must enforce/resolve uniqueness atomically
- The CLI must not create silent duplicate or ambiguous identity state
- Server changes outside the approved product scope are a high-cost option
- Operational requirements must be proportional to realistic service-account
  permission grants

## Options considered

### Option A — Require a role that makes Datasource enumeration complete

**Description**: Before any Datasource apply, verify that the invoking
principal holds global-admin, global-maintainer, or project-admin status via
`GET /v1/user`, the same preflight used for Workflow and Skill. Project-admin
status qualifies only when the same response entry names the exact effective
project. If the valid response fails that predicate, exit
`E_VISIBILITY_UNPROVEN`, exit 2, before any Datasource write; missing or invalid
consumed response fields are `E_API_INCOMPATIBLE`, exit 2, before write.

**Server evidence gap**: The pinned source (`index_service.py:162-218`) shows
that `is_project_admin` grants access to project-visible rows plus project-admin
rows. It is not confirmed whether this scope is equivalent to "all rows for
that project." The platform owner must confirm whether project-admin visibility
covers all Datasource rows in a project, or whether global-admin/maintainer is
required for complete visibility.

**Advantages**:
- Mechanically consistent with Workflow/Skill: same preflight, same exit code
- No server changes required
- Complete visibility makes zero/one/multiple decisions reliable

**Disadvantages**:
- Requires CI service accounts to hold project-admin or global-admin role
- Breaks non-admin Datasource automation that currently relies on creator-scoped
  writes
- Server visibility scope needs confirmation from platform owner

**Evidence needed for acceptance**:
1. Platform owner confirms which role makes `GET /v1/index` return all project
   Datasource rows (project-admin, global-admin, or higher)
2. Adopting teams accept the role requirement for CI service accounts

---

### Option B — Add or use an atomic server natural-key contract

**Description**: The CodeMie server adds a unique database constraint on
`(project, repo_name, kind)` and exposes an atomic upsert or natural-key
conflict endpoint. The CLI uses this server guarantee instead of client-side
completeness proof.

**Server evidence**: No such constraint or endpoint exists in the pinned
version. This option requires a server-side change.

**Advantages**:
- Permanent race-safe uniqueness at the database level
- Simpler client code: no completeness-proof preflight needed
- Eliminates the hidden-duplicate problem permanently

**Disadvantages**:
- Requires server changes outside the approved product scope
- Migration path needed for existing duplicate rows in production
- Timeline depends on the CodeMie platform team's roadmap

**Evidence needed for acceptance**:
1. Server change approved by the CodeMie platform team
2. Migration plan for existing duplicate rows
3. Compatible delivery timeline with the CLI product

---

### Option C — Fail Datasource apply when completeness cannot be proven

**Description**: The same preflight check as Option A, but with an explicit
fail-closed default: if the current principal's visibility scope cannot be
proven complete (non-admin), the Datasource apply is refused with
`E_VISIBILITY_UNPROVEN` rather than proceeding under uncertainty. This is
the conservative form of Option A.

Options A and C converge in practice when the visibility check is implemented
correctly. The distinction matters in edge cases where `GET /v1/user` returns
role information that cannot be definitively mapped to complete Datasource
visibility.

**Advantages**:
- Most conservative; never creates duplicates under any uncertainty
- No server changes required
- Clear operator error message explains the required role

**Disadvantages**:
- Same operational requirements as Option A
- Breaks existing non-admin automation

**Evidence needed for acceptance**:
1. Same as Option A
2. Adopting teams accept fail-closed behavior when role is absent

---

### Current behavior — status quo, not a candidate

ADR-009 proceeds with exhaustive enumeration without a completeness-proof
precondition. This is the behavior that SEC-004 found insufficient. It is
documented here as context only; it is not a proposed resolution option.

## Recommendation

**Option A** is recommended as the least-disruptive option that does not
require server changes. It aligns Datasource with the established
Workflow/Skill model, uses the existing `GET /v1/user` preflight, and is
operationally transparent.

**Critical prerequisite before implementing Option A**: The CodeMie platform
owner must confirm which role grants complete `GET /v1/index` visibility for
a project. If `is_project_admin` is insufficient (i.e., global-admin is
required), Option A raises the operational bar significantly and Option B
(server uniqueness enforcement) should be reconsidered.

## Deferred decision

The product-spec-owner and CodeMie platform owner must review and approve one
of Options A, B, or C before D-001 implementation begins.

Until this ADR is accepted:

- ADR-009 exhaustive resolver remains the normative model for architecture
  documentation purposes
- **D-001 implementation is not authorized**
- Datasource must be added to serialized-writer governance, duplicate
  inventory, and post-write ambiguity runbooks alongside Workflow and Skill
  (O-001/O-002 scope)

## Consequences if Option A is accepted

### Positive

- Datasource zero/one/multiple decisions are provably complete
- Consistent privilege model across all three resolved-identity entity kinds
- No server changes required

### Negative

- CI service accounts need project-admin or global-admin role for Datasource
  operations
- Existing non-admin Datasource automation requires re-authorization before
  migrating to this CLI

### Risks

- Platform owner may confirm that even project-admin is insufficient; if so,
  global-admin is required and adoption cost rises substantially
- The `GET /v1/user` preflight creates a read-API dependency before every
  Datasource apply

## Follow-up actions

1. Platform owner: confirm which role makes `GET /v1/index` complete for a
   project and whether `is_project_admin` is sufficient
2. Product-spec-owner: select one option and authorize D-001 implementation
3. Architecture: update ADR-009 with the accepted visibility precondition
4. Tasks: expand D-001 and O-001/O-002 acceptance evidence to include
   Datasource visibility role requirements and duplicate-governance drill

## References

- SEC-004 finding
- ADR-009 (extends; not superseded until this ADR is accepted)
- Product specification v28: FR-005/033/036, IR-011/012, PA-003/005,
  QR-009-011
- Pinned server: `codemie/src/codemie/service/index/index_service.py:162-218,275-282`
- Tasks: D-001, O-001, O-002
