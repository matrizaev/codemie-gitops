# ADR-004: Derive adapter compatibility from a pinned source baseline

## Status

Proposed

## Context

IR-002 requires incompatible targets to fail fast. The provided backend source
is the architecture baseline: tag `2.42.0`, commit
`2a481c290c99bf30ef80aadafa03d876a7f5f732`. The companion UI baseline is tag
`2.42.0`, commit `55945d075d82e771c4a2f4238afec1eb4c79d1e1`.
The backend package version, configured `APP_VERSION`, and Git tag disagree;
`GET /v1/info` returns only configured `APP_VERSION`. Live OpenAPI cannot prove
visibility, pagination, metadata preservation, write authorization, or service
behavior and must not broaden the authoring schema.

Direct source inspection has closed the architecture feasibility question for
all four kinds. Deployment drift is a verification/release concern, not an
unresolved architecture decision.

## Decision drivers

- Fail closed before writes when consumed target behavior is incompatible
- Keep lint offline and the author schema immutable at runtime
- Make every adapter field/route traceable to exact reviewed source
- Detect relevant drift without coupling to an unreliable version string

## Options considered

### A. Trust `/v1/info`

Rejected: the value is not a reliable source or contract identity.

### B. Dynamically accept live OpenAPI

Rejected: it could broaden declarations and cannot establish behavioral
semantics.

### C. Require an exact whole-OpenAPI hash

Rejected: unrelated additions cause false failures and deployments may not
expose OpenAPI.

### D. Checked-in source-derived per-adapter manifest plus strict probes

Selected. The architect reviews and checks in the normalized manifest from the
pinned source paths used by the CLI. Implementation reproduces/tests it rather
than defining it. Target deployments are tested against only that required
subset and behavior not expressible in OpenAPI.

## Decision

The release bundles:

- immutable declaration schemas;
- a manifest that records the exact backend commit and every consumed route,
  parameter, operation request field, required/null/default class, consumed
  response field, transform/encoding, pagination rule, ability field, and
  operation-inapplicable field;
- sanitized contract fixtures and behavioral tests for Assistant exact slug,
  Workflow full pagination/`meta_config`, Skill exhaustive visibility and
  creator duplicates, Datasource subtype ordinary CRUD, and auth behavior.

At runtime the client performs strict decoding and the cheapest non-mutating
capability/preflight reads needed by the requested operation. A missing or
changed required field fails before a modifying request when discoverable.
`/v1/info` is observability only. Live OpenAPI, if used by CI, is checked only
against the required manifest subset and never expands authoring.

Verification/release runs the same contract suite against each target
deployment. Drift blocks that deployment/release until reviewed; it does not
retroactively make the source-based architecture incomplete.

## Consequences

### Positive

- Exact traceability to reviewed source and reproducible fixtures.
- Behavioral contracts complement schema checks.
- Unrelated additive APIs do not break compatibility.

### Negative

- Maintainers must regenerate/review the manifest when changing baseline.
- Runtime probes cannot detect every semantic drift before a write.

### Risks

- An incomplete generator or probe suite creates false confidence.
- A deployment can drift after qualification; strict response decoding and
  preflight contain but cannot eliminate that risk.

## Follow-up actions

- Independently verify Q-001–Q-003 against the pinned commit and checked-in
  contract before implementation.
- Add source-manifest reproducibility and additive/breaking drift tests.
- Run the deployment contract suite during environment/release qualification.

## References

- Product specification v24: IR-002/003/005/008–010, QR-008/009
- `research.md`, section 1 and section 2
- `contracts/http-adapter.md`, section 3
