# ADR-0004: Reconcile Skills in the authenticated creator namespace

## Status

Accepted (originally ADR-007 mechanics and ADR-017; current behavior).

## Context

The pinned server persists Skills with creator-scoped uniqueness
`(name, created_by.id, project)`: another creator's same-name Skill is a
distinct row, and ordinary members do not see every project row. Caller-
supplied Skill IDs are not an evidenced create contract. The approved identity
is `(project, name)`, but a project-wide scan cannot prove absence and
display names are not identity.

## Decision

- Enumerate zero-based pages of `GET /v1/skills` (`per_page=100`, page 0
  first, exact page-range and fingerprint rules; drift/cycles fail closed),
  strictly decode creator, and filter exact
  `(project, authenticated_user_id, name)`. Other creators' rows are
  excluded, never ambiguity.
- Zero matches: POST once. One: prove exact `write`, then PUT on every valid
  apply. Multiple same-creator matches: `E_AMBIGUOUS_IDENTITY`, exit 1, no
  write.
- A create HTTP 409 triggers exactly one exhaustive page-0-origin same-creator
  read-only scan and no second POST or PUT/PATCH/DELETE. One exact collision is
  `ServerRejected` exit 1; multiple are ambiguity exit 1; stable zero is
  reconciliation instability exit 1; compatibility/connectivity failure is
  exit 2.
- Every success is post-write verified at the expected route ID. Server UUIDs
  live only in the invocation resolution map; outcomes and cross-references
  retain `(project,name)`.
- Rename/project change is a new identity: zero-match creates the new Skill;
  the old row remains (delete is out of scope).

## Consequences

- Works without server changes; hidden foreign rows do not block reconciliation.
- Identity matches the server constraint; same-name rows owned by different
  creators coexist intentionally.
- Exhaustive pagination costs more than direct lookup; correctness depends on
  visibility of the authenticated creator's own rows.

## Alternatives considered

- Project-wide `(project,name)` reconciliation: rejected (claims scope the
  server does not expose to ordinary members).
- First/newest match or deterministic caller UUID: rejected (not the approved
  identity; create path does not accept caller IDs).
- Server natural-key endpoint: architecturally strongest but requires a server
  change; not required.

## References

- [adapter-manifest-v2.42.0.json](../../contracts/adapter-manifest-v2.42.0.json)
- [http-adapter.md](../../contracts/http-adapter.md) §6
- `src/adapters/skill.rs`, `src/pagination.rs`
- Related: ADR-0003
