# ADR-018: Treat Datasource create 409 as authoritative collision

## Status

Proposed; supersedes ADR-012 and the complete-visibility portions of ADR-009.

## Context

Ordinary-member Datasource lists are partial. A list miss cannot prove absence,
and the client has no safe lookup fallback or natural-key upsert.

## Decision drivers

- ordinary-member creation
- no guessed update
- one bounded mutation
- server authority for project-wide collisions

## Options considered

1. Require admin-complete enumeration. Rejected by v32.
2. Guess a hidden row after a miss/409. Rejected: unauthorized and ambiguous.
3. One create with authoritative 409. Selected.

## Decision

A visible exact row may select update only after exact `write`; multiple visible
exact rows fail ambiguity. A visible miss permits exactly one create. Success is
post-write verified through safe applicable reads. HTTP 409 is authoritative
collision evidence: exit 1, no retry, no `find_id`, no guessed GET, no update,
and no response-body disclosure. Other failure statuses follow the typed error
contract. Admin visibility may improve diagnostics and detect ambiguity before
create, but is optional and does not change the mutation matrix.

## Consequences

### Positive

Members can create with at most one mutation and hidden rows are never guessed.

### Negative

The CLI cannot identify or update a colliding hidden row.

### Risks

A deployment that does not preserve authoritative 409 semantics is incompatible
and must fail target qualification.

## Follow-up actions

Contract-test every Datasource kind's create collision and safe post-write read.

## References

Specification v32 FR-036/037, AC-FR-037-01/02; pinned CodeMie 2.42.0.
