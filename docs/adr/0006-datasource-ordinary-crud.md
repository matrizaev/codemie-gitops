# ADR-0006: Datasource ordinary per-kind CRUD with authoritative create-collision

## Status

Accepted (originally ADR-009 and ADR-018; current behavior).

## Context

Datasource is one of exactly four entities, with per-kind create/update
contracts (JSON or multipart) and no dedicated lifecycle surface. Ordinary
member Datasource lists are visibility-filtered, so a list miss cannot prove
project-wide absence, and there is no safe lookup fallback or natural-key
upsert. The pinned `find_id` route selects a first row and persistence has
no natural-key uniqueness.

## Decision

- Implement one Datasource adapter with a closed `index_type` union whose
  exact per-kind peer mappings live in
  `declaration-v1alpha1.schema.json` and
  `adapter-manifest-v2.42.0.json`. JSON/multipart representation and
  asymmetric create/update fields remain per-kind mappings, not separate
  product entities.
- Resolution uses exhaustive list/exact filtering. A visible exact row may
  select update only after exact `write`; multiple visible exact rows fail
  ambiguity. A visible miss permits exactly one create; it does not prove
  absence.
- HTTP 409 on create is authoritative collision evidence: exit 1, no retry, no
  `find_id`, no guessed GET, no update, and no response-body disclosure.
  Other failure statuses follow the typed error contract.
- Opaque integration identifiers are pre-existing external configuration; the
  CLI validates the closed local field form and sends it without provisioning,
  discovery, or credential access. Provider authoring is accepted only with an
  exact reviewed deployment schema; none is bundled in this baseline.
- Success is post-write verified through safe applicable reads and reports only
  `created` or `updated`.

## Consequences

- Ordinary members can create with at most one mutation; hidden rows are never
  guessed or updated.
- Every valid existing-entity apply sends PUT; exhaustive resolution adds list
  requests.
- A deployment that does not preserve authoritative 409 semantics is
  incompatible and must fail target qualification.

## Alternatives considered

- Metadata-only updates: rejected (prevents source/content/file reconciliation).
- Separate lifecycle controller: rejected (no such command, flag, or endpoint).
- Admin-complete enumeration as a create prerequisite: rejected by v32
  (optional; improves diagnostics only, never changes the mutation matrix).
- Guessing a hidden row after a miss/409: rejected (unauthorized and ambiguous).

## References

- [adapter-manifest-v2.42.0.json](../../contracts/adapter-manifest-v2.42.0.json)
- [http-adapter.md](../../contracts/http-adapter.md)
- `src/adapters/datasource.rs`
- Related: ADR-0003
