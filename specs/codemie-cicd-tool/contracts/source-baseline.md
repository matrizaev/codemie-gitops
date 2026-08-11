# Pinned CodeMie source baseline

Status: NORMATIVE ARCHITECTURE BASELINE. These repositories are external,
reference-only evidence and are not product build or runtime dependencies.

## Revisions

| Reference checkout | Tag | Commit | Audit state (2026-08-09) |
|---|---|---|---|
| `codemie/` backend | `2.42.0` | `2a481c290c99bf30ef80aadafa03d876a7f5f732` | clean |
| `codemie-ui/` | `2.42.0` | `55945d075d82e771c4a2f4238afec1eb4c79d1e1` | clean |

Backend package `0.8.0`, UI package `0.0.0`, and default
`APP_VERSION=0.16.0` are informational. `GET /v1/info` does not reliably name
the source/API contract.

## Exact contract artifacts

- [`adapter-manifest-v2.42.0.json`](adapter-manifest-v2.42.0.json) records every
  consumed route, request field inventory, response/pagination surface,
  transform, operation-specific field boundary, and pinned source location.
- [`declaration-v1alpha1.schema.json`](declaration-v1alpha1.schema.json) is the
  closed authoring language derived from v24 plus this baseline.
- [`repository-config.schema.json`](repository-config.schema.json) is the
  closed, non-secret `.codemie/config.yaml` contract. It permits only `url`,
  `auth_url`, and `project`; authentication endpoint precedence and credential
  sources are product/CLI contracts rather than server-derived behavior.
- `research.md` section 2 gives the human-readable source trace.

The manifest has exactly four entity entries: Assistant, Workflow, Datasource,
and Skill. Datasource contains peer per-kind mappings for ordinary CRUD. A kind
may use an opaque reference to pre-existing external integration configuration;
all such references share the same out-of-scope provisioning/credential
boundary and server-authoritative rejection behavior.

Provider authoring is accepted only if an exact deployment-specific provider
schema has been reviewed and bundled; none is present in this baseline. The
pinned Bedrock vendor import is not an ordinary create/update contract and is
unsupported.

## Source-supported identity constraints

| Entity | Evidence-backed resolution and write boundary |
|---|---|
| Assistant | direct slug/project read, POST, PUT-by-ID; partial unique project/slug persistence constraint |
| Workflow | zero-indexed full-list pagination starts at `page=0` and uses `offset=page*per_page`; exhaustive pages expose editable persisted string `meta_config`, project, ID and abilities; POST/PUT preserve identity record; no record uniqueness/conditional write |
| Skill | zero-indexed list pagination starts at `page=0` and uses `offset=page*per_page`; the CLI pins `per_page=100`; list/detail/POST/PUT plus creator-scoped uniqueness `(name, creator, project)` require exhaustive, ambiguity-refusing resolution |
| Datasource | zero-indexed `GET /v1/index` pages return `data` plus `pagination`; detail and ordinary per-kind create/update exist; no database natural-key uniqueness evidenced and `find_id` returns a first row, so exhaustive resolution is required |

Capability preflight is `GET /v1/user`, whose pinned response exposes
`is_admin`, `is_maintainer`, and `projects[].{name,is_project_admin}`. Existing
entity write proof additionally consumes per-row `user_abilities`. For
Workflow, Skill, and Datasource, project-admin evidence qualifies only when `projects[].name`
equals the declaration's exact effective project and that same entry has
`is_project_admin=true`; an admin entry for another project is not evidence for
the requested operation. Assistant uses its exact `(project, slug)` read and
does not require `GET /v1/user` admin evidence.

The Keycloak token endpoint is not inferred from this source baseline,
`GET /v1/info`, or an API hostname. Product specification v24 requires one
explicit endpoint from the CLI/environment/repository-config precedence chain.
The backend source only supplies evidence for the separate local-development
`POST /v1/local-auth/login` route.

## Drift policy

Implementation tests reproduce and validate the checked-in manifest; they do not
create its architecture semantics. Pre-implementation verification validates
schema/manifest/spec convergence. Deployment and release contract tests run the
same non-mutating capability/fixture suite against the target. Breaking drift
blocks that deployment/release and is escalated; it does not make architecture
incomplete or authorize a live OpenAPI response to widen declarations.
