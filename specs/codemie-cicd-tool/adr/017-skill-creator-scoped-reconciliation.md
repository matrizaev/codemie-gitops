# ADR-017: Reconcile Skills in the authenticated creator namespace

## Status

Proposed; supersedes project-wide identity portions of ADR-007.

## Context

Pinned persistence uniqueness is `(name,created_by.id,project)`. Another
creator's same-name Skill is a distinct row.

## Decision drivers

- align reconciliation with server uniqueness
- support ordinary members
- never guess among same-creator duplicates

## Options considered

1. Project-wide `(project,name)`. Rejected by v32.
2. First visible match. Rejected as unsafe.
3. Exact creator-scoped filtering. Selected.

## Decision

Enumerate the current-user-visible zero-based pages, strictly decode creator,
and filter exact `(project,authenticated_user_id,name)`. Other creators are
excluded. Zero creates once; one requires exact `write` and updates; multiple
same-creator matches fail ambiguity. A create 409 triggers exactly one
exhaustive page-0-origin same-creator read-only scan and no second POST or
PUT/PATCH/DELETE. One exact collision is `ServerRejected` exit 1; multiple are
ambiguity exit 1; stable zero is reconciliation instability exit 1; a
compatibility/connectivity failure is exit 2. Every success is post-write
verified at the expected ID.

Skill creator tooling is limited to authoring the declared Skill content and
inline-content projection. It does not create projects, users, memberships,
integrations, generic ownership metadata, or adopt another creator's Skill.

## Consequences

### Positive

Identity matches the server constraint and hidden foreign rows do not block.

### Negative

Same-name rows owned by different creators coexist intentionally.

### Risks

Malformed/missing creator evidence must fail compatibility before write.

## Follow-up actions

Test foreign creator exclusion, same-creator ambiguity, 409 recovery, and
selected-declaration scope.

## References

Specification v32 FR-031–034, PA-005/008, VR-017.
