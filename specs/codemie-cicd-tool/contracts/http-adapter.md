# HTTP adapter contract

Source: product specification v28, IR-001–012,
FR-005/006/011/016/017/021/022/024, FR-028–036, DR-003–012, and VR-016.

Status: NORMATIVE against backend tag `2.42.0`, commit
`2a481c290c99bf30ef80aadafa03d876a7f5f732`. Exact consumed routes and field
projections are in
[`adapter-manifest-v2.42.0.json`](adapter-manifest-v2.42.0.json).

Version: v28 (source-derived compatibility and exact-effective-project pre-write
evidence clarification; prior SEC-001/SEC-002/SEC-003 remediation and Mode (c)
Keycloak ROPC retained). See ADR-004/012 for compatibility/visibility and
ADR-011 for credential input, ValidatedUrl, TLS, and redirect policy.

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
and pagination check have succeeded. Workflow, Skill, and Datasource require
the exact-project capability predicate. Assistant instead requires its strict
direct `(project, slug)` lookup result and does not call `GET /v1/user`. No
adapter may call POST/PUT while evidence is partial or directly from a
resolve/read error branch.

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
sources. No credential is ever loaded from repository config.

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
| YAML/config bytes per file | 1 MiB (1,048,576 B) | Generous for CI declarations; covers complex Workflows |
| YAML documents per file | 1 | Each declaration file contains exactly one entity |
| YAML nesting depth | 32 levels | Covers all defined schema structures with headroom |
| YAML alias/anchor expanded nodes | 1,000 | Prevents alias-bomb expansion |
| YAML scalar length | 128 KiB | Covers large system prompts; well above 30,000 byte Skill content cap |
| YAML collection members (array or object) | 10,000 | Per-level cap |
| Repository files visited (discovery) | 10,000 files | CI workspace ceiling |
| Sidecar / File per-file bytes | 32 MiB | Per-file upload ceiling |
| Aggregate upload bytes per invocation | 128 MiB | All File parts in one apply call |
| Multipart parts per File Datasource | 10 | Existing schema cap; enforced before upload |
| Multipart basename length | 255 bytes | Filesystem maximum |
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
- YAML/config limits: reject before allocation (refuse to allocate an AST
  that would exceed nesting or node budget; fail with `E_YAML_PARSE` or
  `E_SCHEMA`, exit 2)
- File/sidecar limits: reject before streaming (check size before opening;
  stream with bounded read; fail with `E_SIDECAR`, exit 2)
- Response body: bounded drain (do not allocate the full body before reaching
  the limit; discard excess and classify as `E_CONNECTIVITY` or
  `E_API_INCOMPATIBLE`, exit 2)
- Pagination: page count and item count checked incrementally; exceeding
  either limit is `E_API_INCOMPATIBLE`, exit 2
- Deadline: applies to the entire invocation including parsing, reads,
  retries, and pagination; deadline cancellation propagates to all in-flight
  operations

### 2.5 Safe file open (SEC-003)

File handles must be opened and validated on the same file object (open-then-
fstat on the same descriptor). The tool must never canonicalize a
repository-relative path and then reopen the canonical path as a separate
operation. This prevents TOCTOU races where a symlink or file is swapped
between the check and the read.

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
uses exactly `--auth-url` > `CODEMIE_AUTH_URL` > repository config `auth_url`;
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
  "version": 1,
  "project": "<effective-project>",
  "slug": "<slug>"
}
```

- zero exact valid markers: inspect unmarked exact display-name rows only as a
  nonselecting adoption-required guard; otherwise create;
- one: prove exact project, complete visibility and write capability, read the
  detail required for metadata preservation, and update by server ID;
- more than one: `E_AMBIGUOUS_IDENTITY`, exit 1, no write;
- malformed/conflicting reserved member affecting the project:
  `E_IDENTITY_MARKER_INVALID`, exit 1, no write; and
- incomplete visibility/write evidence: exit 2 before write.

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

With `--adopt-workflow-id`, first prove there is no existing exact identity
record, then fetch only the supplied canonical UUID. Require exact project,
complete visibility, write permission, no reserved identity member, and safely
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

1. Enumerate every `GET /v1/skills` page with `per_page=100`, project,
   marketplace-inclusive, and search hints where compatible. Skill pagination
   is zero-indexed: always request page 0 first; for `pages > 0` request exactly
   `0..pages-1`; for `pages == 0` stop after page 0.
2. Client-filter exact decoded `(project,name)` over the complete visible set.
3. Zero creates once. One requires write proof and required detail, then always
   updates by returned ID. More than one is `E_AMBIGUOUS_IDENTITY`, exit 1,
   with no write.
4. Never select current-principal, newest, first, or list-order duplicate.
5. A same-principal create 409 permits one full re-resolution and never a
   second POST. Post-write resolution must find exactly one identity.

Every Skill page must echo the requested zero-based `page`, return
`perPage=100`, satisfy `pages=ceil(total/perPage)`, and report `pages==0` iff
`total==0`. A page-1 initial request/response, wrong echo/size, or inconsistent
page-count formula is `E_API_INCOMPATIBLE`, exit 2 before write. Across otherwise
compatible responses, changing `(pages,total,perPage)`, repeated IDs, or a final
accumulated count different from `total` is entity-resolution instability,
exit 1 before write. Initial, post-write, and create-409 re-resolution use this
same scan.

The server uniqueness tuple is creator-scoped, so complete manager/admin
visibility, serialized per-environment CI, governed concurrent writers, and a
duplicate-remediation runbook are normative controls. Different-principal races
can leave duplicates and are reported without rollback.

`contentFrom` resolves relative to the declaring YAML and is sent only as inline
`content`; path and content never cross the diagnostic boundary.

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

File alone uses `POST`/`PUT /v1/index/knowledge_base/file`. Resolve every
authored path relative to its declaring YAML, require a repository-contained
regular file, and stream its bytes as a repeated multipart `files` part using
the basename as the upload filename. Scalar fields are query parameters because
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

Before Workflow, Skill, or Datasource resolution, `GET /v1/user` must prove
either global admin/maintainer status or an entry where
`projects[].name` equals the declaration's exact effective project and that same
entry has `projects[].is_project_admin=true`. Project-admin status for any other
project is insufficient. Per-row `user_abilities` must additionally contain
write for an existing target where the pinned entity contract consumes that
field. Missing or invalid consumed role/project fields are
`E_API_INCOMPATIBLE`; a valid response that fails the predicate is
`E_VISIBILITY_UNPROVEN`. Both are exit 2 before any modifying request. These
capability checks cannot widen visibility by themselves.

Assistant is not subject to this complete-visibility admin prerequisite. Its
operation-applicable evidence is the strictly decoded direct
`GET /v1/assistants/slug/{slug}?project={effective_project}` result plus any
required write evidence from that response. This preserves PA-003 least
privilege while retaining the same sealed `PreparedWrite` boundary.

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
