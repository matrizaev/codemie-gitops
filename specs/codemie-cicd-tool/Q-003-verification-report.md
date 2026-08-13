# Verification Report: Q-003 — Verify Peer Datasource Ordinary-CRUD Mappings

**Status**: PASS — 0 blocking findings. 0 high findings. Implementation task D-001 may proceed.
**Date**: 2026-08-10
**Eligibility**: ARCHITECTURE-REVIEW
**Depends on**: Q-001 (PASS, 2026-08-10)

---

## Scope

Task: independently validate every bundled per-kind create/update mapping as one Datasource contract.

Requirements in scope: FR-006/021/022/036, DR-010–012, IR-008, QR-002, VR-014–016.

Artifacts examined (no implementation files, no `codemie/` or `codemie-ui/`):

- `specs/codemie-cicd-tool.md` (v26) — §15 Datasource inventory, DR-010–012, FR-006/021/022/036, AC-IR-008-01, AC-DR-010-01/02, AC-DR-011-01
- `contracts/adapter-manifest-v2.42.0.json` — Datasource entity, all 10 type entries, classificationByKind
- `contracts/declaration-v1alpha1.schema.json` — all 10 per-kind datasourceSpec branches
- `contracts/declaration-v1alpha1.md` — §6 Datasource discriminated union
- `contracts/http-adapter.md` — §7 Datasource, §3 request projection
- `contracts/cli.md` — §5 apply
- `adr/009-datasource-ordinary-crud-boundary.md` — decision, field handling, File exception
- `plan.md` — §10 API and Datasource mapping, §7 write policy, §7 presence and null policy
- `data-model.md` — §6 Datasource union, §7 request planning
- `Q-001-verification-report.md` — prior positive/negative probe evidence

---

## 1. Sources of Truth Applied

| Document | Role |
|---|---|
| `specs/codemie-cicd-tool.md` v26 | Approved product specification |
| `contracts/adapter-manifest-v2.42.0.json` | Pinned server operation contract (normative) |
| `contracts/declaration-v1alpha1.schema.json` | Authorable field inventory (normative) |
| `adr/009-datasource-ordinary-crud-boundary.md` | Accepted architecture decision |
| `contracts/declaration-v1alpha1.md` | Contract prose; cannot widen schema |
| `contracts/http-adapter.md` | Transport and projection rules |

---

## 2. Methodology

For each of the 10 authorable types, the following checks were performed:

1. Every field in the schema spec is located in the manifest's authorFields or is explicitly classified as identity-injected, authoring-only, or prohibited.
2. The create request field list and update request field list are compared; the difference must equal the declared createOnlyFields (or updateOnlyFields for File).
3. Schema nullability (`type: ["T","null"]`) is mapped to manifest `optionalNull` classification; schema required fields map to `authoringRequired`.
4. Server-owned fields (id, created_by, processed_files, new_project_name, access_token) are absent from all request fields.
5. File null-encoding exception is verified against the manifest transportTransform and ADR-009.
6. Provider and Bedrock absence is verified against manifest status and ADR-009 reasons.

---

## 3. Per-Type Field Mapping Analysis

### 3.1 git

**Routes**: `POST /v1/application/{project}/index` (create), `PUT /v1/application/{project}/index/{repo_name}` (update)

**Schema required fields**: `description`, `link`, `branch`, `indexType`, `projectSpaceVisible`
**Schema optionalNull fields**: `filesFilter`, `embeddingsModel`, `summarizationModel`, `prompt`, `docsGeneration`, `setting_id`, `guardrail_assignments`, `cron_expression`, `timezone`
**Schema discriminator**: `index_type: "git"` (authoring-only, selects route; not a request body field)

**Manifest authoringRequired**: `["description", "link", "branch", "indexType", "projectSpaceVisible"]` — matches schema ✓
**Manifest optionalNull**: `["filesFilter", "embeddingsModel", "summarizationModel", "prompt", "docsGeneration", "setting_id", "guardrail_assignments", "cron_expression", "timezone"]` — matches schema ✓
**Manifest createOnlyFields**: `["indexType", "summarizationModel"]`
**Create fields**: name + description + link + branch + filesFilter + indexType + embeddingsModel + summarizationModel + prompt + docsGeneration + projectSpaceVisible + setting_id + guardrail_assignments + cron_expression + timezone
**Update fields**: name + description + prompt + embeddingsModel + projectSpaceVisible + docsGeneration + branch + link + filesFilter + setting_id + guardrail_assignments + cron_expression + timezone
**Create–Update diff**: `{indexType, summarizationModel}` — exactly equals createOnlyFields ✓
**Identity injection**: `metadata.project → route {project}`, `metadata.repo_name → create name / update route {repo_name}` — explicitly documented ✓
**Server-owned absent from all requests**: ✓

**Verdict: PASS**

---

### 3.2 svn

**Routes**: `POST /v1/application/{app_name}/index/svn` (create), `PUT /v1/application/{app_name}/index/svn/{repo_name}` (update)

**Schema required fields**: `description`, `link`, `branch`, `indexType`, `projectSpaceVisible`
**Schema optionalNull fields**: `filesFilter`, `embeddingsModel`, `summarizationModel`, `prompt`, `docsGeneration`, `setting_id`, `guardrail_assignments`, `cron_expression`, `timezone`
**Schema `link` pattern**: extends git pattern to allow `svn://` and `svn+ssh://` ✓

**Manifest authoringRequired**: `["description", "link", "branch", "indexType", "projectSpaceVisible"]` ✓
**Manifest optionalNull**: matches git pattern ✓
**Manifest createOnlyFields**: `["indexType", "summarizationModel"]`
**Create–Update diff**: `{indexType, summarizationModel}` — exactly equals createOnlyFields ✓
**Identity injection**: `metadata.project → route {app_name}`, `metadata.repo_name → create name / update route {repo_name}` — explicitly documented ✓

**Verdict: PASS**

---

### 3.3 confluence

**Routes**: `POST /v1/index/knowledge_base/confluence`, `PUT /v1/index/knowledge_base/confluence`

**Schema required fields**: `description`, `cql`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `setting_id`, `include_restricted_content`, `include_archived_content`, `include_attachments`, `include_comments`, `keep_markdown_format`, `keep_newlines`, `embedding_model`, `cron_expression`, `timezone`

**Manifest authoringRequired**: `["description", "cql"]` ✓
**Manifest optionalNull**: all 12 optional fields match schema ✓
**Manifest createOnlyFields**: `["include_restricted_content", "include_archived_content", "include_attachments", "include_comments", "keep_markdown_format", "keep_newlines", "embedding_model"]`
**Create–Update diff**: exactly equals createOnlyFields ✓
**Identity injection**: `metadata.project → project_name (body)`, `metadata.repo_name → name (body)` — implied by common authoringOnly and request field presence ✓

**Verdict: PASS**

---

### 3.4 jira

**Routes**: `POST /v1/index/knowledge_base/jira`, `PUT /v1/index/knowledge_base/jira`

**Schema required fields**: `description`, `jql`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`

**Manifest authoringRequired**: `["description", "jql"]` ✓
**Manifest optionalNull**: 6 fields match schema ✓
**Manifest createOnlyFields**: `["embedding_model"]`
**Create–Update diff**: `{embedding_model}` — exactly equals createOnlyFields ✓

**Verdict: PASS**

---

### 3.5 xray

**Routes**: `POST /v1/index/knowledge_base/xray`, `PUT /v1/index/knowledge_base/xray`

**Schema required fields**: `description`, `jql`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`

Field structure identical to jira; separate route and separate manifest entry. ✓
**Manifest createOnlyFields**: `["embedding_model"]`
**Create–Update diff**: `{embedding_model}` ✓

**Verdict: PASS**

---

### 3.6 azure_devops_wiki

**Routes**: `POST /v1/index/knowledge_base/azure_devops_wiki`, `PUT /v1/index/knowledge_base/azure_devops_wiki`

**Schema required fields**: `description`, `wiki_query`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `wiki_name`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`

**Manifest authoringRequired**: `["description", "wiki_query"]` ✓
**Manifest optionalNull**: `["project_space_visible", "guardrail_assignments", "wiki_name", "setting_id", "embedding_model", "cron_expression", "timezone"]` — matches schema ✓
**Manifest createOnlyFields**: `["embedding_model"]`
**Create fields include**: `wiki_name`, `wiki_query`; **Update fields include**: `wiki_name`, `wiki_query` (wiki_name is optional-null and updatable; only embedding_model is create-only) ✓
**Create–Update diff**: `{embedding_model}` ✓

**Verdict: PASS**

---

### 3.7 azure_devops_work_item

**Routes**: `POST /v1/index/knowledge_base/azure_devops_work_item`, `PUT /v1/index/knowledge_base/azure_devops_work_item`

**Schema required fields**: `description`, `wiql_query`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`

**Manifest authoringRequired**: `["description", "wiql_query"]` ✓
**Manifest optionalNull**: 6 fields match schema ✓
**Manifest createOnlyFields**: `["embedding_model"]`
**Create–Update diff**: `{embedding_model}` ✓

**Verdict: PASS**

---

### 3.8 sharepoint

**Routes**: `POST /v1/index/knowledge_base/sharepoint`, `PUT /v1/index/knowledge_base/sharepoint`

**Schema required fields**: `description`, `site_url`, `auth_type`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `include_pages`, `include_documents`, `include_lists`, `max_file_size_mb`, `files_filter`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`, `oauth_client_id`, `oauth_tenant_id`

**Manifest authoringRequired**: `["description", "site_url", "auth_type"]` ✓
**Manifest optionalNull**: all 13 optional fields match schema ✓
**Manifest createOnlyFields**: none — `operationInapplicableOnUpdate: []` (explicitly empty)
**Create == Update field set**: both contain the same 19 fields ✓
**Prohibited**: `access_token` absent from schema (`additionalProperties: false`) and listed in manifest prohibited ✓
**Note**: Unlike all other KB types, SharePoint allows `embedding_model` updates (not create-only). This is the specified behavior per manifest; the empty `operationInapplicableOnUpdate` list is intentional.

**Verdict: PASS**

---

### 3.9 file

**Routes (singular)**: `POST /v1/index/knowledge_base/file` (create), `PUT /v1/index/knowledge_base/file` (update) — multipart/form-data plus query parameters on both methods

**Schema required fields**: `description`, `files` (array, minItems 1, maxItems 10), `include_email_attachments`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `uploaded_files`, `csv_separator`, `csv_start_row`, `csv_rows_per_document`, `embedding_model`

**Manifest authoringRequired**: `["description", "files", "include_email_attachments"]` ✓
**Manifest optionalNull**: `["project_space_visible", "guardrail_assignments", "uploaded_files", "csv_separator", "csv_start_row", "csv_rows_per_document", "embedding_model"]` — matches schema ✓
**Manifest operationInapplicableOnCreate**: `["uploaded_files"]` (update-only field)

**Null-encoding exception verification** (ADR-009 requirement):
- Transport: multipart/form-data for `files` parts; all other fields are query parameters
- Manifest `transportTransform.nullEncoding`: "there is no JSON-property channel; a logical nullable value is represented only by an absent query parameter and reaches the request model as None"
- ADR-009: "Its nullable logical model values use absent query parameters because that transport has no JSON-null token"
- http-adapter.md §7: "The logical typed projection contains null for omitted/explicit-null optional members; the query codec represents null by omitting that parameter"
- Result: null-encoding exception is correctly and consistently specified across all three sources ✓

**Singular route verification**: Both POST and PUT use `POST/PUT /v1/index/knowledge_base/file` (same base path, different methods). ADR-009 describes "the singular `/v1/index/knowledge_base/file` multipart route". ✓

**Transport transform completeness**:
- `files` → repeated UploadFile multipart parts ✓
- `uploaded_files` → JSON-encoded UTF-8 compact array query parameter (PUT only) ✓
- `guardrail_assignments` → JSON-encoded UTF-8 compact array query parameter ✓
- All other scalar fields → FastAPI Depends scalar query parameters ✓
- `new_project_name` → prohibited in manifest readOnlyOrProhibited ✓

**Dual classification of `uploaded_files`**: classified as both `optionalNull` and `operationInapplicableOnCreate`. This is internally consistent:
- On CREATE: operationInapplicable takes precedence — parameter is entirely absent
- On UPDATE: optionalNull applies — if null/omitted in YAML, parameter is absent (per null-encoding exception)
The implementation must apply operationInapplicableOnCreate before optionalNull on update; both resolve to absent parameter. This is a documented non-trivial interaction (see Finding VER-Q003-001 below).

**Verdict: PASS** (with NOTE VER-Q003-001)

---

### 3.10 google

**Routes**: `POST /v1/index/knowledge_base/google`, `PUT /v1/index/knowledge_base/google`

**Schema required fields**: `description`, `googleDoc`, `setting_id`
**Schema optionalNull fields**: `project_space_visible`, `guardrail_assignments`, `embedding_model`, `cron_expression`, `timezone`

**Manifest authoringRequired**: `["description", "googleDoc", "setting_id"]` ✓
**Manifest optionalNull**: `["project_space_visible", "guardrail_assignments", "embedding_model", "cron_expression", "timezone"]` ✓
**Manifest createOnlyFields**: `["googleDoc", "setting_id", "embedding_model"]`

**Create-only field verification** (task requirement §6):
- `googleDoc` is required in schema (not nullable) and create-only in manifest ✓
- `setting_id` is required in schema (not nullable) and create-only in manifest ✓
- `embedding_model` is optionalNull in schema and create-only in manifest ✓
- Update request fields: `["name", "project_name", "description", "project_space_visible", "cron_expression", "timezone", "guardrail_assignments"]` — `googleDoc`, `setting_id`, and `embedding_model` are all absent ✓
- `operationInapplicableOnUpdate: ["googleDoc", "setting_id", "embedding_model"]` ✓

**Create–Update diff**: `{googleDoc, setting_id, embedding_model}` — exactly equals createOnlyFields ✓

**Verdict: PASS**

---

## 4. Provider and Bedrock Rejection Verification

### 4.1 Provider

**Schema**: `datasourceSpec.oneOf` contains no `provider` branch. `additionalProperties: false` at each branch ensures no undeclared index_type values. Negative probe N9 in Q-001 confirmed `provider` is rejected by schema. ✓

**Manifest**: `"provider": {"status": "conditionally-authorable", "condition": "an exact deployment-provider schema is bundled before accepting a declaration"}`

**ADR-009**: "Provider declarations require an exact reviewed deployment schema to be bundled before admission. None is present in this baseline."

**Match**: manifest condition ↔ ADR-009 rationale ✓

**Conclusion**: Provider is correctly absent from the authorable schema. The manifest records the gate condition that would enable it in a future baseline. No implementation path for provider exists in this baseline.

### 4.2 Bedrock

**Schema**: `datasourceSpec.oneOf` contains no `bedrock` branch. Negative probe N8 in Q-001 confirmed `bedrock` is rejected by schema. ✓

**Manifest**: `"bedrock": {"status": "unsupported", "reason": "pinned source exposes vendor import rather than ordinary Datasource CRUD"}`

**ADR-009**: "The pinned Bedrock vendor import is not ordinary CRUD and remains unsupported."

**Match**: manifest reason ↔ ADR-009 rationale — both cite non-ordinary-CRUD nature ✓

**Conclusion**: Bedrock is correctly absent and the rejection reason is consistent.

---

## 5. Cross-Cutting Verification

### 5.1 Optional-null handling (FR-021, VR-016)

All optional fields across all 10 types are typed `["T", "null"]` or `["array", "null"]` in the schema, indicating JSON null acceptance. The manifest `projectionPolicy.optionalNull` states: "omission or YAML null becomes an explicit JSON null in every applicable JSON request." The per-type `optionalNull` lists match the schema nullable types exactly.

File transport exception applies: the null-encoding for File's optionalNull fields is absent query parameter rather than JSON null, documented in `transportTransform.nullEncoding` and http-adapter.md §7.

No required field (type: string/boolean without null) appears in any `optionalNull` list. ✓

### 5.2 Server-owned exclusion (DR-011)

Common `readOnlyOrProhibited`: `["id", "created_by", "processed_files", "new_project_name", "SharePoint access_token", "dedicated lifecycle-control parameters"]`

None of these appear in any `createRequestFields` or `updateRequestFields` for any type. ✓

### 5.3 No equality-based write suppression (QR-002, FR-006)

`manifest.Datasource.ordinaryCrudOnly: true` ✓
ADR-009: "Existing entities receive PUT on every valid invocation." ✓
plan.md §7: "Rejected: field-state-dependent write suppression, because v24 explicitly defines repeat apply as update." ✓
http-adapter.md §3: "No ETag/conditional-write contract exists. A modifying operation is sent once and followed by bounded exact identity re-resolution." ✓
cli.md §5: "No field-state test suppresses the selected write. Every valid invocation performs one modifying request." ✓

### 5.4 Dedicated lifecycle surface absence (FR-036)

`manifest.Datasource.dedicatedLifecycleSurface: "not exposed by codemie-gitops"` ✓
ADR-009: "The CLI publishes no dedicated Datasource lifecycle command, flag, or endpoint." ✓
cli.md §1: "No plan, delete, batch, generic-adoption, or dedicated Datasource lifecycle option/command exists." ✓

### 5.5 Create-only field exclusion on update (DR-011)

Verified for each type that create-only fields in the manifest are absent from updateRequestFields:
- git/svn: `indexType`, `summarizationModel` absent from update ✓
- confluence: 7 fields absent from update ✓
- jira/xray/azure_devops_wiki/azure_devops_work_item: `embedding_model` absent from update ✓
- sharepoint: no create-only fields (empty operationInapplicableOnUpdate) ✓
- file: `uploaded_files` absent from create (operationInapplicableOnCreate) ✓
- google: `googleDoc`, `setting_id`, `embedding_model` absent from update ✓

### 5.6 Field name casing (DR-010)

- git/svn use camelCase for some fields: `filesFilter`, `indexType`, `embeddingsModel`, `summarizationModel`, `docsGeneration`, `projectSpaceVisible` — these match the server request field names in the manifest sourceEvidence path ✓
- All KB types (confluence, jira, xray, azure_devops_wiki, azure_devops_work_item, sharepoint, google, file) use snake_case — matching server request field names ✓
- The schema preserves this asymmetry; no renaming for consistency occurs ✓

---

## 6. Peer Matrix — Acceptance Evidence

This matrix constitutes the acceptance evidence required by Q-003. For each type it shows: (A) author fields required for create, (B) fields present in create but excluded from update (create-only), (C) fields present in both create and update, (D) fields present in update but not create (update-only), and (E) repeat-PUT confirmation.

### git

**A — Required on create (authoringRequired)**:
`name` (from repo_name), `description`, `link`, `branch`, `indexType`, `projectSpaceVisible`

**B — Create-only (absent from update)**:
`indexType`, `summarizationModel`

**C — Shared create+update (optionalNull)**:
`description`, `prompt`, `embeddingsModel`, `projectSpaceVisible`, `docsGeneration`, `branch`, `link`, `filesFilter`, `setting_id`, `guardrail_assignments`, `cron_expression`, `timezone`

**D — Update-only**: none

**E — Repeat PUT**: no equality branch; existing entity always receives `PUT /v1/application/{project}/index/{repo_name}`

**Negative fixture**: A declaration that includes `indexType` must send it on create. On update (repeat apply), the projector excludes `indexType` and `summarizationModel`; their presence in the request body would be a bug.

---

### svn

Identical pattern to git except:
- Create route: `POST /v1/application/{app_name}/index/svn`
- Update route: `PUT /v1/application/{app_name}/index/svn/{repo_name}`
- `link` accepts `svn://` and `svn+ssh://` URL schemes in addition to `https?://`

---

### confluence

**A — Required on create**: `name`, `project_name`, `description`, `cql`

**B — Create-only**:
`include_restricted_content`, `include_archived_content`, `include_attachments`, `include_comments`, `keep_markdown_format`, `keep_newlines`, `embedding_model`

**C — Shared create+update (optionalNull)**:
`project_space_visible`, `setting_id`, `cron_expression`, `timezone`, `guardrail_assignments`

**D — Update-only**: none

**E — Repeat PUT**: `PUT /v1/index/knowledge_base/confluence`

**Negative fixture**: Sending `embedding_model` on a confluence update would be a bug; the manifest's updateRequestFields omits it. The projector must not fabricate an update position for it.

---

### jira

**A — Required on create**: `name`, `project_name`, `description`, `jql`
**B — Create-only**: `embedding_model`
**C — Shared create+update**: `project_space_visible`, `setting_id`, `cron_expression`, `timezone`, `guardrail_assignments`
**D — Update-only**: none
**E — Repeat PUT**: `PUT /v1/index/knowledge_base/jira`

---

### xray

Same structure as jira. Route: `.../xray`.

---

### azure_devops_wiki

**A — Required on create**: `name`, `project_name`, `description`, `wiki_query`
**B — Create-only**: `embedding_model`
**C — Shared create+update**: `project_space_visible`, `wiki_name`, `setting_id`, `cron_expression`, `timezone`, `guardrail_assignments`
**D — Update-only**: none
**E — Repeat PUT**: `PUT /v1/index/knowledge_base/azure_devops_wiki`

---

### azure_devops_work_item

**A — Required on create**: `name`, `project_name`, `description`, `wiql_query`
**B — Create-only**: `embedding_model`
**C — Shared create+update**: `project_space_visible`, `setting_id`, `cron_expression`, `timezone`, `guardrail_assignments`
**D — Update-only**: none
**E — Repeat PUT**: `PUT /v1/index/knowledge_base/azure_devops_work_item`

---

### sharepoint

**A — Required on create**: `name`, `project_name`, `description`, `site_url`, `auth_type`
**B — Create-only**: none
**C — Shared create+update (all fields sent both times)**:
`project_space_visible`, `include_pages`, `include_documents`, `include_lists`, `max_file_size_mb`, `files_filter`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`, `oauth_client_id`, `oauth_tenant_id`, `guardrail_assignments`
**D — Update-only**: none
**E — Repeat PUT**: `PUT /v1/index/knowledge_base/sharepoint`

**Negative fixture**: Unlike all other KB types, `embedding_model` is NOT create-only for sharepoint — it must be sent on update as well (if non-null). The projector must not treat it as operationInapplicable on update for this type. `access_token` must never appear in any request (prohibited by schema's `additionalProperties: false` and manifest).

---

### file

**A — Required on create**: `name`, `project_name`, `description`, `files` (multipart parts), `include_email_attachments`
**B — Create-only**: none (file has update-only instead)
**C — Shared create+update**: `project_space_visible`, `csv_separator`, `csv_start_row`, `csv_rows_per_document`, `embedding_model`, `guardrail_assignments` (all as query params or JSON-encoded query params), plus `files` multipart parts
**D — Update-only**: `uploaded_files` (JSON-encoded UTF-8 compact array query parameter; operationInapplicableOnCreate)
**E — Repeat PUT**: `PUT /v1/index/knowledge_base/file` (multipart)

**Null encoding**: All optional scalar fields (csv_separator, csv_start_row, csv_rows_per_document, embedding_model, project_space_visible) are absent query parameters when null; guardrail_assignments is an absent query parameter when null; files is always present as multipart parts (required).

**Negative fixture — create**: `uploaded_files` must be absent; including it in a create request is a bug.
**Negative fixture — update**: all scalar optionalNull fields omitted or null → absent query parameters; files (required) → multipart parts; uploaded_files provided or null → either JSON-encoded array or absent query parameter.

**Positive fixture — repeat PUT**: second apply sends `PUT /v1/index/knowledge_base/file` with identical multipart parts and query parameters; no equality check suppresses the PUT.

---

### google

**A — Required on create**: `name`, `project_name`, `description`, `googleDoc`, `setting_id`
**B — Create-only**: `googleDoc`, `setting_id`, `embedding_model`
**C — Shared create+update**: `project_space_visible`, `cron_expression`, `timezone`, `guardrail_assignments`
**D — Update-only**: none
**E — Repeat PUT**: `PUT /v1/index/knowledge_base/google`

**Negative fixture**: On repeat apply (update), the projector excludes `googleDoc`, `setting_id`, and `embedding_model`. Sending any of them in the update request body is a bug; the fields are still authored in YAML (required at declaration time for create) but must be omitted from PUT projection.

---

## 7. Repeat Existing-Entity PUT Request Capture (Cross-Type)

For all 10 types:

1. Identity resolution exhausts visible pages of `GET /v1/index?full_response=true&page={page}&per_page=100&filters={json}`.
2. Exactly one match → proved present → projector builds `Update{server_id, request}`.
3. The update request is sent unconditionally with `PUT <update_route>`.
4. No field equality check occurs between desired and current state.
5. The response is verified by post-write resolution.
6. Output is `{"action":"updated","kind":"Datasource","project":"<project>","repo_name":"<repo_name>"}`.

Source evidence in manifest: `ordinaryCrudOnly: true`, `dedicatedLifecycleSurface: "not exposed by codemie-gitops"`.
Source evidence in ADR-009: "Every valid existing-entity apply sends PUT."
Source evidence in cli.md §5: "No field-state test suppresses the selected write."

---

## 8. Optional-Null Handling Findings

All 10 types: for every field classified `optionalNull` in the manifest, the corresponding schema property has type `["T", "null"]` or `{"anyOf": [{...}, {"type": "null"}]}`. No field with a non-null type (`type: string`) appears in any optionalNull list. ✓

For JSON transport types (git, svn, confluence, jira, xray, azure_devops_wiki, azure_devops_work_item, sharepoint, google): explicit null is sent as JSON `null` for omitted/null optionalNull fields. ✓

For file transport: explicit null is represented as absent query parameter (null-encoding exception). This is correctly and consistently documented in three independent places:
- `manifest.entities.Datasource.types.file.transportTransform.nullEncoding`
- `adr/009-datasource-ordinary-crud-boundary.md` decision paragraph
- `contracts/http-adapter.md` §7

No specification gap in optional-null handling. ✓

---

## 9. Findings

```
Finding ID: VER-Q003-001
Severity: NOTE
Status: OPEN (non-blocking observation for implementation task D-001)

Title:
File Datasource `uploaded_files` dual classification requires explicit implementation sequencing

Evidence:
- contracts/adapter-manifest-v2.42.0.json, Datasource.types.file.classificationByKind.file
- "optionalNull": [..., "uploaded_files", ...]
- "operationInapplicableOnCreate": ["uploaded_files"]

Expected:
Both classifications are simultaneously present and are internally consistent per the
architecture: operationInapplicable takes precedence on create; optionalNull with
null-encoding-exception applies on update.

Actual:
The field appears in two classification lists simultaneously. The correct
interpretation is: omit the parameter entirely on create; treat as absent query
parameter (null) on update when YAML value is null or omitted.

Impact:
Implementation engineer may incorrectly apply optionalNull to create (producing
an unwanted query parameter) or may incorrectly suppress the field on update.
The null-encoding exception compounds this: even on update, null → absent
parameter (not JSON null).

Required action:
D-001 implementation must explicitly handle the dual classification by applying
operationInapplicableOnCreate first for the create path, then applying the
null-encoding exception for optionalNull fields on the update path.
The acceptance evidence for D-001 must include a fixture that proves `uploaded_files`
is absent on create and correctly encoded on update.

Owner:
implementation-engineer

Verification:
Test fixture: create request has no `uploaded_files` query parameter even if YAML
declares `uploaded_files: null`. Update request has `uploaded_files` as absent query
parameter when YAML omits it; as a JSON array string when YAML provides values.
```

---

```
Finding ID: VER-Q003-002
Severity: LOW
Status: OPEN

Title:
KB-family types lack explicit identityInjection documentation in the manifest

Evidence:
- contracts/adapter-manifest-v2.42.0.json: identityInjection is documented only for
  git and svn entries
- KB types (confluence, jira, xray, azure_devops_wiki, azure_devops_work_item,
  sharepoint, google, file) have no identityInjection field in their manifest entries

Expected:
Explicit documentation of how metadata.project maps to project_name and
metadata.repo_name maps to name in the KB request body, as is done for git/svn
(which document route parameter injection).

Actual:
The mapping is implied by (a) metadata.project and metadata.repo_name appearing in
commonFieldClasses.authoringOnly, and (b) name and project_name appearing in all
KB createRequestFields and updateRequestFields. The intent is unambiguous but
not explicitly stated in the per-type entry.

Impact:
Implementation engineer must correctly infer the identity injection rule.
No architectural gap exists; the information is derivable. However, an explicit
identityInjection entry would remove the inference requirement and reduce D-001
defect risk.

Required action:
Optional: solution-architect may add identityInjection documentation to KB type
entries in the manifest for implementation clarity. No blocking impact.

Owner:
solution-architect (optional improvement, non-blocking)

Verification:
Not required for architectural approval. For implementation, test that create and
update requests for confluence/jira/xray/azure_devops_wiki/azure_devops_work_item/
sharepoint/google/file include name == metadata.repo_name and
project_name == effective_project in the request body.
```

---

```
Finding ID: VER-Q003-003
Severity: NOTE
Status: INFORMATIONAL

Title:
SharePoint embedding_model is updatable; all other KB types make it create-only

Evidence:
- manifest classificationByKind.sharepoint: operationInapplicableOnUpdate: [] (empty)
- manifest classificationByKind.confluence/jira/xray/azure_devops_wiki/
  azure_devops_work_item: operationInapplicableOnUpdate includes "embedding_model"
- manifest classificationByKind.google: operationInapplicableOnUpdate includes
  "embedding_model"

Expected:
Type-specific behavior is acceptable given separate server models (per
rest_api/models/index.py sourceEvidence).

Actual:
SharePoint allows embedding_model updates; other KB types do not. This is
consistent with the manifest specification.

Impact:
No specification gap. Implementation must not generalize the embedding_model
create-only rule to SharePoint.

Required action:
D-001 must include a per-type test that confirms sharepoint embedding_model is
sent on update while confluence/jira/xray/azure_devops_wiki/
azure_devops_work_item/google embedding_model is excluded from update.

Owner:
implementation-engineer (test coverage)

Verification:
Test fixture: SharePoint update request includes embedding_model when authored.
Confluence update request excludes embedding_model regardless of authored value.
```

---

## 10. Requirements Coverage

| Requirement | Status | Evidence |
|---|---|---|
| FR-006 — no write suppression, always create or update | SATISFIED | manifest ordinaryCrudOnly, ADR-009, cli.md §5, http-adapter.md §3 |
| FR-021 — omitted optionalNull → explicit null | SATISFIED | manifest projectionPolicy.optionalNull; classificationByKind optionalNull lists match schema; File null-encoding exception documented |
| FR-022 — required/optional classification per operation | SATISFIED | classificationByKind.xxx.authoringRequired and optionalNull match schema types; createOnlyFields exclude from update |
| FR-036 — ordinary per-kind create/update; no dedicated lifecycle | SATISFIED | all 10 type entries with exact routes, fields, create/update split; dedicatedLifecycleSurface absent |
| DR-010 — closed discriminated union; exact field names | SATISFIED | 10 schema branches with additionalProperties:false; field names match server models per sourceEvidence |
| DR-011 — send exact operation-specific request; no diff or default skip | SATISFIED | createRequestFields and updateRequestFields separate per type; operationInapplicable correctly classified |
| DR-012 — field classification completeness | SATISFIED | all fields classified as authoringRequired, optionalNull, authoringOnly, operationInapplicable, or readOnlyOrProhibited |
| IR-008 — use selected kind's ordinary create/update route | SATISFIED | exact routes in manifest for all 10 types |
| QR-002 — repeat apply sends update | SATISFIED | no equality branch; every resolved existing entity receives PUT |
| VR-014 — Datasource schema validated locally before network | SATISFIED | schema has additionalProperties:false per branch; unknown fields fail schema |
| VR-015 — cross-kind fields rejected locally | SATISFIED | per-branch additionalProperties:false prevents cross-kind fields |
| VR-016 — operation nullability determines required/optional | SATISFIED | schema types match createOnlyFields; update-only correctly classified |

---

## 11. Architecture Conformance

All 10 Datasource types conform to ADR-009:
- One adapter with a closed `index_type` union
- JSON/multipart representation per manifest — not separate entities
- Create-only properties omitted on PUT — not synthesized
- Exhaustive list resolution before write
- Missing identity → create; one match → update
- Provider absent (conditional gate)
- Bedrock absent (unsupported)
- No dedicated lifecycle surface

No architecture violations found.

---

## 12. Unverified Areas

This is an architecture-review task. The following are deferred to implementation tasks as designed:

- **Actual runtime API behavior**: no network calls were made; all verification is against the pinned manifest (commit `2a481c290c99bf30ef80aadafa03d876a7f5f732`). Deployment drift is the responsibility of V-000 (Deployment-Verification task).
- **File multipart part construction**: the basename safety check (SEC-005 requirement) is documented in http-adapter.md §7 and data-model.md §11 but is not verified here — it is an implementation concern for D-001 and F-003.
- **Pagination cap enforcement**: 1,000 pages / 100,000 items caps are documented in data-model.md §11 but are an implementation concern for D-001.
- **Project-admin visibility preflight** (`GET /v1/user`): documented in ADR-012 Option A and manifest capabilityPreflight; implementation is D-001's responsibility.

---

## 13. Files Changed During Verification

This report: `specs/codemie-cicd-tool/Q-003-verification-report.md` (created)

No production code, no implementation files, no other specs files were modified.

---

## 14. Verdict

**PASS — 0 blocking findings, 0 high findings.**

All 10 authorable Datasource types have complete, consistent field mapping specifications. Every authored field in the declaration schema maps to exactly one operation field or is explicitly classified as identity-injected, authoring-only, or prohibited. Create-only and update-only asymmetries are correctly and consistently specified across schema, manifest, ADR-009, and http-adapter.md. The File null-encoding exception is correctly documented in three independent normative sources. Provider and Bedrock absence matches ADR-009 rationale in the manifest. No equality-based write suppression exists. The peer matrix above constitutes the required acceptance evidence.

Two non-blocking findings are open: VER-Q003-001 (a NOTE for D-001 implementation sequencing) and VER-Q003-002 (a LOW suggesting optional manifest documentation improvement).

Implementation task D-001 may proceed after Q-006 convergence verification and Q-005 security review, per the task dependency graph.
