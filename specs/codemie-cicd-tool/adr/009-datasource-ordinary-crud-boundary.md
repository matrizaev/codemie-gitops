# ADR-009: Use ordinary write-through CRUD for Datasource

## Status

Accepted for ordinary per-kind CRUD projections; identity absence and
visibility decisions are superseded by ADR-018.

## Context

Specification v24 defines Datasource as one of exactly four entities and admits
the source, content, file, configuration, and ordinary scheduling fields
supported by each selected kind's ordinary create/update contract. The pinned
kinds use different JSON or multipart projections and operation-specific field
sets. The CLI has no separate Datasource lifecycle surface.

## Decision drivers

- Complete ordinary desired-state CRUD
- One uniform identity and operation boundary across peer kinds
- Exact closed per-kind fields without invented generic aliases
- One selected write on every valid apply
- External integrations uniformly outside CLI ownership

## Options considered

### A. Metadata-only updates

Rejected: prevents specified source/content/file reconciliation.

### B. Add a separate lifecycle controller

Rejected: the product defines no such command, flag, or endpoint.

### C. Ordinary peer-kind write-through CRUD

Selected. The resolved operation sends every field admitted by the selected
POST or PUT mapping.

## Decision

Implement one Datasource adapter with a closed `index_type` union whose exact
peer mappings are in `declaration-v1alpha1.schema.json` and
`adapter-manifest-v2.42.0.json`. JSON/multipart representation and asymmetric
create/update fields remain per-kind mappings, not separate product entities or
readiness gates.

For JSON peers, every source-pinned `optionalNull` property is present as JSON
null when omitted or explicitly null in YAML. Null-rejecting/defaulted members
are authoring-required. Create-only properties are operation-inapplicable on
PUT. File uses the singular `/v1/index/knowledge_base/file` multipart route:
relative contained files become byte-preserving UploadFile parts and
`uploaded_files`/`guardrail_assignments` are compact JSON query strings. Its
nullable logical model values use absent query parameters because that transport
has no JSON-null token.

All kinds use exhaustive list/exact filtering because `find_id` selects a first
row and persistence has no natural-key unique constraint. Missing identity uses
ordinary create. One exact match proves identity cardinality only and always
selects ordinary update. Complete visibility must already have passed, and an
existing target independently proves its operation-specific write ability.
Duplicates fail without selection.

**V32 boundary (ADR-015/018)**: exact membership qualifies creation. A visible
exact row selects update only with exact `write`. A visible miss proves no
absence and permits one create; HTTP 409 is authoritative collision with no
retry, lookup, or guessed update. Administrator visibility is optional. A
field admitted only by create is omitted on update.

Every opaque integration identifier points to pre-existing external platform
configuration. The CLI validates the closed local field form, sends it without
provisioning, discovery, or credential access, and accepts server validation and
authorization as authoritative. This is uniform across kinds.

The CLI publishes no dedicated Datasource lifecycle command, flag, or endpoint.
Successful application reports only `created` or `updated` for the Datasource
record.

Provider declarations require an exact reviewed deployment schema to be bundled
before admission. None is present in this baseline. The pinned Bedrock vendor
import is not ordinary CRUD and remains unsupported.

## Consequences

### Positive

- Satisfies the full approved write-through scope.
- Preserves exact per-kind server contracts under one entity boundary.
- Removes the need for remote content fingerprints or client state.

### Negative

- Every valid existing-entity apply sends PUT.
- Create-only fields cannot be changed through the pinned update route.
- Exhaustive resolution adds list requests.

### Risks

- A deployment can drift from pinned per-kind mappings.
- Races can create duplicate natural identities.
- Operators may expect product behavior beyond the synchronous CRUD response.

## Follow-up actions

- Independently verify every checked-in peer mapping and positive/negative
  schema branch before implementation.
- Test repeat apply request counts and exact operation payloads.
- Run deployment contract tests and activate serialized CI/remediation controls.

## References

- Product specification v24: FR-006/021/022/036, DR-010–012, IR-008,
  QR-002, VR-014–016, PA-007
- ADR-002
- `contracts/adapter-manifest-v2.42.0.json`
