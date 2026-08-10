# Data model

Source: `specs/codemie-cicd-tool.md` v26. Status: NORMATIVE architecture
model against backend `2.42.0` commit
`2a481c290c99bf30ef80aadafa03d876a7f5f732`.

Revision: v26 — SEC-001 credential input, SEC-002 ValidatedUrl/TLS/redirect,
SEC-003 resource budgets, SEC-005 identifier constraints (architecture
remediation, 2026-08-10); Mode (c) Keycloak ROPC added (v26, 2026-08-10).

## 1. Ownership and lifetime

Git declarations and sidecars are the desired-state source; external CodeMie
owns remote records. The CLI owns no persistent state. These models live for one
invocation only:

- marked YAML AST and source coordinates;
- resolved repository config and typed closed declaration;
- repository and Workflow-local symbol tables;
- capability/visibility snapshot and ephemeral server-ID maps;
- typed create or update request plan; and
- one safe success outcome or safe failure diagnostic.

Tokens, credentials, request/response bodies, payloads, server error text,
declaration/sidecar values, arbitrary headers, secret-bearing fields, and
secret-like values are outside every diagnostic, log, and persistence type.

## 2. Declaration and effective identity

```text
RawDeclaration = {
  apiVersion: "codemie.epam.com/v1alpha1",
  kind: Assistant | Workflow | Datasource | Skill,
  metadata: { project?: NonEmptyString, kind_key },
  spec: ClosedKindSpec
}

EffectiveProject = metadata.project ?? RepoConfig.project
AssistantKey  = {project: EffectiveProject, slug}
WorkflowKey   = {project: EffectiveProject, slug}
DatasourceKey = {project: EffectiveProject, repo_name}
SkillKey      = {project: EffectiveProject, name}
```

The syntax model permits omitted `metadata.project`; the semantic aggregate
does not. Resolution precedes cross-reference validation and fails locally if
neither source supplies it. Effective values are not written back to Git.
Identity equality is exact and case-sensitive after decoding. Server IDs never
inhabit natural-key types.

## 3. Source and repository models

`SourceSpan = {file,line?,column?,field_path?}` holds coordinates only.
`SidecarRef = {declaring_file, relative_path}` retains a path solely for safe
local loading. The scalar `contentFrom` path resolves from the declaring YAML's
directory; its bytes become transient `SkillContent` and neither path nor value
enters output.

`RepositoryIndex` maps exact
`(apiVersion,kind,effective_project,kind_key)` to a source location. Duplicate
keys are ambiguous. `WorkflowLocalIndex` maps each unique actor `id` to its
actor; state `assistant_id` resolves only in this local map.

## 4. Workflow actor and identity models

```text
WorkflowActor =
  Persisted {id: LocalActorId, assistantRef: AssistantKey, common...}
| Inline {id: LocalActorId, system_prompt,
          skillRefs?: [SkillKey], datasourceRefs?: [DatasourceKey], common...}
```

Persisted actors reject inline resource lists; inline actors reject
`assistantRef`. Both reject authored server-position IDs. Request projection is:

```text
Persisted.assistantRef -> server assistant_id
Inline.skillRefs       -> server skill_ids
Inline.datasourceRefs  -> server datasource_ids
Actor.id and State.assistant_id remain local graph IDs
```

Reserved server identity:

```text
WORKFLOW_IDENTITY_KEY = "codemie.epam.com/gitops/workflow-identity"
WorkflowIdentityV1 = closed {version: 1, project: NonEmptyString, slug: NonEmptyString}
```

`WorkflowResolution = Zero | Unique{server_id,detail} | Ambiguous | Invalid |
Unstable | AdoptionRequired | IncompleteVisibility`. UUIDs occur only in a
unique result or validated invocation-only adoption selector. A Workflow update
merge preserves unmentioned non-reserved server metadata, applies authored
non-reserved members, then installs the exact reserved record. There is no
metadata tombstone.

`WorkflowMetaConfigWire = null | UTF8JsonObjectString`. The decoder rejects
malformed JSON, duplicate keys, a non-object root, invalid UTF-8, or an invalid
reserved member. The encoder emits compact UTF-8 JSON with recursively sorted
object keys, no BOM, and no non-finite numbers. The merge operates on decoded
objects; the string is only a server transport representation.

Explicit adoption selects only its UUID. Another unmarked same-display-name row
has no selection or veto role. Without explicit adoption, display-name evidence
can only yield `AdoptionRequired`.

## 5. Skill identity model

```text
SkillCandidate = {server_id, project, name, created_by, abilities, detail?}
SkillSnapshot  = {all_pages, pagination_fingerprint, exact_candidates}
SkillResolution = Zero | Unique | Ambiguous | Unstable | IncompleteVisibility
```

Creator, list order, and age are evidence but never identity or tie-breaks.
Every page and required visibility scope must be complete. One returned ID is a
transient route selector. More than one exact candidate is ambiguous. A
creator-scoped server uniqueness constraint does not change the authored
`(project,name)` key.

## 6. Datasource union

```text
DatasourceResolution = Zero | Unique{server_id,detail} | Ambiguous |
                       Unstable | IncompleteVisibility
DatasourceSpec = ClosedUnion<index_type, PerKindAuthoredFields>
```

Resolution uses every visible list page and exact
`(project,repo_name,persisted_kind_equivalence)` filtering. `find_id` is a
non-authoritative hint because the pinned source shows first-match behavior and
no natural unique constraint.

Peer per-kind operation projections are versioned in the adapter manifest:

| Kind group | Request boundary |
|---|---|
| Git/SVN | ordinary repository JSON create/update |
| Confluence/Jira/Xray | ordinary flat knowledge-base JSON create/update |
| Azure DevOps Wiki/Work Item | ordinary flat knowledge-base JSON create/update |
| SharePoint | ordinary flat knowledge-base JSON create/update |
| File | ordinary multipart file/content/configuration create/update |
| Google | ordinary flat knowledge-base JSON create/update |

An opaque integration reference is transient desired configuration, not an
entity identity, credential, or managed reference. External integration
provisioning is uniformly outside this product. `provider` needs an exact
deployment-specific schema before admission; Bedrock vendor import is not an
ordinary CRUD variant in this baseline.

Create and update field sets may differ. A field absent from the pinned update
request is create-only and is not sent on update. Every field present in the
selected update projection is sent on every valid existing-entity apply. The
CLI exposes no dedicated Datasource lifecycle operation.

File is the only non-JSON operation projection: repository-contained relative
paths become transient byte streams and repeated `files` multipart parts;
`uploaded_files` and `guardrail_assignments` become compact JSON strings in
query parameters. Other scalar File fields are query parameters. Paths and
bytes are never retained after the request or admitted to output.

## 7. Request planning and write state

```text
FieldPresence = Authored(value) | AuthoredNull | Omitted
FieldClass = Required | OptionalNull | AuthoringOnly | OperationInapplicable |
             MixedOwned | ReadOnlyOrProhibited
DesiredEntity = {api_version, kind, natural_key, classified_fields}
WritePlan = Create{request} | Update{server_id, request}
```

Projection is pure and versioned by the checked-in schema and manifest. It:

- does not insert server defaults;
- maps `Omitted` and `AuthoredNull` to an explicit null for every applicable
  `OptionalNull` JSON property;
- rejects either state for `Required` fields;
- does not fabricate members for `AuthoringOnly`, `OperationInapplicable`,
  `MixedOwned`, or prohibited classes;
- applies only FR-021 transformations;
- resolves natural references from the exact invocation map;
- excludes server-owned and audit fields;
- emits only fields admitted by the selected POST or PUT contract; and
- fails before write when a required reference, preservation read, request
  field, or server shape cannot be represented safely.

Resolution zero selects `Create`; resolution one selects `Update`. No
field-state condition suppresses the selected write. Reads of existing records
are permitted only for identity,
authorization, Workflow metadata preservation/adoption, representation needs,
and post-write identity verification.

```text
ApplyState =
  LocalValidated
  -> Authenticated
  -> CompatibilityChecked
  -> IdentityResolved
  -> CreateProjected | UpdateProjected
  -> WriteAttempted
  -> IdentityVerified
  -> Succeeded(created | updated)
```

Any failure transitions directly to `Failed(SafeDiagnostic)`. A write-attempt
failure may be uncertain; the CLI performs no automatic delete or rollback.

## 8. Success and failure models

[`outcome.schema.json`](contracts/outcome.schema.json) is the only per-entity
stdout type:

```text
SuccessOutcome = closed {
  action: valid | created | updated,
  kind,
  project,
  exactly one of slug | name | repo_name
}
```

`valid` is lint-only. Apply emits `created` after an absent-identity POST and
`updated` after a present-identity or adoption PUT. No server ID, external
integration reference, remote runtime state, payload, or server message is
eligible. Non-fatal warnings are separate safe stderr records governed by
`contracts/warning.schema.json` and never enter the success outcome.

[`diagnostic.schema.json`](contracts/diagnostic.schema.json) is the only failure
stderr type. Its closed union binds each `errorCode` to exactly one category and
exit code. Optional members are limited to safe source coordinates, HTTP
status/method/route template, a local request ID, and a validated dedicated
server correlation ID. There is no generic message, details, value, body,
header, raw URL, or exception field.

## 9. Configuration, consistency, and recovery

```text
ValidatedUrl = absolute URL satisfying:
  scheme ∈ {http, https}
  no userinfo (no '@' in authority)
  no fragment (no '#')
  no C0/C1 control characters (U+0000–U+001F, U+007F–U+009F)
  non-empty syntactically valid host
  port, if present, is a decimal integer

  HTTPS required for auth_url at all times.
  HTTPS required for target_url except when resolved address is loopback
    (127.0.0.0/8 or ::1); runtime enforces; schema accepts http://localhost forms.
  HTTP loopback exception applies only to target_url, never to auth_url.

ResolvedConfig = {
  target_url?: Resolved<ValidatedUrl, Flag | Environment | Repository>,
  auth_url?: Resolved<ValidatedUrl, Flag | Environment | Repository>,
  project_default?: Resolved<NonEmptyString, Repository>,
  bearer_token?: Secret<Environment>,              // CODEMIE_TOKEN only; --token flag not accepted
  keycloak_credentials?: {client_id: NonSecret<Flag | Environment>,  // --client-id / CODEMIE_CLIENT_ID
                           client_secret: Secret<Environment>},      // CODEMIE_CLIENT_SECRET only; --client-secret flag not accepted
  keycloak_ropc_credentials?: {                                       // Mode (c) — v26; no client_secret
                           client_id: NonSecret<Flag | Environment>,  // --client-id / CODEMIE_CLIENT_ID; defaults to "codemie-sdk" if unset
                           email: NonSecret<Flag | Environment>,      // --email / CODEMIE_EMAIL
                           password: Secret<Environment>},            // CODEMIE_PASSWORD only; --password flag not accepted
  local_credentials?: {email: NonSecret<Flag | Environment>,  // --email / CODEMIE_EMAIL
                       password: Secret<Environment>},        // CODEMIE_PASSWORD only; --password flag not accepted
  auth_mode?: bearer | keycloak_client_credentials | keycloak_ropc | local_auth,
  timeouts,
  output_mode
}
```

`target_url` resolves `--url` > `CODEMIE_URL` > config `url`; `auth_url`
resolves `--auth-url` > `CODEMIE_AUTH_URL` > config `auth_url`. Keycloak
requires `auth_url` and never derives it from `target_url` or another value.
The target and auth URLs are transport configuration and never output members.
Secret wrappers are non-debug/non-serializable and zeroized where practical.
Secret credentials (`bearer_token`, `client_secret`, `password`) resolve from
their named environment variable only; the corresponding CLI flags (`--token`,
`--client-secret`, `--password`) are not accepted and their presence is an
`E_USAGE` exit-2 failure before network access. Non-secret selectors
(`client_id`, `email`) resolve flag over environment. No credential has a
repository-config or built-in default. Repository config is the closed
`{url?,auth_url?,project?}` model; unknown or credential keys fail locally.
(SEC-001 remediation, v25.)

Login mode selection (`auth_mode`) is determined before any network access:

1. `keycloak_client_credentials` — **Mode (a)**: `CODEMIE_CLIENT_SECRET` is
   set AND `auth_url` is configured. Populates `keycloak_credentials`.
2. `keycloak_ropc` — **Mode (c)** (v26): `CODEMIE_CLIENT_SECRET` is **not**
   set, `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` are set, AND `auth_url` is
   configured. Populates `keycloak_ropc_credentials`. `CODEMIE_CLIENT_ID`
   defaults to `codemie-sdk` if unset.
3. `local_auth` — **Mode (b)**: `CODEMIE_CLIENT_SECRET` is **not** set,
   `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` are set, AND `auth_url` is **not**
   configured. Populates `local_credentials`.

Any other combination is `E_CONFIGURATION`, exit 2, before network access.

One HTTP POST or PUT is the server transaction boundary; cross-entity apply is
caller-ordered. A valid repeat invocation against an existing identity sends a
new PUT and reports `updated`. The Skill create-409 exception performs one
bounded full re-resolution and never repeats POST. Post-write identity
verification does not inspect desired-state equality. Serialized CI and
governed external identity writers contain races. Recovery is Git revert and a
new apply for writable fields, or manual platform remediation for duplicate or
invalid identity state.

## 10. Identifier constraints (SEC-005)

Identity-bearing string fields admitted to text or JSON output must satisfy
the following constraints. The schema definitions in
`contracts/declaration-v1alpha1.schema.json` and `contracts/outcome.schema.json`
enforce these at validation time.

```text
SafeIdentifier = string satisfying:
  minLength as defined per field (see individual schema definitions)
  maxLength as defined per field:
    project (all kinds):    maxLength 100
    Assistant/Workflow slug: maxLength 100
    Skill name:              maxLength 64  (aligns with schema definition)
    Datasource repo_name:    maxLength 50  (aligns with schema definition)

  no C0/C1 control characters:
    U+0000–U+001F (NUL, CR, LF, TAB, and all other C0 controls)
    U+007F (DEL)
    U+0080–U+009F (C1 controls)
  no bidi formatting controls:
    U+202A–U+202E (LEFT-TO-RIGHT EMBEDDING, RIGHT-TO-LEFT EMBEDDING,
                   POP DIRECTIONAL FORMATTING, LEFT-TO-RIGHT OVERRIDE,
                   RIGHT-TO-LEFT OVERRIDE)
    U+2066–U+2069 (LEFT-TO-RIGHT ISOLATE, RIGHT-TO-LEFT ISOLATE,
                   FIRST STRONG ISOLATE, POP DIRECTIONAL ISOLATE)
```

Field paths in diagnostics and warnings (`source.fieldPath`) are generated
canonically by the YAML parser from the AST structure. Source file paths
(`source.file`) are bounded to 4,096 bytes. These limits align with the
`contracts/diagnostic.schema.json` and `contracts/warning.schema.json`
definitions.

## 11. Resource budgets (SEC-003)

The following are normative versioned defaults for all local and remote
resource dimensions. These limits apply starting at v25. Limits that would
reject currently-valid declarations require product-spec-owner review before
being changed.

**Flagged for product-spec-owner review**: The 1 MiB YAML-per-file limit
and 128 MiB aggregate upload limit are conservative CI defaults. If any
currently-deployed declaration exceeds these values, the product-spec-owner
must confirm revised limits before implementation enforces them.

```text
ResourceBudgets_v1 = {
  // Local parsing
  yaml_bytes_per_file:         1 MiB     (1,048,576 bytes)
  yaml_documents_per_file:     1
  yaml_nesting_depth:          32 levels
  yaml_alias_expanded_nodes:   1,000
  yaml_scalar_bytes:           128 KiB
  yaml_collection_members:     10,000    (per array or object)
  repository_files_visited:    10,000

  // File/sidecar uploads
  sidecar_file_bytes:          32 MiB    (per file)
  aggregate_upload_bytes:      128 MiB   (all parts in one invocation)
  multipart_parts:             10        (File Datasource; matches schema cap)
  multipart_basename_bytes:    255       (filesystem maximum)
  query_params_total_bytes:    8 KiB

  // HTTP responses
  response_header_bytes:       16 KiB
  response_body_bytes:         8 MiB
  json_nesting_depth:          64 levels (response decode)

  // Pagination / enumeration
  pagination_pages:            1,000     (per entity scan)
  pagination_items_total:      100,000   (duplicate-tracking set)

  // Retries and timing
  get_retry_attempts:          3
  request_timeout_seconds:     60
  invocation_deadline_seconds: 300       (5 minutes; entire lint or apply)

  // Concurrency
  concurrent_requests:         1         (sequential; one entity per invocation)
}
```

Enforcement order:
1. Reject before allocation or network where possible
2. Stream files and response bodies with bounded read; never accumulate full
   content before the budget check
3. Deadline cancellation propagates to parsing, file reads, retries, and
   pagination — all in-flight operations must respect the invocation deadline
