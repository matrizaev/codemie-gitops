# ADR-0005: Workflow identity via reserved meta_config marker and explicit adoption

## Status

Accepted (originally ADR-008 and ADR-016; current behavior).

## Context

The Workflow record has a server UUID and a display name but no persisted slug.
CRUD addresses an existing Workflow by server ID. Natural `(project,slug)`
cannot safely select a legacy row by display name. The API exposes and preserves
`meta_config`, which can carry a Workflow-only identity record without a
server migration. Ordinary members see their own Workflows and create records
that principal as creator, so project-wide absence is not available to them.

## Decision

- **Reserved record**: the adapter exclusively owns the closed object
  `"codemie.epam.com/gitops/workflow-identity"` in `meta_config`,
  currently shaped `{version: 2, project, creator_user_id, slug}`. Authors
  may not set it; empty/wrong-typed/unknown/mismatched values are invalid.
  Non-reserved `meta_config` members are preserved across updates.
- **Ordinary resolution**: exhaustive zero-based scans across the required
  scopes, matching only exact current-principal v2 records. Other creators'
  rows are excluded, never ambiguity. Zero matches creates (guarded against
  unmarked same-display-name candidates, which require adoption); one matches
  updates with exact `write`; more than one fails ambiguity; invalid
  markers fail `E_IDENTITY_MARKER_INVALID`. Every write is post-write
  verified at the expected route ID.
- **Explicit adoption**: `--adopt-workflow-id <uuid>` is Workflow-only
  invocation input. It selects one same-project, same-creator, writable legacy
  row with no reserved marker and zero existing exact v2 matches, then installs
  the v2 marker and reconciles desired content in one PUT. No UUID ever enters
  declaration YAML or outcomes.
- **Scope**: v1 markers and unmarked rows never ordinary-match. Marker removal
  yields an unmarked candidate requiring re-adoption. Rename/project change is
  a new identity; the old row remains (delete is out of scope). The marker is
  not database-unique; operational writer serialization and post-write
  verification are required.

## Consequences

- Safely reconciles new and explicitly selected legacy Workflows in place while
  keeping authoring portable and stateless.
- Fails closed on tampering, ambiguity, and wrong-creator candidates.
- Migration is lazy and explicit: inventory, freeze writers, adopt one reviewed
  UUID at a time, verify v2.
- Correctness depends on exhaustive privileged reads and metadata preservation.

## Alternatives considered

- Display-name list resolution: rejected (mutable, not identity).
- Deterministic caller UUID (v1): rejected (cannot discover/adopt existing rows;
  v1 scope was project-wide and superseded by the creator-scoped v2 marker).
- UUID in YAML or import map: rejected (violates portable authoring).

## References

- [http-adapter.md](../../contracts/http-adapter.md) §5
- [adapter-manifest-v2.42.0.json](../../contracts/adapter-manifest-v2.42.0.json)
- `src/adapters/workflow.rs`
- Related: ADR-0003
