  # Feature specification: CodeMie Platform CI/CD (GitOps) Tool

  ## 1. Document status

  * **Status:** DRAFT — v26 — **READY FOR IMPLEMENTATION**
  * **Tool name:** `codemie-gitops`
  * **Owner:** Product Specification Owner (pending assignment to a named product owner)
  * **Source request:** User-provided request on 2026-08-06: "create a CI/CD tool for the CodeMie platform that will be able to store assistants, workflows, datasources, and skills in YAML files, lint them, and create/update on the server side."
  * **Related Jira issues:** *None yet — recommended to open a tracker issue in the relevant CodeMie project.*
  * **Related Confluence pages:** *None yet.*
  * **Referenced repositories:**
    * `https://github.com/codemie-ai/codemie` — server (FastAPI + LangChain/LangGraph + PostgreSQL/SQLModel + Elasticsearch)
    * `https://github.com/codemie-ai/codemie-ui` — UI (out of scope for this tool)
    * `https://github.com/codemie-ai/codemie-code` — **different, existing** local-agent CLI; not to be confused with `codemie-gitops`
  * **Last reviewed:** 2026-08-10 (v26)
  * **Revision history:**
    * v1 (2026-08-06 15:27 UTC+3) — initial DRAFT, based on public docs only.
    * v2 (2026-08-06 15:53 UTC+3) — post-repo-analysis: schema, identity, cross-entity refs, auth resolved or narrowed.
    * v3 (2026-08-06 16:08 UTC+3) — OQ-9 name resolved: `codemie-gitops`.
    * v4 (2026-08-06 16:33 UTC+3) — OQ-5 resolved: continue-and-report default, `--fail-fast` opt-in (FR-018).
    * v5 (2026-08-06 16:57 UTC+3) — OQ-7 resolved: refuse-existing default, `--adopt-existing` opt-in (FR-019, FR-020).
    * v6 (2026-08-06 20:46 UTC+3) — OQ-8 initially resolved strict declarative behavior; its completeness rule was later superseded by v23 while retaining YAML as the source of authored request values.
    * v7 (2026-08-07) — Grilling session rounds 1 & 2: one entity per invocation; `plan` command removed; then-current mandatory-completeness behavior (superseded by v23); no secret interpolation in phase 1; multi-environment support deferred; GitHub Actions + GitLab CI named first-class; `codemie` assistant type only in phase 1; `codemie-gitops login` command added; Kubernetes-style envelope (`apiVersion`/`kind`/`metadata`/`spec`) confirmed; config file model defined; traceable management marker format defined; FR-018 removed; FR-021/022/023 revised; FR-024 added; OQ-10/12/13/19/21 resolved.
    * v8 (2026-08-07) — Grilling session round 3: file naming free-form (Q19); directory structure free-form, `kind` read from inside YAML (Q20); config file renamed to `.codemie/config.yaml` (Q21); `metadata.project` omittable when config provides default (Q22); `contentFrom` sidecar for skills in phase 1 (Q23); `--output json` flag added (Q24), QR-004 resolved; FR-017 updated; FR-025/026/027 added; §30 round 3 closed.
    * v9 (2026-08-07) — Source code analysis: OQ-2 resolved (`GET /v1/info` returns server version); OQ-17 resolved (`WorkflowMode` confirmed `"Sequential"` default / `"Autonomous"` deprecated); VR-004 resolved (full enum inventory from models); OQ-16 upgraded to code-evidence-available; A-3 confirmed (local-auth only); C-6 corrected (`custom_metadata` on Assistant only, not Skill); FR-020 corrected accordingly; §3 evidence updated.
    * v10 (2026-08-07) — Product decision: no management marker in v1; FR-019 and FR-020 deleted; `--adopt-existing` flag removed; `apply` now unconditionally creates or updates by natural key; `refused-existing` and `adopted` action labels removed; managed/unmanaged terminology removed; C-6 and OQ-20 closed as N/A.
    * v11 (2026-08-07) — OQ-16 recorded an early Datasource interpretation that was later superseded; v22 is authoritative.
    * v12 (2026-08-07) — A-3 fully resolved (platform team response): authentication MUST use Keycloak OIDC `client_credentials` grant only. `POST /v1/local-auth/login` dropped as a supported CI path. FR-024 `login` command revised to wrap the OIDC token endpoint. New operational constraint: service-account credentials access Project integrations only (personal integrations ignored). `CODEMIE_CLIENT_ID`, `CODEMIE_CLIENT_SECRET`, `CODEMIE_AUTH_URL` env vars added. v24 later made the exact endpoint an explicit configuration prerequisite.
    * v13 (2026-08-07) — Product decision: re-add `POST /v1/local-auth/login` support for local dev against a dev server. `login` command now supports two modes selected by which credentials are supplied: Keycloak client_credentials (CI) vs. email/password local-auth (local dev). SC-012 added. §24 constraint relaxed to "CI MUST use Keycloak; local dev MAY use local-auth". Edge case added for 400 when server is not in local-auth mode.
    * v14 (2026-08-09) — Product decisions: Workflow authored identity remains `(project, slug)` and is persisted in a Workflow-only reserved `meta_config` identity record; unmarked legacy Workflows require explicit adoption by supplied server UUID; ordinary reconciliation never selects by display name and fails on invalid or duplicate identity records. Skill identity remains `(project, name)` and is resolved by exhaustive paginated enumeration plus exact client filtering, with zero/one/multiple matches producing create/update/ambiguity failure (update timing later refined by v22). Server IDs remain internal. Privileged visibility, serialized CI, governed UI/API writes, post-write verification, and residual-race behavior are now explicit requirements and acceptance criteria. Generic ownership markers and generic adoption remain out of scope.
    * v15 (2026-08-09) — OQ-28 resolved: all local parsing, schema, semantic-validation, repository-reference, and configuration failures use exit code 2. Exit code 1 is reserved for entity reconciliation or server-side failures reached after valid local input; exit code 2 also covers authentication/authorization preconditions, connectivity/compatibility, and fatal failures. Workflow/Skill ambiguity and invalid/unstable server identity evidence use exit 1; incomplete identity visibility or write permission uses exit 2.
    * v16 (2026-08-09) — OQ-29 resolved: failed one-entity invocations leave stdout empty and emit safe diagnostics to stderr in text or JSON mode. Raw request/response bodies, server-provided error text, payloads, credentials, headers, cookies, secret fields, and secret-like values are never persisted or printed, with no debug/verbose exception. Diagnostics are constructed from an explicit non-sensitive allowlist. The successful `login` token line remains the sole intentional token-output exception required for shell composition.
    * v17 (2026-08-09) — OQ-31 fixed Workflow persisted/inline assistant forms and `skillRefs`/`datasourceRefs` positions from current server/UI evidence. Its initial OQ-30/OQ-32 Datasource interpretations were superseded; v22 is authoritative.
    * v18 (2026-08-09) — OQ-30 retained the existing exposed request format rather than an invented integration-alias abstraction. v20 later folded that technical clarification into the ordinary per-kind Datasource inventory.
    * v19 (2026-08-09) — OQ-32 restored ordinary Datasource create/update; v22 later reduced the product boundary to ordinary per-kind CRUD and no dedicated lifecycle-control surface, with optional authorable body fields subsequently governed by v23.
    * v20 (2026-08-09) — Corrected entity terminology and Datasource scope: the four entities are Assistant, Workflow, Datasource, and Skill. Google Docs is an ordinary Datasource kind, not a separate product concern. Its format evidence was folded into the same per-kind Datasource inventory as every other current kind; dedicated scenarios, requirements, acceptance criteria, dependencies, gates, and readiness emphasis were removed.
    * v21 (2026-08-09) — Resolved PRODUCT-OQ-01/VER-011: CLI output carries no Git commit SHA, target environment origin, Git author, or CI-run identity. Git, CI, and platform logs retain their own provenance externally. FR-013 was deleted; PA-004, output/security rules, acceptance criteria, traceability, handoff, and readiness were aligned without changing successful action/kind/project/natural-key output.
    * v22 (2026-08-09) — Resolved ARCH-B01: `apply` is an always-write authoring operation. After safe identity resolution, each valid invocation creates a missing entity or updates an existing entity and reports `created` or `updated`; it does not compare desired/current state to skip a write and never reports `unchanged`. Datasource requirements were also reduced to ordinary per-kind CRUD using the existing server format, with no dedicated lifecycle-control command or endpoint and no product contract about server-internal behavior. v23 governs omitted optional body fields.
    * v23 (2026-08-09) — Resolved VER-012: optional authorable request fields may be omitted from YAML and are materialized as explicit JSON null in both applicable create and update payloads. Identity/envelope, structurally required, conditionally required, and any request field whose pinned contract rejects null remain authoring-required. Explicit YAML null has the same outbound meaning where allowed. The former mandatory-completeness/non-null-default rule was removed.
    * v24 (2026-08-09) — Resolved the Keycloak endpoint configuration decision: `login` requires an explicitly configured token endpoint from `--auth-url`, `CODEMIE_AUTH_URL`, or non-secret `.codemie/config.yaml` `auth_url`, in that precedence order. It never derives the endpoint from `CODEMIE_URL`, `url`, a hostname convention, or another value. Missing endpoint configuration is an exit-2 local failure before network access. Pre-implementation verification and the subsequent security review are lifecycle stages rather than product-readiness blockers merely because a future review artifact does not yet exist.
    * v25 (2026-08-10) — SEC-001 remediation: secret-bearing CLI flags (`--token`, `--client-secret`, `--password`) removed from the approved command surface. Bearer token, client secret, and password are now accepted exclusively through environment variables (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, `CODEMIE_PASSWORD`). The non-secret selector `--client-id` (and `--email`) MAY remain flags. Passing a secret value as a flag is an exit-2 local failure before any network access. FR-009, FR-017, FR-024, IR-006, and QR-007 updated; CLI contract, data model, tasks, and adapter manifest updated consistently.
    * v26 (2026-08-10) — Added Mode (c) Keycloak ROPC (`grant_type=password`) to login command: human-user Keycloak auth with `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` + `auth_url`; `CODEMIE_CLIENT_ID` defaults to `codemie-sdk`. Modes (a) `client_credentials` and (b) local-auth unchanged. FR-017, FR-024, IR-006, QR-007, §12 Authentication, §24 Constraints, §29 Handoff updated; SC-020 and AC-FR-024-08 added.

  ---

  ## 2. Executive summary

  `codemie-gitops` is a CLI tool that lets teams manage the four CodeMie entity types—Assistant, Workflow, Datasource, and Skill—as **YAML files in a Git repository**, using **natural keys** (project + slug/name/repo_name) for managed-asset identity and managed-asset references.

  **Core semantics** (product-decided, 2026-08-06–09):

  * **YAML supplies authored request values.** Each YAML file declares exactly one entity. The tool preserves explicit authored values and does not silently substitute a server default. For an optional authorable request field, omission and explicit YAML null both produce explicit JSON null in each applicable create/update payload. It also performs the bounded transformations required for effective identity, natural references, Skill sidecars, Workflow identity metadata, and the target API request shape. (FR-021, FR-022, DR-012)
  * **Required fields stay explicit.** Identity/envelope fields, structurally or conditionally required fields, and any field whose pinned applicable server request rejects null must be authored; omission is a local validation failure. Optional authorable fields may be omitted to reduce YAML noise. (FR-022, FR-023)
  * **One entity per invocation.** `apply` targets exactly one YAML file. CI orchestration (ordering, looping) is the caller's responsibility.
  * **Apply** creates or updates by authored natural key using the safe resolution contract for each kind. Every valid invocation sends a create for a missing entity or an update for an existing entity; the tool does not compare desired/current state to skip a write and does not report `unchanged`. Workflow uses a Workflow-only reserved server `meta_config` identity record and explicit server-ID adoption for an unmarked legacy Workflow. Skill uses exhaustive paginated resolution and refuses ambiguity. Neither kind exposes its server ID in declarations or outcomes. (FR-005, FR-006, FR-028–FR-031)
  * **No `plan` command.** The YAML is the plan.
  * **No secret interpolation.** Platform integrations handle external credentials; credentials are not stored in YAML.
  * **Non-interactive and CI-first:** `lint` (offline), `apply` (online), `login` (token acquisition).
  * **Stable exit taxonomy:** 0 = success; 1 = entity reconciliation or server-side failure after valid local input; 2 = local parsing/schema/validation/configuration failure, authentication/authorization precondition, connectivity/compatibility failure, or fatal error. (FR-011)
  * **Safe output boundary:** successful outcomes go to stdout; all failure/error diagnostics go to stderr and leave stdout empty. Raw bodies, payloads, credentials, tokens, authorization/cookie data, secret fields, and secret-like values are never logged or persisted. Diagnostics use only explicitly allowlisted non-sensitive fields. Successful `login` token stdout is the sole intentional exception. (FR-011, FR-016, FR-024, FR-026, QR-007)
  * **No deletes** in phase 1. (FR-008)

  **Status:** `READY FOR ARCHITECTURE PLANNING`. Workflow/Skill identity, always-write apply behavior, omission-to-null payload semantics, exit codes, safe diagnostics, Workflow reference shapes, inline assistants, and generic Datasource authoring are product-approved. Every authorable Datasource kind uses its ordinary existing create/update format with field requiredness/nullability pinned under DR-012. Implementation remains gated on a pinned target API contract and proof of the required visibility, pagination, metadata-preservation, and authorization behavior.

  ---

  ## 3. Evidence consulted

  ### Jira
  *No Jira issues were provided or retrieved.*

  ### Confluence
  *No Confluence pages were provided or retrieved.*

  ### Repository / web sources — CodeMie server (`github.com/codemie-ai/codemie`)

  Retrieved and analyzed from the reference-only source on 2026-08-06–09:

  * `pyproject.toml` — package `codemie` v0.8.0, Python ≥3.12, FastAPI 0.133, Pydantic 2.9, SQLAlchemy 2.0, LangChain 1.2.15, LangGraph 1.1.6. Persistence: PostgreSQL (via SQLModel) with `sqlalchemy.dialects.postgresql.JSONB` for complex fields.
  * `config/templates/assistant/` (24 files) — assistants already stored as YAML as bootstrap templates.
  * `config/templates/workflow/` (64 files) — workflows already stored as YAML as bootstrap templates.
  * `config/templates/skill/` (7 files) — skills already stored as YAML as bootstrap templates.
  * `config/datasources/datasources-config.yaml` — NOT per-datasource declarations; this is loader/chunking tuning. Datasources are runtime entities per project.
  * `src/codemie/rest_api/models/assistant.py` — `AssistantRequest` / `AssistantBase` / `Assistant`. Has `from_yaml(yaml_str, project)` classmethod. Unique index: partial `(project, slug)` where `slug IS NOT NULL`. **`custom_metadata: Optional[dict]` is present on `AssistantRequest` and `Assistant` DB model ONLY.** Confirmed enums: `AssistantType` (`"codemie"`, `"A2A"`, `"bedrock_agent"`, `"bedrock_agentcore_runtime"`); `AgentMode` (`"general"`, `"plan_execute"`). `llm_model_type` required when `type == "codemie"`.
  * `src/codemie/rest_api/models/skill.py` — `SkillCreateRequest` / `SkillUpdateRequest` / `SkillBase`. Unique index: `(name, created_by.id, project)`. Name is kebab-case, 3–64 chars (regex: `^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$`). Content is markdown 100–30 000 chars. **No `custom_metadata` on Skill model.** Confirmed enum: `SkillVisibility` (`"private"`, `"project"`, `"public"`). Create supplies defaults for `visibility` and several list fields through non-null types, while update declares nullable counterparts; under the v23 rule these fields remain authoring-required because create rejects explicit null.
  * `src/codemie/rest_api/models/index.py` — `IndexInfo` (datasource model), keyed by `(project_name, repo_name)`. Mixes desired-state and runtime-state fields. **No `custom_metadata` on `IndexInfo`.** Desired-state fields defined by `update_index()` method.
  * `src/codemie/core/workflow_models/workflow_models.py` — `CreateWorkflowRequest` / `WorkflowMode`. **Confirmed `WorkflowMode` enum: `"Sequential"` (default, non-deprecated) and `"Autonomous"` (deprecated).** Required fields: `name`, `description`, `project`. Default mode: `"Sequential"`. **No `custom_metadata` on `CreateWorkflowRequest`.**
  * `src/codemie/workflows/execution_config_schema.yaml` — authoritative JSON Schema (Draft 2020-12) for workflow `execution_config`. Directly reusable as the tool's validator.
  * `src/codemie/workflows/config_yaml_validation.py` and `config_resources_validation.py` — the server's own workflow YAML validators.
  * `src/codemie/service/kata_import_service.py` — SHA-256 content checksum pattern, kebab-case IDs, semver version, staged validation. Strongest existing pattern to reuse.
  * `src/codemie/rest_api/routers/local_auth_router.py` — `POST /v1/local-auth/login` requires **both** `config.ENABLE_USER_MANAGEMENT == True` **and** `config.IDP_PROVIDER == "local"`; returns 400 otherwise. Rate-limited 5/15 min.
  * `src/codemie/rest_api/routers/common.py` — `GET /v1/info` returns `{message, version, description}` where `version = APP_VERSION` (current: `"0.16.0"`). This is the server-side version indicator.
  * `src/codemie/rest_api/security/authentication.py` — standard auth delegates to `IdpFactory.create()`. Internal service-to-service auth uses HMAC `X-Bind-Key` (not for external use). **No static API key mechanism in base package.**
  * `src/codemie/rest_api/security/idp/factory.py` — `IdpFactory` registry: base package registers `LocalIdp` only. Enterprise providers (Keycloak, OIDC) registered at startup via `IdpFactory.register()`. In enterprise deployments, `POST /v1/local-auth/login` returns 400.
  * `src/codemie/core/constants.py` — `IdentityProvider.LOCAL = "local"` only in base. `APP_VERSION = "0.16.0"` in config.
  * `Makefile` — has `import-katas` target; no general-purpose apply target.
  * `codemie/src/codemie/rest_api/models/index.py`, `core/models.py`, `rest_api/routers/index.py`, `service/provider/datasource/`, `service/aws_bedrock/`, `codemie-ui/src/constants/dataSources.ts`, `codemie-ui/src/pages/dataSources/components/DataSourceForm/hooks/{useCreateIndex,useEditPopupForm}.ts`, and `codemie-ui/src/types/entity/dataSource.ts` (reference-only, inspected 2026-08-09) — current Datasource support is a discriminated family of per-kind request/read formats rather than one uniform subtype object. The inspected UI enumerates git, SVN, Confluence, Jira, Xray, file, Google Docs, Azure DevOps Wiki/Work Item, SharePoint, provider-backed, and Bedrock kinds. Each kind has its own exact field names, requiredness, mutability, integration-reference form, and ordinary CRUD availability; the inventory in §15 records this without introducing a client alias layer.
  * `codemie/src/codemie/core/workflow_models/workflow_models.py`, `workflows/execution_config_schema.yaml`, `workflows/validation/resources.py`, and `codemie-ui/src/types/workflowEditor/configuration.ts` (reference-only, inspected 2026-08-09) — `execution_config.assistants[].id` is a workflow-local actor ID referenced by `states[].assistant_id`; a persisted actor uses server `assistant_id`, while an inline actor omits it and supplies `system_prompt`. Skill and Datasource server IDs are stored as `skill_ids[]` and `datasource_ids[]` on each assistant entry. Current runtime resource use confirms those lists configure inline/virtual assistants.

  ### Repository — architecture and contract evidence

  Inspected on 2026-08-09. These artifacts document technical feasibility and constraints; the user-provided decisions below are the authority for the new product behavior:

  * `specs/codemie-cicd-tool/adr/007-skill-exhaustive-list-resolution.md` — the server does not enforce Skill uniqueness at `(project, name)`; exhaustive client resolution, ambiguity refusal, privileged visibility, and residual create races are required for a no-server-change solution.
  * `specs/codemie-cicd-tool/adr/008-workflow-meta-config-identity-and-adoption.md` — the current Workflow API has no persisted slug lookup; a reserved Workflow `meta_config` identity record and explicit in-place legacy adoption preserve natural-key authoring without server modification.
  * `specs/codemie-cicd-tool/plan.md`, `data-model.md`, `research.md`, `tasks.md`, and `contracts/{cli,declaration-v1alpha1,http-adapter}.md` — trace the identity decisions to visibility, pagination, failure, concurrency, output, and operational prerequisites.
  * `specs/codemie-cicd-tool/contracts/outcome.schema.json` — Workflow and Skill outcomes use authored natural keys and omit server IDs.

  ### User-provided decisions
  * Target entity types: assistants, workflows, datasources, skills.
  * Storage format: YAML files.
  * Required capabilities: store, lint, create/update on the server.
  * **Authentication (platform team response, 2026-08-07, resolves A-3; endpoint configuration refined by product decision v24):** CI auth uses Keycloak OIDC `client_credentials` grant exclusively. Service account credentials (`client_id` + `client_secret`) are obtained by raising a request at `https://epa.ms/codemie-support`. The platform response showed a deployment endpoint shaped like `https://auth.<codemie-domain>/realms/codemie-prod/protocol/openid-connect/token`; this is evidence, not a derivation convention. The operator must explicitly configure the actual endpoint under v24. Token lifetime ~8 h; should be cached. The Bearer token is then passed as `Authorization: Bearer <token>` on every API call. **Critical operational constraint:** service-account credentials operate under their own service account — only Project-level integrations (Jira, Git, etc.) are accessible; personal integrations are silently ignored. Teams must migrate any personal integrations to project level before using the tool.
  * **Local dev auth (product decision, 2026-08-07, v13):** `POST /v1/local-auth/login` (email + password) MUST also be supported in the `login` command for local development against a dev server configured with `ENABLE_USER_MANAGEMENT=True` and `IDP_PROVIDER="local"`. CI pipelines MUST use Keycloak; local-auth is for development use only.
  * **Workflow identity (product decision, 2026-08-09, v14):** authored identity remains exact `(project, slug)`. The tool persists that identity server-side only for Workflow in a reserved `meta_config` record. Workflow server UUIDs are internal transport handles and MUST NOT be authored or reported. An unmarked legacy Workflow may be adopted in place only when the operator explicitly supplies its current server UUID; adoption persists the slug identity record and reconciles desired state. Ordinary reconciliation MUST NOT select by display name and MUST fail safely for invalid or duplicate identity records.
  * **Skill identity (product decision, 2026-08-09, v14; apply behavior refined v22):** authored identity remains exact `(project, name)`. The tool resolves it by exhausting every page of the compatible list/read API and client-filtering exact project and name. Zero exact matches creates; one updates on every valid invocation; multiple exact matches fail as ambiguous. Returned server UUIDs are internal only. Because uniqueness is client-enforced rather than server-enforced, complete manager/admin visibility, serialized CI, governed UI/other-client writes, post-create verification, and visible residual-race failure are required.
  * **Exit taxonomy (product decision, 2026-08-09, v15):** 0 success; 1 entity reconciliation/server-side failure after valid local input; 2 local parsing/schema/validation/configuration, authentication/authorization/visibility, compatibility/connectivity, or fatal failure.
  * **Safe output boundary (product decision, 2026-08-09, v16):** all failures use stderr and leave stdout empty in text and JSON modes. Diagnostics are allowlist-constructed and never reproduce bodies, server error text, payloads, credentials, security headers/cookies, secret fields, or secret-like values. Successful `login` token stdout is the sole intentional exception.
  * **Datasource and Workflow authoring closure (product decisions, 2026-08-09, v17–v23):** Workflow uses the exact natural-reference forms established in FR-035 and retains inline assistants. Datasource is one of the four entities; its server-supported kinds use their existing per-kind CodeMie formats without client-invented aliases or a privileged subtype. Optional authorable fields follow DR-012. Provisioning integrations or granting access to them is uniformly out of scope.
  * **CLI provenance boundary (product decision, 2026-08-09, v21; resolves PRODUCT-OQ-01/VER-011):** Git commit SHA, target environment origin, Git author, and CI-run identity belong to Git, CI, and platform logs. The CLI MUST NOT emit them in successful output, warnings, or failure diagnostics and does not provide audit correlation metadata.
  * **Always-write apply (product decision, 2026-08-09, v22; resolves ARCH-B01):** every valid invocation creates a safely missing entity or updates an exactly resolved existing entity. The tool does not compare desired/current state to skip the request and reports only `created` or `updated`.
  * **Optional-field materialization (product decision, 2026-08-09, v23; resolves VER-012):** an optional authorable server-request field may be omitted from YAML; the CLI emits it as explicit JSON null in every applicable create or update request rather than allowing the server to select a default. Explicit YAML null produces the same payload value where null is accepted. A field that is required by structure or whose pinned applicable request rejects null remains authoring-required and omission fails locally with exit code 2.
  * **Explicit Keycloak endpoint (product decision, 2026-08-09, v24):** Keycloak `login` requires a token endpoint supplied through `--auth-url`, `CODEMIE_AUTH_URL`, or `.codemie/config.yaml` `auth_url`. The CLI never derives this URL from the CodeMie API URL or a domain/path convention. The endpoint is non-secret; client IDs, client secrets, bearer tokens, email addresses, and passwords remain flag/environment inputs only and are prohibited from repository configuration.

  ---

  ## 4. Problem statement

  Teams building on CodeMie must currently define assistants, workflows, datasources, and skills through the UI (or ad-hoc API/SDK calls). This makes it hard to:

  * Version-control platform assets alongside application code.
  * Review changes via pull requests before they take effect.
  * Reproduce environments (e.g. re-creating the same assistant set in dev, staging, and prod).
  * Recover from accidental deletions or misconfiguration.
  * Use Git, CI, and CodeMie platform logs—not CLI provenance fields—to audit who changed what, when, and why.
  * Automate rollouts as part of an existing CI/CD pipeline.

  ---

  ## 5. Business objective

  Enable teams to manage CodeMie platform assets as code, so that:

  * Asset definitions live next to the source code they support,
  * Changes are reviewed and merged through standard Git workflows, and
  * Environments are provisioned and updated automatically and repeatably.

  ---

  ## 6. Success measures

  ### Product success
  * Teams can create or update assistants, workflows, Datasources, and skills using only Git and CI, with zero UI clicks. *(binary)*
  * Number of platform assets managed via the tool as a share of all assets in target environments — **target: UNRESOLVED.**

  ### User success
  * Time to make and roll out a reviewed change is reduced compared to the current UI-driven workflow — **baseline and target: UNRESOLVED.**
  * Rate of production configuration incidents attributable to manual UI edits — **target: UNRESOLVED.**

  ### Operational success
  * Successful CLI output is limited to the per-entity reconciliation result and identity; it contains no Git commit SHA, target environment origin, Git author, or CI-run identity — **binary; MUST be true.**
  * Rollback of a bad change to fields supported by the selected kind's ordinary update contract is achievable via `git revert` + re-run — **binary; MUST be true.** A change to any create-only per-kind field requires manual replacement/remediation because delete and automatic recreation are out of scope.

  ---

  ## 7. Actors and stakeholders

  | Actor | Goal | Permissions | Responsibilities |
  |---|---|---|---|
  | **Platform asset author** | Define/change assets declaratively | Edit YAML in Git; open PRs | Correct, reviewable declarations with required fields and intentional optional omissions |
  | **Reviewer / approver** | Approve changes before rollout | Approve PRs; access CI logs | Enforce standards, catch regressions |
  | **CI runner (service account)** | Apply approved declarations | API token scoped to target environment | Non-interactive apply, produces logs |
  | **Platform administrator** | Ensure environments stay consistent | Full control; manages tokens; can override | Governance, secret rotation, access control |
  | **Local developer** | Iterate on YAML before opening a PR | Run tool locally against a dev environment | Fast feedback via lint + optional apply |
  | **Auditor** | Verify who changed what outside the CLI | Read access to Git history, CI logs, and platform audit records | Compliance; does not rely on CLI provenance output |

  ---

  ## 8. Goals

  1. Define a **declarative YAML representation** for CodeMie assistants, workflows, datasources, and skills.
  2. Provide a **linter/validator** that runs offline (no server needed) against these YAML files.
  3. Provide an **apply** operation that creates or updates declared assets in a target CodeMie environment via the platform API.
  4. Support running **in CI** (non-interactive, exit-code-driven, log-friendly) and optionally locally.
  5. Provide concise per-entity reconciliation results that external Git, CI, and platform logs may retain, without duplicating their provenance fields in CLI output.

  ---

  ## 9. Non-goals

  * Providing a runtime replacement for the CodeMie UI.
  * Managing platform-level infrastructure (server deployment, database, Keycloak, etc.).
  * Managing non-listed entity types (users, tenants, categories, integrations) — deferred, see §10.
  * Implementing the CodeMie server or SDK itself.
  * Executing assistants/workflows (runtime concerns).
  * Replacing or duplicating the existing `codemie-code` CLI's local-agent responsibilities.

  ---

  ## 10. Scope

  ### In scope

  * YAML schema definitions for: `Assistant`, `Workflow`, `Datasource`, `Skill`.
  * Offline **lint/validate** (schema + intra-repo referential integrity).
  * **Apply** (create if safely resolved as missing, update on every invocation when exactly one safe match exists) — one entity per invocation, including ordinary Datasource CRUD.
  * **Workflow-only identity persistence and legacy adoption** through the reserved server `meta_config` record and `--adopt-workflow-id <uuid>`.
  * **Skill exhaustive identity resolution** with ambiguity refusal.
  * **Datasource authoring** through the existing per-kind CodeMie formats in DR-010/011, without a client-invented common integration schema.
  * **Authentication** via credentials supplied out-of-band (env vars / CI secrets).
  * **`login` command** for CI token acquisition (`codemie-gitops login` → stdout token).
  * **CI integration** patterns — worked examples for **GitHub Actions** and **GitLab CI**.
  * Phase 1 supports **`codemie` assistant type only.**

  ### Out of scope

  * Managing users, roles, permissions, tenants inside CodeMie.
  * Executing workflows or invoking assistants.
  * UI or web front-end for the tool.
  * General-purpose config-management for arbitrary CodeMie resources.
  * Generic management/ownership markers or generic asset-adoption commands. The Workflow identity record is a narrow identity exception, not an ownership claim.
  * Dedicated Datasource lifecycle-control commands, flags, and endpoint operations, including the former proposed `--reindex-datasources` flag.

  ### Deferred

  * **Dry-run / plan** preview command.
  * **Multi-environment** overlay or templating support (env-specific values handled at CI level via env vars).
  * **Secret interpolation** syntax (`${env:X}`, vault paths).
  * **A2A**, **bedrock_agent**, **bedrock_agentcore_runtime** assistant types.
  * Managing integrations and assistant-categories.
  * **Delete** semantics (destructive operations).
  * **Drift detection** and reconciliation of out-of-band UI edits.
  * **Bidirectional sync** (server → YAML export).

  ---

  ## 11. Terminology

  | Term | Definition |
  |---|---|
  | **Asset** | A CodeMie entity managed by this tool: an assistant, workflow, datasource, or skill. |
  | **Declaration** | A YAML file describing exactly one asset. |
  | **Environment** | A specific CodeMie server instance (URL + credentials) the tool targets. |
  | **Apply** | The operation that safely resolves the declared server entity and sends one ordinary create request when missing or one ordinary update request when present. Resolution is kind-specific; ambiguity or incomplete identity evidence fails without selecting a target. |
  | **Plan / dry-run** | Not provided in phase 1. The YAML is the plan; the author reviews it in Git. |
  | **Natural key** | The stable identifier used to match a YAML declaration to a server-side entity: `(project, slug)` for Assistant/Workflow; `(project, name)` for Skill; `(project, repo_name)` for Datasource. |
  | **Datasource kind** | The `spec.index_type` discriminator selecting one existing CodeMie Datasource request/read format. It is configuration within the Datasource entity, not another entity type. |
  | **Authorable Datasource kind** | A Datasource kind for which the pinned target exposes an ordinary create, read, and update contract that can be represented and validated under the phase-1 security rules. A read/import-only server kind remains inventoried but is not given an invented authoring contract. |
  | **Workflow identity record** | The Workflow-only reserved `meta_config` member that persists the authored `(project, slug)` on the server. It is identity metadata, not a generic management or ownership marker. |
  | **Workflow adoption** | An explicit, one-invocation operation that selects an unmarked legacy Workflow by a supplied current server UUID, persists its Workflow identity record, and reconciles it. The UUID is not written to YAML, local state, or outcomes. |
  | **Ambiguous identity** | More than one exact server match for an authored natural key. The tool fails and selects none. |
  | **Envelope** | The top-level YAML structure: `apiVersion`, `kind`, `metadata` (identity), `spec` (configuration). |
  | **Safe diagnostic** | A tool-authored failure description built only from the FR-016 allowlist. It is not copied from a request/response body, server error string, declaration value, credential source, or exception dump. |
  | **Sensitive material** | Tokens, passwords, client secrets, credentials, authorization/proxy-authorization headers, cookies/set-cookie values, values under secret-classified fields, and complete request/response bodies or payloads. Diagnostics omit all non-allowlisted values rather than attempting to discover every arbitrary secret in free-form content. |
  | **Authoring-required field** | An identity/envelope, structurally required, conditionally required, or applicable server-request field for which the pinned contract does not accept JSON null. It must be present in YAML with a schema-valid non-null value unless its specific constraint explicitly permits null. |
  | **Optional authorable field** | A wholly author-controlled `spec` field that maps directly or through an approved transformation to an applicable server create/update request property whose pinned applicable requests accept JSON null. Omission and explicit YAML null both produce null in that outbound property. Authoring-only fields with no corresponding outbound property, create-only fields during update, read-only response fields, and tool-owned or mixed-ownership structures are not optional authorable payload fields for an operation where they do not map to such a property. |

  ---

  ## 12. Current behavior

  ### Persistence and API
  * FastAPI backend backed by PostgreSQL (via SQLModel) with JSONB columns for complex fields.
  * REST routers exist for every in-scope entity: `assistant.py`, `workflow.py`, `skill.py`, `index.py` (datasources are called *indexes* internally).

  ### Existing YAML usage on the server
  * Assistants, workflows, and skills already live as YAML files under `config/templates/{assistant,workflow,skill}/` as bootstrap templates.
  * `AssistantBase.from_yaml(yaml_str, project)` classmethod — the server already parses assistant YAML natively.
  * Workflow `execution_config` validated against a formal JSON Schema (Draft 2020-12) at `src/codemie/workflows/execution_config_schema.yaml`.

  ### Existing import/lint mechanisms
  * **Kata import service** (`kata_import_service.py`): file-size caps, staged validation, cross-reference checking, kebab-case ID regex, semver version, SHA-256 content checksum for update detection. Strongest pattern to reuse.

  ### Natural-key support on the server

  | Entity | Current server support and phase-1 consequence |
  |---|---|
  | Assistant | `(project, slug)` — partial unique index, `slug IS NOT NULL` |
  | Skill | The server uniqueness key includes creator: `(project, created_by.id, name)`. It does **not** enforce the authored `(project, name)` identity. The client must exhaustively resolve and reject multiple exact matches. |
  | Datasource (`IndexInfo`) | `(project_name, repo_name)` |
  | Workflow | Persisted Workflows have no native slug lookup or uniqueness constraint. The tool stores `(project, slug)` in the reserved Workflow `meta_config` identity record and resolves it client-side. |

  ### Cross-entity references (as observed today)
  Existing workflow templates embed server-generated UUIDs for datasource references (e.g. `datasource_ids: [72817832-970e-4dde-aad4-c81d73c82b54]`). This is fragile across environments. The tool replaces this with natural-key references — see FR-004, DR-003.

  ### Authentication
  * Production/enterprise deployments use **Keycloak OIDC** registered at startup via `IdpFactory.register()`. The server validates tokens issued by the Keycloak realm.
  * **CI authentication** uses the **OIDC `client_credentials` grant** against an explicitly operator-configured Keycloak token endpoint:
    ```
    POST <explicit-auth-url>
    Content-Type: application/x-www-form-urlencoded

    client_id=<client-id>&client_secret=<client-secret>&grant_type=client_credentials
    ```
    The endpoint comes only from `--auth-url`, `CODEMIE_AUTH_URL`, or `.codemie/config.yaml` `auth_url`; it is never derived from the API URL or a convention. The request returns a Bearer token valid for ~8 hours. Credentials are obtained by requesting a service account at `https://epa.ms/codemie-support`.
  * **Keycloak ROPC `grant_type=password`** (human user access, no `client_secret`): a public-client ROPC request against an explicitly configured Keycloak token endpoint:
    ```
    POST <explicit-auth-url>
    Content-Type: application/x-www-form-urlencoded

    grant_type=password&client_id=<client-id>&username=<email>&password=<password>
    ```
    No `client_secret` is sent. `client_id` defaults to `codemie-sdk`. The endpoint is the same token endpoint shape as `client_credentials`. This path is used by human users and developers accessing Keycloak-backed instances; Mode (c) in FR-024 and SC-020. The tool selects this mode when `auth_url` is configured, email+password are present, and `CODEMIE_CLIENT_SECRET` is NOT set.
  * **`POST /v1/local-auth/login`** (email + password): available only when the server is configured with `ENABLE_USER_MANAGEMENT=True` AND `IDP_PROVIDER="local"`. This is the case for a local dev server. **Not available in production/enterprise deployments.** Rate-limited to 5 requests / 15 min. This path is supported by the tool for local development only; CI MUST use Keycloak.
  * No static API-key mechanism in base package. Internal service-to-service HMAC `X-Bind-Key` is not designed for external use.
  * **Project integrations only (production):** Under Keycloak service-account credentials, only Project-level integrations (Jira, Git, etc.) are accessible. Personal integrations are silently ignored. Teams must migrate personal integrations to project level before adopting the tool in production.

  ## 13. User and system scenarios

  ### SC-001 — Author a new assistant declaratively
  * **Actor:** Platform asset author
  * **Trigger:** Author adds `assistants/support-triage.yaml` and opens a PR.
  * **Preconditions:** Repo contains the tool's config file; YAML schemas are available.
  * **Main flow:** 1. Author writes the YAML. 2. Author runs `lint` locally. 3. Lint passes; author opens PR. 4. CI runs `lint` on the changed file. 5. Reviewer approves; PR is merged. 6. CI on `main` runs `apply` against the target env. 7. Assistant is created on the server.
  * **Expected outcome:** Assistant visible in the target environment after the server accepts the payload projected from YAML under FR-021/DR-012.
  * **Postconditions:** The authored request is reproducible from Git, including omission-to-null semantics. Any Git/CI/platform audit correlation remains external to the CLI.

  ### SC-002 — Update an existing workflow
  * **Actor:** Platform asset author
  * **Trigger:** Author changes a YAML file whose `(project, slug)` has exactly one valid Workflow identity record on the server.
  * **Main flow:** Same as SC-001, but `apply` resolves the exact Workflow identity record and sends the declaration's update projection rather than a create request.
  * **Failure flow:** If the update is rejected server-side, `apply` exits code 1, leaves stdout empty, and writes a safe synthesized diagnostic to stderr containing only allowlisted non-sensitive context such as status/category and request/correlation ID when available. It never prints the server error text/body or request payload. The server entity remains unchanged when rejection occurred before commit.

  ### SC-003 — Lint fails locally
  * **Actor:** Author
  * **Trigger:** Author saves a YAML file with a typo in a field name.
  * **Expected outcome:** Lint reports the offending file, line, and field; exits code 2. No server call is made.

  ### SC-004 — Cross-reference validation
  * **Actor:** Author
  * **Trigger:** An assistant declaration references a datasource name that does not exist in the repo.
  * **Expected outcome:** Lint reports the missing repository reference; exits code 2. No server call is made.

  ### SC-005 — Reapply performs another update
  * **Actor:** CI runner
  * **Trigger:** The same YAML file is applied twice in a row.
  * **Expected outcome:** The first invocation creates or updates according to existence. The second invocation resolves the now-existing entity, sends another update request, reports `updated`, and exits zero. The CLI does not compare desired/current state to suppress the write.

  ### SC-006 — Entity fails server-side validation
  * **Actor:** CI runner
  * **Trigger:** `apply` sends a YAML-derived payload that lint accepted but the server rejects (e.g. 422 Unprocessable Entity).
  * **Main flow:** 1. `apply` sends the payload. 2. Server returns an entity-rejection 4xx. 3. The tool discards the response body from output and persistence. 4. The tool exits code 1 with stdout empty. 5. A safe synthesized diagnostic is written to stderr using only allowlisted non-sensitive fields.
  * **Expected outcome:** Server-side asset is unchanged. CI job is marked failed. The diagnostic identifies the failure category and non-sensitive HTTP status plus request/correlation ID when available, but contains no request/response body, server-provided error text, declaration value, credential, or secret-like value. Re-running with corrected YAML will re-attempt.

  ### SC-007 — Author avoids embedding credentials in YAML
  * **Actor:** Author
  * **Trigger:** A datasource or assistant declaration requires connection to an external service with an API key.
  * **Expected outcome:** Author uses the platform's integration configuration for credentials, not an inline YAML field value. YAML declares only non-sensitive configuration. Lint warns if a field that typically carries credentials contains a value resembling a plaintext secret.
  * **Postconditions:** No credentials appear in the YAML file or in Git history.

  ### SC-008 — Server rejects a change
  * **Actor:** CI runner
  * **Trigger:** Apply sends a schema-valid payload that the server rejects (e.g. name collision, quota exceeded).
  * **Expected outcome:** The tool exits code 1, leaves stdout empty, and writes a safe synthesized rejection diagnostic to stderr. It does not print or persist the raw server error, request payload, response body, or sensitive material.

  ### SC-009 — Unauthenticated / expired token
  * **Actor:** CI runner
  * **Trigger:** Token is missing or expired.
  * **Expected outcome:** Tool refuses to run `apply`, exits code 2, leaves stdout empty, and writes a safe authentication-category diagnostic to stderr; no token, credential, authorization header, cookie, raw body, or server-provided error text is printed. No server write is attempted.

  ### SC-010 — Concurrent CI run
  * **Actor:** Two CI runs
  * **Trigger:** Two commits merged near-simultaneously; both trigger `apply` against the same environment.
  * **Expected outcome:** Deployment jobs are serialized per target environment before invoking `apply`. If another client bypasses this control and creates a duplicate Workflow marker or Skill natural key, post-write resolution reports an ambiguity that may follow a committed write; the tool does not select, delete, or roll back either entity.

  ### SC-011 — CI acquires a token before apply
  * **Actor:** CI runner
  * **Trigger:** CI pipeline starts; no pre-issued token is in the environment.
  * **Preconditions:** `CODEMIE_CLIENT_ID` and `CODEMIE_CLIENT_SECRET` are available as CI secrets; the Keycloak token endpoint is explicitly supplied by `--auth-url`, `CODEMIE_AUTH_URL`, or `.codemie/config.yaml` `auth_url`.
  * **Main flow:** 1. CI calls `codemie-gitops login` with the credential environment variables and one explicit endpoint source. 2. Tool resolves the endpoint using `--auth-url` > `CODEMIE_AUTH_URL` > `.codemie/config.yaml` `auth_url` and posts `grant_type=client_credentials` to that exact URL. 3. Tool writes the resulting bearer token as a single line to stdout. 4. CI captures: `CODEMIE_TOKEN=$(codemie-gitops login)`. 5. Subsequent `apply` calls use `CODEMIE_TOKEN` without re-authenticating.
  * **Expected outcome:** Token acquired once, valid ~8 h, reused across steps. No credentials appear in CI logs.
  * **Failure flows:**
    * If no explicit Keycloak endpoint source is configured, the tool exits code 2 before network access, writes a safe missing-configuration diagnostic to stderr, and writes nothing to stdout. It does not attempt to derive an endpoint from `CODEMIE_URL` or another value.
    * If the explicitly configured endpoint is unreachable or credentials are invalid, the tool exits code 2, writes a safe category/status diagnostic to stderr, and writes nothing to stdout. It never prints or persists credential values, request/response bodies, identity-provider error text, or a token.

  ### SC-012 — Developer tests against a local dev server
  * **Actor:** Local developer
  * **Trigger:** Developer runs a local CodeMie server configured with `ENABLE_USER_MANAGEMENT=True` and `IDP_PROVIDER="local"` and wants to apply a YAML without setting up Keycloak credentials.
  * **Preconditions:** Dev server is running; developer has a local account with email and password; server is not a production deployment.
  * **Main flow:** 1. Developer sets `CODEMIE_PASSWORD=mypassword` and runs `codemie-gitops login --url http://localhost:8080 --email dev@example.com`. 2. Tool calls `POST /v1/local-auth/login`. 3. Tool writes the bearer token to stdout. 4. Developer sets `CODEMIE_TOKEN` and runs `apply`.
  * **Expected outcome:** Token acquired; apply succeeds against the dev server.
  * **Failure flows:**
    * Server returns 400 (not in local-auth mode) → tool exits code 2 with message explaining the server must have `ENABLE_USER_MANAGEMENT=True` and `IDP_PROVIDER="local"`.
    * Wrong credentials → tool exits code 2 with a fixed authentication-failed category; it does not echo the server response or supplied credentials.
    * Rate limit hit (5 / 15 min) → tool exits code 2 with a fixed rate-limited category and non-sensitive status; it does not echo the response body.
    * Every failure leaves stdout empty and writes only the safe diagnostic to stderr.

  ### SC-013 — Reconcile a Workflow by persisted slug identity
  * **Actor:** CI runner
  * **Trigger:** A Workflow declaration is applied and the server contains exactly one valid Workflow identity record for its exact `(project, slug)`.
  * **Preconditions:** The CI principal can prove complete project visibility and write permission.
  * **Main flow:** The tool exhaustively examines every relevant Workflow result page, selects only the exact identity record, and sends the declaration's update projection.
  * **Failure flows:** An invalid identity record, duplicate exact identity record, or entity-resolution instability in otherwise compatible server responses is an entity-reconciliation failure (exit 1). Incomplete visibility or missing write permission is an authorization/precondition failure (exit 2). A connectivity or response-contract compatibility failure while enumerating is exit 2. No ordinary reconciliation write occurs.
  * **Expected outcome:** The intended Workflow is reconciled; its server UUID remains absent from authored YAML and normal outcome identity.

  ### SC-014 — Explicitly adopt an unmarked legacy Workflow
  * **Actor:** Platform administrator or authorized asset owner
  * **Trigger:** The operator applies a Workflow declaration with `--adopt-workflow-id <current-server-uuid>`.
  * **Preconditions:** No valid Workflow identity record already matches the declaration; the supplied entity is an unmarked, writable Workflow in the exact project; its existing `meta_config` can preserve non-reserved values.
  * **Main flow:** The tool selects only the supplied server UUID, validates the candidate, persists the reserved `(project, slug)` identity record, and reconciles the declared desired state in the same operation.
  * **Failure flows:** A missing flag never causes implicit adoption. Display name may warn that adoption is required but never selects a candidate. A wrong-project, already-marked, or unmergeable candidate is an entity-reconciliation failure (exit 1); an unauthorized candidate is an authorization failure (exit 2). No write occurs.
  * **Expected outcome:** The legacy Workflow is adopted in place and future ordinary applies resolve it by `(project, slug)`; the UUID is not persisted in Git, client state, or outcomes.

  ### SC-015 — Skill natural-key resolution
  * **Actor:** CI runner
  * **Trigger:** A Skill declaration is applied.
  * **Preconditions:** The CI principal can prove complete project visibility and write permission for an existing match.
  * **Main flow:** The tool exhausts every compatible Skill list page and exact-filters decoded `(project, name)`. Zero exact matches creates; one updates on every invocation; more than one fails as ambiguous and selects none.
  * **Failure flows:** Multiple matches or entity-resolution instability in otherwise compatible server responses is an entity-reconciliation failure (exit 1). Incomplete visibility or missing write permission is an authorization/precondition failure (exit 2). Connectivity or response-contract compatibility failure is exit 2.
  * **Expected outcome:** The authored natural key remains the only declared and reported identity; any returned server UUID is invocation-local.

  ### SC-016 — Skill create race becomes visible
  * **Actor:** CI runner and another creator
  * **Trigger:** Both create the same authored `(project, name)` after independently observing no match.
  * **Expected outcome:** The tool re-resolves after its create. If multiple exact matches are then visible, it reports an exit-1 ambiguous failure and states that its create may already have committed. It performs no automatic delete, rollback, or tie-break. Serialized CI and governed UI/other-client writes are the required prevention controls.

  ### SC-017 — Omit an optional authorable field
  * **Actor:** Platform asset author
  * **Trigger:** The declaration omits a `spec` field that the pinned create and update request schemas classify as optional and null-accepting.
  * **Main flow:** Lint accepts the omission. Apply constructs the applicable create or update payload and includes the omitted property with JSON null. The same projection occurs when the declaration explicitly contains YAML null.
  * **Failure flow:** If either applicable request rejects null, the bundled schema classifies the field as authoring-required; omission or explicit null fails locally with exit code 2 and no server call.
  * **Expected outcome:** YAML avoids optional-field noise without silently delegating the field value to a server default, and create/update apply the same omission rule.

  ### SC-019 — Resolve Workflow persisted and inline assistant resources
  * **Actor:** CI runner
  * **Trigger:** A Workflow declaration contains persisted and/or inline entries in `spec.execution_config.assistants`.
  * **Main flow:** Each entry retains its workflow-local `id`. A persisted entry uses `assistantRef`; an inline entry supplies `system_prompt` and no `assistantRef`. Inline `skillRefs` and `datasourceRefs` resolve exact natural keys to invocation-local server IDs. `states[].assistant_id` remains the workflow-local `assistants[].id` reference.
  * **Failure flows:** A persisted/inline form collision, server ID in an authored resource field, or resource refs on a persisted entry fails local validation with exit 2. A locally valid missing/ambiguous server resource ref fails reconciliation with exit 1.
  * **Expected outcome:** The server request receives its required ID-bearing execution shape while Git contains no server asset UUID.

  ### SC-020 — Human developer authenticates via Keycloak ROPC
  * **Actor:** Human developer or authorized human user
  * **Trigger:** Developer wants to authenticate against a Keycloak-backed instance using personal email and password credentials, without a service-account `client_secret`.
  * **Preconditions:** `CODEMIE_AUTH_URL` (or equivalent `--auth-url` / config `auth_url`) is configured; `CODEMIE_EMAIL` and `CODEMIE_PASSWORD` are available; `CODEMIE_CLIENT_SECRET` is NOT set; developer has a Keycloak account in the target realm.
  * **Main flow:** 1. Developer sets `CODEMIE_PASSWORD` as an environment variable and runs `codemie-gitops login --auth-url $AUTH_URL --email dev@example.com`. 2. Tool determines Mode (c) is active: `auth_url` is configured, email and password are set, and no `client_secret` is present. 3. `CODEMIE_CLIENT_ID` defaults to `codemie-sdk` when not explicitly set. 4. Tool sends `POST <auth_url>` with `grant_type=password&client_id=<effective-client-id>&username=<email>&password=<password>` (no `client_secret`). 5. Tool writes the resulting bearer token as a single line to stdout. 6. Developer sets `CODEMIE_TOKEN=$(codemie-gitops login ...)` and runs subsequent `apply` calls.
  * **Expected outcome:** Token acquired via ROPC; apply succeeds against the Keycloak-backed instance. Token and credentials never appear in stderr, logs, caches, or persisted tool data.
  * **Failure flows:**
    * No `auth_url` is configured (no `--auth-url`, no `CODEMIE_AUTH_URL`, no config `auth_url`) → tool resolves Mode (b) local-auth (or fails if a CodeMie API URL also absent). To force ROPC, the developer must supply `auth_url`; see AC-FR-024-08.
    * Keycloak returns 401 or 400 (invalid credentials, realm misconfiguration, or ROPC not enabled for the client) → tool exits code 2 with a fixed authentication-failed category; stdout remains empty; no credential, token, request body, or identity-provider error text is printed.
    * `auth_url` is missing and email+password are present but no CodeMie API URL is configured either → exit code 2 before network access with missing-configuration diagnostic.
    * Every failure leaves stdout empty and writes only a safe synthesized diagnostic to stderr.

  ---

  ## 14. Functional requirements

  | ID | Requirement | Rationale | Related scenarios |
  |---|---|---|---|
  | **FR-001** | The tool MUST accept YAML files as the sole authoring format for assistants, workflows, datasources, and skills. | User requirement. | SC-001, SC-002 |
  | **FR-002** | The tool MUST provide a `lint` command that runs without network access to a CodeMie environment. | Fast local feedback; CI without secrets. | SC-003, SC-004, SC-007 |
  | **FR-003** | The tool MUST validate each YAML file against the bundled schema for its declared `kind`. Any local schema failure MUST use exit code 2 and prevent server calls. | Prevent malformed submissions reaching the server. | SC-003 |
  | **FR-004** | The tool MUST validate intra-repo cross-references between assets at lint time. A missing, ambiguous, duplicate, or wrong-kind repository reference MUST use exit code 2 and prevent server calls. | Catch broken refs before apply. | SC-004 |
  | **FR-005** | The tool MUST provide an `apply` command. Each invocation MUST target exactly one YAML file containing exactly one entity. Apply MUST resolve the entity by the declared kind's approved identity contract, create when that contract proves the entity missing, and update when it returns exactly one safe match. It MUST fail without selecting a target when identity is ambiguous or cannot be proven safely. Datasource apply uses ordinary per-kind CRUD under FR-036. | Core authoring behavior without unsafe tiebreaking. One entity per invocation keeps orchestration with CI. | SC-001, SC-002, SC-005, SC-013, SC-015 |
  | **FR-006** | Every valid `apply` invocation MUST issue one ordinary create request when the safely resolved target is missing or one ordinary update request when exactly one target is present. It MUST NOT compare desired and current state to skip the write, MUST NOT report `unchanged`, and MUST report the accepted operation as `created` or `updated`. | Makes apply a predictable authoring command and leaves write semantics to the ordinary server contract. | SC-005 |
  | **FR-007** | ~~DELETED in v7.~~ No `plan` / dry-run command in phase 1. The YAML is the plan. | Deferred — see §10. | — |
  | **FR-008** | The tool MUST NOT delete server-side assets in phase 1. | Avoid destructive drift; delete semantics deferred. | — |
  | **FR-009** | The tool MUST authenticate using credentials supplied via the `CODEMIE_TOKEN` environment variable only; the `--token` CLI flag MUST NOT be accepted. Credentials MUST NOT be supplied via YAML asset files. Any attempt to pass a bearer token as a CLI flag value MUST fail with exit code 2 before any network access. The `login` command acquires a short-lived token as an alternative. | Keep secrets out of the process argument vector and the repository. SEC-001 remediation (v25). | SC-007, SC-009, SC-011 |
  | **FR-010** | ~~DEFERRED to phase 2.~~ Per-environment overlay/templating. In phase 1, the target environment is specified via `CODEMIE_URL` + `CODEMIE_TOKEN` env vars (or `--url` flag for the URL). | Multi-env deferred — see §10. | — |
  | **FR-011** | The tool MUST use this exit taxonomy: **0** = success (`valid`, `created`, or `updated`, including warnings); **1** = entity reconciliation or server-side failure reached after the declaration and local inputs have passed local validation; **2** = local CLI usage, file parsing, schema, semantic-validation, repository-reference, sidecar, or configuration failure; authentication/authorization or identity-visibility precondition failure; API compatibility or connectivity failure; or fatal/internal error. Identity ambiguity, invalid server identity metadata, adoption-required outcomes, entity-resolution instability in otherwise compatible server responses, and server rejection after valid local input are exit 1. Missing/expired credentials, insufficient visibility/write permission, response-contract incompatibility, network failure, and unavailable server are exit 2. Successful per-entity outcomes MUST go to stdout. Every failure MUST leave stdout empty and write its safe diagnostic only to stderr, in both text and JSON modes. A successful `login` token line is the sole intentional sensitive stdout exception. | Gives CI one decision-oriented classification and an unambiguous success/failure stream contract. | SC-003, SC-004, SC-006, SC-008, SC-009, SC-011–SC-017, SC-019 |
  | **FR-012** | On a successful `apply`, the tool MUST print the server-accepted operation to stdout as `created` or `updated`. It MUST NOT print `unchanged`. | Provides a CI-readable authoring outcome. | SC-001, SC-005 |
  | **FR-013** | ~~DELETED in v21.~~ The CLI does not emit Git/CI/environment provenance; the prohibition and retained success fields are defined by FR-016, FR-026, DR-006, and PA-004. | Provenance belongs to external Git, CI, and platform logs. | — |
  | **FR-014** | The linter MUST warn when a field that typically carries credentials contains a value that resembles a plaintext secret (e.g. a high-entropy string). The warning MUST go to stderr and identify only the source location, field path, and fixed warning category; it MUST NOT echo, hash, encode, truncate, or otherwise reproduce the value. No secret interpolation syntax is provided in phase 1. | Discourage accidental credential storage without exposing the suspected value. | SC-007 |
  | **FR-015** | Renaming a YAML file MUST NOT cause a duplicate asset on the server. Satisfied by natural-key identity (DR-002). | Refactoring safety. | SC-005 |
  | **FR-016** | The tool MUST synthesize failure diagnostics from an explicit non-sensitive allowlist and write them only to stderr. Allowed diagnostic data is limited to stable tool error code/category, exit code, local source file/line/column/field path, non-sensitive HTTP status and method/route template, locally generated request ID, and a dedicated server correlation/request ID when available and safely representable. Values not explicitly allowlisted MUST be omitted. A failure diagnostic MUST NOT contain raw or full request/response bodies, server-provided error text, declaration/sidecar values, request payloads, tokens, credentials, authorization headers, cookies, secret-classified fields, secret-like values, Git commit SHA, target environment origin, Git author, or CI-run identity. The tool MUST NOT persist those sensitive transport/authentication artifacts. Debug, verbose, trace, panic, and internal-error paths MUST obey the same boundary. | Enables CI diagnosis without attempting unreliable arbitrary-secret discovery, leaking sensitive payloads, or duplicating external provenance. | SC-002, SC-006, SC-008, SC-009, SC-011, SC-012 |
  | **FR-017** | The tool MUST support a version-controllable config file at `.codemie/config.yaml` in the repository root. This file MAY contain only these non-secret connection/default fields: `url` (target CodeMie API URL), `auth_url` (exact Keycloak token endpoint URL), and `project` (default project key). Resolution MUST be deterministic per field: `--url` > `CODEMIE_URL` > config `url`; `--auth-url` > `CODEMIE_AUTH_URL` > config `auth_url`; declaration `metadata.project` > config `project`; bearer token: `CODEMIE_TOKEN` (environment only — no flag); client ID: `--client-id` > `CODEMIE_CLIENT_ID` (non-secret selector, flag permitted; applies to Mode (a) client_credentials and Mode (c) Keycloak ROPC; when not explicitly set in Mode (c) it defaults to `codemie-sdk`); client secret: `CODEMIE_CLIENT_SECRET` (environment only — no flag; Mode (a) only); email: `--email` > `CODEMIE_EMAIL` (applies to Mode (b) local-auth and Mode (c) Keycloak ROPC); and password: `CODEMIE_PASSWORD` (environment only — no flag; applies to Mode (b) local-auth and Mode (c) Keycloak ROPC). Secret credentials (bearer token, client secret, and password) MUST NOT be accepted as CLI flag values and MUST NOT appear in the config file. An attempt to supply a secret credential as a flag MUST fail with exit code 2 before any network access. The non-secret client ID MAY be supplied as a flag. | Reproducibility without repeating non-secret endpoints while keeping secret credentials out of the repository and out of the process argument vector. SEC-001 remediation (v25). | SC-001, SC-011 |
  | **FR-018** | ~~DELETED in v7.~~ `--fail-fast` / continue-and-report model superseded by one-entity-per-invocation (FR-005). | Superseded. | — |
  | **FR-019** | The tool MUST NOT provide generic pre-existing-asset adoption. The narrowly scoped Workflow adoption operation is defined by FR-030 and MUST NOT be generalized to another kind without a new product decision. | Preserves the v10 generic-adoption decision while recording the v14 Workflow exception. | SC-014 |
  | **FR-020** | The tool MUST NOT persist generic management/ownership markers. The Workflow identity record in FR-028 is identity metadata only and MUST NOT be treated as proof of ownership or authorization. | Preserves the v10 ownership decision while recording the v14 Workflow identity exception. | SC-013, SC-014 |
  | **FR-021** | `apply` MUST preserve every explicit authored desired-state value without silently substituting a server-selected default. For every omitted optional authorable request field defined by DR-012, it MUST include that property as explicit JSON null in the applicable create or update payload; explicit YAML null MUST produce the same JSON null. The rule MUST be identical for create and update whenever the property exists in both applicable request contracts. Apply MAY otherwise perform only these product-required transformations: use the effective project and authored natural key as server identity input; translate natural-key references to server request references; expand Skill `contentFrom`; persist the reserved Workflow identity record; preserve non-reserved Workflow `meta_config` values; and adapt the declared structure to the documented operation-specific request shape. | Keeps authored values deterministic while allowing concise YAML and preventing implicit server-default selection. | SC-002, SC-013–SC-015, SC-017 |
  | **FR-022** | The bundled schema MUST classify every authorable field for each applicable operation as authoring-required or optional authorable under DR-012. `lint` MUST fail with exit code 2 when a declaration omits an identity/envelope, structurally required, conditionally required, or null-rejecting applicable request field. An optional authorable field MAY be omitted and an explicitly nullable field MAY contain YAML null. Explicit null for a non-nullable field MUST fail locally with exit code 2. **Project resolution:** `metadata.project` is logically required but MAY be omitted from YAML when `.codemie/config.yaml` supplies a non-empty project; lint MUST fail if neither source supplies it. Failure output MUST identify the field path without reproducing its value. | Establishes one source-pinned omission/null rule for all four entities while retaining required structure and the repository project default. | SC-003, SC-017 |
  | **FR-023** | `lint` and `apply` MUST fail with exit code 2 before any server call when a YAML contains a field not present in the tool's bundled schema for the declared `kind` and `apiVersion`. The output MUST name the offending field and its location. | Catches typos (e.g. `sytem_prompt`) that would otherwise be silently ignored. | SC-003 |
  | **FR-024** | The tool MUST provide a `login` sub-command that acquires a bearer token and, on success only, writes it as a **single line to stdout** with no decoration. This is the sole intentional token-output exception; the token MUST NOT also appear in stderr, logs, outcomes, files, caches, or later diagnostics. Login failure MUST leave stdout empty and write only an FR-016-safe synthesized diagnostic to stderr; identity-provider request/response bodies and error text MUST NOT be printed or persisted. The command MUST support **three mutually exclusive auth modes**, selected by which credential set and configuration are present through the FR-017 non-argv sources. Mode is determined in this order: **(a)** if `CODEMIE_CLIENT_SECRET` is set and `auth_url` is configured → Mode (a); **(c)** else if `CODEMIE_EMAIL` and `CODEMIE_PASSWORD` are set and `auth_url` is configured → Mode (c); **(b)** else if `CODEMIE_EMAIL` and `CODEMIE_PASSWORD` are set and `auth_url` is not configured → Mode (b). Supplying `CODEMIE_CLIENT_SECRET` together with `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` is an error (exit 2) before network access. **(a) Keycloak client_credentials** (for CI and production): requires `--client-id` / `CODEMIE_CLIENT_ID` (client ID is a non-secret selector; flag is permitted), `CODEMIE_CLIENT_SECRET` (environment only — the `--client-secret` flag MUST NOT be accepted), and an explicit token endpoint resolved as `--auth-url` > `CODEMIE_AUTH_URL` > `.codemie/config.yaml` `auth_url`; it calls that exact URL with `grant_type=client_credentials`. The CLI MUST NOT derive the endpoint from `--url`, `CODEMIE_URL`, config `url`, a hostname/path convention, or another value. If the explicit endpoint is missing, `login` MUST exit code 2 before network access. **(c) Keycloak ROPC** (for human users and developer access to Keycloak-backed instances): active when `CODEMIE_CLIENT_SECRET` is NOT set, `CODEMIE_EMAIL` and `CODEMIE_PASSWORD` are set, and `auth_url` is configured. Requires `--email` / `CODEMIE_EMAIL` (non-secret; flag permitted) and `CODEMIE_PASSWORD` (environment only — the `--password` flag MUST NOT be accepted). Uses `--client-id` / `CODEMIE_CLIENT_ID` as a non-secret selector (flag permitted); when not explicitly set it MUST default to `codemie-sdk`. Calls the explicit token endpoint (resolved as `--auth-url` > `CODEMIE_AUTH_URL` > `.codemie/config.yaml` `auth_url`) with `grant_type=password`, `client_id=<effective-client-id>`, `username=<email>`, and `password=<password>`. This is a public-client ROPC flow — no `client_secret` is sent. The CLI MUST NOT derive the endpoint from `--url`, `CODEMIE_URL`, config `url`, a hostname/path convention, or another value. If the explicit endpoint is missing, `login` MUST exit code 2 before network access. **(b) Local-auth** (for local dev only): requires `--email` / `CODEMIE_EMAIL` and `CODEMIE_PASSWORD` (environment only — the `--password` flag MUST NOT be accepted); resolves the CodeMie API URL under FR-017 and calls `POST /v1/local-auth/login`; it is available only when the server is configured with `ENABLE_USER_MANAGEMENT=True` and `IDP_PROVIDER="local"`. If the local-auth endpoint returns 400, the tool MUST emit a fixed safe diagnostic stating that local authentication is unavailable without echoing the response. Any attempt to supply `--client-secret` or `--password` as a CLI flag value MUST fail with exit code 2 before any network access. | Enables composable token acquisition with an explicit, reviewable identity-provider endpoint and a no-leak failure boundary. Secret credentials never enter argv. SEC-001 remediation (v25). Mode (c) Keycloak ROPC added (v26). | SC-011, SC-012, SC-020 |
  | **FR-025** | For `kind: Skill`, the `spec.content` markdown field MAY be expressed as a sidecar file reference using `spec.contentFrom: <relative path to .md file>` instead of inlining the content. `spec.content` and `spec.contentFrom` are mutually exclusive; both absent or both present is a local validation failure (exit 2). At lint time, the tool MUST verify the sidecar file exists relative to the YAML file; missing, unreadable, unsafe, invalid, or out-of-bounds sidecar content MUST use exit code 2. At apply time, the tool MUST inline the sidecar file content before constructing the API payload. | Long skill markdown is unreadable and un-diffable when inlined in YAML. Sidecar pattern preserves diff quality without changing the API contract. Q23. | SC-001, SC-003 |
  | **FR-026** | All commands that produce per-entity output MUST accept `--output text|json`. On success, text mode MUST emit one human-readable outcome line to stdout; JSON mode MUST emit one single-line JSON success object containing `action`, `kind`, `project`, and the natural-key field. Success output MUST NOT contain Git commit SHA, target environment origin, Git author, CI-run identity, or replacement provenance fields. On failure, stdout MUST be empty in both modes. Text mode MUST emit one or more safe human-readable diagnostics to stderr; JSON mode MUST emit a single-line safe diagnostic JSON object to stderr containing only FR-016-allowlisted fields, with `errorCode`, `category`, and `exitCode`. A failure diagnostic MUST NOT contain an outcome action, server message/body, payload, sensitive value, or external provenance. | Preserves machine-readable CI handling while keeping success identity, failures, and externally owned provenance separate. | SC-001–SC-006, SC-008, SC-009, SC-013–SC-016 |
  | **FR-027** | The tool MUST NOT enforce any file naming convention or directory structure. The entity identity is determined solely by the `kind`, `metadata` fields inside the YAML file, not by the filename or directory path. A recommended (non-enforced) convention is to organize files by kind (`assistants/`, `workflows/`, `skills/`, `datasources/`) with filenames matching the natural key. | Free-form layout accommodates monorepos, existing project structures, and refactoring. Q19, Q20. | FR-015 |
  | **FR-028** | For Workflow only, `apply` MUST persist the exact authored identity in the reserved top-level `meta_config` member `codemie.epam.com/gitops/workflow-identity` with value `{version: 1, project: <effective-project>, slug: <metadata.slug>}`. | Gives persisted Workflows a portable slug identity without changing the server or authoring a UUID. | SC-013, SC-014 |
  | **FR-029** | Ordinary Workflow reconciliation MUST exhaust all relevant visible result pages and select a target only by exact row project plus the valid reserved Workflow identity record. Zero exact records means missing, one means update, and more than one means ambiguous failure (exit 1). Any invalid reserved identity record or entity-resolution instability in otherwise compatible server responses that could affect the effective project MUST fail with exit 1 before a Workflow write. Incomplete visibility, missing write permission, connectivity failure, or response-contract incompatibility MUST fail with exit 2. Display name, creator, recency, and list order MUST NOT select a target. | Prevents accidental overwrite when the server has no native slug lookup or marker uniqueness constraint. | SC-013 |
  | **FR-030** | An unmarked legacy Workflow MAY be adopted only when the operator explicitly supplies its current server UUID with `--adopt-workflow-id`. Invalid flag syntax is a local validation failure (exit 2). Adoption MUST validate that no matching identity record already exists, the candidate is in the exact project, writable, unmarked, and able to preserve its non-reserved metadata; it MUST then persist the identity record and reconcile desired state in the same operation. A wrong-project, already-marked, unmergeable, or adoption-required result is exit 1; insufficient visibility or write permission is exit 2. The supplied UUID MUST NOT be persisted in YAML, client state, or the normal outcome identity. Without the flag, one or more unmarked exact display-name matches MUST cause an adoption-required failure (exit 1) and MUST NOT be selected. | Enables safe in-place reconciliation of legacy Workflows while preserving history and avoiding name-based guesses. | SC-014 |
  | **FR-031** | Skill target and Skill-reference resolution MUST exhaust every page returned by the compatible list API and client-filter exact decoded `(project, name)` values. Zero exact matches means create for a target, one means update for a target, and more than one means ambiguous failure (exit 1) with no target selected. Pagination/detail entity-resolution instability in otherwise compatible responses is exit 1; incomplete visibility, missing write permission, connectivity failure, or response-contract incompatibility is exit 2. A Skill reference requires exactly one match, never creates the referenced Skill, and reports a missing or ambiguous server reference as exit 1. | Implements the approved Skill natural key despite creator-scoped server uniqueness. | SC-015, SC-016 |
  | **FR-032** | Workflow and Skill server IDs returned during resolution MUST be used only as invocation-local request handles. They MUST NOT appear in declaration metadata, local persistent state, or normal outcome identity fields. | Keeps Git and reporting portable across environments. | SC-013–SC-016 |
  | **FR-033** | Before resolving or writing a Workflow or Skill, the tool MUST verify that the current principal has complete visibility for the effective project; before updating or adopting an existing entity, it MUST verify write permission. If either guarantee cannot be demonstrated, the tool MUST fail with exit code 2 before the write. | Client-side exact resolution is unsafe when duplicates may be hidden; this is an authorization/precondition failure, not an entity reconciliation result. | SC-013–SC-015 |
  | **FR-034** | After creating or updating a Workflow, creating a Skill, or adopting a Workflow, the tool MUST re-resolve the authored natural key. It MUST report success only when exactly one match identifies the just-written entity. A missing, invalid, ambiguous, or otherwise compatible but unstable identity result MUST fail with exit code 1 and state that a write may already have committed. A connectivity or response-contract compatibility failure during verification MUST use exit code 2 and also state that commit status is uncertain. The tool MUST NOT automatically retry the create, choose a match, delete, or roll back an entity. | Makes residual no-server-constraint races observable without destructive recovery while preserving the exit taxonomy. | SC-014, SC-016 |
  | **FR-035** | Each Workflow `spec.execution_config.assistants[]` entry MUST have a unique workflow-local `id` and exactly one actor form: **persisted**, using `assistantRef: {project, slug}`; or **inline**, omitting `assistantRef` and supplying the schema-valid inline definition including `system_prompt`. Inline entries MAY use `skillRefs: [{project, name}]` and `datasourceRefs: [{project, repo_name}]`; those fields MUST be absent from persisted entries because the current server applies them only to inline/virtual assistants. Authored `assistant_id`, `skill_ids`, and `datasource_ids` are prohibited at these server-resource positions. `states[].assistant_id` remains authored and MUST resolve to a workflow-local `assistants[].id`. Apply MUST replace the three natural-reference properties with the server ID-bearing properties only in the invocation payload. | Makes the source-observed Workflow actor/resource model portable without confusing graph-local IDs with server asset IDs. | SC-019 |
  | **FR-036** | Datasource apply MUST use the compatible selected kind's existing ordinary create or update format, including supported source/content/file fields. Optional authorable body fields follow FR-021/022; the tool MUST NOT omit such a field to invoke a server default. It MUST classify the result from the synchronous CRUD response under FR-011. It MUST NOT expose a dedicated Datasource lifecycle-control command or flag or call a dedicated lifecycle-control endpoint. A supplied lifecycle-control option MUST fail local validation with exit code 2 before any server call. | Keeps Datasource authoring aligned with the existing CodeMie CRUD contract and phase-1 command scope. | SC-001, SC-002, SC-017 |

  ---

  ## 15. Data requirements

  ### General

  * **DR-001** — The tool's YAML schema for each entity MUST cover the user-editable subset of the server's request contract and record operation-specific requiredness and nullability. Authoritative sources: `AssistantRequest`, `SkillCreateRequest`, the Workflow model plus `execution_config_schema.yaml`, and for Datasource the selected kind's ordinary create/update request models plus identity fields required from read responses. A single Datasource model MUST NOT be assumed across kinds.
  * **DR-002** — Every declaration MUST use the following envelope:
    ```yaml
    apiVersion: codemie.epam.com/v1alpha1
    kind: Assistant          # or Workflow | Datasource | Skill
    metadata:
      project: <project-key>
      slug: <natural-key>    # field name varies by kind: slug / name / repo_name
    spec:
      # all user-editable fields
    ```
    `apiVersion` is tool-owned and independent of the server's `AssistantConfiguration.version_count`. Unknown `apiVersion` values are a hard error (exit code 2).
  * **DR-003** — Authored managed-asset cross-entity references MUST use natural keys, never server-generated asset UUIDs. The tool MAY resolve an exact natural key to an invocation-local server ID when the target API requires it:
    * Assistant → Skill: `spec.skills: [{project, name}]` → resolved to `skill_ids` at apply time.
    * Assistant → Sub-assistant: `spec.sub_assistants: [{project, slug}]` → resolved to `assistant_ids` at apply time.
    * Assistant → Datasource: `spec.context: [{context_type, ref: {project, repo_name}}]` → resolved at apply time.
    * Workflow persisted actor: `spec.execution_config.assistants[].assistantRef: {project, slug}` → server `assistant_id` for that actor entry.
    * Workflow inline actor resources: `spec.execution_config.assistants[].skillRefs: [{project, name}]` → server `skill_ids`; `datasourceRefs: [{project, repo_name}]` → server `datasource_ids`. These lists are not valid on persisted `assistantRef` entries (FR-035).
  * **DR-004** — Secret values MUST NOT be embedded in YAML files. External-service credentials MUST be managed through the platform's integration configuration. The tool provides no secret interpolation syntax in phase 1. Lint warns on suspicious high-entropy values in credential fields without reproducing the value (FR-014).
  * **DR-005** — The tool MUST NOT persist a local state database or environment-specific ID mapping. Identity is derivable from authored natural keys plus server state; the Workflow identity record is stored on the Workflow server entity, not by the client.
  * **DR-006** — Command output MUST be CI-log-friendly (line-oriented, no interactive UI). Successful outcomes use stdout; warnings and failure diagnostics use stderr. A failed invocation produces no stdout record. Successful per-entity output contains only the FR-026 reconciliation fields and no Git commit SHA, target environment origin, Git author, CI-run identity, or substitute provenance. The tool MUST NOT create or retain its own output log. Retention and provenance correlation performed by Git, the invoking shell/CI system, or the platform are outside the tool's control.
  * **DR-007** — The Workflow identity record MUST use reserved key `codemie.epam.com/gitops/workflow-identity`. Its value MUST be a closed object containing integer `version: 1` and exact, non-empty `project` and `slug` strings. User-authored Workflow `spec.meta_config` MAY contain non-reserved members but MUST NOT contain the reserved key. Existing non-reserved server members MUST be preserved unless an authored value explicitly replaces the same member. Phase 1 provides no syntax for deleting an unmentioned non-reserved member.
  * **DR-008** — Workflow and Skill resolution evidence, including returned server ID, permissions, and pagination metadata, is invocation-local. It MUST NOT be written to the declaration, repository config, outcome identity, or a client state file.
  * **DR-009** — Credentials, tokens, authorization headers, cookies, request payloads, secret-classified fields, secret-like values, and complete request/response bodies MAY be held transiently only as needed to validate input, authenticate, or perform the intended server operation. The tool MUST NOT persist copies of them in files, caches, client state, outcome records, diagnostics, or tool-created logs. Declaration and sidecar values MUST NOT be copied into diagnostics. This does not prohibit writing declared desired state to the target server or reporting the non-sensitive success identity required by FR-026. The successful `login` token line required by FR-024 is the sole intentional sensitive-output exception. Diagnostic safety MUST be achieved by constructing output from the FR-016 allowlist, not by attempting to discover and redact every arbitrary secret after composing a message.
  * **DR-010** — A Datasource declaration MUST be a closed discriminated union selected by `spec.index_type`. For every selected kind, the declaration MUST use the exact existing CodeMie authoring field names, casing, nesting, enum values, requiredness, and validation constraints recorded in the ordinary per-kind inventory in §15. The tool MUST NOT invent a common integration-reference object, rename a kind's fields for consistency, or accept fields belonging to another kind. `metadata.repo_name` and the effective project remain the tool envelope identity and are projected to the exact identity inputs required by that kind's request.
  * **DR-011** — Datasource create and update MUST use the selected kind's pinned ordinary request formats. Apply MUST construct the exact operation-specific request projection from the declaration and send it on every valid invocation; it MUST NOT read current Datasource configuration to compute a difference or skip an update. Optional authorable fields materialize under DR-012 in both operations. A field exposed only by the create format is projected on create and is not invented in the update format. Integration identifiers exposed by a kind's ordinary authoring request are opaque configuration values, not managed-entity identity or cross-entity references; the tool sends them as authored and does not provision integrations, grant integration access, retrieve credentials, or invent a resolver. Secret-bearing request members remain prohibited from YAML by DR-004/009.
  * **DR-012** — For every field in each entity and per-kind Datasource schema, the pinned applicable create and update contracts MUST identify whether the field is: (a) authoring-required, (b) optional authorable and null-accepting, (c) authoring-only and transformed, (d) operation-inapplicable, (e) tool-owned or mixed-ownership, or (f) read-only/prohibited. A field may be classified as optional authorable only when every request operation in which its direct or transformed outbound property exists accepts JSON null. Omission or explicit YAML null for that class MUST serialize as an explicit JSON property with value null. If any operation containing the outbound property rejects null, the corresponding YAML field MUST instead be authoring-required, and omission or explicit null MUST fail locally before target existence changes the applicable operation. An authoring-only selector with no outbound property, such as the unused side of the `content`/`contentFrom` choice, is transformed away without a fabricated null; a natural-reference field that maps to an outbound ID property inherits that target property's requiredness/nullability. Create-only fields are absent from an update projection because they are operation-inapplicable there, not because their omission invokes a default. The Workflow `meta_config` container is mixed-ownership because FR-028 requires the reserved identity member and DR-007 preserves existing non-reserved members; its outbound container is assembled under those rules rather than treated as a wholly authorable optional field.

  ### Per-entity field lists

  Fields marked *(runtime/derived)* MUST NOT appear in YAML. Fields marked *(deprecated)* MUST NOT be emitted by the tool.

  #### Assistant (`kind: Assistant`)

  **`metadata` identity:** `slug`; `project` is required in the declaration unless supplied by `.codemie/config.yaml` (FR-022).

  **`spec` required fields:** `name`; `type` (`codemie` is the only supported phase-1 value); the non-null request collections/flags `context`, `toolkits`, `conversation_starters`, `shared`, `mcp_servers`, `sub_assistants`, `enabled_builtin_subagents`, `skills`, and `categories`; and, for `type: codemie`, `system_prompt` and `llm_model_type`. The authoring reference fields `sub_assistants` and `skills` project to the request's non-null `assistant_ids` and `skill_ids` properties. The inspected shared `AssistantRequest` create/update model rejects JSON null for these properties, so its defaults do not make them optional under DR-012.

  **Additional `spec` authoring fields** (see `AssistantRequest`): `description`, `icon_url`, `enable_image_generation`, `image_generation_model`, `agent_mode` (`"general"` | `"plan_execute"`), `plan_prompt` (required if `agent_mode: plan_execute`), `temperature` (0.0–2.0), `top_p` (0.0–1.0), `tools_tokens_size_limit`, `smart_tool_selection_enabled`, `hedging_config`, `interactive_features`, `prompt_variables`, `custom_metadata`, `guardrail_assignments`; required collection items retain their exposed structures, including `toolkits` items `{toolkit, tools, label?, settings_config?, settings?, is_external?}`, `sub_assistants` natural-key refs, `skills` natural-key refs, and at most three `categories`. Presence in this list does not by itself mean omission/null is allowed; the bundled pinned schema MUST classify each field under DR-012. A field with a server default but a null-rejecting type is authoring-required rather than optional.

  **Runtime/derived (never in YAML):** `id`, `created_by`, `created_date`, `updated_date`, `creator`, `system_prompt_history`, `version_count`, `unique_users_count`, `unique_likes_count`, `unique_dislikes_count`, `nested_assistants`, `origin`

  **Deprecated (do not emit):** `is_react`

  #### Skill (`kind: Skill`)

  **`metadata` identity:** `name` (kebab-case `^[a-z0-9]+(-[a-z0-9]+)*$`, 3–64 chars); `project` is required in the declaration unless supplied by `.codemie/config.yaml` (FR-022).

  **`spec` required fields:** `description` (10–1 000 chars); `visibility` (`"private"` | `"project"` | `"public"`); `categories` (max 3); `toolkits`; `mcp_servers`; `companion_files`; `enabled_builtin_subagents`; and exactly one of `content` (inline markdown 100–30 000 chars) **or** `contentFrom: <relative path to .md file>` (sidecar reference, resolved to inline content at apply time). The inspected `SkillCreateRequest` supplies defaults for the listed visibility/list fields but uses non-null types, so explicit JSON null is rejected and DR-012 makes them authoring-required. Providing both or neither content form is a lint failure. (FR-022, FR-025)

  **Runtime/derived:** `id`, `created_by`, `created_date`, `updated_date`, `unique_likes_count`, `unique_dislikes_count`

  #### Datasource (`kind: Datasource`) — desired-state subset only

  **`metadata` identity:** `repo_name`; `project` is required in the declaration unless supplied by `.codemie/config.yaml` (FR-022).

  **`spec` required fields:** `index_type`, `description` (max 500 chars)

  **Ordinary per-kind authoring inventory:** `metadata.repo_name` and effective project provide the Datasource identity envelope. The remaining fields stay flat and retain the selected kind's existing CodeMie request names; this table does not define client aliases. Requiredness, nullability, constraints, and operation applicability MUST be pinned from the same target release under DR-012 before that row is implementation-ready.

  | `spec.index_type` | Existing CodeMie authoring fields and current contract note |
  |---|---|
  | `git` | `description`, `link`, `branch`, `filesFilter`, `indexType`, `embeddingsModel`, `summarizationModel`, `prompt`, `docsGeneration`, `projectSpaceVisible`, `setting_id`, `guardrail_assignments`, `cron_expression`, `timezone`. Uses the code-repository create/update format and its camelCase members where exposed. |
  | `svn` | The repository format above with the existing SVN link/branch constraints; `setting_id`, `guardrail_assignments`, `cron_expression`, and `timezone` retain their exposed names. |
  | `confluence` | Knowledge-base common fields `description`, `project_space_visible`, `guardrail_assignments`, plus `cql`, `setting_id`, `include_restricted_content`, `include_archived_content`, `include_attachments`, `include_comments`, `keep_markdown_format`, `keep_newlines`, `embedding_model`, `cron_expression`, `timezone`. |
  | `jira` | Knowledge-base common fields plus `jql`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`. |
  | `xray` | Knowledge-base common fields plus `jql`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`. |
  | `file` | Knowledge-base common fields plus create/update multipart fields `files`, `uploaded_files` (update retained-file list), `csv_separator`, `csv_start_row`, `csv_rows_per_document`, `embedding_model`, `include_email_attachments`; guardrail assignments use the exact body/query representation exposed by the operation. File additions, removals, and content changes are ordinary authoring inputs. |
  | `google` | Knowledge-base common fields plus exact create-format `googleDoc`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`. `googleDoc` and `setting_id` are operationally required on create. The current ordinary update accepts `description`, `project_space_visible`, `guardrail_assignments`, `cron_expression`, and `timezone`; create-only fields are not invented in its update projection. Persisted server type `llm_routing_google` maps to authored discriminator `google`. |
  | `azure_devops_wiki` | Knowledge-base common fields plus `wiki_query`, `wiki_name`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`. |
  | `azure_devops_work_item` | Knowledge-base common fields plus `wiql_query`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`. |
  | `sharepoint` | Knowledge-base common fields plus `site_url`, `include_pages`, `include_documents`, `include_lists`, `max_file_size_mb`, `files_filter`, `auth_type`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`, `oauth_client_id`, and `oauth_tenant_id`. The server's secret-bearing `access_token` member is not authorable in YAML under DR-004; integration provisioning/authentication remains outside this entity declaration. |
  | `provider` | Exact `provider_name`, `toolkit_id`, and provider-defined create/update request fields exposed by the pinned provider schema. Because the schema is provider- and deployment-specific, a kind is authorable only when its exact schema can be bundled for offline validation; the tool MUST NOT infer or normalize it. |
  | `bedrock` | The inspected source exposes a persisted/imported Bedrock Datasource form but no ordinary create/update request in the Datasource router/UI flow. The tool MUST NOT invent one; this row becomes authorable only when the pinned target exposes an ordinary CRUD format compatible with FR-005. |

  For knowledge-base creates, `metadata.repo_name` projected as request `name` uses the exposed 4–50 character `^[a-zA-Z0-9][\w-]*$` constraint; `description` is 1–500 characters. `guardrail_assignments` items require `guardrail_id`, `mode` (`all` or `filtered`), and `source` (`input`, `output`, or `both`) and may include `editable` and `guardrail_name`. A non-empty `cron_expression` MUST be valid and no more frequent than hourly; an authored `timezone` MUST be a valid IANA time-zone name. A field whose pinned request rejects null remains authoring-required even if the server model supplies a default.

  **Read-only response fields:** fields not present in the selected kind's ordinary authoring request are not authorable under DR-010 and VR-015.

  #### Workflow (`kind: Workflow`)

  **`metadata` identity:** `slug`; `project` is required in the declaration unless supplied by `.codemie/config.yaml` (FR-022). The effective pair is persisted by FR-028.

  **`spec` required fields:** `name`, `description`, `mode` (confirmed enum: `"Sequential"` [default, non-deprecated] | `"Autonomous"` [deprecated — do not use in new declarations]), `shared`, `execution_config` (validated against `execution_config_schema.yaml`). The inspected update request rejects JSON null for `mode` and `shared`; both therefore remain authoring-required even though the server models supply defaults.

  **Additional `spec` authoring fields:** `supervisor_prompt` (required when `mode: Autonomous`), `start_hint`, `icon_url`, `meta_config` (mixed-ownership container; non-reserved authored members plus the tool-owned Workflow identity member), `guardrail_assignments`. Field-level omission/null classification follows DR-012; `meta_config` follows the specific DR-007/012 merge rule.

  **`spec.execution_config` fields** (from JSON Schema): `messages_limit_before_summarization` (int 1–10 000), `tokens_limit_before_summarization`, `type`, `enable_summarization_node`, `recursion_limit` (int 1–5 000), `max_concurrency` (int 1–100), `verbose`, `max_iteration_key_output_limit`, `assistants`, `tools`, `custom_nodes`, `states`, `retry_policy`.

  **Workflow assistant/resource authoring:** each `assistants[]` item has workflow-local `id` and exactly one of persisted `assistantRef: {project, slug}` or inline `system_prompt` form. Only inline items may declare `skillRefs: [{project, name}]` and `datasourceRefs: [{project, repo_name}]`. `states[].assistant_id` continues to point to the local `assistants[].id`; it is not a server UUID. See FR-035.

  **Do not emit deprecated fields:** `keep_history`, `wait_for_user_confirmation`

  ### Cross-entity reference resolution

  At apply time, the tool translates natural-key references to the server representation required by the target API. A Skill reference uses the exhaustive, exact, ambiguity-refusing resolver in FR-031; a Workflow reference uses the exact reserved-record resolver in FR-029 when server-ID translation is required. Resolution order is the caller's responsibility (FR-005: one entity per invocation). If a referenced entity is missing, ambiguous, or not completely visible at apply time, the tool MUST fail with a safe diagnostic identifying the source file and reference field path, without reproducing the reference value, and MUST NOT guess a target (VR-005).

  ---

  ## 16. Integration requirements

  * **IR-001** — The tool MUST integrate with the CodeMie platform via its REST API (`/openapi.json`, `/docs`) directly. No SDK wrapper in phase 1. The tool MUST NOT hard-depend on the `codemie` server package (the Rust binary cannot import Python packages). (OQ-4 resolved.)
  * **IR-002** — The tool MUST include `apiVersion` in every YAML and reject files with an unrecognised version. Mismatch between tool schema version and target server: fail fast. Server-side version indicator: `GET /v1/info` → `{"version": "..."}` (OQ-2 resolved). Enforcement policy (whether to gate `apply` on server version) is an architect decision; per-entity output remains limited to FR-026.
  * **IR-003** — The tool MUST support HTTPS and respect environment-provided CA bundles / proxies.
  * **IR-004** — Network failures (connection error, timeout, 5xx) during apply SHOULD be retried with bounded exponential backoff. **Concrete policy: implementation decision — see §29.**
  * **IR-005** — The tool MUST NOT require write access to entity types outside the four in-scope kinds.
  * **IR-006** — **Authentication modes (phase 1):**
    * `CODEMIE_TOKEN` environment variable — **primary CI mode.** Tool sends `Authorization: Bearer $CODEMIE_TOKEN` on every request. The caller is responsible for acquiring and caching the token. The `--token` CLI flag is **not accepted**; supplying it is an exit-2 local failure before network access (SEC-001 remediation, v25).
    * `codemie-gitops login` command — acquires a bearer token and writes it as a single line to stdout. Supports three mutually exclusive modes as defined by FR-024: **(a)** `CODEMIE_CLIENT_ID` (or `--client-id`) + `CODEMIE_CLIENT_SECRET` (environment only) + explicit `auth_url` → `grant_type=client_credentials` at that exact Keycloak token endpoint (CI / production); **(c)** `CODEMIE_EMAIL` (or `--email`) + `CODEMIE_PASSWORD` (environment only) + explicit `auth_url` (no `CODEMIE_CLIENT_SECRET`) → `grant_type=password` at that exact Keycloak token endpoint, `client_id` defaults to `codemie-sdk` (human users / developer access to Keycloak-backed instances); **(b)** `CODEMIE_EMAIL` (or `--email`) + `CODEMIE_PASSWORD` (environment only), no `auth_url` (no `CODEMIE_CLIENT_SECRET`) → `POST /v1/local-auth/login` (local dev only). For all modes, the CLI MUST use the explicitly configured endpoint and MUST NOT derive it from `CODEMIE_URL`, config `url`, or a convention. Missing endpoint configuration where required is exit code 2 before network access. Secret credentials MUST come from CI secret stores via environment variables and MUST NOT be stored in `.codemie/config.yaml`.
    * **Token lifetime:** ~8 hours. CI pipelines MUST cache the token across steps and MUST NOT call `login` once per entity. The typical pattern is: `CODEMIE_TOKEN=$(codemie-gitops login ...)` at the start of the job, then reuse across all subsequent `apply` invocations.
    * **Not supported:** browser-redirect / OAuth2 authorization-code flow; static API keys. `POST /v1/local-auth/login` is supported but **only for local dev** (see mode (b) above); MUST NOT be used in CI pipelines.
    * **Project integrations constraint (A-3):** Service-account credentials access only Project-level integrations. Personal integrations (user's own Jira/Git connections) are silently ignored by the server. This is an operational prerequisite, not a tool behavior. Teams MUST migrate personal integrations to project level before the tool's datasource apply will function correctly.
  * **IR-007** — The tool MUST NOT retry create requests on 4xx responses. On 400/409/422, it MUST emit an FR-016-safe rejection diagnostic to stderr, leave stdout empty, and exit code 1.
  * **IR-008** — Every Datasource write MUST use the selected kind's compatible ordinary create/update route and operation-specific request format. Optional authorable request properties MUST be present as explicit null under FR-021/DR-012 rather than omitted to select a server default.
  * **IR-009** — The target CodeMie deployment MUST expose list/read responses sufficient for the tool to prove complete Workflow and Skill visibility, enumerate every result page, exact-filter the approved natural key, verify write permission, and detect post-write ambiguity. If the compatible API cannot supply that evidence, Workflow or Skill apply MUST fail before the affected write.
  * **IR-010** — The target Workflow API MUST preserve the reserved Workflow identity record and unrelated `meta_config` members on create/update and return them on every response used for exhaustive identity resolution. The target Skill API MUST return stable pagination, exact project/name values, permissions, and server IDs for exhaustive resolution. These are compatibility prerequisites, not permissions to modify the server implementation from this repository.

  ---

  ## 17. Quality requirements

  * **QR-001 (Reliability)** — Given a valid declaration and a healthy environment, `apply` MUST succeed or fail deterministically; MUST NOT leave an asset in a half-created state that a subsequent apply cannot reconcile.
  * **QR-002 (Repeat-apply consistency)** — Reapplying the same valid YAML MUST follow FR-006: the tool resolves the same natural key, sends another update for an existing entity, reports `updated` when accepted, and never reports `unchanged`.
  * **QR-003 (Safety)** — No destructive operations in phase 1 (FR-008).
  * **QR-004 (Output integrity)** — Every successful apply produces a per-asset reconciliation outcome on stdout without external provenance. A failed apply produces no stdout outcome and instead emits a safe failure diagnostic to stderr. With `--output json`, the applicable stream contains one single-line JSON object; text is the default. Git/CI/platform audit correlation is outside the CLI. **RESOLVED (Q24, refined by OQ-29 and PRODUCT-OQ-01).**
  * **QR-005 (Performance)** — Each `apply` invocation (single entity) SHOULD complete within a CI-acceptable time budget. **Concrete threshold: UNRESOLVED.**
  * **QR-006 (Portability)** — The tool MUST run on Linux x86_64 CI runners without OS-specific dependencies and be distributed as the standalone Rust binary required by §24.
  * **QR-007 (Security)** — The tool MUST NOT print or persist tokens, credentials, authorization headers, cookies, secret-classified fields, secret-like values, request payloads, server-provided error text, or complete request/response bodies. The successful `login` token line is the sole intentional exception. All diagnostic-producing paths, including debug, verbose, trace, panic, and fatal/internal errors, MUST construct output exclusively from the FR-016 allowlist. Secret credentials (bearer token, client secret, and password) MUST be accepted exclusively through environment variables (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, `CODEMIE_PASSWORD`) and MUST NOT be accepted as CLI flag values. `CODEMIE_PASSWORD` applies to both Mode (b) local-auth and Mode (c) Keycloak ROPC. The non-secret client ID (`--client-id` / `CODEMIE_CLIENT_ID`) and email (`--email` / `CODEMIE_EMAIL`) MAY be supplied as flags. All secret credentials are normally populated from the invoking CI secret store. (SEC-001 remediation, v25; ROPC mode added v26.)
  * **QR-008 (Compatibility)** — Assets whose natural key is NOT declared in any YAML targeted by the current invocation MUST remain untouched (BR-002).
  * **QR-009 (Identity safety)** — Workflow and Skill resolution MUST prefer a visible failure over selecting from incomplete, invalid, unstable, or multiple identity evidence. No “first”, “newest”, “owned”, display-name, or list-order tiebreak is permitted.
  * **QR-010 (Concurrency control)** — Production CI apply jobs MUST be serialized per target environment. During a serialized deployment window, the adopting team MUST govern UI and other API clients so they do not concurrently create the same Workflow/Skill natural key, adopt another Workflow for the same key, or modify the reserved Workflow identity record. This is an operational prerequisite because the server does not enforce the Workflow record or Skill natural key globally.
  * **QR-011 (Residual-race visibility)** — A Workflow or Skill write followed by ambiguous identity evidence MUST produce a safe stderr diagnostic that distinguishes “no write performed” from “write may already have committed,” while leaving stdout empty. Automatic deletion, rollback, or arbitrary target selection is prohibited.

  ---

  ## 18. Business rules

  * **BR-001** — A YAML declaration supplies the request values for the safely resolved entity it declares. Every valid invocation creates when the identity contract proves the entity missing or updates when it returns exactly one safe match; it does not suppress an update because current values appear equal. Committing YAML does not by itself authorize name-based selection or resolution through ambiguous identity evidence.
  * **BR-002** — Assets not declared in the current invocation MUST NOT be modified.
  * **BR-003** — The tool applies exactly one entity per invocation. Dependency ordering (e.g. skills before referencing assistants) is the caller's responsibility.
  * **BR-004** — The reserved Workflow identity record establishes identity only. It does not establish ownership or bypass server authorization.
  * **BR-005** — Changing Workflow project/slug or Skill project/name declares a new natural key. The tool reconciles or creates the new key and leaves the previous entity because automatic rename, move, and delete are out of scope. Renaming or moving only the YAML file does not change identity.
  * **BR-006** — Known duplicate Workflow identity records or duplicate exact Skill natural keys require manual platform remediation outside this CLI. Phase 1 MUST NOT choose or delete one automatically.

  ---

  ## 19. Permissions and authorization

  * **PA-001** — The tool operates under a single principal per invocation. Authorization is enforced by the server.
  * **PA-002** — The tool MUST NOT attempt to read/write assets outside the declared input file.
  * **PA-003** — For CI, a service account with only the read/create/update permissions required for in-scope entity types is REQUIRED. Provisioning is a platform-admin responsibility.
  * **PA-004** — Git history, CI logs, and CodeMie platform audit records independently own provenance and audit correlation. The CLI MUST expose none of their provenance fields and MUST NOT emit Git commit SHA, target environment origin, Git author, CI-run identity, or a replacement provenance field. Establishing a cross-system audit trail is outside the product scope. This does not remove the bounded request/correlation ID permitted for safe failure diagnosis by FR-016 and VR-011.
  * **PA-005** — The Workflow/Skill CI principal MUST be able to prove complete visibility for the effective project (project manager/admin or an equivalent role in the pinned target contract). A principal with only creator-scoped or partial visibility is unsupported for Workflow/Skill apply.
  * **PA-006** — Only authorized platform operations may create, change, restore, or remove the reserved Workflow identity record outside the tool. The adopting team MUST maintain an inventory and a manual remediation path for invalid or duplicate records.
  * **PA-007** — Provisioning Datasource integrations, assigning access to them, and retrieving their credentials are outside the tool's entity-management scope for every Datasource kind. Apply sends only the non-secret integration configuration exposed by the selected kind's ordinary authoring format; the server enforces integration existence, type, ownership/access, and use permission. A server rejection after locally valid input follows FR-011 and never reproduces the submitted value or server body.

  ---

  ## 20. Validation rules

  * **VR-001** — Every YAML MUST declare a `kind` from: `Assistant`, `Workflow`, `Datasource`, `Skill`.
  * **VR-002** — Every asset MUST have a non-empty natural key in `metadata`.
  * **VR-003** — Authoring-required fields per entity and operation MUST be present with values allowed by the pinned schema; optional authorable fields MAY be omitted; unknown fields MUST be rejected (FR-022/023).
  * **VR-004** — Enumerated fields MUST be validated against values defined in the tool's bundled schema. **Confirmed enum values (source: model inspection 2026-08-07):**
    * `WorkflowMode`: `"Sequential"` (default, non-deprecated), `"Autonomous"` (deprecated — lint SHOULD warn)
    * `AgentMode` (Assistant): `"general"` (default), `"plan_execute"`
    * `AssistantType`: `"codemie"` (phase 1 only), `"A2A"`, `"bedrock_agent"`, `"bedrock_agentcore_runtime"` (deferred)
    * `SkillVisibility`: `"private"` (default), `"project"`, `"public"`
  * **VR-005** — Cross-references MUST resolve at apply time; server-side check is OPTIONAL for lint (offline) and REQUIRED for apply. A missing/invalid repository reference is a local validation failure (exit 2); a locally valid natural reference that is missing or ambiguous on the server is an entity-reconciliation failure (exit 1).
  * **VR-006** — File paths and asset identifiers MUST NOT contain characters that break shell or URL usage.
  * **VR-007** — Workflow `metadata.project` and `metadata.slug` MUST be non-empty exact strings suitable for persistence in the Workflow identity record. User-authored `spec.meta_config` containing `codemie.epam.com/gitops/workflow-identity` MUST fail lint and apply before network writes.
  * **VR-008** — `--adopt-workflow-id` MUST accept one syntactically valid current server UUID and MUST be rejected for non-Workflow declarations. The value is an invocation selector, not declaration data.
  * **VR-009** — Natural-key equality for Workflow and Skill resolution MUST be exact after decoding. The tool MUST NOT trim, case-fold, fuzzy-match, or use display-name similarity as identity.
  * **VR-010** — Any failure determined solely from CLI arguments, repository configuration, declaration files, sidecar files, or the bundled schema MUST use exit code 2 and MUST prevent network access. Online identity ambiguity, invalid server identity evidence, or entity-resolution instability in otherwise compatible responses MUST use exit code 1; inability to establish authentication, authorization, complete visibility, write permission, response-contract compatibility, or connectivity MUST use exit code 2.
  * **VR-011** — A server request/correlation identifier MAY appear in a diagnostic only when obtained from the dedicated `X-Request-ID` or `X-Correlation-ID` response header and when its complete value matches `[A-Za-z0-9._:-]{1,128}`; otherwise it MUST be omitted. Target environment URL/origin, Git commit SHA, Git author, and CI-run identity are never eligible for CLI output. No response-body field, arbitrary response header, or URL component is eligible for diagnostic output.
  * **VR-012** — Secret classification MUST use a fixed, reviewable set of credential inputs, standard sensitive HTTP headers/cookies, and schema fields explicitly designated as secret-bearing. Values from those sources MUST never be output. Heuristic warnings MAY identify a suspicious field (FR-014), but the no-leak guarantee MUST NOT depend on detecting secrets in arbitrary free-form data because request/response bodies, payloads, declaration values, and sidecar values are never diagnostic inputs.
  * **VR-013** — A Workflow assistant entry MUST contain exactly one of `assistantRef` or inline `system_prompt`. `skillRefs` and `datasourceRefs` MUST be rejected on an `assistantRef` entry. Server-resource fields `assistant_id`, `skill_ids`, and `datasource_ids` MUST be rejected in `assistants[]`; every `states[].assistant_id` MUST exactly match one unique workflow-local `assistants[].id`.
  * **VR-014** — Datasource declarations and CLI arguments MUST reject dedicated lifecycle-control commands, flags, and endpoint-operation fields. A supplied `--reindex-datasources` or equivalent unknown/forbidden option is a local CLI usage/validation failure (exit 2). Ordinary per-kind source/content/file and scheduling fields remain valid when they are members of that kind's existing create/update request.
  * **VR-015** — A Datasource declaration MUST satisfy the selected kind's exact DR-010/§15 field names, casing, nesting, required/optional rules, enum values, and constraints. Fields from another kind, client-invented aliases/wrappers, runtime/read-only fields, and secret-bearing request members MUST be rejected locally with exit code 2. A locally valid opaque integration identifier rejected by the server is an exit-1 server rejection; the diagnostic MUST NOT reproduce the identifier or server body.
  * **VR-016** — For each direct or transformed create/update request property, validation MUST use pinned operation nullability rather than infer optionality from the existence of a server default. If JSON null is accepted in every operation containing the property, omission and explicit YAML null are valid and project to JSON null. If null is rejected in any operation containing the property, the corresponding field is authoring-required in YAML; omission or explicit YAML null is exit code 2 before any server call. Authoring-only fields with no outbound property, operation-inapplicable fields, tool-owned/mixed-ownership structures, and read-only fields follow DR-012 and MUST NOT receive a fabricated null property.

  ---

  ## 21. Acceptance criteria

  ### AC-FR-001-01 — Lint accepts a valid YAML
  ```gherkin
  Given a YAML conforming to the bundled schema for its kind
  When lint is run
  Then the tool exits code 0
  And prints a passing summary
  ```

  ### AC-FR-003-01 — Lint rejects unknown field
  ```gherkin
  Given a YAML with an unrecognised field in spec
  When lint is run
  Then the tool exits code 2
  And stdout is empty
  And stderr names the file, line, and offending field without reproducing its value
  And no server call is made
  ```

  ### AC-FR-004-01 — Lint catches broken cross-reference
  ```gherkin
  Given an assistant referencing a datasource name not present in the repo
  When lint is run
  Then the tool exits code 2
  And stdout is empty
  And stderr names the offending reference field without reproducing its value
  And no server call is made
  ```

  ### AC-FR-005-01 — Apply creates a missing non-Datasource asset
  ```gherkin
  Given a non-Datasource YAML whose kind-specific resolver proves the natural key is missing
  When apply is invoked
  Then the tool creates the entity
  And prints "created"
  And exits 0
  ```

  ### AC-FR-005-02 — Apply updates one safely resolved existing asset
  ```gherkin
  Given the kind-specific resolver returns exactly one safe match for the authored natural key
  When apply is invoked
  Then the tool sends one update request containing the declaration's operation-specific request projection
  And prints "updated"
  ```

  ### AC-FR-006-01 — Repeat apply always updates an existing entity
  ```gherkin
  Given apply has just completed successfully
  When apply is invoked again with the same YAML
  Then the tool resolves the existing entity
  And sends another ordinary update request
  And reports "updated"
  And never reports "unchanged"
  And the tool exits 0
  ```

  ### AC-FR-008-01 — Apply does not touch entities outside the target YAML
  ```gherkin
  Given the server contains an assistant "legacy-A" not declared in the target YAML
  When apply is invoked for a different entity
  Then "legacy-A" is not modified or deleted
  ```

  ### AC-FR-009-01 — Apply fails cleanly without credentials
  ```gherkin
  Given no token is provided
  When apply is invoked
  Then the tool exits code 2
  And stdout is empty
  And stderr contains a safe authentication-category diagnostic without a token, credential, authorization header, cookie, payload, or body
  And makes no server-modifying calls
  ```

  ### AC-FR-011-01 — Server rejection surfaces cleanly
  ```gherkin
  Given local input is valid
  And the server rejects entity reconciliation with 400, target 404, 409, or 422
  When apply is invoked
  Then the tool exits code 1
  And stdout is empty
  And stderr contains a safe synthesized diagnostic with a stable error code, category, non-sensitive HTTP status, and request or correlation ID when safely available
  And neither the server-provided error text/body nor the request payload or sensitive values appear in any output or persisted tool data
  ```

  ### AC-FR-011-02 — Local failures consistently use exit code 2
  ```gherkin
  Given an invocation has a CLI usage, YAML parse, schema, semantic-validation, repository-reference, sidecar, or configuration failure
  When lint or apply is invoked
  Then the tool exits code 2
  And stdout is empty
  And stderr contains only FR-016-allowlisted diagnostic fields and does not reproduce an invalid declaration or sidecar value
  And no server call is made
  ```

  ### AC-FR-011-03 — Online entity reconciliation failures use exit code 1
  ```gherkin
  Given local input is valid
  And online reconciliation finds a missing server reference, adoption-required Workflow, invalid or duplicate Workflow identity, ambiguous Skill identity, entity-resolution instability in otherwise compatible responses, or server-side declaration rejection
  When apply completes
  Then the tool exits code 1
  And stdout is empty
  And stderr contains a safe category-specific diagnostic
  ```

  ### AC-FR-011-04 — Online precondition and infrastructure failures use exit code 2
  ```gherkin
  Given local input is valid
  And authentication, authorization, complete identity visibility, write permission, API compatibility, connectivity, or server availability cannot be established
  When apply completes
  Then the tool exits code 2
  And stdout is empty
  And stderr contains a safe precondition, compatibility, connectivity, or availability diagnostic as applicable
  ```

  ### AC-FR-014-01 — Lint warns on suspicious credential field
  ```gherkin
  Given a YAML contains a high-entropy string in a credential field
  When lint is run
  Then the tool writes a warning to stderr naming the source location and suspicious field
  And the warning does not contain the field value or any derivative of that value
  ```

  ### AC-QR-007-01 — Failure diagnostics are allowlisted under every verbosity setting
  ```gherkin
  Given a valid local declaration whose server request is rejected
  And the request payload and server response body contain known test secrets
  And the response includes an X-Request-ID containing whitespace or control characters
  When apply is invoked in text or JSON mode under each supported verbosity setting
  Then the tool exits code 1
  And stdout is empty
  And stderr contains only the fields permitted by FR-016 for the selected mode
  And neither test secret, complete body, payload, server error text, exception dump, nor malformed X-Request-ID appears in output or persisted tool data
  ```

  ### AC-FR-015-01 — Renaming a YAML does not duplicate the asset
  ```gherkin
  Given an asset was previously applied from A.yaml
  When A.yaml is renamed to B.yaml with no change to metadata
  And apply is invoked with B.yaml
  Then the same server-side entity is updated (no duplicate created)
  ```

  ### AC-FR-021-01 — Omitted optional field becomes explicit null
  ```gherkin
  Given an Assistant, Workflow, Skill, or authorable Datasource-kind declaration omits a spec field
  And the pinned applicable create and update request contracts accept JSON null for that property
  And the entity is initially missing
  When apply is invoked once to create it and again to update it
  Then both the create payload and the update payload contain that property with JSON null
  And neither payload omits the property for the server to select a default
  ```

  ### AC-FR-021-02 — Explicit YAML null has the same outbound meaning
  ```gherkin
  Given a declaration explicitly sets an optional authorable field to YAML null
  And the pinned applicable request contract accepts JSON null for that property
  When lint and apply are invoked
  Then lint accepts the field
  And the applicable create or update payload contains that property with JSON null
  And the result is identical to omitting that field from YAML
  ```

  ### AC-FR-022-01 — Lint fails on missing required field
  ```gherkin
  Given an assistant YAML omitting spec.system_prompt
  When lint is run
  Then the tool exits code 2
  And the output names spec.system_prompt as missing
  ```

  ### AC-FR-022-04 — A null-rejecting field is authoring-required
  ```gherkin
  Given the pinned applicable request contract defines a field with a server default but rejects JSON null
  And a declaration omits that field or explicitly sets it to YAML null
  When lint or apply is invoked
  Then the tool exits code 2 before any server call
  And stdout is empty
  And stderr identifies the required field path without reproducing its value
  ```

  ### AC-FR-024-01 — Login writes token to stdout only
  ```gherkin
  Given a valid client_id and client_secret for a provisioned service account
  And CODEMIE_CLIENT_SECRET is set to a valid client secret
  And the exact reachable Keycloak token endpoint is supplied as --auth-url $AUTH_URL
  When codemie-gitops login --auth-url $AUTH_URL --client-id $CLIENT_ID is invoked
  Then a single bearer token line is written to stdout
  And the tool exits 0
  And no token appears in stderr
  And no credentials appear in stdout or stderr
  And the token and credentials are not written to any file, cache, outcome, or tool-created log
  ```

  ### AC-FR-024-02 — Login fails cleanly on bad credentials
  ```gherkin
  Given an invalid client_secret
  And an explicit Keycloak token endpoint is configured
  When codemie-gitops login is invoked
  Then the tool exits code 2
  And a fixed safe authentication-category diagnostic and non-sensitive status are written to stderr
  And stdout is empty
  And the supplied credentials, identity-provider error text, request payload, response body, and any token are absent from output and persisted tool data
  ```

  ### AC-FR-024-03 — Login token is usable for apply
  ```gherkin
  Given CODEMIE_AUTH_URL, CODEMIE_CLIENT_ID, and CODEMIE_CLIENT_SECRET are set
  And CODEMIE_TOKEN=$(codemie-gitops login)
  When apply is invoked with CODEMIE_TOKEN set
  Then the server accepts the request
  And the apply succeeds
  ```

  ### AC-FR-024-04 — Local-auth login succeeds against a dev server
  ```gherkin
  Given a dev server with ENABLE_USER_MANAGEMENT=True and IDP_PROVIDER="local"
  And valid email and password credentials
  And CODEMIE_PASSWORD is set to a valid password
  When codemie-gitops login --url http://localhost:8080 --email $EMAIL is invoked
  Then a single bearer token line is written to stdout
  And the tool exits 0
  And neither the token nor the credentials appear in stderr or persisted tool data
  ```

  ### AC-FR-024-05 — Local-auth against Keycloak server returns informative error
  ```gherkin
  Given a production server backed by Keycloak (IDP_PROVIDER != "local")
  And CODEMIE_PASSWORD is set
  When codemie-gitops login --url $URL --email $EMAIL is invoked
  Then the tool exits code 2
  And stdout is empty
  And stderr contains a fixed safe message indicating that local authentication is unavailable
  And the supplied credentials and server response body or error text are not printed or persisted
  ```

  ### AC-FR-024-06 — Mixing auth mode flags is an error
  ```gherkin
  Given both --client-id and --email flags are supplied in a single login invocation
  Then the tool exits code 2 before making any network call
  And stdout is empty
  And stderr states the flags are mutually exclusive
  And stderr does not reproduce any supplied credential value
  ```

  ### AC-FR-024-07 — Missing explicit Keycloak endpoint fails locally
  ```gherkin
  Given valid Keycloak client credentials are supplied
  And --auth-url is absent
  And CODEMIE_AUTH_URL is absent
  And .codemie/config.yaml has no auth_url
  And a CodeMie API URL is available through --url, CODEMIE_URL, or config url
  When codemie-gitops login is invoked
  Then the tool exits code 2 before making any network call
  And stdout is empty
  And stderr contains a safe missing-auth-endpoint configuration diagnostic
  And the tool does not derive or attempt an endpoint from the CodeMie API URL
  ```

  ### AC-FR-024-08 — Keycloak ROPC login acquires a token
  ```gherkin
  Given CODEMIE_AUTH_URL (or --auth-url or config auth_url) is set to a reachable Keycloak token endpoint
  And CODEMIE_EMAIL is set to a valid user email
  And CODEMIE_PASSWORD is set to the corresponding password
  And CODEMIE_CLIENT_SECRET is NOT set
  When codemie-gitops login is invoked
  Then the tool selects Mode (c) Keycloak ROPC
  And sends a POST request to the configured auth_url with grant_type=password, client_id (defaulting to codemie-sdk when CODEMIE_CLIENT_ID is not set), username, and password
  And no client_secret field is included in the request
  And a single bearer token line is written to stdout
  And the tool exits 0
  And neither the token, email, password, nor any credential appears in stderr or persisted tool data
  ```

  ### AC-FR-017-01 — Keycloak endpoint sources have deterministic precedence
  ```gherkin
  Given .codemie/config.yaml defines auth_url as $CONFIG_AUTH_URL
  And CODEMIE_AUTH_URL is $ENV_AUTH_URL
  And --auth-url $FLAG_AUTH_URL is supplied
  When Keycloak login is invoked with valid client credentials
  Then the token request is sent only to $FLAG_AUTH_URL
  And neither $ENV_AUTH_URL nor $CONFIG_AUTH_URL is contacted
  ```

  ### AC-FR-017-02 — Credentials are rejected in repository configuration
  ```gherkin
  Given .codemie/config.yaml contains client_secret or another credential field
  When a command loads the repository configuration
  Then the tool exits code 2 before making any network call
  And stdout is empty
  And stderr identifies only the prohibited field path and safe configuration category
  And stderr does not reproduce the credential value
  ```

  ### AC-FR-025-01 — contentFrom sidecar resolves at lint time
  ```gherkin
  Given a skill YAML containing spec.contentFrom: ./my-skill.md
  And my-skill.md does not exist relative to the YAML file
  When lint is run
  Then the tool exits code 2
  And the output names the missing sidecar file
  ```

  ### AC-FR-025-02 — contentFrom content is inlined at apply time
  ```gherkin
  Given a skill YAML with spec.contentFrom: ./my-skill.md
  And my-skill.md exists and contains valid markdown within the length constraints
  When apply is invoked
  Then the server receives the markdown content inline
  And spec.contentFrom does not appear in the API payload
  ```

  ### AC-FR-025-03 — Providing both content and contentFrom is a lint failure
  ```gherkin
  Given a skill YAML with both spec.content and spec.contentFrom set
  When lint is run
  Then the tool exits code 2
  And the output states the fields are mutually exclusive
  ```

  ### AC-FR-026-01 — JSON output contains required fields
  ```gherkin
  Given a successful apply invocation with --output json
  When the entity is created
  Then stdout contains a single-line JSON object with action "created", kind, project, natural key, and exits 0
  ```

  ### AC-FR-026-02 — JSON failure diagnostics use stderr only
  ```gherkin
  Given an apply invocation with --output json that is rejected by the server
  Then stdout is empty
  And stderr contains exactly one single-line JSON diagnostic with errorCode, category, and exitCode
  And the diagnostic may contain only FR-016-allowlisted non-sensitive context such as HTTP status and a safe request or correlation ID
  And the diagnostic contains no action, server message or body, request payload, token, credential, authorization header, cookie, secret field, secret-like value, Git commit SHA, target environment origin, Git author, or CI-run identity
  And the tool exits code 1
  ```

  ### AC-FR-026-03 — Successful output excludes externally owned provenance
  ```gherkin
  Given a successful apply invoked from a Git and CI environment where commit, target origin, author, and run metadata are available
  When output is produced in text or JSON mode
  Then stdout reports only the action, kind, project, and natural key
  And neither stdout nor stderr contains a Git commit SHA, target environment origin, Git author, CI-run identity, or replacement provenance field
  ```

  ### AC-IR-008-01 — Datasource apply uses ordinary per-kind CRUD
  ```gherkin
  Given a valid Datasource declaration for an authorable kind
  When apply is invoked
  Then a missing target receives that kind's ordinary create request and reports "created" when accepted
  And an existing target receives that kind's ordinary update request on every invocation and reports "updated" when accepted
  And each request uses the existing operation-specific format
  And every omitted optional authorable request field is present as JSON null
  ```

  ### AC-DR-010-01 — Every Datasource kind preserves its existing authoring format
  ```gherkin
  Given a Datasource declaration selects an authorable kind from the pinned §15 inventory
  And it uses that kind's exact field names, casing, nesting, required values, and constraints
  When lint and apply are invoked
  Then lint accepts the per-kind shape
  And apply projects only the envelope identity and sends the selected kind's ordinary create or update format
  And the tool introduces no integration alias, common subtype wrapper, or cross-kind field translation
  ```

  ### AC-DR-010-02 — Cross-kind or invented Datasource fields fail locally
  ```gherkin
  Given a Datasource declaration contains a field from a different kind or a client-invented alias or wrapper
  When lint or apply is invoked
  Then the tool exits code 2 before any server request
  And stdout is empty
  And stderr identifies only the safe local field path and validation category without reproducing its value
  ```

  ### AC-DR-011-01 — Existing Datasource receives the exact update projection every time
  ```gherkin
  Given an existing Datasource and a valid declaration for its kind
  When apply is invoked
  Then the tool constructs the selected kind's exact ordinary update projection from the declaration
  And sends the update without comparing it to current configuration
  And reports "updated" when the server accepts the request
  And a create-only field is not invented in the update format
  ```

  ### AC-FR-035-01 — Workflow natural references translate at their exact positions
  ```gherkin
  Given a Workflow contains one persisted assistant entry with assistantRef
  And one inline assistant entry with system_prompt, skillRefs, and datasourceRefs
  And states reference both entries by their workflow-local ids
  When apply resolves every natural reference exactly once
  Then the request uses assistant_id only for the persisted actor and skill_ids/datasource_ids only for the inline actor
  And states[].assistant_id remains the workflow-local actor id
  And no server asset UUID appears in the declaration or success outcome
  ```

  ### AC-FR-035-02 — Invalid Workflow actor/resource forms fail locally
  ```gherkin
  Given a Workflow assistant entry contains both assistantRef and system_prompt, resource refs on a persisted entry, or an authored server-resource ID field
  When lint is invoked
  Then the tool exits code 2 before any server call
  And stdout is empty
  And stderr identifies the invalid field path without reproducing its value
  ```

  ### AC-FR-027-01 — Filename does not affect entity identity
  ```gherkin
  Given an assistant YAML at any/path/renamed.yaml with metadata.slug: support-triage
  And exactly one assistant exists on the server under project and slug support-triage
  When apply is invoked
  Then the entity is updated (not duplicated)
  And no identity field changes because of the file path
  ```

  ### AC-FR-028-01 — Workflow creation persists natural-key identity server-side
  ```gherkin
  Given no Workflow identity record exists for project example and slug release-flow
  And no unmarked exact display-name candidate requires adoption review
  When the Workflow declaration is applied
  Then the created Workflow contains meta_config member codemie.epam.com/gitops/workflow-identity
  And that member equals {version: 1, project: "example", slug: "release-flow"}
  And the declaration and outcome contain no Workflow server UUID
  ```

  ### AC-FR-028-02 — Unrelated Workflow metadata is preserved
  ```gherkin
  Given a uniquely resolved Workflow contains non-reserved meta_config members
  When apply adds or updates the reserved Workflow identity record
  Then the non-reserved members remain present unless the declaration explicitly replaces the same member
  ```

  ### AC-FR-029-01 — Ordinary Workflow reconciliation uses only the exact identity record
  ```gherkin
  Given exhaustive Workflow enumeration returns exactly one valid identity record for the exact project and slug
  And another Workflow has the same display name but no matching identity record
  When apply is invoked without --adopt-workflow-id
  Then only the exactly marked Workflow is eligible for update
  And display name does not select the other Workflow
  ```

  ### AC-FR-029-02 — Invalid or duplicate Workflow identity fails safely
  ```gherkin
  Given exhaustive Workflow enumeration reveals an invalid reserved identity record or more than one exact record for the authored project and slug
  When apply is invoked
  Then the tool exits code 1 with an identity-specific failure
  And stdout is empty
  And stderr contains only FR-016-allowlisted diagnostic fields
  And no Workflow write is attempted by ordinary reconciliation
  ```

  ### AC-FR-030-01 — Explicit Workflow adoption preserves the server entity
  ```gherkin
  Given no matching Workflow identity record exists
  And --adopt-workflow-id identifies one writable unmarked Workflow in the exact project
  And its non-reserved meta_config is preservable
  When apply is invoked
  Then that same server entity receives the reserved project and slug identity record
  And its desired state is reconciled in the same operation
  And future ordinary apply resolves it by project and slug
  And neither YAML nor the outcome reports the supplied UUID
  ```

  ### AC-FR-030-02 — Unmarked display-name match is never implicitly adopted
  ```gherkin
  Given no valid Workflow identity record matches the declaration
  And an unmarked Workflow has the same display name
  When apply is invoked without --adopt-workflow-id
  Then the tool does not select or update that Workflow
  And it fails with instructions to inspect and explicitly adopt or change the declaration
  And exits code 1
  ```

  ### AC-FR-031-01 — Skill exhaustive resolution handles zero, one, and multiple matches
  ```gherkin
  Given compatible Skill results span more than one server page
  When apply exhausts all pages and exact-filters project and name
  Then zero exact matches causes one create attempt
  And one exact match causes one update attempt on every invocation
  And multiple exact matches cause an exit-1 ambiguous failure with no match selected
  ```

  ### AC-FR-031-02 — Skill search hints do not define identity
  ```gherkin
  Given the server search response contains case-insensitive, substring, or differently scoped Skill candidates
  When Skill identity is resolved
  Then only exact decoded project and name equality counts as a match
  And creator, recency, write ability, relevance, and list order are not tiebreakers
  ```

  ### AC-FR-033-01 — Incomplete visibility prevents Workflow or Skill writes
  ```gherkin
  Given the current principal cannot prove complete visibility for the effective project
  When Workflow or Skill apply is invoked
  Then the tool exits code 2 before create, update, or adoption
  And stdout is empty
  And a safe stderr diagnostic explains that identity visibility is unproven
  ```

  ### AC-FR-034-01 — Post-write ambiguity is visible and non-destructive
  ```gherkin
  Given a Workflow create/update, Skill create, or Workflow adoption request may have committed
  And post-write exhaustive resolution finds multiple exact matches
  When the invocation completes
  Then the tool exits code 1
  And stdout is empty
  And a safe stderr diagnostic states that a write may already have committed
  And the tool does not retry the create, select a match, delete, or roll back an entity
  ```

  ### AC-QR-010-01 — CI examples serialize target-environment applies
  ```gherkin
  Given the phase-1 GitHub Actions and GitLab CI examples
  When their deployment controls are reviewed
  Then each serializes apply jobs by target environment
  And the accompanying prerequisites prohibit concurrent UI or other-client Workflow/Skill identity writes during the deployment window
  ```

  ### AC-FR-022-02 — Config project default satisfies metadata.project requirement
  ```gherkin
  Given .codemie/config.yaml defines project: my-project
  And a YAML file omits metadata.project
  When lint is run
  Then the tool exits 0 and uses my-project as the effective project
  ```

  ### AC-FR-022-03 — No project from either source fails lint
  ```gherkin
  Given no .codemie/config.yaml project default
  And a YAML file omits metadata.project
  When lint is run
  Then the tool exits code 2
  And stdout is empty
  And stderr states that metadata.project is required but not provided
  ```

  ---

  ## 22. Edge cases and failure scenarios

  * Empty YAML or YAML with only comments → lint fails with exit code 2: "no entity declared."
  * `kind` not in the allowed set → lint fails with exit code 2 (VR-001).
  * Unknown `apiVersion` → lint and apply fail, exit code 2.
  * **Unknown/extra fields in YAML** → lint and apply fail with exit code 2 before network access; offending field name and location reported (FR-023).
  * **Missing authoring-required field** → lint fails with exit code 2, reports the field path, and makes no server call (FR-022).
  * **Optional authorable field omitted** → lint accepts it; create and update include the applicable request property as explicit JSON null (FR-021/022).
  * **Optional authorable field explicitly set to YAML null** → same JSON null projection as omission when the pinned applicable request accepts null.
  * **Server-defaulted field rejects null** → it is authoring-required; omission or explicit YAML null fails locally with exit code 2 rather than allowing the server default to materialize.
  * **Create-only, authoring-only, read-only, or mixed-ownership field omitted** → handled by its DR-012 class; the tool does not fabricate an inapplicable null property. Workflow `meta_config` retains its reserved identity/preservation behavior.
  * YAML anchors / aliases → handled deterministically by the parser.
  * **Existing entity with one safe natural-key match** → updated on every valid invocation (FR-005/006), including when the same YAML was just applied. For Workflow this requires exactly one valid reserved identity record; for Skill it requires exactly one exact result after exhaustive resolution.
  * **Runtime/derived fields in YAML** → rejected as unknown fields with exit code 2 (FR-023); datasource runtime-state fields especially.
  * **UI-authored Workflow without an identity record** → never selected by ordinary reconciliation, even when its display name matches. It requires explicit reviewed UUID adoption. An exact display-name match blocks create with an exit-1 adoption-required error.
  * **UI-authored Skill with the same exact project/name** → reconciled only when exhaustive resolution returns exactly one visible match and write permission is proven. Multiple exact matches fail with exit code 1; unproven write permission fails with exit code 2.
  * **Invalid Workflow identity record** → Workflow apply fails with exit code 1 before ordinary reconciliation writes; the tool does not repair, ignore, or reinterpret the record automatically.
  * **Duplicate Workflow identity records or Skill natural keys** → apply fails with exit code 1, reports ambiguity, and selects none. Cleanup is a manual authorized platform operation outside this CLI.
  * **Workflow/Skill principal has partial visibility** → apply fails with exit code 2 before write because absence or uniqueness cannot be proven.
  * **Malformed `--adopt-workflow-id`** → local CLI validation fails with exit code 2 before network access.
  * **Workflow adoption selector names wrong-project, marked, or unmergeable entity** → adoption fails with exit code 1 and no write. An unauthorized candidate fails with exit code 2. Display name is not a fallback selector.
  * **Workflow slug/project or Skill name/project changes** → declares a new entity identity; the prior entity remains because delete and automatic rename are out of scope.
  * **Datasource repeated apply** → a missing Datasource receives its kind's ordinary create request; an existing Datasource receives its ordinary update request on every valid invocation, including supported source/content/file values. The CLI never reports `unchanged`.
  * **Datasource has create-only fields** → those fields are used in the exact create projection and are not invented in an update request that does not expose them; apply still sends the kind's ordinary update projection rather than comparing current state.
  * **Datasource declaration mixes fields from two kinds or uses an invented integration wrapper/alias** → local validation fails with exit code 2 before any server request. The diagnostic identifies only the safe field path/category and never reproduces submitted configuration.
  * **Datasource integration identifier is locally valid but rejected by the server** → apply exits code 1 as a server rejection after valid input; stdout is empty and stderr contains no identifier, credential, or server body. Integration provisioning/access remediation occurs outside this tool for every kind.
  * **Workflow persisted actor contains `skillRefs` or `datasourceRefs`** → lint exits code 2 because the current runtime uses those resource lists only for inline/virtual assistants; resources for a persisted Assistant belong in its Assistant declaration.
  * **Circular sub-assistant references accepted locally but rejected by the server** → exit code 1; stdout remains empty and stderr contains a safe entity-rejection category/status diagnostic, never the server error text/body or submitted values.
  * Non-UTF-8 files → rejected with exit code 2 and a clear error.
  * Symbolic links → refused by default with exit code 2; explicit opt-in flag required.
  * Date/time fields → normalized to UTC in YAML.
  * **Locally valid cross-reference to an entity not yet present on the server** → apply fails with exit code 1; stdout remains empty and stderr identifies the source file/reference field path and missing-reference category without reproducing the authored reference value (VR-005). The caller must apply dependencies first.
  * **`login` with email+password+auth_url and no client_secret** — tool selects Mode (c) Keycloak ROPC; sends `grant_type=password` to the configured endpoint. If the Keycloak realm or client does not support ROPC, the server returns a 4xx; tool exits code 2 with fixed authentication-failed category, leaving stdout empty and never printing credentials, request body, or server error text.
  * **`CODEMIE_CLIENT_ID` not set in Mode (c)** — tool defaults `client_id` to `codemie-sdk` in the ROPC request; the default is applied silently without a warning.
  * **`login --email` against a Keycloak-backed server** — server returns 400. Tool exits code 2; stdout remains empty and stderr uses the fixed local-auth-unavailable category/message without the response body, server error text, email, or password.
  * **`login --client-id` and `--email` supplied together** — ambiguous auth mode; tool exits code 2 before making any network call, leaves stdout empty, and does not reproduce supplied credential values in stderr.
  * **Keycloak login has no explicit `auth_url` source** — even when a CodeMie API `url` is configured, login exits code 2 before network access, leaves stdout empty, and emits a safe missing-configuration diagnostic. It does not derive or probe a token endpoint.
  * **Multiple Keycloak endpoint sources are present** — the tool deterministically uses `--auth-url` over `CODEMIE_AUTH_URL` over `.codemie/config.yaml` `auth_url`; lower-precedence endpoints are not contacted.
  * **Credential material appears in `.codemie/config.yaml`** — configuration validation fails with exit code 2 before network access; the diagnostic identifies only the prohibited field path and never its value.
  * **Local-auth rate limit exceeded** — server returns 429 (or a rate-limit 400); tool exits code 2; stdout remains empty and stderr contains a fixed rate-limited category and non-sensitive status, never the server message/body or supplied credentials. The developer should wait before retrying (5 attempts / 15 min).
  * **Keycloak token expiry mid-pipeline** — if a token expires during a long pipeline run (~8 h), subsequent `apply` calls receive 401. The tool exits code 2; stdout remains empty and stderr contains a safe authentication-expired category/status without the token, authorization header, or response body. The pipeline must re-run `login`; token caching policy is the caller's responsibility.
  * **Server/API unreachable, incompatible, or unavailable after applicable read retries** → exit code 2; stdout remains empty and stderr contains only an applicable safe category and allowlisted context.
  * **Concurrent CI runs against one target environment** → deployment configuration must serialize them. A UI/other-client race that bypasses governance can commit a duplicate before post-write detection; the tool exits code 1, leaves stdout empty, emits a safe ambiguity/commit-uncertain diagnostic to stderr, and does not automatically delete or roll back.
  * **Server returns a body or header containing credentials, tokens, cookies, declaration fragments, stack traces, control characters, or attacker-controlled text** → all bodies and non-allowlisted headers are omitted. `X-Request-ID` and `X-Correlation-ID` are emitted only when they satisfy VR-011; otherwise the diagnostic contains the tool-generated request ID and safe category/status only.
  * **Debug, verbose, trace, panic, or unexpected internal failure path** → the same stream and allowlist rules apply: exit code 2, stdout empty, no exception dump or sensitive values, and a stable fatal/internal category on stderr.

  ---

  ## 23. Dependencies

  | Dependency | Owner | Why it matters | Blocking? |
  |---|---|---|---|
  | Stable CodeMie REST API for the four entity types | CodeMie platform team | Schemas and endpoints are the tool's contract. | **Blocking** for schema finalization |
  | API tokens / service-account model | CodeMie platform team | Required for non-interactive CI use. | Blocking for apply |
  | Exact Keycloak token endpoint | CodeMie platform team supplies it; adopting team configures it | Required by Keycloak `login`; the CLI does not derive or discover it. | Blocking for Keycloak login; not for lint or apply with a pre-issued token |
  | Test/dev CodeMie environment for CI | CodeMie platform team | Needed for apply on PRs. | Blocking for CI workflow; not for lint |
  | Pinned Workflow/Skill identity API evidence | CodeMie platform team | Must prove complete manager/admin visibility, exhaustive stable pagination, exact identity fields, permissions, Workflow `meta_config` preservation, and post-write reads on the target deployment. | **Blocking for Workflow/Skill implementation and verification** |
  | Serialized CI and governed UI/other-client writes | Adopting team and platform administrator | Prevents duplicate Workflow identity records and creator-scoped Skill duplicates that the server cannot atomically prevent. | **Blocking operational prerequisite for Workflow/Skill production use** |
  | Manual identity-remediation procedure | Platform administrator | Invalid/duplicate Workflow records and duplicate Skill keys cannot be repaired by this non-destructive CLI. | Blocking for production support readiness |
  | Pinned Datasource CRUD contracts | CodeMie platform team | Must fix every authorable Datasource kind's ordinary create/update/read field shape, exact discriminator, requiredness, nullability, validation constraints, mutability, and integration-identifier form. Provider-defined kinds require a bundleable offline schema; a server-visible kind without ordinary CRUD cannot be invented by the tool. | **Blocking for Datasource implementation and verification** |
  | CI system's secret-injection mechanism | Team adopting the tool | How credentials reach the tool. | Non-blocking; contract is env-var based |

  ---

  ## 24. Constraints

  * Server communication MUST go through the documented API surface. No direct DB access.
  * The tool MUST NOT require modifications to the CodeMie server.
  * **Authentication:** CI and production use MUST use Keycloak OIDC `client_credentials` grant (Mode (a)). Human users and developers accessing Keycloak-backed instances MAY use Keycloak ROPC `grant_type=password` (Mode (c)); this mode requires an explicit `auth_url` and uses `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` without a `client_secret`; `CODEMIE_CLIENT_ID` defaults to `codemie-sdk`. `POST /v1/local-auth/login` (Mode (b)) is supported by the tool for local development against a dev server only; it MUST NOT appear in CI pipeline documentation or examples. For both Keycloak modes, the exact token endpoint MUST be supplied by `--auth-url`, `CODEMIE_AUTH_URL`, or non-secret config `auth_url`, with FR-017 precedence. The CLI MUST NOT derive it from the CodeMie API URL, a hostname/path convention, or another value. Client ID, client secret, token, email, password, and other credential material MUST NOT be stored in repository configuration.
  * Tool name is **`codemie-gitops`** — used consistently in CLI binary, package name, repository name, and docs. Must not be confused with `codemie-code`.
  * Packaging shape: **standalone Rust binary** (single self-contained executable, no Python or any other runtime required in CI). Distributed as a pre-built binary; optional thin GitHub Actions and GitLab CI wrapper scripts. Not a subcommand of `codemie-code`. Schema definitions for lint are bundled as Rust types or embedded schema files within the binary — the server's Python Pydantic models are NOT directly reused (consequence of choosing Rust over Python).
  * All authored and reported managed-asset identity and managed-asset cross-references MUST use natural keys, not server UUIDs. Workflow/Skill server IDs MAY be used internally and ephemerally after exact resolution; explicit Workflow adoption MAY accept a current server UUID as invocation input, but MUST NOT persist or report it as identity. Opaque integration configuration fields exposed by a Datasource kind are not managed-asset identity or cross-entity references.
  * The only phase-1 server-side client-owned identity metadata is the reserved Workflow `meta_config` record in FR-028. It MUST NOT be generalized into an ownership marker for another kind.
  * Phase 1 supports `codemie` assistant type only.

  ---

  ## 25. Assumptions

  | ID | Assumption | Evidence | Risk if false | Confirmation owner |
  |---|---|---|---|---|
  | **A-1** | ~~Schemas via OpenAPI~~ **RESOLVED** — schemas readable from Pydantic models and `execution_config_schema.yaml`. | Direct source inspection 2026-08-06. | — | — |
  | **A-2** | ~~Stable identifiers~~ **RESOLVED at product level with kind-specific guarantees.** Assistant uses server-backed `(project, slug)`. Workflow uses client-managed reserved server metadata with client-detected uniqueness. Skill uses exhaustive client resolution and ambiguity refusal because server uniqueness includes creator. Datasource retains `(project, repo_name)` subject to the target API contract. | Model inspection plus product decisions 2026-08-09. | Target-deployment visibility or pagination that is weaker than required blocks Workflow/Skill apply; no fallback identity is allowed. | CodeMie platform owner |
  | **A-3** | ~~Service-account tokens for CI are supported in target deployments~~ **FULLY RESOLVED (platform team, 2026-08-07):** CI auth uses Keycloak OIDC `client_credentials`. Service accounts are provisioned via `https://epa.ms/codemie-support`; token lifetime is ~8 h; project integrations only. Product decision v13 separately supports `POST /v1/local-auth/login` for local development, never CI. | Platform response and v13 product decision. | — | — |
  | **A-4** | ~~`plan` is a required capability~~ **SUPERSEDED in v7.** No `plan` command in phase 1. | Product decision Q3, 2026-08-07. | — | — |
  | **A-5** | Initial release is single-target, single-entity per invocation. | Product decisions Q1, Q2, Q3. | Multi-env atomic promotion requires CI-level orchestration. | — |
  | **A-6** | The tool is a new artifact, not an addition to `codemie-code`. | User asked to "create" a CI/CD tool; `codemie-code` overloads "workflow" and "skill" for local artifacts. | If it should be a subcommand, packaging changes. | Product owner |
  | **A-7** | Existing YAML templates in `config/templates/*` are seed examples only. The tool MUST NOT preserve the UUID pattern. | `workflow_autonomous_template.yaml` hard-codes datasource UUIDs. | Migration friction — teams must re-express UUIDs as natural-key refs. | Product owner + platform team |
  | **A-8** | `AssistantBase.from_yaml()` may be exposed as a REST endpoint in the future, but the tool MUST NOT depend on it today. | `from_yaml` classmethod in `assistant.py`. | Tool must construct API payloads from YAML itself. | — |
  | **A-9** | Datasource declarations contain only fields exposed by the selected kind's ordinary authoring request; read-only response fields are not authorable. | `IndexInfo` and per-kind request model inspection. | Bundled schemas would accept fields that the ordinary create/update request does not own. | Product owner |

  ---

  ## 26. Conflicts and inconsistencies

  * **C-1 — Naming collision (RESOLVED):** tool is named `codemie-gitops`.
  * **C-2 — Overloaded term "Skills":** in `codemie-code` = Claude slash commands (local). In `codemie-gitops` = platform Skill entities (server-managed). Docs MUST disambiguate.
  * **C-3 — Overloaded term "Workflows":** in `codemie-code` = CI code-review pipelines. In `codemie-gitops` = platform Workflow entities. Docs MUST disambiguate.
  * **C-4 — Datasource-UUID fragility in existing templates (RESOLVED in tool design):** `codemie-gitops` YAML rejects UUID references in favour of `{project, repo_name}` natural-key refs. Migration path: a converter script — see §29.
  * **C-5 — Omitted-field semantics: RESOLVED in v23 (VER-012), superseding the earlier mandatory-completeness rule.** Required and null-rejecting fields remain authored. A wholly authorable optional request field may be omitted and is then sent as explicit null in each applicable create/update payload; explicit YAML null behaves identically where accepted. Operation-inapplicable and mixed-ownership fields follow DR-012 rather than receiving a fabricated null.
  * **C-6 — Generic management marker vs. Workflow identity metadata: RESOLVED in v14.** Generic management/ownership markers remain excluded. Workflow alone persists the reserved `meta_config` identity record defined by FR-028 so `(project, slug)` can be resolved without a server change. The record is not an ownership claim. `custom_metadata` on Assistant remains ordinary user-authored data and is not used for bookkeeping.
  * **C-7 — Skill server uniqueness: RESOLVED at product level in v14.** The server does not enforce `(project, name)` globally because its constraint includes creator. The product accepts exhaustive client resolution, ambiguity refusal, privileged complete visibility, serialized writers, governed UI/API creation, post-create verification, and a residual different-principal race. This resolution does not claim server-enforced uniqueness.
  * **C-8 — Local validation exit code: RESOLVED in v15.** All failures determined from CLI arguments, repository configuration, declaration/sidecar files, or bundled schemas use exit code 2 and occur before network access. Exit code 1 begins only after valid local input reaches entity reconciliation or server-side processing. Authorization/visibility/write-permission, connectivity, compatibility, and fatal failures remain exit 2.
  * **C-9 — Failure output and sensitive data: RESOLVED in v16.** Failures emit only safe synthesized diagnostics to stderr and leave stdout empty in text and JSON modes. Server error text and bodies, request payloads, credentials, security headers/cookies, declaration/sidecar values, and other non-allowlisted data never enter failure diagnostics, and sensitive transport/authentication artifacts are not persistently copied by the tool. Diagnostic safety is allowlist-based and has no debug/verbose exception. The successful `login` token line is the sole intentional sensitive-output exception.
  * **C-10 — Datasource authoring surface: RESOLVED in v22/v23, superseding the v17–v19 interpretations.** The tool uses ordinary create/update formats, including supported source/content/file fields, and applies DR-012 to optional authorable properties. No product behavior is defined for server-internal processing.
  * **C-11 — Datasource integrations vs. managed-asset natural references: RESOLVED generically in v20.** Opaque integration identifiers exposed by an existing per-kind Datasource request are ordinary configuration values, not one of the four managed entities and not cross-entity references. The tool sends the exact non-secret per-kind value without inventing an alias/resolver. Integration provisioning, access grants, and credential acquisition remain uniformly out of scope.
  * **C-12 — Explicit versus derived Keycloak endpoint: RESOLVED in v24.** The explicit-only alternative is approved. Keycloak `login` uses `--auth-url`, then `CODEMIE_AUTH_URL`, then config `auth_url`; it never derives or probes an endpoint from the CodeMie API URL or a convention. Architecture language that retains a possible future derivation branch is superseded and requires refresh, not another product decision.

  ---

  ## 27. Open questions

  | ID | Question | Status | Resolution | Affects | Owner |
  |---|---|---|---|---|---|
  | OQ-1 | Field schema per entity? | ✅ RESOLVED | See §15. | §15 | — |
  | OQ-2 | Schema versioning? | ✅ RESOLVED | Tool uses `apiVersion: codemie.epam.com/v1alpha1`. Server exposes version via `GET /v1/info` → `{"version": "APP_VERSION"}` (current: `"0.16.0"`). Compatibility gating is an architecture decision; per-entity output remains limited to FR-026. | DR-002, IR-002 | — |
  | OQ-3 | Identity model? | ✅ RESOLVED | Authored/reported natural keys per entity; no client state file. Workflow persists `(project, slug)` in its reserved server `meta_config` identity record. Skill uses exhaustive exact client resolution. Returned server IDs are internal. | FR-015, FR-028–032, DR-003/005/007/008 | — |
  | OQ-4 | REST directly or wrap SDK? | ✅ RESOLVED | REST direct. No SDK wrapper in phase 1. Rust rules out Python SDK reuse regardless. | IR-001 | — |
  | OQ-5 | Partial-failure behavior? | ✅ RESOLVED (superseded) | One entity per invocation. No batch. FR-018 deleted. | FR-005 | — |
  | OQ-6 | Concurrent apply? | ✅ RESOLVED for phase 1 | CI MUST serialize apply jobs per target environment; adopting teams MUST govern UI/other-client Workflow/Skill identity writes during the deployment window. Residual races fail visibly after re-resolution and are never auto-deleted or rolled back. General server writes remain without atomic conditional-update guarantees. | SC-010, FR-034, QR-010/011 | Adopting team + platform administrator |
  | OQ-7 | Adopt or refuse pre-existing assets? | ✅ RESOLVED by kind | Ordinary natural-key reconciliation applies where a safe exact match exists. Workflow alone supports explicit in-place adoption with `--adopt-workflow-id <current-server-uuid>`; no generic `--adopt-existing` exists. Display name never selects. | FR-019, FR-029/030 | — |
  | OQ-8 | Omitted-field behavior? | ✅ RESOLVED (v23; VER-012) | Optional authorable request fields may be omitted and become explicit JSON null in every applicable create/update payload. Required, conditional, identity/envelope, and null-rejecting fields remain authoring-required. | FR-021–023, DR-012, VR-016 | — |
  | OQ-9 | Tool name? | ✅ RESOLVED | `codemie-gitops`. | §24 | — |
  | OQ-10 | Which CI systems first-class? | ✅ RESOLVED | GitHub Actions + GitLab CI. | §10 | — |
  | OQ-11 | Deferred entity types roadmap? | ✅ RESOLVED | Not in v1. Integrations, categories, users explicitly excluded from phase 1 scope. | §10 | — |
  | OQ-12 | Secret referencing syntax? | ✅ RESOLVED | None in phase 1. Platform integrations handle credentials. Lint warns on suspicious values. | DR-004 | — |
  | OQ-13 | Per-environment differences? | ✅ RESOLVED | Deferred to phase 2. Phase 1: env vars at CI level. | §10 | — |
  | OQ-14 | Export server → YAML? | 🟢 PATH CLEAR | Deferred to phase 2. | §10 | Product owner |
  | OQ-15 | Implementation language? | ✅ RESOLVED | **Rust binary** — single self-contained executable; no Python runtime required. Schema definitions bundled as Rust types or embedded schema files; server Pydantic models cannot be directly reused (schema must be maintained separately and kept in sync with server releases). Product decision 2026-08-07. | QR-006 | — |
  | OQ-16 | Datasource authoring boundary? | ✅ RESOLVED (v22/v23) | Source/content/file/configuration and ordinary scheduling body fields supported by a Datasource kind's create/update contract are authorable. The tool uses that operation's existing format and applies DR-012 to requiredness and nullability. | FR-036, DR-010–012, §15, IR-008 | — |
  | OQ-17 | Full enum of workflow `mode` values? | ✅ RESOLVED | `WorkflowMode.SEQUENTIAL = "Sequential"` (default, non-deprecated); `WorkflowMode.AUTONOMOUS = "Autonomous"` (deprecated). Source: `src/codemie/core/workflow_models/workflow_models.py`. | §15, VR-004 | — |
  | OQ-18 | `apiVersion`/`metadata` envelope client-side vs. server-side? | 🟡 RECOMMENDATION | Client-side in phase 1 (confirmed envelope structure Q13). | FR-005, IR-001 | Architect |
  | OQ-19 | A2A / Bedrock types in phase 1? | ✅ RESOLVED | `codemie` type only. Others deferred. | §10, §15 | — |
  | OQ-20 | Management marker storage? | ✅ RESOLVED with a narrow exception | No generic management/ownership marker exists. Workflow alone stores the FR-028 reserved identity record in `meta_config`; Assistant `custom_metadata`, Skill, and Datasource are not used for tool bookkeeping. | FR-020, FR-028 | — |
  | OQ-21 | Default discovery mechanism? | ✅ RESOLVED (refined v23) | The tool does not discover or copy server default values. It emits null for omitted optional authorable fields and requires authored values for fields whose applicable request rejects null. | FR-021/022, DR-012 | — |
  | OQ-22 | File naming convention enforced? | ✅ RESOLVED (Q19) | Free-form. Filename is irrelevant; identity comes from `kind` + `metadata`. Recommended convention is `<kind>s/<natural-key>.yaml` but not enforced. | FR-027 | — |
  | OQ-23 | Directory structure enforced? | ✅ RESOLVED (Q20) | Free-form. Tool reads `kind` from inside the YAML. No kind-based directory requirement. | FR-027 | — |
  | OQ-24 | Config file name and location? | ✅ RESOLVED (Q21) | `.codemie/config.yaml` at the repo root. Namespaced under `.codemie/` for co-location with future `codemie-*` tooling. | FR-017 | — |
  | OQ-25 | `metadata.project` required in every YAML? | ✅ RESOLVED (Q22) | MAY be omitted when `.codemie/config.yaml` defines a `project` default; lint fails if neither source provides it. | FR-022 | — |
  | OQ-26 | `contentFrom` for skills in phase 1? | ✅ RESOLVED (Q23) | In scope. `spec.contentFrom: <path>` references a sidecar `.md` file; content inlined at apply time. Mutually exclusive with `spec.content`. | FR-025, §15 | — |
  | OQ-27 | Machine-readable output format? | ✅ RESOLVED (Q24, refined v16) | `--output json` flag; default is human-readable text. A successful per-entity JSON object is written to stdout; a failed invocation leaves stdout empty and writes one safe JSON diagnostic to stderr. | FR-026, QR-004 | — |
  | OQ-28 | Which exit code applies to local schema/validation failures? | ✅ RESOLVED (v15) | All local parsing, schema, semantic-validation, repository-reference, sidecar, and configuration failures use exit code 2 and make no network call. Exit 1 is reserved for entity reconciliation/server-side failure after valid local input. Authentication/authorization/visibility, compatibility/connectivity, and fatal failures use exit 2. | FR-003/004/011/022/023/025/029–034, VR-005/010, SC-003/004/013–016 | — |
  | OQ-29 | How are server error bodies split between stdout/stderr and protected? | ✅ RESOLVED (v16) | Every failure leaves stdout empty and emits an allowlist-constructed diagnostic to stderr in the selected text or JSON mode. Non-allowlisted values never enter diagnostics. Raw/full request or response bodies, server-provided error text, payloads, credentials, tokens, authorization headers, cookies, secret fields, and secret-like values are never printed or persistently copied by the tool; debug/verbose paths have no exception. Successful `login` token stdout is the sole intentional output exception. | FR-011, FR-014, FR-016, FR-024, FR-026; DR-006/009; QR-004/007/011; VR-011/012 | — |
  | OQ-30 | Is Google a separate phase-1 entity or product capability? | ✅ RESOLVED / HISTORICAL (v20) | No. The four entities are Assistant, Workflow, Datasource, and Skill. Google Docs is one ordinary Datasource kind and follows the same DR-010/011 per-kind format, CRUD, integration-scope, and validation rules as every other Datasource kind. Its technical fields appear only in the §15 inventory. | DR-010/011, FR-036, IR-008 | — |
  | OQ-31 | What are the exact authored Workflow Skill/Datasource reference properties, and are inline assistants supported? | ✅ RESOLVED (v17) | `assistants[].assistantRef` identifies a persisted Assistant; inline assistants are supported through the `system_prompt` form. Inline-only `skillRefs` and `datasourceRefs` replace server `skill_ids`/`datasource_ids`. `assistants[].id` and `states[].assistant_id` remain workflow-local graph references. | FR-035, DR-003, VR-013, §15 Workflow | — |
  | OQ-32 | Does `--reindex-datasources` support file Datasources? | ✅ RESOLVED / HISTORICAL (v22/v23) | No such CLI option or dedicated lifecycle-control operation exists for any Datasource. Ordinary create/update—including supported content/source/file and scheduling fields—uses the existing per-kind format and the shared DR-012 field-projection rule. | FR-036, DR-012, IR-008, VR-014 | — |
  | OQ-33 | Does the CLI emit Git/CI/environment provenance? | ✅ RESOLVED (v21; PRODUCT-OQ-01/VER-011) | No. Git commit SHA, target environment origin, Git author, and CI-run identity are excluded from all CLI output. Git, CI, and platform records independently own their provenance. | FR-013, FR-016/026, DR-006, PA-004, QR-004, VR-011 | — |
  | OQ-34 | Does apply compare current state and skip an equal write? | ✅ RESOLVED (v22; ARCH-B01) | No. Each valid invocation creates a missing entity or updates an existing entity, reports `created` or `updated`, and never reports `unchanged`. | SC-005, FR-005/006/012/029/031/036, DR-011, QR-002 | — |
  | OQ-35 | What happens when YAML omits an optional server-request field? | ✅ RESOLVED (v23; VER-012) | The applicable create/update payload includes the property as explicit JSON null. Explicit YAML null is equivalent where accepted. A field whose pinned applicable contract rejects null is authoring-required and omission/null fails locally with exit code 2. | SC-017, FR-021/022, DR-001/012, VR-003/016 | — |
  | OQ-36 | How is the Keycloak token endpoint determined? | ✅ RESOLVED (v24) | It is explicitly supplied as `--auth-url`, `CODEMIE_AUTH_URL`, or `.codemie/config.yaml` `auth_url`, in that precedence order. The CLI never derives it from a CodeMie API URL or convention. Missing explicit configuration fails locally with exit code 2 before network access. | SC-011, FR-017/024, IR-006 | — |

  ---

  ## 28. Traceability matrix

  | Source | Scenario | Requirement | Acceptance criteria |
  |---|---|---|---|
  | User request | SC-001 | FR-001, FR-005 | AC-FR-005-01 |
  | User request | SC-003 | FR-002, FR-003 | AC-FR-003-01 |
  | Derived (safety) | SC-007 | FR-014, DR-004 | AC-FR-014-01 |
  | Product decision v22 (ARCH-B01) | SC-005 | FR-005/006/012/029/031/036; DR-011; QR-002 | AC-FR-005-02, AC-FR-006-01, AC-DR-011-01, AC-IR-008-01 |
  | Derived (safety) | — | FR-008 | AC-FR-008-01 |
  | Derived (CI ergonomics) | SC-009 | FR-009, FR-011 | AC-FR-009-01 |
  | Product decision Q3/Q12 | SC-011 | FR-024, IR-006 | AC-FR-024-01 |
  | Product decision v24 (explicit Keycloak endpoint) | SC-011 | FR-017/024; IR-006 | AC-FR-017-01/02, AC-FR-024-01/03/07 |
  | Product decision v26 (Keycloak ROPC Mode (c)) | SC-020 | FR-017/024; IR-006; QR-007 | AC-FR-024-08 |
  | Product decision v23 (VER-012; supersedes Q4 completeness rule) | SC-003, SC-017 | FR-021/022; DR-001/012; VR-003/016 | AC-FR-021-01/02, AC-FR-022-01/04 |
  | Reference source plus product decisions v20/v22 (OQ-16/OQ-32) | SC-001, SC-002, SC-005 | FR-036; DR-010/011; IR-008; VR-014/015 | AC-IR-008-01, AC-DR-010-01/02, AC-DR-011-01 |
  | Product decision v14; ADR-008 feasibility evidence | SC-013 | FR-028, FR-029, FR-032, FR-033; DR-007/008 | AC-FR-028-01/02, AC-FR-029-01/02, AC-FR-033-01 |
  | Product decision v14; ADR-008 feasibility evidence | SC-014 | FR-030, FR-034; BR-004/006 | AC-FR-030-01/02, AC-FR-034-01 |
  | Product decision v14; ADR-007 feasibility evidence | SC-015 | FR-031–033; DR-008 | AC-FR-031-01/02, AC-FR-033-01 |
  | Product decision v14; accepted residual race | SC-010, SC-016 | FR-034, QR-010/011, BR-006 | AC-FR-034-01, AC-QR-010-01 |
  | Product decision v15 (OQ-28) | SC-003/004/006/008/009/013–016 | FR-003/004/011/022/023/025/029–034; VR-005/010 | AC-FR-003-01, AC-FR-004-01, AC-FR-009-01, AC-FR-011-01–04, AC-FR-022-01/03, AC-FR-025-01/03, AC-FR-029-02, AC-FR-030-02, AC-FR-031-01, AC-FR-033-01, AC-FR-034-01 |
  | Product decision v16 (OQ-29) | SC-002/006/008/009/011/012 | FR-011/014/016/024/026; DR-004/006/009; IR-007; QR-004/007/011; VR-011/012 | AC-FR-003-01, AC-FR-004-01, AC-FR-009-01, AC-FR-011-01–04, AC-FR-014-01, AC-QR-007-01, AC-FR-024-01/02/04/05/06, AC-FR-026-02, AC-FR-029-02, AC-FR-033-01, AC-FR-034-01 |
  | Product decision v21 (PRODUCT-OQ-01/VER-011) | SC-001/002/006 | FR-013/016/026; DR-006; PA-004; QR-004; VR-011 | AC-FR-026-02/03 |
  | Product decision v17 plus current Workflow server/UI evidence (OQ-31) | SC-019 | FR-035; DR-003; VR-013 | AC-FR-035-01/02 |

  ---

  ## 29. Handoff to solution architect

  **Approved product behavior:**
  * YAML-first declarative management of assistants, workflows, datasources, skills.
  * One YAML file = one entity; one `apply` invocation = one entity. CI orchestration (loops, ordering) is the caller's responsibility.
  * Non-destructive, always-write apply: create when safely missing; update on every valid invocation when safely present; no desired/current comparison, skipped write, or `unchanged` outcome.
  * Offline `lint` + online `apply` + `login` token acquisition. No `plan` in phase 1.
  * Per-environment targeting via `CODEMIE_URL` + `CODEMIE_TOKEN`. No overlay/templating in phase 1.
  * Explicit authored desired-state values are preserved. Required/null-rejecting fields must be authored; optional authorable request fields may be omitted and then materialize as explicit JSON null in every applicable create/update payload. Explicit YAML null is equivalent where the pinned request accepts it. The bounded transformations and mixed-ownership Workflow `meta_config` rule in FR-021/DR-012 still apply.
  * Phase 1: `codemie` assistant type only. CI docs: GitHub Actions + GitLab CI.
  * Kubernetes-style envelope: `apiVersion` / `kind` / `metadata` (identity) / `spec` (config).
  * Config file at **`.codemie/config.yaml`** (not a flat dotfile): non-secret `url`, `auth_url`, and optional `project` default; no credentials. `metadata.project` may be omitted from YAML when config provides the default. Keycloak endpoint precedence is `--auth-url` > `CODEMIE_AUTH_URL` > config `auth_url`; it is never derived from `url`.
  * **Workflow identity:** authored/reported identity is exact `(project, slug)`. Workflow alone persists that identity in reserved `meta_config` member `codemie.epam.com/gitops/workflow-identity` with `{version: 1, project, slug}`. Ordinary reconciliation exhausts all relevant pages and selects only an exact valid record; invalid or duplicate records fail. Display name never selects.
  * **Workflow legacy adoption:** an unmarked Workflow is adopted only through `--adopt-workflow-id <current-server-uuid>`, with exact project, visibility, write, unmarked, metadata-preservation, and zero-existing-marker checks. Adoption persists the record and reconciles in one operation. The UUID remains absent from YAML, state, and outcomes.
  * **Skill identity:** authored/reported identity is exact `(project, name)`. The tool exhausts all compatible list pages and exact-filters client-side: zero creates, one updates, multiple fails ambiguous. Server uniqueness remains creator-scoped; returned UUIDs are internal only.
  * **Operational identity controls:** Workflow/Skill apply requires provably complete manager/admin visibility, write permission for existing targets, post-write re-resolution, serialized CI per environment, governed UI/other-client writes, and manual duplicate remediation. Residual races may be detected after a write committed; the CLI never chooses, deletes, or rolls back automatically.
  * **Workflow execution references:** `assistants[].id` and `states[].assistant_id` are workflow-local. Persisted entries use `assistantRef`; inline `system_prompt` entries may use `skillRefs` and `datasourceRefs`. Server ID-bearing fields exist only in the invocation payload.
  * **No generic management/ownership marker or generic adoption command.** The Workflow identity record/adoption path is the only phase-1 exception. Assistant `custom_metadata` remains user data, not tool bookkeeping.
  * Exit codes: 0 = success; 1 = entity reconciliation or server-side failure after valid local input; 2 = local parsing/schema/validation/configuration, authentication/authorization/visibility, compatibility/connectivity, or fatal failure.
  * **Output and sensitive-data boundary:** successful per-entity outcomes use stdout and contain action (`created` or `updated`), kind, project, and natural key. They do not contain Git commit SHA, target environment origin, Git author, CI-run identity, or replacement provenance. Every failure leaves stdout empty and emits only a safe synthesized diagnostic to stderr, in text or JSON mode. Diagnostics are constructed from the FR-016 allowlist and never include server-provided error text, request/response bodies, payloads, credentials, authorization headers, cookies, declaration/sidecar values, secret-classified values, or externally owned provenance. There is no debug/verbose exception. A successful `login` token line is the sole intentional sensitive-output exception and is never otherwise printed or persisted.
  * **File naming and directory structure are free-form.** Identity comes from `kind` + `metadata` inside the YAML. Tool reads `kind` from the file content, not the path. Recommended (non-enforced) layout: `assistants/`, `workflows/`, `skills/`, `datasources/` with filenames matching the natural key.
  * **`contentFrom` for skills** is in scope for phase 1. `spec.contentFrom: <path>` references a sidecar `.md` file; content is inlined before the API call. Mutually exclusive with `spec.content`.
  * **`--output json` flag** on all commands that produce per-entity output. A success object is written to stdout; a failure diagnostic object is written to stderr with stdout empty.
  * **Datasource boundary:** Every authorable Datasource kind uses its existing compatible create/update format, including supported content/source/file fields and exact opaque integration-configuration members. Optional authorable body fields follow the same omission-to-null rule as the other three entities. Integration provisioning/access is uniformly out of scope.
  * **Rust binary** — single self-contained executable; no runtime dependency in CI.
  * **Authentication — three modes in `codemie-gitops login`:**
    * **Keycloak client_credentials** (CI / production, Mode (a)): `--client-id` (or `CODEMIE_CLIENT_ID`) + `CODEMIE_CLIENT_SECRET` (environment only) plus an explicit `--auth-url`, `CODEMIE_AUTH_URL`, or config `auth_url` → that exact OIDC token endpoint. Token ~8 h; cache across steps. Project-level integrations only. (`--client-secret` flag is not accepted; SEC-001 remediation.)
    * **Keycloak ROPC** (human users / developer access to Keycloak-backed instances, Mode (c)): `--email` (or `CODEMIE_EMAIL`) + `CODEMIE_PASSWORD` (environment only) + an explicit `--auth-url`, `CODEMIE_AUTH_URL`, or config `auth_url` (no `CODEMIE_CLIENT_SECRET`) → `grant_type=password` at that exact endpoint. `CODEMIE_CLIENT_ID` (or `--client-id`) defaults to `codemie-sdk` when not set. Public-client ROPC flow — no `client_secret` sent. (`--password` flag is not accepted; SEC-001 remediation still applies.)
    * **Local-auth** (local dev only, Mode (b)): `--email` (or `CODEMIE_EMAIL`) + `CODEMIE_PASSWORD` (environment only), no `auth_url` → `POST /v1/local-auth/login`. Requires dev server with `ENABLE_USER_MANAGEMENT=True` and `IDP_PROVIDER="local"`. Rate-limited 5 / 15 min. MUST NOT appear in CI examples. (`--password` flag is not accepted; SEC-001 remediation.)
    * Supplying `CODEMIE_CLIENT_SECRET` together with `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` → error before any network call.
    * `CODEMIE_TOKEN` env var (pre-acquired bearer token) remains the primary runtime mechanism for `apply` regardless of how the token was obtained.

  **Hard constraints:**
  * MUST use documented API surface (no direct DB).
  * MUST NOT delete server-side assets (phase 1).
  * MUST NOT print or persist tokens, credentials, authorization headers, cookies, secret-classified/secret-like values, request payloads, server-provided error text, or full request/response bodies. Successful `login` stdout is the sole token-output exception.
  * MUST NOT emit Git commit SHA, target environment origin, Git author, CI-run identity, or replacement provenance in success, warning, or failure output.
  * MUST NOT collide with `codemie-code` naming.
  * MUST NOT discover, copy, or invoke server defaults for omitted optional authorable fields; those fields become explicit JSON null under FR-021/DR-012. Fields whose pinned applicable request rejects null remain authoring-required.
  * MUST keep Workflow/Skill server IDs out of declarations, persistent client state, and normal outcomes.
  * MUST fail safely on incomplete visibility, invalid/duplicate Workflow records, and ambiguous Skill identity.

  **Decisions the architect must make:**
  * Concrete schema representation in Rust (JSON Schema embedded, generated Rust types, etc.).
  * Retry/backoff policy (IR-004).
  * `apiVersion`/`metadata` envelope treatment — client-side strip before API call vs. server-aware (OQ-18).
  * Strategy for keeping bundled Rust schema in sync with server Pydantic model changes.
  * Whether and how to use `GET /v1/info` for compatibility checks without adding fields to per-entity output.
  * **Token caching in CI:** The tool itself MUST NOT cache tokens across invocations (stateless binary). The recommended pattern is a shell variable assigned once per job. The architect should include this pattern prominently in CI integration examples.
  * Pin and contract-test requiredness, nullability, operation applicability, and the ordinary create/update/read request, response, discriminator, validation, integration-reference, and mutability shape for every entity and authorable Datasource kind. Do not infer a format for a provider-defined or import/read-only kind.

  **Decisions the architect MUST NOT reinterpret without product sign-off:**
  * Adding delete semantics.
  * Adding a generic management-marker or ownership-tracking system. The required Workflow identity record is not optional and MUST NOT be generalized or reinterpreted as ownership.
  * Replacing Workflow identity with a client-authored/deterministic UUID, selecting a Workflow by display name, or persisting the adoption UUID.
  * Assuming the server enforces Skill `(project, name)` uniqueness, or selecting a Skill by owner/newest/first when multiple exact matches exist.
  * Removing the Workflow/Skill complete-visibility, serialized-CI, governed-writer, post-write verification, or visible residual-race requirements without product sign-off.
  * Adding desired/current comparison, skipped-write behavior, an `unchanged` outcome, or another no-op action. A safely resolved existing entity receives an update on every valid invocation.
  * Omitting an optional authorable property from an outbound payload, replacing omission with a concrete server default, or allowing a null-rejecting field to be omitted. Apply the DR-012 classification and omission-to-null rule identically to applicable create and update requests.
  * Reclassifying a local validation failure as exit 1 or an online identity ambiguity as exit 2. Authentication/authorization, visibility/write-permission, response-contract compatibility, connectivity, and fatal failures remain exit 2 under FR-011 and are distinct from identity ambiguity.
  * Writing a failure outcome to stdout, copying server-provided error content into diagnostics, adding request/response-body logging, or weakening the FR-016 diagnostic allowlist in a debug, verbose, trace, panic, or fatal path.
  * Emitting Git commit SHA, target environment origin, Git author, CI-run identity, or replacement provenance in any CLI output. Cross-system audit correlation remains external.
  * Suppressing the successful `login` token stdout contract or permitting the token to appear anywhere else.
  * Moving `skillRefs`/`datasourceRefs` onto persisted Workflow assistant entries, replacing workflow-local `states[].assistant_id` with a server UUID, or dropping inline assistant support.
  * Adding non-listed entity types.
  * Adding a `plan` command (deferred, not forbidden).
  * Adding A2A/Bedrock Assistant types without product decision.
  * Introducing secret interpolation syntax.
  * Deriving or probing a Keycloak token endpoint from the CodeMie API URL, hostname, realm convention, or another value; Keycloak login requires one of the explicit FR-017 endpoint sources.

  **v26 addition (Keycloak ROPC Mode (c)):** the architect must design the three-mode selection logic in FR-024 — specifically the `auth_url`-presence discriminator that distinguishes Mode (c) from Mode (b) when `CODEMIE_CLIENT_SECRET` is absent. The `codemie-sdk` default for `CODEMIE_CLIENT_ID` in Mode (c) must be a named constant, not a hardcoded string. The ROPC token request MUST NOT include a `client_secret` field. No architecture change is required for Modes (a) and (b).

  **Required lifecycle sequence:** after the architecture artifacts are refreshed against this specification, pre-implementation verification assesses consistency across the specification, plan, ADRs, contracts, schemas, and tasks. The security reviewer then performs the next required review of the security-sensitive authentication and diagnostic boundaries before implementation proceeds. At this product-specification stage, the expected absence of that future security-review artifact is not a product blocker and does not prevent architecture planning; the later lifecycle stage must still complete before implementation/release readiness is claimed.

  ---

  ## 30. Readiness assessment

  ```
  Specification status: READY FOR ARCHITECTURE PLANNING
  ```

  Product behavior is now bounded and testable: the four-entity model, Workflow/Skill identity, always-write apply semantics, omission-to-null payload semantics, explicit Keycloak endpoint configuration, adoption, ambiguity, visibility, serialization, governance, residual races, exit codes, safe non-provenance output, Workflow resource-reference shapes, inline assistants, per-kind ordinary Datasource CRUD, and the three-mode login command including Mode (c) Keycloak ROPC are approved. PRODUCT-OQ-01/VER-011, ARCH-B01, VER-012, and OQ-36 are resolved, and no open product question blocks architecture planning. The pinned target contract and deployment evidence in §23 remain implementation/verification gates. Pre-implementation verification next assesses the refreshed artifact set, followed by the required security review; a future review artifact is not required to exist before product readiness or architecture planning. Architecture must preserve operation-specific field nullability, explicit non-derived authentication endpoint resolution, and each Datasource kind's existing format while keeping integration provisioning outside the tool.

  ### Closed product decisions

  **Session 1 (2026-08-06):** OQ-7 (refuse-existing), OQ-8 (strict declarative), OQ-9 (tool name `codemie-gitops`).

  **Grilling round 1 (2026-08-07 Q1–Q12):**
  * Q1: one file, one entity
  * Q2: no automatic dependency ordering — engineer controls invocation sequence
  * Q3: no `plan` command — FR-007 deleted
  * Q4: verbatim send + mandatory completeness — initial FR-021/022/023 rule, **superseded by v23/VER-012**
  * Q5: exit codes 0 / 1 / 2; one entity per invocation — FR-018 deleted
  * Q6: `--adopt-existing` → stamp + overwrite in one step
  * Q7 / OQ-12: no secret interpolation in phase 1
  * Q8 / OQ-13: multi-environment deferred to phase 2
  * Q9 / OQ-10: GitHub Actions + GitLab CI first-class
  * Q10 / OQ-19: `codemie` type only in phase 1
  * Q11: `apiVersion` required; unknown = hard error (exit 2)
  * Q12 / FR-024: `CODEMIE_TOKEN` + `login` command (note: `--token` flag removed in v25 / SEC-001)

  **Grilling round 2 (2026-08-07 Q13–Q18):**
  * Q13: Kubernetes-style envelope (`apiVersion`/`kind`/`metadata`/`spec`) — DR-002 confirmed
  * Q14: `CODEMIE_URL` + `CODEMIE_TOKEN` env vars primary; `.codemie/config.yaml` config file fallback (`url`, project default, no secrets) — FR-017 confirmed then and extended by v24 with non-secret `auth_url`
  * Q15: initial optional/non-null-default rule — **superseded by the source-pinned omission/nullability rule in v23/VER-012**
  * Q16: traceable generic management marker `{managed, repo, file}` — FR-020 confirmed at the time; **DELETED in v10**. The distinct Workflow-only identity record was later approved in v14.
  * Q17: `login` → single token line to stdout, errors to stderr — FR-024 confirmed
  * Q18: unknown fields = hard fail — FR-023 confirmed

  **Post-grilling product decision (2026-08-07):**
  * **OQ-15 / Language:** **Rust binary** — single self-contained executable; no Python runtime. Server Pydantic models cannot be directly reused; lint schema must be maintained as Rust types or embedded schema files and kept in sync with server releases. (§24 Constraints, §27 OQ-15)

  **Grilling round 3 (2026-08-07 Q19–Q24):**
  * Q19: File naming free-form — identity from `metadata`, not filename — FR-027
  * Q20: Directory structure free-form — tool reads `kind` from inside YAML — FR-027
  * Q21: Config file at `.codemie/config.yaml` (namespaced dotdir) — FR-017 updated
  * Q22: `metadata.project` MAY be omitted when config provides default; lint fails if neither provides it — FR-022 updated
  * Q23: `contentFrom` sidecar for skills in phase 1 — FR-025 added
  * Q24: `--output json` flag; human-readable default — FR-026 added; QR-004 resolved

  **Product decision (2026-08-07, v10):**
  * Generic management/ownership markers and generic `--adopt-existing` were removed. This remains authoritative for Assistant, Skill, Datasource, and generic ownership behavior. It is narrowly supplemented—not generally reversed—by the v14 Workflow identity decision below.

  **Source code analysis (2026-08-07, v9):**
  * OQ-2 RESOLVED: `GET /v1/info` returns `{"version": "APP_VERSION"}` — server version indicator confirmed.
  * OQ-17 RESOLVED: `WorkflowMode` enum confirmed: `"Sequential"` (default, non-deprecated), `"Autonomous"` (deprecated).
  * VR-004 RESOLVED: full enum inventory confirmed from model inspection (WorkflowMode, AgentMode, AssistantType, SkillVisibility).
  * A-3 (code finding): `POST /v1/local-auth/login` gated on `ENABLE_USER_MANAGEMENT=True` AND `IDP_PROVIDER="local"`. Enterprise SSO cannot use it. **Superseded by v12 platform team response: Keycloak client_credentials is the only supported CI auth path.**
  * C-6 CORRECTED: `custom_metadata` exists on Assistant only (not Skill, Workflow, or Datasource). FR-020 corrected accordingly. OQ-20 scope widened to cover all three non-Assistant entity types.

  **Platform team response (2026-08-07, v12):**
  * A-3 FULLY RESOLVED: CI auth is Keycloak OIDC `client_credentials` grant. Service accounts via `https://epa.ms/codemie-support`. Token ~8 h. Project integrations only under service-account credentials. FR-024, IR-006, §12, §24 updated accordingly. (Note: `POST /v1/local-auth/login` was initially dropped but restored in v13 for local dev — see below.)

  **Product decision (2026-08-07, v13):**
  * `POST /v1/local-auth/login` re-added as a supported local dev auth path in the `login` command. CI MUST use Keycloak. Mode is selected by which credential flags are present. SC-012 added; AC-FR-024-04/05/06 added.

  **Product decisions (2026-08-09, v14):**
  * Workflow authored/reported identity remains `(project, slug)` and is persisted in the Workflow-only reserved `meta_config` identity record. Ordinary reconciliation uses exhaustive exact record resolution, never display name, and fails on invalid or duplicate records.
  * An unmarked legacy Workflow can be adopted in place only through an explicitly supplied current server UUID. Adoption persists the natural-key record and reconciles desired state; the UUID remains ephemeral and unreported.
  * Skill authored/reported identity remains `(project, name)` and is enforced by exhaustive paginated client resolution: zero/create, one/update, multiple/ambiguous failure. No server-global uniqueness is claimed. v22 later established that the one-match update occurs on every valid invocation.
  * Workflow/Skill use requires complete manager/admin visibility, serialized per-environment CI, governed UI/other-client writes, post-write exact re-resolution, visible residual-race failure, and manual duplicate remediation. The CLI never automatically chooses, deletes, or rolls back an ambiguous entity.

  **Product decision (2026-08-09, v15):**
  * OQ-28 resolved with one exit taxonomy: 0 success; 1 entity reconciliation/server-side failure after valid local input; 2 local parsing/schema/validation/configuration, authentication/authorization/visibility, compatibility/connectivity, or fatal failure. Online identity ambiguity, invalid server identity evidence, or otherwise compatible entity-resolution instability is exit 1; inability to prove visibility or write permission is exit 2.

  **Product decision (2026-08-09, v16):**
  * OQ-29 resolved: every failure uses stderr and leaves stdout empty, including JSON mode. Failure diagnostics are synthesized only from the explicit non-sensitive allowlist; bodies, server error text, request payloads, credentials/security artifacts, and declaration/sidecar values never enter diagnostics. Sensitive transport/authentication artifacts are not persistently copied, with no debug/verbose exception. The successful `login` token line is the sole intentional exception and remains stdout-only for shell composition.

  **Product decisions (2026-08-09, v17):**
  * OQ-30 initially resolved with a proposed integration alias. **Superseded by v18 and generalized as a historical clarification in v20/OQ-30.**
  * OQ-31 resolved: Workflow persisted actors use `assistantRef`; inline actors remain supported through `system_prompt`; inline-only `skillRefs`/`datasourceRefs` translate to server ID lists. Workflow-local actor/state IDs remain authored graph identifiers.
  * OQ-32's initial Datasource interpretation was superseded; v22 is authoritative.

  **Product correction (2026-08-09, v19; superseded in scope wording by v22):**
  * OQ-32 restored ordinary Datasource create/update, including supported source/content/file fields.

  **Product correction (2026-08-09, v20):**
  * The product manages exactly four entities: Assistant, Workflow, Datasource, and Skill. No Datasource kind is a separate product entity or receives dedicated scenarios, requirements, acceptance criteria, readiness gates, or compatibility requirements. Every kind follows its exact existing CodeMie format under the same generic Datasource rules; integration provisioning/access is outside the tool for all kinds.

  **Product decision (2026-08-09, v21):**
  * PRODUCT-OQ-01/VER-011 resolved: the CLI emits no Git commit SHA, target environment origin, Git author, CI-run identity, or replacement provenance. Successful per-entity output retains only action, kind, project, and natural key; external systems own audit correlation. FR-013 was deleted.

  **Product decision (2026-08-09, v22):**
  * ARCH-B01 resolved: every valid apply creates a safely missing entity or updates an existing exact match, even when the same YAML was just applied. It never compares desired/current state to suppress the update and never reports `unchanged`.
  * Datasource behavior is ordinary per-kind create/update using the existing request format; v23 governs omitted optional authorable fields, and the specification defines no server-internal processing contract.

  **Product decision (2026-08-09, v23):**
  * VER-012 resolved: omission and explicit YAML null both produce explicit JSON null for a wholly authorable optional property in every applicable create/update payload. Required/conditional/identity fields and any property whose pinned request rejects null remain authoring-required and fail locally when omitted or null. This supersedes mandatory completeness while still preventing silent server-default selection.
  * Field classification is source-pinned per operation. Authoring-only selectors, operation-inapplicable properties, read-only fields, and tool-owned/mixed-ownership structures do not receive fabricated null properties; Workflow `meta_config` retains its approved identity and preservation semantics.

  **Product decision (2026-08-09, v24):**
  * Keycloak token endpoints are explicit configuration, never derived values. Endpoint precedence is `--auth-url` > `CODEMIE_AUTH_URL` > `.codemie/config.yaml` `auth_url`; absence is an exit-2 local configuration failure before network access. `auth_url` is non-secret, while client IDs, client secrets, tokens, email addresses, and passwords remain flag/environment inputs only.
  * Pre-implementation verification evaluates consistency of the refreshed specification-driven artifacts. The required security review follows it in the lifecycle; the absence of a not-yet-due security-review artifact is not a product-readiness blocker.

  ### External dependencies (platform team)
  * Pin the target CodeMie API/OpenAPI contract used to finalize schemas and adapters.
  * Prove complete Workflow/Skill manager/admin visibility, exhaustive stable pagination, exact identity/permission fields, Workflow `meta_config` preservation, and post-write read behavior in the target deployment.
  * Pin ordinary Datasource CRUD/read shapes, discriminators, requiredness, nullability, validation, mutability, and non-secret integration configuration uniformly for every authorable kind. Provider-defined or import/read-only kinds require explicit target evidence; the tool does not invent missing formats.
  * Provide the test environment and service account required to verify the four entity types.

  ### Blocking product questions

  * None.

  ### Statement of readiness
  The specification is **READY FOR ARCHITECTURE PLANNING**. The v14 identity decisions, v15 exit taxonomy, v16 safe output boundary, v17 Workflow reference decision, v20/v22 generic Datasource boundary, v21 provenance boundary, v22 always-write decision, v23 VER-012 omission/nullability rule, v24 explicit Keycloak endpoint rule, and v26 Mode (c) Keycloak ROPC addition require an architecture refresh. Readiness does not prove the target API contract or operational deployment readiness; the solution architect and platform owner must retain the independent evidence gates above. After that refresh, pre-implementation verification assesses artifact consistency and the security reviewer performs the next required review. The absence of a future security-review artifact at this stage is expected and is not a product blocker.

  ---
