# ADR-015: Separate membership creation from entity write authorization

## Status

Proposed

## Context

V30/v31 incorrectly turned administration or a personal-owner detail proof
into a prerequisite. V32 states that exact project membership/access qualifies
creation and exact server `user_abilities` qualifies mutation of an existing
row.

## Decision drivers

- ordinary-member creation
- least privilege
- zero unauthorized writes
- session-bound, strict evidence

## Options considered

1. Admin/personal-owner gate. Rejected by v32.
2. Membership for every write. Rejected: membership is not entity write
   authorization.
3. Membership for create and exact row ability for update/adoption. Selected.

## Decision

Strictly decode `GET /v1/user.user_id` and every `projects[].name`. Exact
membership in the effective project is necessary and sufficient client-side
authorization to reach a create. Role/admin fields are optional visibility
context, never gates. No project-detail call is made.

An update or Workflow adoption additionally requires exact string `write` in
the selected row's `user_abilities`. Missing/malformed consumed evidence is
`E_API_INCOMPATIBLE`, exit 2; valid missing membership or ability is
`E_AUTHORIZATION`, exit 2. Both produce zero modifying requests and allowlisted
diagnostics only. Evidence and request are sealed to one target origin, token,
principal, session, effective project, kind, identity, and operation.

## Consequences

### Positive

Members can create without privilege escalation; updates remain server-led.

### Negative

Create and update paths have distinct evidence types.

### Risks

DTO reuse could accidentally treat a role or creator as ability; distinct
domain types and negative mutation tests are mandatory.

## Follow-up actions

Security review must verify unconstructable unauthorized `PreparedWrite` states.

## References

Specification v32 FR-033/037, DR-013, IR-013, PA-005/008, VR-017.

