# Verification Report: Q-001 — Validate Checked-in Declaration and Adapter Contracts

**Status**: PASS — 0 blocking findings. Implementation may proceed.
**Probes**: 51/51 (23 positive, 28 negative)
**Date**: 2026-08-10

## Scope

Artifacts verified (no implementation files, no `codemie/` or `codemie-ui/`):

- `contracts/declaration-v1alpha1.schema.json`
- `contracts/outcome.schema.json`
- `contracts/diagnostic.schema.json`
- `contracts/warning.schema.json`
- `contracts/repository-config.schema.json`
- `contracts/adapter-manifest-v2.42.0.json`
- `contracts/cli.md`
- `contracts/declaration-v1alpha1.md`
- `contracts/source-baseline.md`
- All ADRs

## 1. Schema Completeness — PASS

### `declaration-v1alpha1.schema.json`

- `$schema` Draft 2020-12, `$id` present. `oneOf` dispatch over all four entity kinds.
- `additionalProperties: false` at every branch including all 10 Datasource union branches.
- 10 Datasource `index_type` values covered: `git`, `svn`, `confluence`, `jira`, `xray`, `file`, `google`, `azure_devops_wiki`, `azure_devops_work_item`, `sharepoint`.
- `provider` and `bedrock` correctly absent (ADR-009).
- Reserved `meta_config` key `codemie.epam.com/gitops/workflow-identity` protected via `propertyNames: {not: {const: ...}}`.
- `content`/`contentFrom` mutual exclusivity enforced via `oneOf` in `skillSpec`.
- `contentFrom` pattern rejects absolute paths.
- Skill `name` pattern `^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$` present.
- `assistantSpec` conditional: `agent_mode=plan_execute` → `plan_prompt` required.
- `workflowSpec` conditional: `mode=Autonomous` → `supervisor_prompt` required.
- `fileDatasourceSpec`: `files` `minItems: 1, maxItems: 10`.

### `outcome.schema.json`

- `additionalProperties: false`. Required: `action`, `kind`, `project`.
- `action` enum: `["valid", "created", "updated"]`.
- Kind-specific key requirements enforced: `slug` for Assistant/Workflow, `name` for Skill, `repo_name` for Datasource. Cross-kind prohibition enforced.
- All identifier fields reject C0/C1 controls and bidi formatting characters.

### `diagnostic.schema.json`

- `additionalProperties: false`. Required: `errorCode`, `category`, `exitCode`.
- `category` enum: 10 values. Exit/category matrix enforced via `oneOf`:
  - `reconciliation`, `server-rejection` → exit 1
  - All other 8 categories → exit 2
- Each `errorCode` constrained to exactly one `category`/`exitCode` combination.

### `warning.schema.json`

- `additionalProperties: false`. Required: `warningCode`, `category`, `source`.
- Two codes: `W_SUSPECTED_PLAINTEXT_SECRET`, `W_DEPRECATED_VALUE`.

### `repository-config.schema.json` — PASS (SEC-001/SEC-002)

- `additionalProperties: false` — credential keys rejected at schema validation (SEC-001).
- `auth_url` pattern `^https://[^@#\x00-\x1f\x7f]+$`: HTTPS required, userinfo rejected, fragment rejected, C0 controls rejected (SEC-002).
- `url` pattern `^https?://[^@#\x00-\x1f\x7f]+$`: userinfo/fragment/C0 rejected; runtime loopback check handles http restriction.
- All patterns anchored (`^...$`).

## 2. Manifest Audit — PASS

- `backend.commit: "2a481c290c99bf30ef80aadafa03d876a7f5f732"` and `ui.commit: "55945d075d82e771c4a2f4238afec1eb4c79d1e1"` — exact SHA-1 hashes present, consistent with `source-baseline.md`.
- `infoEndpointIsIdentity: false` — matches ADR-004.
- `missingOrInvalidConsumedResponseField: fail-before-write` — safety barrier before writes.
- `secretCredentialFlagsForbidden: ["--token", "--client-secret", "--password"]` — SEC-001.
- All four entities covered with full field class inventories including create-only/update-only/server-owned fields.
- File Datasource null-encoding exception correctly documented.
- Google Datasource `createOnlyFields` documented.

## 3. CLI Contract — PASS

- Three subcommands: `lint`, `apply`, `login`. No lifecycle commands.
- `--token`, `--client-secret`, `--password` absent → `E_USAGE` exit 2 (SEC-001).
- Three auth modes consistent with v26 spec FR-024 (see cli.md §6):
  - Mode (a): `CODEMIE_CLIENT_SECRET` + `auth_url` → `grant_type=client_credentials`
  - Mode (c): no secret, email+password + `auth_url` → `grant_type=password`, no `client_secret` field
  - Mode (b): no secret, email+password, no `auth_url` → `POST /v1/local-auth/login`
- Exit/stream taxonomy matches `diagnostic.schema.json` oneOf matrix.
- SEC-005 output rendering rules present in §10.

## 4. Probe Evidence — 51/51 PASS

### Positive probes (23/23)

| ID | Description |
|---|---|
| P1–P4 | Assistant (with/without `plan_execute`), Workflow (with/without `Autonomous`) |
| P5–P6 | Skill with inline `content`; Skill with `contentFrom` |
| P7–P17 | All 10 Datasource index types including File at boundaries (1 file, 10 files) |
| P18–P19 | OptionalNull field omitted; OptionalNull field set to explicit null |
| P20–P22 | Valid outcome, diagnostic (reconciliation, exit 1), warning records |
| P23 | Valid repo-config (url + auth_url + project) |

### Negative probes (28/28)

| ID | Description |
|---|---|
| N1–N2 | Unknown top-level field; unknown field inside `assistantSpec` |
| N3 | Missing required `system_prompt` |
| N4–N5 | Skill with both/neither `content`/`contentFrom` |
| N6 | `contentFrom` with absolute path (`/etc/passwd`) |
| N7 | Reserved `meta_config` key injection |
| N8–N10 | Unknown `index_type`; `bedrock`; `provider` |
| N11–N12 | `plan_execute` missing `plan_prompt`; `Autonomous` missing `supervisor_prompt` |
| N13 | Repo-config with `token` credential key |
| N14–N16 | `auth_url` with `http://`; userinfo in `url`; userinfo in `auth_url` |
| N17–N18 | File Datasource 11 files; 0 files |
| N19–N22 | Invalid `action`/`kind` in outcome; wrong key for kind (Assistant+`name`, Datasource+`slug`) |
| N23–N25 | Diagnostic: wrong exitCode for category; unknown category |
| N26–N27 | Repo-config with `client_secret`; `password` |
| N28 | Required field explicitly set to `null` |

## 5. Blocking Findings

**None.**

## 6. Non-Blocking Observations

| ID | Severity | Description |
|---|---|---|
| OBS-001 | LOW | ADR-001/002/004/005/010/011 status is "Proposed" but decisions are already materialized in approved contracts. Recommend solution-architect update to "Accepted". |
| OBS-002 | LOW | `manifest.clientConfigurationContract.authUrlRequiredFor` names only `keycloak_client_credentials`; Mode (c) ROPC also requires `auth_url`. No correctness gap (cli.md §6 is normative). Recommend expanding the field for clarity. |
| OBS-003 | NOTE | `declaration-v1alpha1.md` header cites spec v24; current spec is v26. Cosmetic only. |
| OBS-004 | NOTE | `provider` and `bedrock` absence is correct; note exists for future schema-admission work. |

## ADR Status Summary

| ADR | Status |
|---|---|
| ADR-001 | Proposed (OBS-001) |
| ADR-002 | Proposed (OBS-001) |
| ADR-003 | Superseded by ADR-011 |
| ADR-004 | Proposed (OBS-001) |
| ADR-005 | Proposed (OBS-001) |
| ADR-006 | Superseded by ADR-008 |
| ADR-007 | Accepted |
| ADR-008 | Accepted |
| ADR-009 | Accepted (amended by ADR-012) |
| ADR-010 | Proposed (OBS-001) |
| ADR-011 | Proposed (OBS-001) |
| ADR-012 | Accepted — Option A, 2026-08-10 |

## Verdict

**PASS.** All schemas, manifest, and CLI contract are complete, closed, and consistent with spec v26. 51/51 probes confirm correct accept/reject behavior. Q-002, Q-003, Q-004, and downstream implementation tasks may proceed.
