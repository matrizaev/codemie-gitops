# Contracts

Closed machine and wire contracts for `codemie-gitops`. These are the highest
authority sources for closed interfaces: when prose documentation disagrees
with a contract here, the contract wins.

## Authoring language

| File | Role |
|---|---|
| `declaration-v1alpha1.schema.json` | Closed JSON Schema for the `codemie.epam.com/v1alpha1` authoring language. Consumed at build time (`build.rs`) and embedded at compile time (`src/schema.rs`). |
| `declaration-v1alpha1.md` | Prose companion: envelope, presence/projection rules, Skill content, references. |

## Server and wire contracts

| File | Role |
|---|---|
| `openapi.json` | Pinned backend OpenAPI baseline (tag 2.42.0). Consumed by `tests/openapi_contract.rs`. |
| `openapi-contract-metadata.json` | Baseline metadata (openapi version, server version, sha256). Consumed by `tests/openapi_contract.rs`. |
| `adapter-manifest-v2.42.0.json` | Declaration-to-request adapter manifest: routes, field projections, pagination, authorization. Consumed by a contract test in `src/adapters/mod.rs`. |
| `source-baseline.md` | Pinned reference-only source checkouts and identity constraints. |
| `http-adapter.md` | HTTP adapter contract: transport, authorization order, per-kind resolution, exit taxonomy. |
| `repository-config.schema.json` | Retired deny-all marker: the CLI must not load `.codemie/config.yaml`. |

## Output contracts

| File | Role |
|---|---|
| `outcome-v2.schema.json` | Closed successful per-entity stdout outcome (supersedes outcome-v1). |
| `diagnostic-v2.schema.json` | Closed failure diagnostic for stderr (supersedes diagnostic-v1). |
| `warning.schema.json` | Closed non-fatal stderr warning. |

## Save contracts

| File | Role |
|---|---|
| `cli-save-v1.md` | Save command surface, configuration ordering, and output. |
| `save-read-reverse-v2.42.0-v1.json` | Normative reverse adapter manifest (GET routes, field classes, exclusions, exportability). |
| `save-read-reverse-v1.schema.json` | JSON Schema closing the reverse manifest's shape. |
| `canonical-yaml-v1.md` | Canonical declaration YAML serialization rules. |
| `publication-v1.md` | Direct create-new write contract (single YAML; File Datasource placeholders first, YAML last). |
| `single-declaration-validation-v1.md` | In-memory validation of the generated declaration. |
| `skill-snapshot-v1.md` | Bounded observed-stable Skill read sequence. |

## Changing a contract

A wire-format change requires updating the schema/manifest, its contract
tests, and the documentation that references it. Prose can never widen a
closed contract. See `docs/adr/` for the rationale behind contract decisions.
