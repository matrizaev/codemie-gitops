# ADR-0003: Resolve natural identity, prove authorization, write exactly once, verify

## Status

Accepted (originally ADR-002, ADR-014, and ADR-015; current behavior).

## Context

Requirements demand natural-key resolution and exactly one modifying request
for every valid apply: POST when identity is absent, PUT when it is present.
The external API uses different lookup and update forms per kind, has no
uniform natural-key upsert or conditional-write contract, and offers different
trustworthy selectors per kind (so project-wide identity assumptions are
wrong). Earlier versions incorrectly turned administration or a personal-owner
detail proof into a create prerequisite.

## Decision

Use kind-specific resolve-project-write reconciliation:

- **Authorization boundary**: exact membership in the effective project
  (from a strict decode of `GET /v1/user` and `projects[].name`) is
  necessary and sufficient client-side authorization to reach a create. An
  update or Workflow adoption additionally requires exact string `write` in
  the selected row's `user_abilities`. Role/admin fields are optional
  visibility context, never gates. Missing/malformed consumed evidence is
  `E_API_INCOMPATIBLE`; valid missing membership/ability is
  `E_AUTHORIZATION`; both are exit 2 with zero modifying requests.
- **Identity per kind**: Assistant uses the exact server `(project,slug)`
  lookup; Workflow uses the creator-scoped reserved v2 marker
  (ADR-0005); Skill uses `(project,authenticated_user_id,name)` (ADR-0004);
  Datasource uses visible exact `(project,repo_name,kind)` with create-on-
  miss (ADR-0006). Server IDs are invocation-local route selectors only.
- **Write**: resolution zero creates `Create(request)` and sends one POST;
  resolution one creates `Update(server_id, request)` and sends one PUT on
  every valid apply (repeat apply is always a write, never a no-op). Projection
  contains authored values plus bounded transforms; it never inserts server
  defaults. Omitted/explicit-null optional fields become present JSON null;
  authoring-only, operation-inapplicable, mixed/tool-owned, and prohibited
  fields receive no fabricated null.
- **Verification**: after the write, one bounded re-resolution must find the
  expected natural identity at the expected route ID. A failed post-write check
  is reported as uncertain (may-have-committed) and is never blindly retried or
  rolled back. No delete exists.

## Consequences

- Duplicate, unstable, incompatible, or unauthorized resolution fails before a
  write; the matrix is contract-testable per kind.
- Existing identities receive a modifying request on every valid invocation.
- Races remain possible without server conditional writes; operational
  serialization and post-write verification are required.
- Resolvers and race handling are intentionally non-uniform per kind because
  the server evidence is non-uniform.

## Alternatives considered

- Blind create/update: rejected (cannot establish identity or capability).
- Trust server upsert semantics: rejected (no uniform contract exists).
- Suppress PUT when fields appear equal: rejected (always-write authoring).
- Admin/personal-owner create gate: rejected by specification v32.

## References

- [http-adapter.md](../../contracts/http-adapter.md)
- [adapter-manifest-v2.42.0.json](../../contracts/adapter-manifest-v2.42.0.json)
- `src/coordinator.rs`, `src/adapters/*`
- Related: ADR-0004, ADR-0005, ADR-0006
