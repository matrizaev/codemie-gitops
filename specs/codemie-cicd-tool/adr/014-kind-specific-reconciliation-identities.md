# ADR-014: Use kind-specific reconciliation identities

## Status

Proposed

## Context

Product specification v32 replaces project-wide identity assumptions. The
pinned server offers different trustworthy selectors for each kind.

## Decision drivers

- UUID-free authored identity and outcomes
- zero unauthorized writes
- no absence claim stronger than server evidence
- safe behavior under partial visibility and races

## Options considered

1. One project-wide exhaustive-list algorithm. Rejected: ordinary visibility
   is incomplete and Datasource absence is unknowable.
2. Persist client UUID state. Rejected: conflicts with the product contract.
3. Kind-specific resolution. Selected.

## Decision

- Assistant: exact server `(project,slug)` lookup.
- Workflow: exact creator-scoped v2 marker
  `(project,authenticated_user_id,slug)`.
- Skill: exact `(project,authenticated_user_id,name)`.
- Datasource: visible exact `(project,repo_name,kind)` can select update; a miss
  permits one create and does not prove absence.

Server IDs are invocation-local route selectors only. Exact decoding, one
session/capability binding, and post-write verification apply to every kind.

## Consequences

### Positive

Each identity claim matches pinned evidence.

### Negative

Resolvers and race handling are intentionally non-uniform.

### Risks

Target drift blocks writes; it never authorizes a fallback identity.

## Follow-up actions

Contract tests and pre-implementation verification must assert all four
mutation matrices.

## References

Specification v32 FR-028–034/037, DR-007/013, PA-005/008, VR-017.

