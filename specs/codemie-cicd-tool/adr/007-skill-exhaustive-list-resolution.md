# ADR-007: Resolve Skill identity through exhaustive list filtering

## Status

Accepted

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

1. Preflight proves the CI principal has project-manager/admin visibility and
   write capability for the target project. Failure is exit 2 before write.
2. Enumerate every page of `GET /v1/skills` with `per_page=100`, using exact
   `project`, `project_with_marketplace`, and `search` hints where compatible.
   Hints do not replace client filtering.
3. Detect pagination cycles, repeated row IDs, changing totals/cursors, or
   inconsistent snapshots. Compatible but unstable resolution is exit 1.
4. Client-filter the complete visible set by exact project and exact name.
5. Zero matches: POST once. One: prove write ability, GET any detail required by
   the pinned request mapping, and PUT by returned ID on every valid apply. More than one:
   `E_AMBIGUOUS_IDENTITY`, exit 1, no write.
6. A same-principal concurrent-create 409 allows one bounded full
   re-resolution; POST is never repeated.
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

Different-principal concurrent create can still produce duplicates because the
server's uniqueness boundary differs. Production use therefore requires
per-environment serialization, governed UI/API writers, periodic duplicate
inventory, and a platform-owned remediation runbook. The CLI never chooses a
duplicate and never deletes one.

## Consequences

### Positive

- Works without server changes and safely preserves unique existing Skills.
- Makes ambiguity deterministic rather than creator-biased.
- Natural identities remain portable and no client state is introduced.

### Negative

- Exhaustive pagination costs more than direct lookup.
- Correctness depends on complete privileged visibility.
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
- Test 0/1/>1, >100 items, marketplace collisions, pagination drift, 409
  recovery, forbidden detail/update, and post-write ambiguity.

## References

- Product specification v24: FR-005/006/011/021/022/031–034, DR-012,
  PA-005, VR-009/010/016
- `contracts/http-adapter.md` section 6
