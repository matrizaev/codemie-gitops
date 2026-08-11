# Architecture research

Reviewed 2026-08-09; v25 security remediation addendum 2026-08-10. Product
source: `specs/codemie-cicd-tool.md` v25. Reference repositories are read-only
external evidence, never product components or build inputs.

## 1. Pinned baseline

| Reference | Exact revision | Audit state | Informational versions |
|---|---|---|---|
| backend `codemie/` | tag `2.42.0`, `2a481c290c99bf30ef80aadafa03d876a7f5f732` | clean | package `0.8.0`, default `APP_VERSION=0.16.0` |
| `codemie-ui/` | tag `2.42.0`, `55945d075d82e771c4a2f4238afec1eb4c79d1e1` | clean | package `0.0.0` |

`configs/config.py:50` supplies the application default and
`rest_api/routers/common.py:28-33` returns it from `/v1/info`, so that response
does not prove source/API compatibility. The commits pin the architecture;
target drift is downstream verification and release evidence.

There is no current CLI product implementation outside the reference trees.

## 2. Four-entity API and persistence evidence

### 2.1 Assistant

- `AssistantRequest` is at `rest_api/models/assistant.py:294-354`.
- list/detail/exact slug reads are at `rest_api/routers/assistant.py:194-364`;
  exact identity is `GET /v1/assistants/slug/{slug}?project=...`.
- create/update are `POST /v1/assistants` and
  `PUT /v1/assistants/{id}` (`routers/assistant.py:669-809`) with project/write
  checks.
- partial unique index `uq_assistants_project_slug` covers `(project,slug)`
  where slug is present (`models/assistant.py:728-735`).

The current defaulted `skip_integration_validation` member demonstrates why the
source-pinned adapter manifest, not live OpenAPI, controls authoring.

### 2.2 Workflow

- Full list pages use `GET /v1/workflows?minimal_response=false`; minimal
  response omits `meta_config` (`routers/workflow.py:109-142`;
  `core/workflow_models/workflow_config.py:343-374`).
- detail/create/update are `GET /v1/workflows/id/{id}`,
  `POST /v1/workflows`, and `PUT /v1/workflows/{id}` with read/write checks
  (`routers/workflow.py:224-392`).
- requests expose nullable string `meta_config` (`workflow_models.py:406-436`);
  persistence and
  update preserve it (`workflow_config.py:69-125`;
  `service/workflow_service.py:54-66,651-683`).
- count/offset/limit, visibility scopes, and returned abilities are implemented
  in `workflow_config_index_service.py:46-163,222-265`. There is no persisted
  slug or marker uniqueness/conditional-write constraint.
- actor fields distinguish graph-local `id`, persisted resource
  `assistant_id`, inline `system_prompt`/`skill_ids`/`datasource_ids`, and state
  graph reference `assistant_id` (`workflow_models.py:34-64`). Requests use
  `yaml_config` plus flattened execution fields.

This proves reserved-marker resolution/adoption and the need for exhaustion,
serialization, governance, and post-write fail-closed identity checks.

Workflow pagination is zero-indexed. The pinned router defaults `page` to `0`,
and the index service uses `offset(page * per_page)`. Each project/marketplace
pass and post-write scan must request page 0 first; starting at page 1 skips the
first result page.

### 2.3 Skill

- `GET /v1/skills` supports `per_page <= 100`, project,
  marketplace-inclusive, and search hints; search is containment, not identity
  (`routers/skill.py:198-316`).
- detail/create/update are at `routers/skill.py:351-436`; update checks `WRITE`
  (`service/skill_service.py:602-667`). Rows expose ID, project, name, creator,
  and abilities (`models/skill.py:421-471`).
- count/page queries and owner/member/admin/public visibility are at
  `repository/skill_repository.py:46-253,432-631`.
- database uniqueness is `(name,created_by.id,project)`, not `(project,name)`
  (`models/skill.py:531-539`).

Every page must therefore be exact-filtered and duplicates must never be
tie-broken. Cross-principal duplicates remain possible; the approved operational
response is serialized CI, governed writers, inventory, and manual remediation.

Skill pagination is zero-indexed. The pinned router declares
`page: Query(0, ge=0)`, the service/repository defaults are `0`, and repository
queries use `offset(page * per_page)`. Exhaustive, post-write, and create-409
scans must request page 0 first; starting at page 1 skips the first result page.

### 2.4 Datasource

- `GET /v1/index`, detail, and `find_id` are in
  `routers/index.py:188-278`. `find_id` uses first-match lookup
  (`models/index.py:1162-1192`) and the table has no natural-key unique index;
  procedural checks are not concurrency-proof.
- Git/SVN ordinary models/routes are at `core/models.py:121-151` and
  `routers/index.py:450-971`.
- Peer knowledge-base request/update models for every inventoried kind are at
  `rest_api/models/index.py:1379-1775`; ordinary routes are at
  `rest_api/routers/index.py:1009-2068`.
- File create/update are the singular
  `POST`/`PUT /v1/index/knowledge_base/file` routes
  (`routers/index.py:1436-1448,2049-2085`). They bind `UploadFile` parts plus
  scalar query parameters; `uploaded_files` and `guardrail_assignments` are
  compact JSON-encoded strings (`models/index.py:1529-1581,1718-1725`). File
  multipart add/retain/remove/config behavior is at
  `use_cases/datasource/update_file_datasource_use_case.py:34-120`.
- Every peer kind's exact authored/create/update mapping is captured in
  `contracts/adapter-manifest-v2.42.0.json`, including operation-specific field
  asymmetry.
- `IndexInfoProviderFields` is deployment-defined
  (`models/index.py:223-234,1372-1378`), so an exact provider schema must be
  reviewed and bundled before accepting provider declarations. The Bedrock
  surface is vendor import (`rest_api/models/vendor.py:50-58`), not ordinary
  Datasource CRUD, and is unsupported by this baseline.

All supported Datasource kinds use the same exhaustive identity, ordinary CRUD,
external-reference, output, and rejection rules. External integration
references are opaque non-secret fields naming pre-existing platform
configuration. The CLI does not provision integrations or retrieve credentials;
server validation and authorization are authoritative.

Specification v24 makes each valid operation write-through and classifies
field presence/null semantics. Pydantic models at
`models/assistant.py:294-354`, `models/skill.py:165-268`,
`workflow_models.py:406-436`, and `models/index.py:1379-1725` provide the exact
nullability evidence captured in the manifest. Non-null typed defaults remain
authoring-required; Optional JSON-body properties admit explicit null. The
File query/multipart codec has no JSON-property channel and represents a
logical null through the source model's absent-query-to-`None` binding.

The File API's
lack of remote byte-content evidence therefore creates no architecture gap:
the selected create/update request is sent as declared, including files,
source, and content fields. The CLI has no dedicated Datasource lifecycle
command, flag, or endpoint.

## 3. Authorization and race evidence

`rest_api/security/user.py:28-115`, `rest_api/routers/user.py:52-115`, and
`core/models.py:767-797` expose authenticated global/project-admin evidence.
Entity responses expose abilities where the manifest consumes them. This is
sufficient to design preflight, not proof that every runtime principal is
authorized. Implementations fail before write when complete visibility or write
capability cannot be demonstrated.

The source does not provide Workflow marker uniqueness, global Skill natural
uniqueness, Datasource natural uniqueness, or ETag/If-Match semantics.
Normative operational controls are per-environment CI serialization, governed
identity writers, exact pre/post identity snapshots, and manual duplicate or
invalid-marker remediation. These are bounded implementation/operations tasks,
not unresolved architecture choices.

## 4. Conclusions

Facts established:

- all four entities have a source-pinned identity, create, and update path;
- Workflow identity/adoption and Skill exhaustive resolution are technically
  supported without server changes;
- Datasource ordinary write-through CRUD is feasible across every bundled peer
  kind;
- exact declaration schemas and an adapter manifest exist before implementation;
- v24 requires one POST for absent identity or one PUT for present identity and
  only post-write identity verification;
- successful entity output is the closed natural-identity/action record and
  failure output is a separate strict allowlist diagnostic; and
- deployment drift, security review, and pre-implementation verification are
  downstream lifecycle checks rather than architecture blockers.

Product decision v24 also closes authentication endpoint selection. Keycloak
uses only `--auth-url` > `CODEMIE_AUTH_URL` > repository config `auth_url` and
never derives or probes from the CodeMie API URL or a convention. Missing
explicit configuration is a local exit-2 failure before network. This is a
client configuration contract, not a behavior that needs to be inferred from
the reference backend. Credentials remain flag/environment-only.

Product decision v25 closes the secret credential input surface: bearer token,
client secret, and password are accepted only from named environment variables;
the corresponding CLI flags are rejected as `E_USAGE`, exit 2, before network
access.

**SEC-004 (datasource visibility) is deferred and unresolved.** The research
in §2.4 and §3 above establishes that `GET /v1/index` does not expose a
guaranteed server-enforced filter that can prove completeness for a specific
project's Datasource list. Three architectural options have been prepared and
documented in `specs/codemie-cicd-tool/adr/012-datasource-visibility.md`
(PROPOSED, awaiting product-spec-owner decision):
- **Option A**: `GET /v1/user` admin/is_project_admin preflight before every
  exhaustive Datasource scan, to fail closed when completeness cannot be proven.
- **Option B**: require a server-side natural-key constraint (requires server
  change, outside current scope).
- **Option C**: fail closed unconditionally until completeness is proven.
Implementation of D-001 is blocked until the product-spec-owner accepts one
option. Do not proceed with D-001 before that decision.

Non-blocking risks are deployment drift, pagination snapshot churn, writers
bypassing governance, undefined QR-005 latency, and a future
deployment-defined provider schema. Each has a fail-closed or bounded
verification/change path. Pre-implementation convergence verification and its
subsequent security review are downstream lifecycle stages, not missing
architecture evidence.
