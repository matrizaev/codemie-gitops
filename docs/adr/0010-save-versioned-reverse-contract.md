# ADR-0010: Pin save reads and reverse projection in a versioned manifest

## Status

Accepted (originally save ADR-013; current behavior).

## Context

The existing adapter manifest is primarily a declaration-to-request contract.
`save` consumes larger detail responses, reverses server references, and
must classify every field as authorable, transformed, referenced, mixed-owned,
excluded, secret, or non-exportable. Reusing only the write manifest would
leave response additions and reverse-only routes unclassified, and the feature
must remain compatible with the pinned baseline without treating
`GET /v1/info.version` as API identity.

## Decision

- `contracts/save-read-reverse-v2.42.0-v1.json` is the normative reverse
  adapter manifest; its shape is closed by
  `contracts/save-read-reverse-v1.schema.json`. It records every GET
  route, pagination/snapshot invariant, field class, managed-reference
  destination, per-kind Datasource discriminator, exclusion, secret/masked
  position, non-exportability predicate, and compatibility behavior.
- Strict boundary DTOs implement the manifest. Known excluded/secret fields
  decode into non-retaining ignored-field sinks; unknown fields at a contracted
  boundary fail `E_API_INCOMPATIBLE`. The write manifest remains normative
  for apply; neither manifest is generated from the other.
- **API-to-declaration normalization**: OpenAPI response objects are not
  declaration objects. The reverse projector explicitly normalizes Assistant
  context `{context_type, name}` to datasource references
  `{context_type, ref}`; enriched category objects to category-name
  strings; toolkit/tool and MCP server objects to the closed declaration
  schemas (excluding nested API `config` and credential-bearing fields);
  and integration settings to `{id, alias}` selections. Defaults are
  materialized only when declared by the pinned response contract and required
  by the declaration schema.
- Pinned-baseline facts (from the reference-only checkout): Workflow
  `yaml_config` stores the execution configuration at its root; managed
  references live in `yaml_config.assistants[]` and
  `yaml_config.states[]`; an MCP entry with `use_custom_config=true`
  and persisted custom configuration or auth token is non-exportable.

## Consequences

- Reverse behavior and secret handling are reviewable without reading Rust.
- Compatibility failures occur before final-file creation.
- Field additions require reviewed manifest maintenance; the manifest and
  implementation can drift if not regression-tested.

## Alternatives considered

- Extending only the existing write manifest: rejected (mixes two field-
  presence policies; reverse-only routes unclassifiable).
- Inferring reverse projection from the declaration schema at runtime:
  rejected (schema property names do not describe server aliases, IDs,
  masked positions, or non-exportable state).

## References

- [save-read-reverse-v2.42.0-v1.json](../../contracts/save-read-reverse-v2.42.0-v1.json)
- [save-read-reverse-v1.schema.json](../../contracts/save-read-reverse-v1.schema.json)
- `src/save/reverse.rs`
- Related: ADR-0007
