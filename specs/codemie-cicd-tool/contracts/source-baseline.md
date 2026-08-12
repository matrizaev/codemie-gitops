# Pinned CodeMie source baseline

Status: NORMATIVE ARCHITECTURE BASELINE. These repositories are external,
reference-only evidence and are not product build or runtime dependencies.

## Revisions

| Reference checkout | Tag | Commit | Audit state |
|---|---|---|---|
| `codemie/` backend | `2.42.0` | `2a481c290c99bf30ef80aadafa03d876a7f5f732` | clean; v32 membership/creator/ability/collision evidence reinspected 2026-08-12 |
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
| Workflow | the principal sees its own rows; create records the principal as creator; zero-indexed enumeration exposes `created_by`, editable string `meta_config`, ID, project, and `user_abilities`; v2 reconciliation is creator-scoped and has no marker uniqueness/conditional-write guarantee |
| Skill | zero-indexed list/detail/POST/PUT expose creator and abilities; persistence uniqueness is `(name, created_by.id, project)`, so reconciliation is exact `(project,current user_id,name)` |
| Datasource | zero-indexed `GET /v1/index` is visibility-filtered and cannot prove project-wide absence; ordinary create is the authority and HTTP 409 is a collision; a visible exact row may select update only with exact `write` ability |

Capability preflight is `GET /v1/user`. The v32 contract consumes a non-empty
`user_id` and `projects[].name`; an exact effective-project membership is the
creation gate for all four kinds. Administration fields may be present and may
broaden what the server returns, but they are not a client-side creation gate.
The v30/v31 project-detail/personal-owner proof is superseded and the CLI does
not call project detail for authorization.

Every update and Workflow adoption separately consumes the selected entity's
`user_abilities` and requires the exact supported string `write`. Membership,
creator identity, role flags, list visibility, or a successful detail read do
not imply write authorization. Assistant retains direct exact `(project,slug)`
lookup. Workflow and Skill filter to the authenticated creator ID. Datasource
uses visible rows only to select a possible update; a miss permits one create,
and authoritative HTTP 409 fails without retry, guessed lookup, or update.

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
