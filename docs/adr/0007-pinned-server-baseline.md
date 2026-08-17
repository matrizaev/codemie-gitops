# ADR-0007: Pinned backend source baseline and compatibility gate

## Status

Accepted (originally ADR-004; current behavior).

## Context

Incompatible targets must fail fast. The provided backend source is the
architecture baseline: tag `2.42.0`, commit
`2a481c290c99bf30ef80aadafa03d876a7f5f732` (companion UI baseline
`2.42.0`, commit
`55945d075d82e771c4a2f4238afec1eb4c79d1e1`). The backend package
version, configured `APP_VERSION`, and Git tag disagree; `GET /v1/info`
returns only the configured `APP_VERSION` and is observability only. Live
OpenAPI cannot prove visibility, pagination, metadata preservation, write
authorization, or service behavior and must not broaden the authoring schema.

## Decision

- The checked-in OpenAPI (`contracts/openapi.json`) is primary for
  immutable wire shapes. The checked-in
  `contracts/adapter-manifest-v2.42.0.json` records the exact backend
  commit and every consumed route, request field, required/null/default class,
  consumed response field, transform/encoding, pagination rule, ability field,
  and operation-inapplicable field. Pinned reference-only source and the
  save-read/reverse manifest supply behavioral evidence OpenAPI cannot express.
- Implementation reproduces and tests the manifest; it does not define it.
  Runtime compatibility is checked with the cheapest non-mutating
  capability/preflight reads; a missing or changed required field fails before
  a modifying request when discoverable. Strict response decoding is always on.
- Verification/release runs the same non-mutating contract suite against each
  target deployment. Breaking drift blocks that deployment/release until
  reviewed; it never makes the source-based architecture incomplete and never
  widens declarations from live OpenAPI.
- Reference-only source (`codemie/`, `codemie-ui/`) is never
  modified or included in product tasks.

## Consequences

- Exact traceability to reviewed source with reproducible fixtures.
- Unrelated additive APIs do not break compatibility.
- Maintainers must regenerate/review the manifest when the baseline changes.
- Runtime probes cannot detect every semantic drift before a write.

## Alternatives considered

- Trusting `/v1/info`: rejected (unreliable source/contract identity).
- Dynamically accepting live OpenAPI: rejected (could broaden declarations).
- Requiring an exact whole-OpenAPI hash: rejected (false failures from
  unrelated additions; deployments may not expose OpenAPI).

## References

- [openapi.json](../../contracts/openapi.json)
- [openapi-contract-metadata.json](../../contracts/openapi-contract-metadata.json)
- [source-baseline.md](../../contracts/source-baseline.md)
- [http-adapter.md](../../contracts/http-adapter.md)
- `tests/openapi_contract.rs`