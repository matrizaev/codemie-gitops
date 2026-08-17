# HTTP adapter contract

Source: product specification v32, IR-001–013,
FR-005/006/011/016/017/021/022/024, FR-028–037, DR-003–013, and VR-016/017.

Status: NORMATIVE against backend tag `2.42.0`, commit
`2a481c290c99bf30ef80aadafa03d876a7f5f732`. Exact consumed routes and field
projections are in
[`adapter-manifest-v2.42.0.json`](adapter-manifest-v2.42.0.json).

The manifest's top-level `manifestVersion: 3` versions this consumed adapter
contract. V-000 sanitized evidence must identify it as
`adapterManifestVersion: 3`; evidence `schemaVersion: 1` independently versions
the evidence envelope. The legacy V-000 field/value `manifestVersion: 2` names
no approved independent contract and must be rejected rather than translated.

Version: v32 (member-create, exact-ability-update, creator-scoped identity,
Datasource authoritative collision;
source-derived compatibility, prior SEC remediation, and Mode (c) retained).
See `docs/adr/0007` for compatibility, `docs/adr/0005`/0006 for
identity/visibility, and `docs/adr/0009` for credential input, ValidatedUrl,
TLS, and redirect policy.

## 1. Boundary and protocol

The Rust CLI is a direct REST consumer. External CodeMie owns remote records.
The CLI owns only transient declarations, natural-reference maps, operation
requests, and safe output. It owns no state database.

Every adapter implements the same closed protocol:

```text
operation_preflight -> enumerate/resolve_exact
-> resolve_references/read_required_detail/validate_pagination (as applicable)
-> zero: project_create -> establish_prewrite_evidence -> POST -> verify_identity -> created
-> one: prove_write -> project_update -> establish_prewrite_evidence
        -> PUT -> verify_identity -> updated
```

There are exactly four entity adapters: Assistant, Workflow, Datasource, and
Skill. Datasource kind mappings are internal discriminated cases, not separate
product entities. Existing-state reads serve identity, authorization,
Workflow metadata preservation/adoption, source-pinned representation needs,
reference mapping, or post-write identity verification. They never create a
branch that suppresses the selected write.

The transport write entry point accepts only a prepared write carrying a sealed
`PrewriteEvidence` value for the same entity kind and effective project. That
value is constructible only after the operation-applicable preflight and every
operation-applicable identity, reference, detail/preservation, response-shape,
and pagination check have succeeded. Every kind requires exact effective-
project membership from `GET /v1/user`; administration is optional. Assistant
then requires strict direct `(project,slug)` lookup. No
adapter may call POST/PUT while evidence is partial or directly from a
resolve/read error branch.

### 1.1 Authorization applicability and order

| Entity operation | Exact membership | Identity scope | Exact `write` ability |
|---|---|---|---|
| Assistant create/update | required | exact `(project,slug)` | update only |
| Workflow create/update/adopt | required | v2 `(project,user_id,slug)`; v1/unmarked adoption-only | update/adoption only |
| Skill create/update | required | `(project,user_id,name)` | update only |
| Datasource create/update | required | visible exact row or visible miss | update only |

All applicable reads and any later write use one opaque invocation-scoped
`ApiClient` capability owning the validated target origin, bearer token, and
internal session identity:
local validation -> authentication -> strict `/v1/user` exact membership ->
kind-specific resolution -> exact selected-row write evidence when applicable
-> request projection -> sealed
`PreparedWrite` -> POST/PUT. Dispatch takes the capability from the seal and
accepts no separately supplied client, origin/base URL, token, or session.

## 2. Transport and compatibility

### 2.1 URL validation (SEC-002, ADR-011)

Every URL consumed by this transport must be a `ValidatedUrl`:
- Absolute `http` or `https` scheme only
- No userinfo component (no `@` in authority)
- No fragment identifier (no `#`)
- No C0/C1 control characters (U+0000–U+001F, U+007F–U+009F)
- Non-empty syntactically valid host; port, if present, is a decimal integer

`auth_url` (Keycloak): HTTPS required unconditionally. Any `http` value is
`E_CONFIGURATION`, exit 2, before network access.

`target_url` (CodeMie API): HTTPS required for non-loopback hosts. HTTP is
permitted only when the resolved IP address is a loopback address (127.0.0.0/8
or ::1). The runtime must resolve and verify the address at connection time,
not only validate the hostname string.

An invalid higher-precedence URL is `E_CONFIGURATION`, exit 2, before network
access; lower-precedence values are not consulted and are not treated as
fallbacks for a syntactically or semantically invalid higher-precedence value.

No `--insecure` flag or TLS-verification bypass is provided. CA/proxy trust
uses standard system environment variables.

### 2.2 Credential input (SEC-001, ADR-011, v25)

Secret credential values (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
`CODEMIE_PASSWORD`) have environment-only sources; the corresponding flags
(`--token`, `--client-secret`, `--password`) are not accepted and their
presence is an `E_USAGE`, exit-2 failure before this transport is invoked.
Non-secret selectors (`--client-id`, `--email`) have flag-over-environment
sources. No credential or endpoint is ever loaded from repository config.

### 2.3 Redirect policy (SEC-002, ADR-011)

**Keycloak `POST .../token`** (Mode (a) and Mode (c) ROPC) and
**local-auth `POST /v1/local-auth/login`** (Mode (b)):
redirects are **disabled**. A 3xx response exits `E_AUTHENTICATION`, exit 2.
No credential is replayed to a redirect target.

**Authenticated API calls (GET/POST/PUT)**:
- Preferred: disable all redirects
- If any redirect is retained: must be method-aware (POST/PUT not redirected
  to GET), same-origin (same scheme + host + port), maximum 3 hops, and must
  not forward the `Authorization` header to a different authority

### 2.4 Resource budgets (SEC-003)

The following limits are versioned defaults. Values that would reject a
currently-valid declaration require product-spec-owner confirmation before
being reduced below the defaults listed here.

| Dimension | Default limit | Rationale |
|---|---|---|
| Selected YAML bytes | 1 MiB (1,048,576 B) | Bounded single declaration |
| YAML documents per file | 1 | Each declaration file contains exactly one entity |
| YAML nesting depth | 32 levels | Covers all defined schema structures with headroom |
| YAML alias/anchor expanded nodes | 1,000 | Prevents alias-bomb expansion |
| YAML scalar length | 128 KiB | Covers large system prompts; well above 30,000 byte Skill content cap |
| YAML collection members (array or object) | 10,000 | Per-level cap |
| Skill sidecar bytes | 128 KiB (131,072 B); resulting UTF-8 content must be 100–30,000 characters | One explicit `contentFrom` read |
| File Datasource bytes | 32 MiB per file; 128 MiB aggregate; 1–10 files | Explicit `spec.files` paths only |
| Query parameter total length | 8 KiB | Covers compact JSON-encoded query fields |
| Response header bytes | 16 KiB | Per-response header block |
| Response body bytes | 8 MiB | Per API response body drain |
| JSON nesting depth (response decode) | 64 levels | Defensive ceiling for API responses |
| Pagination pages per entity scan | 1,000 pages | Per exhaustive enumeration call |
| Total accumulated items per scan | 100,000 items | Duplicate-tracking set ceiling |
| GET retry attempts | 3 per request | With jitter under deadline |
| Total request timeout | 60 seconds | Per individual HTTP request |
| Total invocation deadline | 300 seconds (5 min) | Entire apply or lint invocation |
| Concurrency | 1 (sequential) | One entity per invocation; no concurrent requests |

Enforcement:
- YAML limits: reject before allocation (refuse to allocate an AST
  that would exceed nesting or node budget; fail with `E_YAML_PARSE` or
  `E_SCHEMA`, exit 2)
- Response body: bounded drain (do not allocate the full body before reaching
  the limit; discard excess and classify as `E_CONNECTIVITY` or
  `E_API_INCOMPATIBLE`, exit 2)
- Pagination: page count and item count checked incrementally; exceeding
  either limit is `E_API_INCOMPATIBLE`, exit 2
- Deadline: applies to the entire invocation including parsing, reads,
  retries, and pagination; deadline cancellation propagates to all in-flight
  operations

### 2.5 Safe file open (SEC-003)

The selected declaration and explicitly authored auxiliary inputs use bounded
open-and-verify reads. No directory enumeration or implicit path lookup occurs.

### 2.6 Other transport controls

- Send bearer credentials only in the `Authorization` header
- Apply the bounded connect/request/body-drain limits in §2.4
- GET may retry transient connection/429/5xx failures with jitter under the
  invocation deadline. POST/PUT are not blindly retried
- Retain only status, method, route template, tool request ID, and a dedicated
  validated response correlation header. Safely discard failure bodies and
  server messages
- Treat `GET /v1/info` as observability, not API identity. Strictly decode all
  consumed response fields against the pinned manifest. A missing or invalid
  consumed field is `E_API_INCOMPATIBLE`, exit 2, before a write when preflight
  can discover it. Only additive fields that no selected operation consumes are
  ignored; an unknown field that replaces, changes, or is used instead of a
  required consumed field is incompatible. Additive unconsumed fields do not
  expand requests or declaration schemas

Deployment/source drift is tested during verification and release; it never
authorizes runtime adaptation or silent contract widening.

Authentication endpoint selection is outside target-API discovery. Keycloak
uses exactly `--auth-url` > `CODEMIE_AUTH_URL`;
the selected endpoint is validated as `ValidatedUrl` with HTTPS required and
contacted as-is. Transport performs no derivation, well-known discovery,
hostname rewrite, realm/path synthesis, or probe from the CodeMie target URL.
Missing or invalid selected endpoint is a local exit-2 failure before this
transport is invoked.

### 2.7 Login call formats (v26)

Mode selection follows ADR-011 §1b. All three modes produce a JSON response
containing `"access_token"`; the token is written to stdout and never persisted.
Failure bodies are discarded.

**Mode (a) — Keycloak `client_credentials`:**

```
POST <auth_url>
Content-Type: application/x-www-form-urlencoded

grant_type=client_credentials&client_id=<CODEMIE_CLIENT_ID>&client_secret=<CODEMIE_CLIENT_SECRET>
```

**Mode (c) — Keycloak ROPC (v26):**

```
POST <auth_url>
Content-Type: application/x-www-form-urlencoded

grant_type=password&client_id=<CODEMIE_CLIENT_ID or "codemie-sdk">&username=<CODEMIE_EMAIL>&password=<CODEMIE_PASSWORD>
```

No `client_secret` field. `CODEMIE_CLIENT_ID` defaults to `codemie-sdk` if
unset. Redirect policy: disabled (§2.3). TLS: HTTPS required for `auth_url`
(§2.1).

**Mode (b) — Local-auth:**

```
POST /v1/local-auth/login   (relative to target_url)
Content-Type: application/json

{"email": "<CODEMIE_EMAIL>", "password": "<CODEMIE_PASSWORD>"}
```

Redirect policy: disabled (§2.3). TLS: HTTPS required for non-loopback
`target_url` (§2.1).

## 3. Request projection and write verification

The create or update request includes every admitted author field after only
the bounded FR-021 transforms. It does not materialize server defaults. For
each property classified `optionalNull`, omission and explicit YAML null both
produce a present JSON property whose value is `null` in every applicable JSON
request. The projector enumerates the pinned manifest, not only the YAML map,
so omission cannot accidentally select a server default.

Authoring-required/null-rejecting fields fail local validation before any
resolution call. Authoring-only selectors are transformed away, create-only
fields are operation-inapplicable on PUT, and read-only/prohibited members are
never sent. Tool/mixed-owned structures use their specific assembly rules; no
generic null filler traverses them. File's non-JSON multipart/query codec is
defined in §7 and the manifest.

No ETag/conditional-write contract exists. A modifying operation is sent once
and followed by bounded exact identity re-resolution. Verification establishes
that the intended natural identity resolves uniquely and that the returned
route ID is consistent; it does not assess field equality. Failed verification
reports whether the write may have committed and never deletes or rolls back.

## 4. Assistant

- Resolve: `GET /v1/assistants/slug/{slug}?project={project}`.
- Create: `POST /v1/assistants`.
- Update: `PUT /v1/assistants/{id}`.

The request is built only from closed author fields plus effective project and
resolved references. Datasource contexts validate the natural Datasource first,
then project to the server's `{context_type,name}` form because the pinned model
stores a name rather than UUID. Returned IDs are invocation-local. A resolved
Assistant always receives PUT.

## 5. Workflow

### 5.1 Exhaustive marker resolution

Exhaust every page of the full Workflow list across the required project and
marketplace-inclusive scopes. Each pass is zero-indexed: always request page 0
first; for `pages > 0` request exactly `0..pages-1`; for `pages == 0` stop after
page 0. Every response must echo the requested `pagination.page`, return
`pagination.per_page=100`, satisfy
`pagination.pages=ceil(pagination.total/pagination.per_page)`, and report zero
pages iff total is zero. Wrong origin/echo/size/count formula is
`E_API_INCOMPATIBLE`, exit 2 before write. Stable-schema snapshot churn within
a pass is entity-resolution instability, exit 1 before write. Scope parameters
are hints; exact client filtering uses effective project plus:

```json
"codemie.epam.com/gitops/workflow-identity": {
  "version": 2,
  "project": "<effective-project>",
  "creator_user_id": "<authenticated-user-id>",
  "slug": "<slug>"
}
```

- zero exact current-creator v2 markers: create; v1/unmarked rows are explicit
  adoption candidates only;
- one: prove exact project and exact write capability, read the
  detail required for metadata preservation, and update by server ID;
- more than one: `E_AMBIGUOUS_IDENTITY`, exit 1, no write;
- malformed/conflicting reserved member affecting the project:
  `E_IDENTITY_MARKER_INVALID`, exit 1, no write; and
- missing membership/write authorization: exit 2 before write.

Page cycles, repeated IDs, unstable totals/cursors, or incompatible pre/post
identity snapshots fail closed. Create/update merges the reserved member while
preserving unmentioned non-reserved `meta_config` and applying authored
non-reserved overlays.

The pinned list/detail/create/update representation of `meta_config` is a
nullable string, not a JSON object. Decode the string as strict UTF-8 JSON with
duplicate-key detection and require an object root. Merge semantic objects,
then encode one compact JSON string with recursively sorted object keys, no
BOM, and no non-finite numbers. A malformed value is
`E_IDENTITY_MARKER_INVALID`, exit 1, and is never overwritten.

### 5.2 Explicit adoption

With `--adopt-workflow-id`, first prove there is no existing exact current-user
v2 record, then fetch only the supplied canonical UUID. Require exact project,
same authenticated creator, exact `write`, a v1 or unmarked candidate, and safely
mergeable non-reserved metadata. Persist the marker and authored state with one
PUT. Another unmarked row with the same mutable display name neither selects nor
vetoes this explicit candidate. Without the flag, unmarked exact display-name
evidence only causes `E_ADOPTION_REQUIRED`.

Wrong project, marked candidate, invalid marker, or unmergeable metadata is exit
1; forbidden/incomplete access is exit 2. The UUID is never authored,
persisted locally, or emitted.

### 5.3 Workflow execution projection

Serialize authored `execution_config` into the source-pinned `yaml_config` and
flattened assistants/tools/states/start-state fields. Preserve workflow-local
`assistants[].id` and `states[].assistant_id`. Persisted actors map
`assistantRef` to server `assistant_id`. Inline actors retain `system_prompt`
and map inline-only `skillRefs`/`datasourceRefs` to server ID lists. Author-only
reference fields are removed from the request.

## 6. Skill

1. Strict exact effective-project membership from the same invocation-bound
   `GET /v1/user` qualifies create and supplies authenticated `user_id`.
   Administration is optional and is not a resolver or mutation gate.
2. Enumerate every `GET /v1/skills` page with `per_page=100`, project,
   marketplace-inclusive, and search hints where compatible. Skill pagination
   is zero-indexed: always request page 0 first; for `pages > 0` request exactly
   `0..pages-1`; for `pages == 0` stop after page 0.
3. Strictly decode every candidate's `id`, `project`, `name`, `created_by`, and
   `user_abilities`; client-filter exact
   `(project,authenticated_user_id,name)`. A foreign creator is excluded, not
   treated as ambiguity or an update candidate.
4. Zero exact current-creator matches creates once. One requires exact string
   `write` in that selected row's `user_abilities` plus required detail, then
   always updates by returned ID. More than one same-creator exact match is
   `E_AMBIGUOUS_IDENTITY`, exit 1, with no write.
5. Never select newest, first, list-order, or foreign-creator rows. Membership,
   creator equality, visibility, roles, and HTTP success never substitute for
   update `write` authorization.
6. A Skill create 409 triggers exactly one exhaustive page-0-origin read-only
   scan filtered to `(project,authenticated_user_id,name)`. It sends no second
   POST and no PUT/PATCH/DELETE. Exactly one collision returns `ServerRejected`,
   exit 1; more than one returns `E_AMBIGUOUS_IDENTITY`, exit 1; stable zero
   returns reconciliation instability, exit 1; compatibility or connectivity
   failure returns exit 2. Post-write resolution must find exactly one identity.

Every Skill page must echo the requested zero-based `page`, return
`perPage=100`, satisfy `pages=ceil(total/perPage)`, and report `pages==0` iff
`total==0`. A page-1 initial request/response, wrong echo/size, or inconsistent
page-count formula is `E_API_INCOMPATIBLE`, exit 2 before write. Across otherwise
compatible responses, changing `(pages,total,perPage)`, repeated IDs, or a final
accumulated count different from `total` is entity-resolution instability,
exit 1 before write. Initial, post-write, and create-409 re-resolution use this
same scan.

The server uniqueness tuple is creator-scoped. Different creators' same-name
rows are distinct identities. Same-principal writers remain serialized;
post-write same-principal ambiguity is reported without rollback and requires
manual remediation.

Workflow inline `skillRefs` resolve through the same exact
`(project,authenticated_user_id,name)` resolver and capability binding. A
foreign-creator same-name row never satisfies a reference. Zero exact current-
creator rows is an unresolved reference; one supplies an invocation-local Skill
ID; multiple same-creator rows are ambiguous. Reference reads require
membership but not Skill `write`, because they do not mutate the Skill.

Skill `contentFrom` is resolved relative to the selected declaration parent and
read once under the sidecar/path/timeout policy; apply sends only inline content.

## 7. Datasource

Exhaust every zero-indexed `GET /v1/index` page with `per_page=100`,
`full_response=true`, and a JSON `filters` hint. Validate
`data` plus `pagination.{page,per_page,total,pages}` on every page and detect
cycles, repeated IDs, and snapshot drift. Client-filter exact effective
project, `repo_name`, and
declared/persisted kind equivalence. `find_id` returns a first match and may only
optimize detail lookup after uniqueness is independently proven.

Zero uses the selected kind's ordinary create route. One proves visibility and
write capability, reads only details required by the operation contract, and
always uses its ordinary update route. More than one is
`E_AMBIGUOUS_IDENTITY`. Post-write full resolution must return one identity.
The checked-in manifest records every supported peer mapping under the same
algorithm and failure taxonomy.

Per-kind authored fields map exactly to the selected create or update request.
File, source, content, scheduling, and configuration fields are ordinary
write-through inputs. JSON-body optional-null properties are explicitly
present as `null`; fields absent from an update request are create-only and are
operation-inapplicable rather than synthesized. Opaque integration references identify
external pre-existing configuration and are passed without discovery or
credential access. CodeMie is authoritative for existence, type, access, and
use; locally valid rejection is exit 1. Provider forms require a separately
reviewed bundled deployment schema. Bedrock vendor import is unsupported
because it is not ordinary CRUD.

File alone uses `POST`/`PUT /v1/index/knowledge_base/file`. Each authored
`spec.files[]` is an explicit relative path under DR-014. Apply resolves it from
the selected declaration parent, rejects escape/symlink/non-regular/duplicate
targets, reads exact bytes under per-file/aggregate/deadline bounds, and emits a
repeated multipart `files` part using the safe basename. It enumerates no
directory and creates no temporary/staging copy. Scalar fields remain query parameters because
the pinned endpoint binds the request with FastAPI `Depends`; compact JSON
strings encode `uploaded_files` (PUT only) and `guardrail_assignments`. The
logical typed projection contains null for omitted/explicit-null optional
members; the query codec represents null by omitting that parameter, the only
wire spelling that binds as `None`. It never sends `new_project_name`.

**Multipart basename safety (SEC-005)**: Before constructing the multipart
request, the implementation must reject any basename that contains C0/C1
control characters (U+0000–U+001F, U+007F–U+009F), CR (U+000D), LF (U+000A),
NUL (U+0000), or path separator characters (`/`, `\`). Basenames that pass
this check are transmitted as the `filename` parameter in `Content-Disposition`.
If the selected HTTP library produces RFC 6266-compliant `filename*=UTF-8''...`
encoded parameters for non-ASCII basenames, that behavior must be verified and
documented as a pinned library feature. If the library does not provide this
guarantee, restrict basenames to printable ASCII only.

Before any kind's resolution, strictly decode `GET /v1/user`: non-empty string
`user_id`, array `projects`, and every entry's non-empty string `name`. At least
one exact name equal to the effective project qualifies creation. Role/admin
fields are optional, unconsumed visibility context and never authorization
gates. No project-detail request is made.

Security-document parsing rejects duplicate keys in every consumed JSON object
before DTO conversion; missing/null/empty/wrong-type consumed evidence is
`E_API_INCOMPATIBLE`. A valid response without exact membership is
`E_AUTHORIZATION`. Both are exit 2 before mutation. Additional unconsumed
fields alone are tolerated. Every update/adoption strictly decodes the selected
entity's `user_abilities` and requires exact string `write`; creator, membership,
role, ownership, and successful HTTP reads never supply it.

Assistant's operation-applicable evidence is the strictly decoded direct
`GET /v1/assistants/slug/{slug}?project={effective_project}` result plus any
required exact write evidence from that response.

Datasource visible-list miss authorizes exactly one create attempt. HTTP 409
is authoritative collision: exit 1, no retry, no fallback lookup, no guessed
update, and no response-body disclosure. An optional admin session may expose
more rows for diagnostics, but follows the same mutation matrix.

The CLI has no dedicated Datasource lifecycle command, flag, or endpoint.

## 8. Failure and concurrency table

| Condition | Exit | Write behavior |
|---|---:|---|
| compatible success | 0 | verify identity and emit `created` or `updated` |
| valid-input server rejection | 1 | no retry; Skill has one resolution-only create-409 exception |
| ambiguity/invalid marker/adoption required/stable-protocol resolution drift | 1 | no write when detected before request |
| auth, authorization, visibility/write proof, compatibility, connectivity | 2 | no write when detectable |
| post-write ambiguity/uncertainty | 1 | report may-have-committed; no delete/rollback |
| internal/fatal | 2 | fixed allowlisted diagnostic only |

Per-environment CI serialization and governed Workflow/Skill identity writers
reduce races. They do not create server uniqueness or conditional writes. Git
revert plus a new apply is recovery for writable desired state; duplicate or
invalid identity requires manual platform remediation.
