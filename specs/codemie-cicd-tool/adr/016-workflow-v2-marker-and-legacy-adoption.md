# ADR-016: Scope Workflow v2 markers to the authenticated creator

## Status

Proposed; supersedes the v1 ordinary-match portions of ADR-008.

## Context

The principal always sees its own Workflows and create records that principal
as creator. Project-wide absence is not available to ordinary members.

## Decision drivers

- safe ordinary-member reconciliation
- preserve UUID-free authored identity
- explicit legacy migration
- visible residual races

## Options considered

1. V1 `(project,slug)` marker. Rejected: claims project-wide scope.
2. Display-name fallback. Rejected: not identity.
3. V2 `(project,creator_user_id,slug)` marker. Selected.

## Decision

The reserved record is the closed object `{version:2, project,
creator_user_id, slug}`. Ordinary resolution scans own-visible rows and matches
only exact current-principal v2 records. Other creators' rows are excluded, not
ambiguity. Create writes the exact v2 marker. Update requires exact `write`,
preserves non-reserved metadata, writes once, then verifies one exact marker at
the expected route ID.

V1 and unmarked rows never ordinary-match. Explicit adoption by server UUID is
allowed only for a same-project, same-creator, writable row with no reserved v2
marker and zero existing exact current-principal v2 matches. The adoption PUT
installs v2 and reconciles desired content in one write. Invalid/foreign v1,
marked, wrong-creator, or unmergeable candidates fail with zero writes.

Migration is lazy and explicit: inventory v1/unmarked own rows, freeze the
same-principal writer, adopt one reviewed UUID at a time, verify v2, and retain
sanitized evidence. Concurrent same-principal creates/adoptions can still race;
post-write ambiguity fails visibly without delete or rollback.

## Consequences

### Positive

Ordinary members can safely reconcile their own namespace.

### Negative

Legacy rows require reviewed adoption and serialization.

### Risks

No database uniqueness exists for the marker; operational writer control and
post-write verification remain required.

## Follow-up actions

Replace v31-produced v1 markers only through this adoption procedure; never
rewrite them blindly.

## References

Specification v32 FR-028–030/034, DR-007, AC-FR-028–030/034.

