# ADR-0012: Bounded observed-stable Skill snapshot

## Status

Accepted (originally save ADR-016; current behavior).

## Context

Skill detail returns main content and companion metadata, while each companion
payload requires another GET. The pinned API exposes no transaction, ETag,
conditional read, revision token, or content digest spanning those calls.
Reading each payload once can combine different revisions.

## Decision

After exhaustive natural-key resolution, the adapter performs a bounded
observation sequence per selected Skill ID:

1. detail A;
2. every companion payload A in normalized-path order;
3. detail B;
4. every companion payload B in normalized-path order; and
5. detail C.

Details A/B/C must have the same selected ID, natural identity, `updatedDate`,
complete authorable fields, main content, and ordered companion metadata after
normalization. Payload sets A and B must have identical path, MIME type,
encoding, declared size, and decoded bytes. A response path must equal the
requested normalized path. Duplicate/unsafe paths, invalid encoding/base64,
size mismatch, missing content, response-limit failure, or incompatible field
types fail the whole save. Individually valid observations that differ produce
`E_RESOLUTION_UNSTABLE` (exit 1); shape or integrity violations produce
`E_API_INCOMPATIBLE` or `E_ENTITY_NOT_EXPORTABLE`.

All calls are sequential, use the inherited bounded GET retry policy, and share
the 300-second invocation deadline and response budgets.

## Consequences

- Detects ordinary concurrent Skill and companion changes without server writes.
- Companion content is read twice; large Skills are more likely to reach the
  invocation deadline.
- This is an observed-stability protocol, not a server transaction: an ABA
  change outside observation points cannot be ruled out (explicit residual
  risk; a future server revision/digest endpoint would close it).

## Alternatives considered

- Trusting one detail plus one read per companion: rejected (mixed snapshot
  risk).
- Requiring a transactional server export endpoint: deferred (server change
  outside the approved feature).

## References

- [skill-snapshot-v1.md](../../contracts/skill-snapshot-v1.md)
- `src/save/reverse.rs`, `src/adapters/skill.rs`
- Related: ADR-0010
