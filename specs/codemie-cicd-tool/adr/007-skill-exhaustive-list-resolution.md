# ADR-007: Resolve Skill identity through exhaustive list filtering

## Status

Accepted only for zero-based exhaustive scanning mechanics; identity and
visibility decisions are superseded by ADR-017.

Amended 2026-08-11 for the pinned Skill pagination origin: page numbering is
zero-based. The router accepts `page >= 0` with default `0`, the service and
repository default to `0`, and repository offsets are `page * per_page`.

## Context

The approved identity is `(project, name)`. The inspected server's persisted ID
is server-generated, list/detail/update are ID-oriented, and uniqueness is
creator-scoped: `(name, created_by.id, project)`. Multiple creators can
therefore expose duplicate `(project,name)` rows. Caller-supplied Skill IDs are
not an evidenced create contract, so the superseded Workflow UUID approach does
not apply.

Product specification v24 formally accepts exhaustive client resolution,
ambiguity failure, complete privileged visibility, serialized CI, UI governance,
and manual duplicate remediation. It does not claim server-enforced global
uniqueness.

## Decision drivers

- Keep authored/reported identity natural and UUID-free
- Use the current server without changes
- Never guess among duplicates
- Preserve existing uniquely resolved Skills in place
- Make visibility and race assumptions explicit

## Options considered

### A. First/newest/current-principal match

Rejected: order and creator are not the approved identity.

### B. Deterministic caller-supplied UUID

Rejected: the current Skill create path does not accept/preserve caller ID as a
supported contract and does not solve adoption of existing rows.

### C. Server uniqueness/by-natural-key endpoint

Architecturally strongest but requires a server change and is not required for
phase 1.

### D. Exhaustive list plus exact client filter

Selected and product-approved.

## Decision

For exact `(project,name)`:

1. ADR-015 exact project membership qualifies creation. Identity is exact
   `(project,authenticated_user_id,name)` per ADR-017; update separately
   requires exact `write` on the selected row.
2. Enumerate every page of `GET /v1/skills` with `per_page=100`, starting at
   `page=0`, using exact `project`, `project_with_marketplace`, and `search`
   hints where compatible. Always request page 0 once. If the response reports
   `pages > 0`, request exactly pages `0..pages-1`; if it reports `pages == 0`,
   stop after the empty page-0 response. Hints do not replace client filtering.
3. Detect pagination cycles, repeated row IDs, changing totals/cursors, or
   inconsistent snapshots. Each response must echo the requested page, return
   `perPage=100`, and satisfy `pages=ceil(total/perPage)` with `pages==0` iff
   `total==0`. An invalid origin, page echo, page size, page-count formula, or
   request sequence is `E_API_INCOMPATIBLE`, exit 2 before write. Across
   individually compatible responses, the `(pages,total,perPage)` fingerprint
   must remain stable and the accumulated unique item count must equal `total`;
   churn, repeated IDs, or totals that change during the scan remain entity-
   resolution instability, exit 1 before write.
4. Client-filter by exact project, authenticated creator ID, and exact name;
   other creators are excluded rather than ambiguity.
5. Zero matches: POST once. One: prove write ability, GET any detail required by
   the pinned request mapping, and PUT by returned ID on every valid apply. More than one:
   `E_AMBIGUOUS_IDENTITY`, exit 1, no write.
6. A create 409 triggers exactly one exhaustive page-0-origin same-creator
   read-only scan. No second POST or PUT/PATCH/DELETE is permitted. One exact
   collision is `ServerRejected` exit 1; multiple are ambiguity exit 1; stable
   zero is reconciliation instability exit 1; compatibility/connectivity
   failure is exit 2.
7. After create/update, one bounded full re-resolution must find exactly one
   identity associated with the expected route ID. Uncertain/duplicate result is
   reported without delete or rollback.

Returned server UUIDs live only in the invocation resolution map. Outcomes and
cross-references retain `(project,name)`.

Skill create rejects null for every authored top-level request member. Because
the same declaration can create or update, all such fields remain
authoring-required even though the update model is nullable. `contentFrom` is
authoring-only and transforms to non-null request `content`; the unused selector
is operation-inapplicable and receives no fabricated null.

Rename or project change is a new identity: zero-match creates the new Skill
and the old Skill remains because delete is out of scope. Existing references
must be updated in Git. No implicit adoption/rename search occurs.

Different-principal same-name rows are distinct identities. Same-principal
races still require serialization, post-write verification, and manual
remediation. The CLI never chooses a same-principal duplicate or deletes one.

## Consequences

### Positive

- Works without server changes and safely preserves unique existing Skills.
- Makes ambiguity deterministic rather than creator-biased.
- Natural identities remain portable and no client state is introduced.

### Negative

- Exhaustive pagination costs more than direct lookup.
- Correctness depends on visibility of the authenticated creator's own rows.
- Operational controls, not the database, contain cross-principal races.

### Risks

- A target release may filter marketplace/project scopes differently.
- Pagination drift can create false zero/one results unless detected.
- Uncontrolled UI writers can create duplicates between scans.

## Follow-up actions

- Generate and contract-test page shape, total/cursor semantics, scope filters,
  detail/update routes, and authorization behavior.
- Establish named owners for serialization, writer governance, inventory, and
  duplicate remediation.
- Test empty page 0, one result on page 0, 101+ results over pages 0 and 1,
  rejection of a first request/response at page 1, marketplace collisions,
  pagination drift, 409 re-resolution, forbidden detail/update, and post-write
  ambiguity. Create/update verification must reuse the same zero-based scan.

## References

- Product specification v31: FR-005/006/011/021/022/031–034/037, IR-012/013, DR-012/013,
  PA-005, VR-009/010/016
- Pinned source: `rest_api/routers/skill.py:198-316`,
  `service/skill_service.py:295-370`, and
  `repository/skill_repository.py:432-631`
- `contracts/http-adapter.md` section 6
