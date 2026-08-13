# ADR-016: Establish a bounded observed-stable Skill snapshot

## Status

Accepted

## Context

Skill detail returns main content and companion metadata, while each companion
payload requires another GET. The pinned API exposes no transaction, ETag,
conditional read, revision token, or content digest spanning those calls.
Reading each payload once can combine different revisions.

## Decision drivers

- FR-SAVE-019/020 and DR-SAVE-006
- No mixed Skill snapshot
- Bounded operation under the inherited deadline
- No backend changes
- Exact content and metadata validation

## Options considered

### A. Trust one detail plus one read per companion

Rejected. Concurrent updates can create a mixed snapshot.

### B. Require a new transactional server export endpoint

Deferred. It would change the server and is outside the approved feature.

### C. Double-read payloads bracketed by stable detail observations

Selected for the pinned baseline.

## Decision

After exhaustive natural-key resolution, the adapter performs:

1. detail A;
2. every companion payload A in normalized-path order;
3. detail B;
4. every companion payload B in normalized-path order; and
5. detail C.

Details A, B, and C must have the same selected ID, natural identity,
`updatedDate`, complete authorable fields, main content, and ordered companion
metadata after normalization. Payload sets A and B must have identical path,
MIME type, encoding, declared size, and decoded bytes. A response path must
equal the requested normalized path. Duplicate or unsafe paths, invalid
encoding, invalid base64, UTF-8 mismatch for text, decoded-size mismatch,
missing content, response-limit failure, or incompatible field type fails the
whole save.

Individually valid observations that differ produce
`E_RESOLUTION_UNSTABLE`, exit 1. Shape or integrity violations produce
`E_API_INCOMPATIBLE` or `E_ENTITY_NOT_EXPORTABLE` according to the reverse
manifest. All calls remain sequential and under the inherited 300-second
deadline and response budgets.

This is an observed-stability protocol, not a server transaction. An ABA
change outside observation points cannot be ruled out without a future server
revision/digest contract. That limitation is an explicit residual risk.

## Consequences

### Positive

- Detects ordinary concurrent Skill and companion changes.
- Requires no server write or new endpoint.
- Produces reproducible evidence for failure-injection tests.

### Negative

- Companion content is read twice.
- Large Skills are more likely to reach the invocation deadline.

### Risks

- ABA changes can evade observation.
- Storage content could mutate without the Skill `updatedDate`; the second
  payload pass mitigates but does not create isolation.

## Follow-up actions

- Ask the platform owner to consider an immutable revision/digest endpoint.
- Add churn tests between every observation boundary.
- Verify no payload, ID, or path enters logs or diagnostics.

## References

- Feature specification v2: FR-SAVE-019/020/030, DR-SAVE-006,
  IR-SAVE-006, QR-SAVE-005/006, AC-SAVE-007–009
- `../contracts/skill-snapshot-v1.md`
