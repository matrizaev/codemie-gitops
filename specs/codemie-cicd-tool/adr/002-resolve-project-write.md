# ADR-002: Use resolve-project-write reconciliation

## Status

Proposed

## Context

FR-005/006/012 require natural-key resolution and one modifying request for
every valid apply: POST when identity is absent, PUT when it is present. The
external API uses different lookup and update forms by kind, and Workflow,
Skill, and Datasource require exhaustive client resolution. No common natural
key upsert or conditional-write contract exists.

## Decision drivers

- Exact natural identity and reference resolution
- One deterministic modifying request after successful resolution
- Operation-specific create/update fields without server-default insertion
- Deterministic ambiguity, authorization, and compatibility failures
- Stateless operation and bounded recovery

## Options considered

### A. Blind create or update

Rejected: it cannot establish unique identity, write capability, or the correct
server route ID.

### B. Trust server upsert semantics

Rejected: no uniform natural-key upsert contract exists.

### C. Resolve exact identity, project the selected operation, write, verify

Selected. Each adapter reads enough state to prove zero or one exact target,
constructs the pinned POST or PUT request, sends it once, and performs bounded
post-write identity verification.

### D. Suppress PUT when authored and remote fields appear equal

Rejected: specification v24 requires an update for every valid invocation
against an existing identity.

## Decision

Use option C. Kind-specific identity rules are part of the resolver:

- Assistant: exact source-pinned natural-key lookup;
- Datasource: exhaustive list/exact filter because the pinned `find_id` route
  selects the first row and persistence lacks natural-key uniqueness;
- Workflow: ADR-008 exhaustive `meta_config` marker resolution/adoption; and
- Skill: ADR-007 exhaustive full-list resolution.

Resolution zero creates `Create(request)` and sends one POST. Resolution one
creates `Update(server_id, request)` and sends one PUT on every valid apply.
Existing reads are limited to identity, visibility/write evidence, Workflow
metadata preservation/adoption, request representation needs, reference
mapping, and post-write identity verification. Verification proves the natural
identity remains unique and is associated with the expected route ID; it does
not determine whether to write.

Projection contains authored values plus bounded FR-021 transforms. It never
inserts server defaults. Required/null-rejecting fields must already have
passed explicit-author validation. Omission and explicit YAML null become a
present JSON null for every applicable `optionalNull` JSON property.
Authoring-only, operation-inapplicable, mixed/tool-owned, and prohibited fields
receive no fabricated null. Each adapter sends exactly its operation-specific
fields; create-only fields are absent from PUT.

## Consequences

### Positive

- Meets v24 repeat-apply and omission/null semantics plus natural-identity safety.
- Handles file/source/content changes through ordinary selected operations.
- Makes duplicate, unstable, and incompatible resolution explicit.
- Keeps operation projections contract-testable.

### Negative

- Existing identities receive a modifying request on every valid invocation.
- Exhaustive resolution adds reads and pagination.
- Races remain possible without server conditional writes.

### Risks

- Incomplete visibility can make an apparent zero/one result unsafe.
- Changed target semantics can invalidate a pinned projection.
- An accepted write followed by failed verification is uncertain and must not
  be blindly retried or rolled back.

## Follow-up actions

- Contract-test every kind's resolver and operation projection against the
  checked-in schema and manifest.
- Test Workflow/Skill/Datasource complete-visibility and write preconditions.
- Require per-environment CI serialization and duplicate remediation.
- Verify repeat apply sends PUT and emits `updated`.

## References

- Product specification v24: FR-005/006/011/012/021/022/028–036,
  DR-012, IR-003/005/008
- ADR-007
- ADR-008
- ADR-009
