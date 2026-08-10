# ADR-006: Derive Workflow transport ID with UUIDv5

## Status

Superseded by ADR-008

This document is historical rationale only. No derivation, caller-assigned ID,
ID-based lookup, conformance vector, or compatibility requirement from this ADR
is executable. Product specification v24 and ADR-008 govern Workflow identity.

## Context

Before Workflow `meta_config` identity/adoption was approved, the server exposed
ID-based CRUD and appeared incidentally to preserve an optional caller-supplied
create ID, but had no persisted slug. A deterministic UUIDv5 derived from
`(project,slug)` was evaluated as a stateless new-entity strategy.

## Decision drivers

- Natural author identity with no UUID in YAML
- No client state or server change
- Deterministic cross-environment routing
- Compatibility with UUID-shaped downstream consumers
- Existing-entity safety

## Options considered

- Plain slug or project/slug composite as ID: global uniqueness, escaping, and
  UUID-consumer risks.
- Deterministic UUIDv5: fixed and portable for newly created objects, but unable
  to discover existing server-generated-ID Workflows.
- Random/server UUID in YAML or sidecar: violates portable stateless authoring.
- Display-name list lookup: cannot prove slug identity.

## Decision

The former UUIDv5 selection is withdrawn. ADR-008's reserved
`codemie.epam.com/gitops/workflow-identity` record and explicit by-ID adoption
solve both new and legacy identity without making caller-supplied create IDs a
platform dependency. Implementations must not derive or submit Workflow IDs.

## Consequences

### Positive

- Historical alternatives and their rejection remain documented.
- Current implementation has one unambiguous identity algorithm.

### Negative

- The old derivation vectors are intentionally removed to prevent accidental
  implementation.

### Risks

- Reusing old task/design text could resurrect an incompatible new-only path;
  consistency checks must reject executable `UUIDv5` requirements outside this
  superseded ADR.

## Follow-up actions

- None. Follow ADR-008 and product specification v24.

## References

- ADR-008
- Product specification v24: FR-028–030/032–034, DR-007/008
