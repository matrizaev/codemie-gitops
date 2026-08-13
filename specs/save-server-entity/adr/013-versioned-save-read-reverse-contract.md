# ADR-013: Pin save reads and reverse projection in a separate manifest

## Status

Accepted

## Context

The existing adapter manifest is primarily a declaration-to-request contract.
`save` consumes larger detail responses, reverses server references, and must
classify every field as authorable, transformed, referenced, mixed-owned,
excluded, secret, or non-exportable. Reusing only the write manifest would
leave response additions and reverse-only routes unclassified.

The approved feature also requires compatibility with backend tag `2.42.0`,
commit `2a481c290c99bf30ef80aadafa03d876a7f5f732`, without treating
`GET /v1/info.version` as API identity.

## Decision drivers

- FR-SAVE-013–023 and DR-SAVE-002–008
- Fail-closed response compatibility
- Reviewable secret and managed-ID boundaries
- Independent evolution of forward and reverse adapters
- Traceability to the pinned reference-only source

## Options considered

### A. Extend only the existing write manifest

Rejected. It would mix two field-presence policies and make reverse-only
routes, snapshot reads, exclusions, and exportability hard to audit.

### B. Infer reverse projection from the declaration schema at runtime

Rejected. Schema property names do not describe server aliases, IDs,
mixed-owned metadata, masked positions, or non-exportable state.

### C. Add a versioned save-read/reverse manifest beside the feature

Selected.

## Decision

`contracts/save-read-reverse-v2.42.0-v1.json` is the normative reverse
adapter manifest. Its shape is closed by
`contracts/save-read-reverse-v1.schema.json`.

The manifest records:

- pinned source tag and commit;
- every GET route and pagination/snapshot invariant used by `save`;
- selection and complete-visibility rules;
- consumed top-level and nested field classes;
- every managed-reference route and natural-key destination;
- per-kind Datasource discriminator and field mappings;
- server/audit/runtime exclusions;
- secret or masked positions that are never projected;
- state-dependent non-exportability predicates; and
- unknown-field and compatibility behavior.

Strict boundary DTOs implement the manifest. Known excluded or secret fields
are decoded into non-retaining ignored-field sinks where their value is not
needed. Unknown fields at a contracted object boundary fail
`E_API_INCOMPATIBLE`; a backend change must update the manifest and its
positive/negative fixtures before acceptance.

The write manifest remains normative for declaration-to-server apply. The two
manifests may share source evidence and natural-key definitions, but neither is
generated from the other.

### Pinned-baseline corrections

The manifest incorporates the following facts verified against the pinned
`codemie/` checkout:

- Workflow `yaml_config` stores the execution configuration at its root, so
  managed references are read from `yaml_config.assistants[]` and
  `yaml_config.states[]`, not from an additional `execution_config` object.
- Workflow, Skill, and Datasource resolution records the `/v1/user` visibility
  preflight before complete-project scans.
- Skill and Datasource reference reads are represented as separate list and
  detail routes rather than descriptive combined route names.
- Datasource list filters are JSON encoded with the server's `project` key.
- An Assistant or Workflow containing an MCP entry with
  `use_custom_config=true` and persisted custom configuration or an auth token
  is non-exportable because the declaration schema has no corresponding
  authoring fields.

These are source-contract corrections, not decisions to expand support.
Autonomous Workflows are explicitly outside v1. Present server nulls are
preserved as YAML null and remain subject to the existing declaration schema
and prospective validation.

## Consequences

### Positive

- Reverse behavior and secret handling are reviewable without reading Rust.
- Compatibility failures occur before direct final-file creation.
- Backend and declaration changes have an explicit update gate.

### Negative

- Field additions require reviewed manifest maintenance.
- Some response models need verbose typed ignored-field declarations.

### Risks

- The manifest and implementation could drift.
- A broad generic JSON value could bypass non-retention and unknown-field
  checks.

## Follow-up actions

- Generate mutation tests for each consumed field and unknown-field boundary.
- Have the security reviewer approve secret/exclusion classes.
- Require verification to compare the manifest to the pinned reference-only
  source and implementation DTOs.

## References

- Feature specification v2: FR-SAVE-007–023/030, DR-SAVE-002–008,
  IR-SAVE-001–006, QR-SAVE-005/009
- Parent ADR-004, ADR-007, ADR-008, ADR-009, ADR-012
- `../contracts/save-read-reverse-v2.42.0-v1.json`
