# Implementation task breakdown

Source: product specification v29 and this architecture set.

Status: **V28 CORRECTIONS VERIFIED; LOCAL O-002/V-000 PREPARATION READY**.
Q-007/T-003/R-001 and Q-008/W-001/S-001 are implemented and independently
verified; the final Q-007 security review is approved for the next stage.
O-001 checked-in controls are also independently verified and security-approved,
but remote activation remains operationally incomplete. O-002A and V-000A
below isolate the useful local work that can proceed now. O-002B, V-000B, and
the later production/release gates still require external evidence and are not
made complete by local examples, credentials, or smoke-harness preparation.

## 1. Dependency policy

The checked-in normative inputs already exist:

- `contracts/declaration-v1alpha1.schema.json`;
- `contracts/repository-config.schema.json`;
- `contracts/adapter-manifest-v2.42.0.json`; and
- success, warning, diagnostic, CLI, declaration, and HTTP contracts.

Q-001 verifies these artifacts; implementation does not design replacements.
Q-006 depends on Q-001/Q-002/Q-003/Q-004. Q-005 follows Q-006. All
implementation is ordered after Q-005, directly or transitively. No
implementation task may
regenerate a different contract, accept live OpenAPI as author schema, or
modify the product specification or reference repositories.

Eligibility:

- `ARCHITECTURE-REVIEW`: independent lifecycle review before implementation;
- `IMPLEMENTATION`: product code/tests outside reference trees;
- `DEPLOYMENT-VERIFICATION`: target/source drift evidence; and
- `PRODUCTION-ENABLEMENT`: operational/release controls after implementation.

Composite task policy:

- O-002 is complete only when both O-002A and O-002B are complete. O-002A may
  proceed from the verified implementation and checked-in O-001 control design;
  O-002B retains the dependency on operationally complete remote O-001.
- V-000 is complete only when both V-000A and V-000B are complete. V-000A is
  local harness preparation; V-000B is live target evidence.
- Enterprise write smoke is separately identified as V-003. It neither
  completes nor supplies evidence for O-001 or O-002B.

## 2. Review and source-contract tasks

### Q-001 — Validate checked-in declaration and adapter contracts

- Eligibility: `ARCHITECTURE-REVIEW`.
- Objective: prove the closed schemas and source-pinned manifest are complete,
  reproducible architecture inputs.
- Requirements: FR-001/003/006/017/021–025/035/036, DR-001–012,
  IR-002/008, VR-001–016.
- Architecture: ADR-001–004/009; declaration, repository-config, CLI, and
  adapter contracts.
- Scope: compile schemas; positive/negative cases for all four entities and
  every Datasource-union branch; required/optional-null/applicability/ownership
  mutations; unknown
  fields; request/response-consumption/source-location manifest audit against
  both pinned commits; repeat-apply operation rules.
- Dependencies: none.
- Acceptance evidence: deterministic schema probes including omitted/null
  `optionalNull` acceptance and omitted/null required-field rejection; manifest/
  source conformance report; no first-time contract design delegated to implementation.
- Completion: every accepted field and adapter transform has a specification
  and pinned-source trace; every forbidden form is rejected.

### Q-002 — Verify Workflow and Skill capability fixtures

- Eligibility: `ARCHITECTURE-REVIEW`.
- Objective: independently validate ADR-007/008 response and preflight evidence.
- Requirements: FR-028–035, PA-005/006, VR-007–010/013.
- Scope: more than 100 rows, scopes, pagination drift, `meta_config`, abilities,
  exact project, zero/one/multiple, forbidden/incomplete visibility, and
  explicit adoption with another same-display-name unmarked row.
- Dependencies: Q-001.
- Acceptance evidence: sanitized fixtures/analysis with no body or entity
  values in report output.
- Completion: algorithms require no invented response fields or display-name
  selection/veto.

### Q-003 — Verify peer Datasource ordinary-CRUD mappings

- Eligibility: `ARCHITECTURE-REVIEW`.
- Objective: independently validate every bundled per-kind create/update
  mapping as one Datasource contract.
- Requirements: FR-006/021/022/036, DR-010–012, IR-008, QR-002,
  VR-014–016.
- Scope: exact operation fields; presence/null/applicability rules; singular
  File route and JSON/multipart/query encodings,
  content, source, configuration, and scheduling fields; create-only fields;
  provider/Bedrock rejection; absence of dedicated lifecycle surface.
- Dependencies: Q-001.
- Acceptance evidence: one peer matrix with positive and negative fixture per
  bundled branch plus repeat existing-entity PUT request capture.
- Completion: all admitted author fields map to the exact selected operation;
  no equality-based write suppression exists.

### Q-004 — Verify explicit-only authentication configuration and SEC-002 URL policy

- Eligibility: `ARCHITECTURE-REVIEW`.
- Architecture contract state: complete; this task collects independent
  consistency evidence rather than requesting a product/platform decision.
- Objective: verify the closed repository schema, Keycloak endpoint and
  credential-source rules, URL validation policy, redirect policy, and TLS
  requirements against v27 (ADR-011).
- Requirements: FR-017/024, IR-006; AC-FR-017-01/02,
  AC-FR-024-01/03/07/08.
- Architecture: ADR-011 (supersedes ADR-003); CLI §2, repository-config schema,
  and HTTP adapter contract §2.1/§2.2/§2.3.
- Dependencies: Q-001.
- Acceptance evidence:
  - Semantic vectors prove `--auth-url` > `CODEMIE_AUTH_URL` > config `auth_url`;
    only the selected endpoint is used; missing/invalid selected endpoint is exit
    2 before network; API URL presence cannot trigger derivation or probing.
  - Credential keys fail repository-schema validation.
  - Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
    `CODEMIE_PASSWORD`) resolve from environment only with no flag source;
    `--token`, `--client-secret`, `--password` flags are `E_USAGE` exit 2 before
    network with empty stdout.
  - Non-secret selectors (`--client-id`, `--email`) resolve flag over environment.
  - `auth_url` schema pattern rejects http:// and userinfo.
  - `url` and `auth_url` schema patterns reject `@` (userinfo) and `#` (fragment)
    and C0/C1 control characters.
  - HTTP redirect is disabled for Keycloak `POST .../token` and
    `POST /v1/local-auth/login`; a 3xx response to either is `E_AUTHENTICATION`,
    exit 2.
  - `http://` target URL is rejected for non-loopback hosts at runtime; a
    valid `http://127.0.0.1/...` URL is accepted.
- Completion: the architecture contains no derived/probed endpoint branch,
  credential-bearing repository-config path, or credential-forwarding redirect.

### Q-006 — Pre-implementation convergence re-verification (v27)

- Eligibility: `ARCHITECTURE-REVIEW`.
- Objective: verify v27, schemas, manifest, ADRs (including ADR-011, ADR-012,
  ADR-005 amendment), plan/data-model/research, and tasks converge.
- Requirements: all active v27 requirements and all acceptance criteria.
- Dependencies: Q-001, Q-002, Q-003, Q-004.
- Acceptance evidence: independent report with no blocking/high inconsistency;
  task graph IDs exist and are acyclic; protected-source integrity is preserved;
  SEC-001–SEC-006 remediation artifacts are present and internally consistent;
  ADR-012 (SEC-004) is accepted with the project-admin preflight and D-001
  authorization consistently represented; AC-FR-014-01 warning scope, order,
  and failure behavior converge across specification, contracts, architecture,
  tasks, code, and tests.
- Completion: an independent report records v27 convergence and routes any
  security-impact delta to Q-005; a prior-version report is not reused as v27
  completion evidence.

### Q-007 — Verify the v28 compatibility correction design

- Eligibility: `ARCHITECTURE-REVIEW`.
- Lifecycle: **COMPLETE**. Architecture verification passed, the bounded
  T-003/R-001 implementation is independently verified, and the final Q-007
  security review is `APPROVED FOR NEXT STAGE`. This does not replace V-000.
- Objective: independently verify that v28 removes semantic application version
  from target acceptance/rejection while retaining all operation-applicable
  pre-write contract evidence.
- Requirements: SC-021, IR-011/012, AC-IR-011-01, AC-IR-012-01.
- Architecture: ADR-004; `plan.md` sections 5, 6, 10, and 13;
  `data-model.md` section 7; `contracts/source-baseline.md` and
  `contracts/http-adapter.md` section 2.6.
- Dependencies: approved product specification v28 and Q-006 baseline evidence.
- Acceptance evidence: verifier confirms no architecture artifact treats
  `/v1/info.version` as a Git/source/API identity or required gate; the exact
  pinned clone reporting `0.16.0` can proceed only after all applicable
  non-mutating evidence passes; missing/invalid required consumed evidence is
  still exit-2 `E_API_INCOMPATIBLE` with zero modifying requests. It also
  confirms that project-admin evidence matches the exact effective project for
  Workflow, Skill, and Datasource, while Assistant uses strict direct lookup
  without `/v1/user`; only a sealed `PreparedWrite` created after all applicable
  reads can reach POST/PUT.
- Completion: no unresolved product or architecture conflict blocks T-003 and
  R-001 correction work.

### Q-008 — Verify zero-based Workflow and Skill pagination design

- Eligibility: `ARCHITECTURE-REVIEW`.
- Lifecycle: **COMPLETE AND IMPLEMENTED**. The decision authorized W-001/S-001/
  R-001; final post-implementation verification proves page-0 behavior and
  permanently supersedes the earlier one-indexed conclusion.
- Objective: independently verify the source-derived page base and invariants
  before W-001/S-001 correction work.
- Requirements: FR-029/031/034, IR-012, PA-005, VR-007–010/016.
- Architecture: ADR-007/008; adapter manifest Workflow/Skill pagination blocks;
  `contracts/http-adapter.md` sections 5/6; `data-model.md` sections 4/5.
- Dependencies: approved v28 specification and pinned backend commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- Acceptance evidence:
  - Verify the Workflow router's `page=0` origin, explicit zero supplied by
    helper call sites, unchanged propagation, and `offset(page * per_page)`.
    Verify Skill's router/service/repository defaults of `0` and the same
    offset formula. Do not require a Workflow service/repository default
    parameter that the pinned source does not define.
  - Verify both manifest entries require page 0 first, exact `0..pages-1`
    traversal, page echo/size/count checks, and a single page-0 request for an
    empty result.
  - Record that the one-indexed Workflow/Skill statement in
    `Q-007-post-implementation-verification.md` is stale and superseded for
    pagination-origin evidence; do not reuse its page-origin conclusion.
- Completion: independent evidence authorizes W-001/S-001 correction without
  changing product behavior or Datasource's already-zero-based contract.

### Q-005 — Security architecture review

- Eligibility: `ARCHITECTURE-REVIEW`.
- Objective: independently review input, authentication, transport, output,
  supply-chain, and privileged-resolution boundaries after convergence passes.
- Requirements: FR-009/011/014/016/017/024/026, QR-006/007/011,
  VR-006/011/012; AC-FR-014-01, AC-FR-017-01/02, and AC-FR-024-07.
- Dependencies: Q-006.
- Acceptance evidence: security report including explicit-endpoint,
  credential-source, no-network, and no-leak boundaries; successful lint warning
  scope/order and failed-lint diagnostic exclusivity from AC-FR-014-01;
  critical/high findings closed or routed to the owning upstream role.
- Completion: security reviewer approves implementation boundaries.

## 3. Local foundations

### F-001 — Scaffold the modular Rust product

- Eligibility: `IMPLEMENTATION`.
- Objective: create one `codemie-gitops` Linux x86_64 binary and testable library
  boundaries outside reference trees.
- Requirements: FR-001, QR-006/008.
- Architecture: ADR-005 (including supply-chain and CI controls amendment, SEC-006).
- Dependencies: Q-005.
- Acceptance evidence:
  - `cargo check`, help/package snapshot naming only `codemie-gitops`.
  - `Cargo.lock` is committed to the repository.
  - All CI builds use `cargo build --locked` and `cargo test --locked`.
  - Dependency/license inventory produced; no reference-tree dependency.
  - RustSec advisory scan (`cargo audit`) is configured as a CI gate.
  - CI workflow files use SHA-pinned action references (not floating tag refs).
  - CI workflows have `permissions:` blocks with least-privilege settings.
  - Secrets are not accessible to fork/PR triggered workflows.
- Completion: shell and modules build without product behavior; supply-chain
  gates are active.

### F-002 — Implement commands, config, and negative surface

- Eligibility: `IMPLEMENTATION`.
- Objective: exact CLI/config precedence and locally forbidden options.
- Requirements: FR-002/009/017/019/020/024/026/036, VR-008/010/014;
  AC-FR-017-01/02 and AC-FR-024-07.
- Scope: lint/apply/login, output mode, Workflow-only adoption, project default,
  unknown plan/delete/batch/generic-adoption/lifecycle controls; closed
  `{url?,auth_url?,project?}` repository config; exact auth URL precedence;
  environment-only resolution for secret credentials (`CODEMIE_TOKEN`,
  `CODEMIE_CLIENT_SECRET`, `CODEMIE_PASSWORD`); flag-over-environment resolution
  for non-secret selectors (`--client-id`, `--email`); and rejection of
  `--token`, `--client-secret`, and `--password` as CLI flags (E_USAGE, exit 2).
- Dependencies: F-001.
- Acceptance evidence:
  - Per-field precedence table and semantic tests.
  - Config schema rejects every credential key.
  - Missing Keycloak endpoint fails despite an API URL.
  - Mixed auth modes produce `E_USAGE`, exit 2.
  - No generic marker/adoption option.
  - Zero transport calls on any usage/config failure.
  - `--token`, `--client-secret`, and `--password` flags cause exit 2 before
    network access with empty stdout.
  - URL validation: `auth_url` with `http://` scheme is `E_CONFIGURATION`, exit 2.
  - URL validation: URL with `@` (userinfo), `#` (fragment), or C0/C1 control
    characters is `E_CONFIGURATION`, exit 2.
  - URL validation: `http://` target URL with non-loopback host is
    `E_CONFIGURATION`, exit 2 (runtime check).
  - URL validation: `http://127.0.0.1/...` target URL is accepted.
  - An invalid higher-precedence URL is `E_CONFIGURATION`, exit 2; lower-precedence
    URLs are not consulted.
- Completion: all usage/config failures are exit 2 with empty stdout.

### F-003 — Implement deterministic discovery and safe file loading

- Eligibility: `IMPLEMENTATION`.
- Objective: layout-neutral, bounded, root-contained discovery.
- Requirements: FR-004/008/025/027, QR-001/007, VR-006.
- Scope: `.git` and config exclusion only; deterministic path order; YAML and
  scalar `contentFrom` relative to the declaring YAML; symlink/cycle/escape/
  size/permission controls.
- Dependencies: F-001.
- Acceptance evidence:
  - Traversal tests including declarations below directories named `codemie`
    and `codemie-ui`; nested sidecar examples from AC-FR-025.
  - Repository files visited cap (10,000) is enforced; exceeding it is a local
    error before further processing.
  - Sidecar per-file bytes cap (32 MiB) is enforced using open-then-fstat on the
    same file descriptor (descriptor-safe open, SEC-003); no canonicalize-then-
    reopen pattern is used.
  - Aggregate upload bytes cap (128 MiB) is tracked across all file parts in one
    invocation and enforced before streaming.
  - Source file path values (`source.file`) are bounded to 4,096 bytes in output.
- Completion: safe source bytes/coordinates are available without network.

### F-004 — Implement marked YAML and exact closed schemas

- Eligibility: `IMPLEMENTATION`.
- Objective: implement the checked-in declaration schema exactly.
- Requirements: FR-001–003/014/021–023/025/027/035/036,
  DR-001/002/004–006/012, VR-001–016.
- Dependencies: F-001, F-003, Q-001.
- Scope: marked AST, duplicate/tag/document/resource bounds, effective project,
  four kind branches, peer Datasource union, actor union, unknown/secret/
  runtime/lifecycle rejection. Never insert defaults.
- Acceptance evidence:
  - Exact positive/negative/mutation corpus; omitted and explicit-null
    optional-null fields validate identically; omitted or null null-rejecting
    fields fail; no network calls.
  - YAML per-file byte limit (1 MiB) is enforced before AST allocation;
    exceeding it is `E_YAML_PARSE` or `E_SCHEMA`, exit 2.
  - YAML nesting depth limit (32 levels) is enforced.
  - YAML alias/anchor expanded node limit (1,000) is enforced (no alias-bomb).
  - YAML scalar length limit (128 KiB) is enforced per scalar.
  - YAML collection member limit (10,000 per array/object) is enforced.
  - Identifier fields (`project`, `slug`, `name`, `repo_name`) are validated
    against the schema maxLength and control/bidi character rejection pattern.
- Completion: implementation schema fingerprint matches the reviewed schema.

### F-005 — Implement offline natural and graph reference validation

- Eligibility: `IMPLEMENTATION`.
- Objective: exact deterministic repository and Workflow-local reference closure.
- Requirements: FR-004/014/025/035, DR-003, VR-005/009/013.
- Dependencies: F-003, F-004.
- Acceptance evidence: duplicate/missing/wrong-kind/cross-project, sidecar,
  actor uniqueness/state-local, and persisted-versus-inline reference tests;
  target and closure-only invalid-declaration cases prove the complete closure
  must pass before any declaration warning becomes eligible for emission.
- Completion: lint validates the complete repository closure with zero server
  access and exposes a single success gate for target-only warning evaluation.

### F-006 — Implement operation request projection

- Eligibility: `IMPLEMENTATION`.
- Objective: produce typed create/update requests from authored intent and the
  selected identity result.
- Requirements: FR-006/012/021/022/028/031/035/036, QR-002.
- Architecture: ADR-002/007–009; data model and adapter manifest.
- Dependencies: F-004, Q-002, Q-003.
- Scope: omission/null-to-explicit-null handling; no default filling; Workflow
  string metadata decode/merge/canonical encode and actor transforms;
  source-pinned create/update fields; create-only operation exclusion;
  server-owned exclusion; file/source/content write-through.
- Acceptance evidence: create and update fixtures prove every omitted or
  explicit-null optional-null JSON property is present as `null`; required
  omissions fail before resolution; inapplicable/author-only/mixed/read-only
  fields receive no fabricated member; zero resolution creates POST plan and
  one creates PUT plan on every invocation.
- Completion: adapters accept only typed `Create` or `Update` plans.

### F-007 — Implement closed success, warning, and diagnostic renderers

- Eligibility: `IMPLEMENTATION`.
- Objective: enforce the exit taxonomy, stream split, deterministic warning
  contract, and no-leak boundary.
- Requirements: FR-011/012/014/016/026, QR-004/007/011, VR-011/012.
- Architecture: ADR-003/010; outcome/warning/diagnostic schemas.
- Dependencies: F-001, F-005.
- Scope: private constructors; text/JSON parity; separate stderr warnings;
  success actions `valid|created|updated`; closed code/category/exit union;
  fixed warning phrases; target-declaration warning scope after complete
  repository-closure validation; bytewise fixed-warning-code then canonical-
  field-path ordering; no warning records on failure; no generic
  message/body/value/raw-URL API.
- Acceptance evidence:
  - Semantic schema probes; forbidden extra success and diagnostic fields
    rejected; failure stdout empty; every failure produces exactly the selected-
    output-mode diagnostic and no warnings.
  - Successful lint emits warnings only for the `--file` declaration after the
    complete repository closure validates; closure-only warning conditions are
    ignored, and output is stable in bytewise `warningCode` then canonical
    `source.fieldPath` order.
  - Canary secrets, bodies, server text, declaration values, and exception
    strings absent in all paths.
  - Each outcome, warning, and diagnostic produces exactly one physical output
    line; no untrusted value can inject an unescaped newline or CR (SEC-005).
  - JSON output uses a serializer, not string concatenation; control characters
    in string values are JSON-escaped by the serializer.
  - Text success output uses only the fixed template `<action> <kind>
    <project>/<key>`; values come from schema-validated fields only.
  - `source.fieldPath` in diagnostics and warnings is generated canonically
    from the YAML AST, not echoed from arbitrary user input.
  - `action`, `kind`, `category`, `errorCode`, `warningCode` come from fixed
    enums and are never derived from untrusted input.
  - Route parameters and query parameter values are percent-encoded by a URL
    encoder, not by string concatenation.
- Completion: every renderer value is schema-allowlisted.

## 4. Authentication, transport, and compatibility

### T-001 — Implement stateless authentication and login

- Eligibility: `IMPLEMENTATION`.
- Objective: bearer, Keycloak client credentials, Keycloak ROPC (v26), and
  local-development auth.
- Requirements: FR-009/017/024, IR-006, PA-001/003;
  AC-FR-017-01/02 and AC-FR-024-01/03/07/08.
- Dependencies: F-002, F-007.
- Acceptance evidence:
  - Mock IdP/API tests; exact selected endpoint contacted; lower-precedence
    endpoints untouched.
  - Missing endpoint with configured API URL makes zero network calls.
  - Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
    `CODEMIE_PASSWORD`) resolved from environment only with no flag source.
  - `--token`, `--client-secret`, and `--password` flags produce exit 2 before
    network access with empty stdout.
  - Non-secret selectors (`--client-id`, `--email`) resolved flag over environment.
  - No repository source for any credential.
  - Successful token is the single intentional stdout exception; no token or
    credential elsewhere.
  - Mock IdP redirect test: Keycloak `POST .../token` receiving a 3xx response
    is `E_AUTHENTICATION`, exit 2; credential is not replayed to redirect target.
  - Local-auth `POST /v1/local-auth/login` receiving a 3xx response is
    `E_AUTHENTICATION`, exit 2.
  - `auth_url` with `http://` scheme is rejected as `E_CONFIGURATION`, exit 2,
    before any network call.
  - Mode (c) ROPC (v26): when `CODEMIE_CLIENT_SECRET` is absent, `CODEMIE_EMAIL`
    and `CODEMIE_PASSWORD` are set, and `auth_url` is configured, the token
    request uses `grant_type=password` with no `client_secret` field;
    `CODEMIE_CLIENT_ID` defaults to `codemie-sdk` when unset; mock IdP verifies
    exact body fields and absence of `client_secret`.
- Completion: every auth result uses the approved output/exit contract.

### T-002 — Implement safe HTTP transport

- Eligibility: `IMPLEMENTATION`.
- Objective: direct bounded REST transport with safe metadata only.
- Requirements: FR-011/014/016, IR-001/003/004/007, QR-007.
- Dependencies: F-007, T-001.
- Acceptance evidence:
  - TLS/proxy/CA, redirect, timeout/retry, malformed/large body, 4xx/5xx,
    correlation-header, and canary fault suite.
  - Response body exceeding 8 MiB is bounded-drained and classified as
    `E_CONNECTIVITY` or `E_API_INCOMPATIBLE`, exit 2; full body is not allocated.
  - Response header block exceeding 16 KiB is classified as `E_CONNECTIVITY`,
    exit 2.
  - Per-request timeout (60 seconds) fires and is classified as `E_CONNECTIVITY`.
  - Invocation deadline (300 seconds) propagates to all in-flight operations
    including pagination, retries, and file streaming.
  - GET retry (max 3 attempts with jitter) does not exceed invocation deadline.
  - POST/PUT is never blindly retried.
  - For API calls: if any redirect is followed, it must be method-aware
    (POST/PUT not redirected to GET), same-origin (same scheme+host+port), and
    must not forward the `Authorization` header to a different authority.
  - JSON response nesting depth exceeding 64 levels is rejected as
    `E_API_INCOMPATIBLE`, exit 2.
  - Route parameters and query values are percent-encoded by a URL encoder.
- Completion: only typed compatible successes or safe classifications survive.

### T-003 — Implement pinned compatibility preflight

- Eligibility: `IMPLEMENTATION`.
- Lifecycle: **COMPLETE — INDEPENDENTLY VERIFIED AND SECURITY-APPROVED**.
- Objective: validate consumed target capabilities against the checked-in
  manifest before writes where detectable.
- Requirements: SC-021, IR-002/005/008–012, QR-009;
  AC-IR-011-01 and AC-IR-012-01.
- Architecture: ADR-004.
- Expected components: `src/preflight/mod.rs`, `src/http/mod.rs`, and manifest
  conformance/preflight tests.
- Dependencies: Q-001, Q-007, T-002.
- Scope:
  - Remove the runtime comparison of `/v1/info.version` with the pinned Git SHA.
    `EXPECTED_BACKEND_COMMIT`, if retained, is build/test provenance only and
    must never be compared with a target response.
  - Do not require `GET /v1/info` for `apply`; its status, shape, absence, and
    semantic value cannot accept or reject an operation.
  - Preserve strict decoding of the manifest's operation-applicable consumed
    fields in `GET /v1/user` and adapter identity/reference/detail/pagination
    reads. Required consumed fields must not silently default when absent.
  - Apply the capability predicate to Workflow, Datasource, and Skill: global
    admin/maintainer, or a `projects[]` entry whose `name` equals the exact
    effective project and whose `is_project_admin` is true. An admin entry for
    another project is insufficient. Assistant remains excluded and uses its
    strict direct `(project, slug)` lookup under PA-003.
  - Keep `E_API_INCOMPATIBLE`, exit 2, empty stdout, safe stderr, and zero
    POST/PUT for missing or invalid required contract evidence.
- Acceptance evidence:
  - Replace SHA-valued `/v1/info` success fixtures with the exact pinned-source
    scenario (`0.16.0`) and prove no semantic-version mismatch is emitted.
  - Add strict `GET /v1/user` missing/invalid consumed-field tests and retain
    adapter response/pagination drift tests; each asserts `E_API_INCOMPATIBLE`.
  - Add Workflow/Skill/Datasource positive exact-project and negative other-
    project-admin tests; the latter assert `E_VISIBILITY_UNPROVEN` and zero
    modifying requests. Add an Assistant test proving no `/v1/user` call is
    required and strict direct-lookup evidence still seals the prepared write.
  - Additional unconsumed response fields pass without widening declaration or
    request contracts; manifest tests pin
    `ignore-only-when-additive-and-unconsumed` and the exact-project predicate.
  - Request-count assertions prove every failed pre-write case issues zero
    POST, PUT, DELETE, or other modifying request.
- Completion: runtime adaptation cannot alter declaration/request contracts.

## 5. Entity adapters and coordinator

### A-001 — Implement Assistant adapter

- Eligibility: `IMPLEMENTATION`.
- Objective: exact `(project,slug)` resolve/create/update.
- Requirements: FR-005/006/015/021, DR-003/005.
- Dependencies: F-005, F-006, T-003.
- Acceptance evidence: missing/create, unique/repeat-update, collision,
  reference projection, authorization, compatibility, and rejection tests.
- Completion: existing identity always receives PUT; server ID is
  invocation-local only.

### W-001 — Implement Workflow identity resolver and adoption

- Eligibility: `IMPLEMENTATION`.
- Lifecycle: **COMPLETE — ZERO-BASED CORRECTION INDEPENDENTLY VERIFIED AND
  SECURITY-APPROVED**.
- Objective: implement ADR-008 exactly.
- Requirements: FR-006/021/022/028–030/032–034, PA-005/006,
  VR-007–010/016.
- Dependencies: F-006, T-003, Q-002, Q-008.
- Scope: all pages/scopes, string `meta_config` strict decode/canonical
  encode/preservation merge, display-name nonselection guard,
  explicit by-ID adoption without display-name veto, rename/new-key semantics,
  unconditional resolved-identity PUT, and post-write identity resolution.
- Pagination scope: each project/marketplace pass starts at page 0, uses
  `per_page=100`, requests `[0]` when pages is zero or exactly `0..pages-1`
  otherwise, and validates page echo, size, page-count formula, stable snapshot,
  caps, and unique accumulated IDs before sealing evidence.
- Acceptance evidence: zero/one/multiple/more-than-100/invalid/drift/permission/
  race; explicit candidate plus another same-display-name row succeeds when
  other checks pass; repeat apply sends PUT; no UUID output. Fixtures cover an
  empty page 0, one item on page 0, 101+ items on pages 0/1, rejection of a
  page-1 origin, and post-write verification using the same page-0 scanner.
- Completion: no derived UUID, implicit adoption, display-name selection, or
  equality branch.

### S-001 — Implement Skill exhaustive resolver and adapter

- Eligibility: `IMPLEMENTATION`.
- Lifecycle: **COMPLETE — ZERO-BASED CORRECTION INDEPENDENTLY VERIFIED AND
  SECURITY-APPROVED**.
- Objective: implement ADR-007 and Skill CRUD.
- Requirements: FR-005/006/015/025/031–034, PA-005, VR-009/010.
- Dependencies: F-005, F-006, T-003, Q-002, Q-008.
- Scope: per-page 100/all pages/scopes/hints, exact filter, zero/one/multiple,
  capability, scalar YAML-relative sidecar expansion, one create-409
  re-resolution, unconditional resolved-identity PUT, post-write scan. Every
  scan starts at page 0, requests `[0]` when pages is zero or exactly
  `0..pages-1` otherwise, and validates page echo, size, page-count formula,
  stable snapshot, caps, and unique accumulated IDs before sealing evidence.
- Acceptance evidence: creator duplicates/order/more-than-100/drift/marketplace/
  permissions/same- and different-principal races; no repeated POST or
  tie-break; repeat apply sends PUT. Fixtures cover empty page 0, one item on
  page 0, 101+ items on pages 0/1, rejection of a page-1 origin, and reuse of
  the same zero-based scanner by initial, post-write, and create-409 resolution.
- Completion: authored/reported identity is only `(project,name)`.

### D-001 — Implement peer Datasource adapters

- Eligibility: `IMPLEMENTATION`.
- **Note (ADR-012 Accepted 2026-08-10)**: Option A selected. Visibility
  precondition: `GET /v1/user` preflight required before any Datasource write;
  exit `E_VISIBILITY_UNPROVEN` (exit 2) if principal lacks global-admin,
  global-maintainer, or project-admin for the exact effective project.
  Project-admin for another project is insufficient. Project-admin scope for
  the matching project is sufficient (confirmed from
  `index_service.py:276-282`).
- Objective: exact exhaustive identity and ordinary write-through CRUD for every
  schema branch.
- Requirements: FR-005/006/015/021/022/036, DR-010–012, IR-008,
  QR-002, VR-014–016, PA-007.
- Architecture: ADR-009 (amended by ADR-012); ADR-012 (accepted).
- Dependencies: F-005, F-006, T-003, Q-003.
- Scope: zero-indexed all-page resolution and capability fields; peer JSON
  mappings plus singular File multipart/query mapping and byte/path bounds;
  create on missing; update on every present identity;
  content/file/source/configuration/scheduling fields; explicit JSON nulls;
  opaque external references; create-only operation exclusion; unsupported
  provider/Bedrock boundaries; no dedicated lifecycle operation.
- Acceptance evidence:
  - Per-branch zero/one/multiple/drift and exact create/update request fixtures.
  - Repeat file apply sends multipart PUT.
  - Locally valid external-reference rejection is safely classified.
  - Pagination cap (1,000 pages, 100,000 items) is enforced; exceeding either
    limit is `E_API_INCOMPATIBLE`, exit 2.
  - File Datasource multipart basename safety: basenames containing C0/C1
    controls, CR, LF, NUL, or path separator characters are rejected before
    the multipart request is constructed.
  - Multipart parts cap (10 per File Datasource) is enforced before upload.
- Completion: one Datasource adapter boundary with visibility preflight per
  ADR-012 Option A; exact-effective-project admin is confirmed sufficient from
  source evidence and a cross-project admin cannot pass.

### W-002 — Implement Workflow execution/reference projection

- Eligibility: `IMPLEMENTATION`.
- Objective: exact graph-local and server-resource transform.
- Requirements: FR-004/032/035, DR-003, VR-005/013.
- Dependencies: A-001, W-001, S-001, D-001, F-005, F-006.
- Acceptance evidence: persisted/inline request goldens; local IDs remain;
  missing/ambiguous refs fail; server fields rejected in YAML.
- Completion: author and transport actor shapes are not conflated.

### R-001 — Implement single-entity write-through coordinator

- Eligibility: `IMPLEMENTATION`.
- Lifecycle: **COMPLETE — INDEPENDENTLY VERIFIED AND SECURITY-APPROVED**.
- Objective: implement one preflight/resolve/project/write/verify state machine.
- Requirements: FR-005/006/008/011/012/015/021/032/034/036,
  IR-003–012, QR-001–003/008/011; AC-IR-011-01 and AC-IR-012-01.
- Expected components: `src/coordinator/mod.rs`, the adapter-to-write-dispatch
  boundary, and coordinator request-count/state-transition tests.
- Dependencies: A-001, W-001, W-002, S-001, D-001, F-007, T-003.
- Acceptance evidence:
  - Remove the unconditional `/v1/info.version` compatibility call from the
    coordinator. The pre-write transition completes only after the kind-
    applicable admin preflight (Workflow/Skill/Datasource only) and the selected
    adapter's required identity/reference/detail/pagination reads have decoded
    successfully.
  - Refactor the coordinator/adapter boundary so the modifying dispatcher
    accepts only a sealed `PreparedWrite` containing the kind-specific preflight
    result, completed operation-specific read evidence, and projected create/
    update request. For Assistant the preflight result records that admin proof
    is not required and the read evidence contains the strict direct lookup; for
    Workflow/Skill/Datasource it contains exact-project visibility proof. No
    earlier or partial-evidence state can call POST/PUT.
  - Coordinator success test exposes `/v1/info.version=0.16.0` (or asserts the
    endpoint is not contacted), supplies valid operation evidence, and observes
    exactly one expected POST or PUT.
  - Coordinator incompatibility tests invalidate one required preflight or
    resolver field at a time and assert exit-2 `E_API_INCOMPATIBLE`, empty
    stdout, FR-016-safe stderr, and exact zero counts for every modifying route.
    A matching, non-matching, missing, or unreachable `/v1/info` cannot override
    either result.
  - State-transition tests prove evidence is sealed only after all applicable
    identity/reference/detail/pagination reads and projection complete; inject
    failure at each read boundary and assert the write dispatcher is never
    reached. Cross-project admin tests cover Workflow, Skill, and Datasource;
    Assistant coverage proves direct lookup reaches the same seal without
    `/v1/user`.
  - End-to-end Workflow and Skill coordinator tests assert the first list
    request is page 0, valid page-0 evidence permits exactly one selected write,
    and an invalid page origin/echo makes zero modifying requests. Post-write
    verification and Skill create-409 re-resolution repeat from page 0.
  - Fake-server state/fault/request-count tests; stopped pre-write failures.
  - Absent identity has one POST/`created`; present identity has one
    PUT/`updated` on every invocation.
  - May-have-committed result is reported safely; no delete or rollback.
  - Stream split: stdout empty on all failure paths.
  - No delete.
  - Invocation deadline (300 seconds) cancellation propagates to all in-flight
    operations in the coordinator: parsing, sidecar reads, file streaming,
    HTTP requests, retries, and pagination; deadline expiry is classified as
    `E_CONNECTIVITY` or `E_INTERNAL`, exit 2.
- Completion: every terminal path maps to the closed schemas and taxonomy.

## 6. Operations, verification, and release evidence

### O-001 — Activate serialization and identity-writer governance

- Eligibility: `PRODUCTION-ENABLEMENT`.
- Lifecycle state: checked-in/local controls implemented, independently
  verified, security-approved, and ready for remote activation; operational
  completion remains blocked on the external evidence below.
- Objective: activate the normative race-prevention controls in the remote
  providers and prove their operation.
- Requirements: FR-033/034, PA-005/006, QR-010/011.
- Dependencies: R-001.
- Checked-in/local evidence: GitHub and GitLab serialized apply definitions,
  control policy and checklist, activation-evidence template, identity inventory
  tooling and fixtures, local control validator, and automated control tests.
- Independent review state: local implementation verification passed and the
  security review approved remote activation; neither review substitutes for
  provider-generated operational evidence.
- Blocking external acceptance evidence:
  - activated GitHub/GitLab provider protections and eligible protected runner
    configuration;
  - same-environment serialization and different-environment independence
    mutex drills;
  - complete-visibility live Workflow marker plus Skill and Datasource
    duplicate inventory;
  - identity-writer freeze plus concurrent-writer and uncertain-write
    remediation with named owners; and
  - checksum verification proving the first remote apply consumed the exact
    tested artifact.
- Completion: production Workflow/Skill apply control checklist passes.

### O-002 — Publish CI examples and recovery runbooks (composite)

- Eligibility: `PRODUCTION-ENABLEMENT`.
- Objective: portable safe `codemie-gitops` lint/login/apply and recovery.
- Requirements: FR-002/008/009/017/024/026/027, IR-006, PA-003/007,
  QR-012; AC-QR-010-01, AC-QR-012-01, and SEC-006.
- Completion: both O-002A and O-002B complete. Local O-002A evidence alone
  must never be reported as completed O-002.

### O-002A — Implement and verify local examples, guide, and runbooks

- Eligibility: `IMPLEMENTATION`.
- Lifecycle: **READY FOR PRE-IMPLEMENTATION VERIFICATION**.
- Objective: create locally verifiable, portable user guidance and inert CI
  examples without requiring a Git remote or claiming provider activation.
- Requirements: same as O-002.
- Architecture: `plan.md` section 12, local documentation and example boundary;
  ADR-005/011/012; `contracts/cli.md`.
- Dependencies: completed/verified R-001 and the independently verified,
  security-approved checked-in portion of O-001. Operational completion of
  remote O-001 is deliberately not a dependency.
- Exact files:
  - `README.md` — primary build/install and lint/login/apply guide, output/exit
    behavior, always-write warning, environment-only credentials, token reuse,
    recovery links, and enterprise-smoke safety boundary.
  - `examples/README.md` plus
    `examples/repository/.codemie/config.yaml`, one schema-valid declaration
    for each of Assistant/Workflow/Skill/Datasource, and the Skill markdown
    sidecar. The Datasource example uses only a documented non-secret opaque
    integration reference and does not authorize live execution.
  - `examples/ci/github-actions.yml` and `examples/ci/gitlab-ci.yml` — inert
    portable examples, not provider-active root configuration.
  - `ops/o002/README.md`, `GIT_REVERT_RECOVERY.md`,
    `WORKFLOW_ADOPTION.md`, and `UNCERTAIN_WRITE.md`.
  - `scripts/check_o002_examples.py` and `tests/test_o002_examples.py`.
- Acceptance evidence:
  - The root README documents a locked build/prebuilt-binary path; offline lint;
    exact config precedence; all three login modes with local-auth explicitly
    excluded from CI; provider-specific process-local token reuse; GitHub fresh
    login plus immediate native add-mask; GitLab pre-supplied
    environment-scoped protected+masked `CODEMIE_TOKEN` with no `login`;
    single-entity apply; repeat apply always updating; no plan/delete; approved
    streams/exits; and all recovery entry points.
  - Every example declaration passes offline lint from the sample repository
    and contains no credential or server ID. The examples index states that
    filenames/directories are conventions, not identity.
  - Secret inputs occur only as `CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, or
    `CODEMIE_PASSWORD` environment variables. No `--token`, `--client-secret`,
    `--password`, `--insecure`, TLS bypass, `.env` loading in CI, `set -x`,
    `RUST_LOG`/HTTP-body debugging, token/body echo, or login-stdout persistence
    appears.
  - GitHub captures one login token in the protected step, immediately invokes
    GitHub's native runtime add-mask control before any later command/output,
    and reuses it only in that step's memory. GitLab does not invoke `login`;
    the protected job consumes a pre-supplied environment-scoped
    protected+masked `CODEMIE_TOKEN` only in process memory. Neither provider
    persists, transfers, re-emits, or simulates masking for the token.
    Pull-request/fork jobs have no deployment secret reference.
  - Build/lint/test/checksum run without deployment credentials. The protected,
    approval-gated apply job consumes that same checksummed binary and does not
    rebuild it.
  - Git recovery creates a new revert/correction commit and then a new
    serialized apply; it does not imply remote rollback. Workflow adoption is
    exact-ID, reviewed, and one invocation only. Uncertain-write recovery holds
    the queue, freezes writers, inventories with complete visibility, and
    forbids blind retry, delete, rollback, and tie-break.
  - The checker parses provider YAML structurally, invokes offline lint for all
    declarations, verifies the fixed layout and runbook invariants, and fails
    closed on each forbidden pattern. Mutation-negative tests cover secret
    flags, PR secret exposure, unprotected apply, protected-job rebuild,
    checksum omission, missing GitHub native masking, GitLab fresh-login or
    simulated-masking attempts, missing GitLab pre-supplied token consumption,
    token persistence/transfer, unsafe logging/TLS, and missing recovery
    prohibitions.
- Validation expectations:
  - `python3 scripts/check_o002_examples.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_o002_examples`
  - offline `codemie-gitops lint` for every file under `examples/repository/`
  - `git diff --check`
- Completion: local examples and recovery guidance are self-consistent and
  independently verified; remote provider behavior remains O-002B.

### O-002B — Activate and validate examples in remote CI providers

- Eligibility: `PRODUCTION-ENABLEMENT`.
- Lifecycle: **BLOCKED ON REMOTE O-001 AND PROVIDER EVIDENCE**.
- Objective: adopt the O-002A examples in a real protected GitHub or GitLab
  repository and prove their provider-side trust and recovery controls.
- Requirements: same as O-002.
- Dependencies: O-002A and operationally complete O-001.
- Acceptance evidence:
  - Protected, approval-gated deployment environment; protected ref/runner;
    environment-scoped masked secrets; and secret-free fork/PR observations.
  - The exact locally checked pattern builds/tests/checksums outside the secret
    boundary and consumes the same artifact inside it.
  - GitHub acquires one fresh login token, immediately applies native runtime
    masking, retains it only in the protected step's memory, and reuses it.
    GitLab consumes one pre-supplied environment-scoped protected+masked
    `CODEMIE_TOKEN` in protected-job memory and never invokes `login`. Provider
    logs and persisted data contain no token, credential, body, or `.env`
    content; neither provider transfers or simulates masking for the token.
  - A non-writing adoption/recovery tabletop links provider run references to
    the O-002 runbooks without weakening O-001's inventory, writer-freeze, or
    uncertain-write controls.
- Completion: provider evidence proves the published examples are adoptable and
  safe; O-002 may then be marked complete.

### V-000 — Verify target/source compatibility (composite)

- Eligibility: `DEPLOYMENT-VERIFICATION`.
- Objective: detect target drift before production writes.
- Requirements: IR-002/005/008–012, QR-009; AC-IR-011-01/AC-IR-012-01.
- Completion: both V-000A and V-000B complete. Local harness tests alone never
  satisfy target/source compatibility.

### V-000A — Prepare the non-mutating target-qualification harness

- Eligibility: `IMPLEMENTATION`.
- Lifecycle: **READY FOR PRE-IMPLEMENTATION VERIFICATION**.
- Objective: implement a locally testable live-target probe that cannot modify
  CodeMie and emits only sanitized qualification evidence.
- Architecture: `plan.md` section 12, target qualification and enterprise smoke
  boundary; pinned adapter manifest and HTTP contract.
- Dependencies: Q-001, Q-002, Q-003, completed/verified R-001.
- Expected files:
  - `ops/v000/README.md` and `ops/v000/enterprise-smoke.example.json`;
  - `scripts/v000_target.py`;
  - `tests/test_v000_target.py`; and
  - narrow `.gitignore` entries for `ops/v000/*.local.json` and local
    run-scoped declaration directories, never the checked-in examples.
- Scope and acceptance evidence:
  - Before authentication, the harness verifies the staged binary's checksum.
    It obtains a token only by invoking that exact staged
    `codemie-gitops login` binary, or consumes an already supplied
    `CODEMIE_TOKEN`; it never reimplements a credential POST or persists login
    stdout. The same unchanged staged binary is retained for V-003, and its
    fixed non-secret SHA-256 is persisted in the sanitized V-000B handoff.
  - `--preflight` exposes only operation-applicable GET/read probes required by
    the pinned contract. The transport primitive below CLI parsing has a closed
    GET-only method type/allowlist, so no POST, PUT, PATCH, or DELETE can be
    constructed or reached in this mode.
  - Probe URLs satisfy ADR-011: verified HTTPS; no userinfo, fragment, TLS
    bypass, or proxy downgrade; redirects disabled for every request; the exact
    authorized scheme/host/port is checked both before request construction and
    after URL resolution; and Authorization is attached only to that origin.
    Neither a same-origin nor cross-origin 3xx is followed.
  - Connect/read timeouts and the 300-second invocation deadline are enforced;
    every response is capped at 8 MiB; and pagination stops at 1,000 pages or
    100,000 items. Every consumed JSON container/member is strictly decoded;
    missing/wrong-type fields fail closed while additional unconsumed fields do
    not widen the contract.
  - Workflow and Skill first request/response is page 0; a non-empty single-page
    fixture prevents a page-1 false-empty pass. Strict required-field,
    capability, visibility, ability, pagination, additive-field, timeout, and
    target-mismatch cases are mutation-tested with a fake server.
  - Output contains only manifest version, pass/fail, safe request IDs, and
    fixed probe categories. No URL/origin, credential, response body, payload,
    entity value, auth endpoint, or exception string is rendered or persisted.
  - If a local credential file is used, it is parsed as data and never executed:
    no `source`, `.`, `eval`, `env $(...)`, or equivalent shell evaluation. The
    loader accepts only documented `CODEMIE_*` keys plus
    `CODEMIE_TEST_PROJECT`; rejects duplicate, unknown, malformed, or multiline
    names/records; and never prints values.
  - Before loading, the harness requires `.env` to be an ignored, untracked,
    owner-only regular file with no symlink traversal. It checks the resolved
    `CODEMIE_URL` equals the separately authorized HTTPS target without printing
    either value. CI examples never load `.env`.
  - The smoke manifest contains no credentials and requires named
    authorization, authorized actor, run ID, exact project, explicit bounded
    exclusive-writer confirmation, durable-record owner, run prefix, and
    explicit declaration paths/natural keys for exactly one Assistant, one
    Workflow, and one Skill. Its schema is closed: a Datasource member, path,
    kind, selector, or authorization exception is rejected before
    authentication or any other network access. The incomplete example must
    fail closed.
  - The read-only compatibility probe still covers the full pinned
    operation-applicable GET contract, including Datasource under IR-008. This
    read coverage cannot produce a Datasource declaration or enable a V-003
    Datasource write.
  - For the downstream V-003 runtime gate only, the probe strictly decodes
    `GET /v1/user.email` as the authenticated actor identifier in addition to
    `is_admin`, `is_maintainer`, `projects[].name`, and
    `projects[].is_project_admin`. It compares actor and role/project data in
    memory and persists only fixed binding pass/fail categories, never the
    actor value.
  - Fake-server tests assert that same-origin and cross-origin redirects are not
    followed, no Authorization header reaches either redirect target, no write
    method is sent, body/timeout/page/item limits stop safely, strict JSON faults
    fail closed, and token/body/exception canaries are absent from stdout,
    stderr, files, and captured harness evidence. Manifest mutation tests
    additionally prove any Datasource member/kind and any missing
    Assistant/Workflow/Skill member fail with zero network requests.
- Validation expectations:
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_v000_target`
  - fake-server negative and zero-write request-count suite
  - `git diff --check`
- Completion: the harness is independently verified locally; V-000 remains
  incomplete until V-000B passes against the actual target.

### V-000B — Execute target/source qualification

- Eligibility: `DEPLOYMENT-VERIFICATION`.
- Lifecycle: **EXTERNAL TARGET EVIDENCE REQUIRED**.
- Objective: execute V-000A against the named deployment before any smoke write.
- Dependencies: V-000A, authorized target and project, and valid credentials.
- Acceptance evidence: sanitized non-production record contains the fixed
  non-secret staged-binary SHA-256, manifest version, pass/fail, safe request
  IDs, and page-0 observations only; no body, payload, entity, origin, or
  secret values. All applicable read-only probes pass, including the pinned
  Datasource GET contract, with a non-empty single-page Workflow and Skill
  target observation. The record is valid only for the exact target, project,
  authenticated principal/session, and staged-binary digest qualified by that
  execution. A mismatch blocks all V-003 writes and escalates to
  verification/release owners.
- Completion: no breaking mismatch for the named target; this completes V-000
  but not O-001, O-002B, or release authorization.

### V-003 — Execute authorized enterprise create/update smoke

- Eligibility: `DEPLOYMENT-VERIFICATION`.
- Lifecycle: **READY AFTER O-002A/V-000B; LIVE WRITES NOT YET AUTHORIZED BY THIS
  TASK BREAKDOWN**.
- Objective: prove serial create then update behavior in the authorized
  enterprise test project without conflating it with production activation.
- Requirements: FR-005/006/008/011/012/016/033/034, PA-003/005/006,
  QR-002/007/009–011.
- Dependencies: O-002A, V-000B, completed/verified R-001, user-confirmed
  `CODEMIE_TEST_PROJECT`, and named platform authorization for the actor and
  exclusive writer window.
- Scope and acceptance evidence:
  - The configured target must equal `https://codemie.lab.epam.com/`; the
    harness verifies this internally and never prints the configured or expected
    origin. A different target fails before network access.
  - The staged binary SHA-256 verified before V-000A login is persisted in the
    V-000B record. V-003 evidence contains that same digest (or its immutable
    evidence-record reference), and the exact unchanged binary is rechecked
    immediately before every V-003 apply. A mismatch fails before
    authentication when first detected and before the next write if it changes
    during execution.
  - Offline lint and authentication pass first. V-003 then reruns the complete
    non-mutating V-000B probe in the same controlled execution with the same
    token/session that will perform apply. A standalone, stale,
    differently-targeted, differently-project-scoped, differently-principaled,
    or differently-digested V-000B record cannot satisfy this pre-write gate.
  - After that fresh V-000B pass and before the first possible modifying call,
    the harness strictly decodes `GET /v1/user.email` as the authenticated
    actor plus the role/project fields and proves exact equality across
    `authorization.project`,
    `CODEMIE_TEST_PROJECT`, every concrete declaration's resolved effective
    project, and the exact `projects[].name` entry used for role evidence. The
    authenticated actor equals `authorization.actor`; the exact project entry
    exists; and the role is global administrator/maintainer or
    `is_project_admin=true` on that same entry. Another accessible project's
    admin role is insufficient. Any mismatch yields zero writes.
  - The authorization record explicitly names a bounded
    `exclusiveWriter` confirmation with `confirmed=true`, confirmer, start/end
    times, and the exact run prefix. The current time and whole smoke sequence
    must be within that window, and the confirmer attests no other writer can
    use the prefix. Missing, false, expired, future, differently scoped, or
    non-covering confirmation yields zero writes.
  - All intended natural keys are explicit, reviewed, run-scoped, and proven
    absent before the first write. A collision aborts the entire smoke before
    any entity is modified.
  - The executable scope is exactly Assistant, Workflow, and Skill. The
    manifest requires one declaration for each and rejects Datasource or any
    other entity member, kind, selector, path, or exception locally before
    authentication/network access. One serialized apply per entity reports
    `created`, followed by one serialized repeat apply of the same declaration
    reporting `updated`. No parallel writes occur.
  - No failure causes blind retry, automatic delete, rollback, tie-break, or
    cleanup. Any may-have-committed/uncertain result stops the sequence, holds
    writers, and requires complete-visibility inventory plus manual resolution
    before another invocation.
  - Evidence retains only the staged-binary SHA-256 (or immutable record
    reference), approved CLI action/kind/project/natural-key outcomes, and
    sanitized binding/window/harness pass/fail/request IDs. Credentials,
    `.env`, actor value, bodies, URLs, payloads, integrations, and declaration
    values are not logged or persisted. Created test entities are intentionally
    not auto-deleted; their disposition is assigned to the named manual owner.
  - Negative harness/fake-server tests cover mismatched declaration project,
    mixed-project declarations, other-project admin role, actor mismatch,
    stale/different-principal V-000B evidence, inactive/non-covering writer
    confirmation, missing Assistant/Workflow/Skill, Datasource presence, and
    staged-binary checksum mutation. Every case proves zero modifying requests.
- Completion: authorized target smoke passes for all three required entity
  kinds. It remains deployment-verification evidence only and does not complete O-001,
  O-002B, V-001, or release authorization.

### V-001 — Post-implementation convergence verification

- Eligibility: `PRODUCTION-ENABLEMENT`.
- Objective: prove code/tests/docs converge on v29 and architecture.
- Requirements: all active v29 requirements and all acceptance criteria.
- Dependencies: O-002, V-000.
- Acceptance evidence: full requirement/acceptance trace, request capture,
  schema/stream/no-leak tests, protected/reference-tree integrity proof. The
  new report must explicitly supersede the stale one-indexed Workflow/Skill
  statement in `Q-007-post-implementation-verification.md` and verify initial,
  post-write, and Skill 409 scans start at page 0.
- Completion: zero blocking findings.

### V-002 — Post-implementation security review

- Eligibility: `PRODUCTION-ENABLEMENT`.
- Objective: independently verify implemented security boundaries including
  post-implementation coverage of SEC-001 through SEC-006 remediation.
- Requirements: FR-009/011/014/016/024/026, QR-007, VR-011/012.
- Dependencies: V-001.
- Acceptance evidence:
  - Code/config/dependency review and dynamic canary/fault suite; critical/high
    findings closed.
  - Supply chain review: Cargo.lock is committed; builds use `--locked`;
    RustSec advisory scan shows no unresolved vulnerabilities; dependency
    license inventory is current; CI action references are SHA-pinned.
  - SEC-001: confirm no `--token`/`--client-secret`/`--password` flag paths in
    compiled binary; env-only credential evidence.
  - SEC-002: URL validation, HTTPS enforcement, redirect disable for auth
    POSTs verified in code.
  - SEC-003: resource budget enforcement verified in code for all 18 dimensions.
  - SEC-005: identifier schema constraint verification and output rendering
    injection test suite.
  - SEC-006: CI controls (permissions blocks, secret isolation, protected
    environments) verified in CI configuration.
- Completion: security approval for release assessment.

### L-001 — Prepare reproducible release evidence

- Eligibility: `PRODUCTION-ENABLEMENT`.
- Objective: prove a self-contained promotable/rollback artifact.
- Requirements: QR-002/006/008.
- Dependencies: V-002.
- Acceptance evidence:
  - Locked (`--locked`) clean Linux x86_64 build.
  - Full test and lint pass.
  - Linkage verification (static or bounded dynamic).
  - Checksum (SHA-256 or SHA-512) and optional GPG/sigstore signature on release
    artifact.
  - SBOM (CycloneDX or SPDX) generated from `cargo cyclonedx` or equivalent;
    attached to the release.
  - Provenance attestation where the CI platform supports it (e.g., SLSA
    level 2+).
  - Smoke test against non-production target.
  - Same-artifact promotion: the artifact tested in staging is identical (by
    checksum) to the artifact released to production.
  - Prior-binary rollback drill documented.
  - Secret scanning confirms release artifact and repository contain no
    committed secrets.
- Completion: release engineer can assess; no release action is authorized.

## 7. Trace and completion audit

| Active requirement group | Implementation owner | Evidence owner |
|---|---|---|
| FR-001–004, FR-022/023/025/027 | F-003/F-004/F-005 | Q-001, V-001 |
| FR-005/006/008/012/015/021 | F-006, A-001/W-001/S-001/D-001, R-001 | Q-001–Q-003, V-001 |
| FR-009/011/014/016/024/026 | F-002/F-004/F-005/F-007/T-001/T-002 | Q-005, V-002 |
| FR-017/019/020 | F-002 | Q-004/Q-006, V-001 |
| FR-028–030/032–035 | W-001/W-002/R-001 | Q-002/Q-008, O-001, V-001 |
| FR-031–034 | S-001/R-001 | Q-002/Q-008, O-001, V-001 |
| FR-036 | D-001/R-001 | Q-003, V-000/V-001 |
| IR-011/012 | T-003/R-001 | Q-007, V-000/V-001 |
| IR-006, QR-012 | O-002 | V-001/V-002 |

### Exact acceptance-criterion ownership

| Acceptance criterion | Implementation evidence task |
|---|---|
| AC-FR-001-01 | F-004 |
| AC-FR-003-01 | F-004 |
| AC-FR-004-01 | F-005 |
| AC-FR-005-01 | A-001/W-001/S-001/D-001, R-001 |
| AC-FR-005-02 | A-001/W-001/S-001/D-001, R-001 |
| AC-FR-006-01 | F-006, R-001 |
| AC-FR-008-01 | R-001 |
| AC-FR-009-01 | F-002, T-001 |
| AC-FR-011-01 | F-007, T-002, R-001 |
| AC-FR-011-02 | F-002/F-004/F-005/F-007 |
| AC-FR-011-03 | W-001/S-001/D-001, R-001 |
| AC-FR-011-04 | T-002/T-003, W-001/S-001 |
| AC-FR-014-01 | F-004, F-005, F-007 |
| AC-QR-007-01 | F-007, T-002 |
| AC-FR-015-01 | A-001/W-001/S-001/D-001 |
| AC-FR-021-01 | F-006, A-001/W-001/S-001/D-001 |
| AC-FR-021-02 | F-004, F-006 |
| AC-FR-022-01 | F-004 |
| AC-FR-022-04 | F-004, F-006 |
| AC-FR-024-01 | T-001 |
| AC-FR-024-02 | T-001, F-007 |
| AC-FR-024-03 | T-001, R-001 |
| AC-FR-024-04 | T-001 |
| AC-FR-024-05 | T-001, F-007 |
| AC-FR-024-06 | F-002 |
| AC-FR-024-07 | F-002/T-001 |
| AC-FR-024-08 | T-001 |
| AC-FR-017-01 | F-002/T-001 |
| AC-FR-017-02 | F-002/T-001 |
| AC-FR-025-01 | F-003/F-005 |
| AC-FR-025-02 | F-006/S-001 |
| AC-FR-025-03 | F-004 |
| AC-FR-026-01 | F-007 |
| AC-FR-026-02 | F-007 |
| AC-FR-026-03 | F-007 |
| AC-IR-008-01 | D-001/R-001 |
| AC-IR-011-01 | T-003/R-001 |
| AC-IR-012-01 | T-003/R-001 |
| AC-DR-010-01 | F-004/F-006/D-001 |
| AC-DR-010-02 | F-004 |
| AC-DR-011-01 | F-006/D-001/R-001 |
| AC-FR-035-01 | W-002 |
| AC-FR-035-02 | F-004/F-005/W-002 |
| AC-FR-027-01 | F-003 |
| AC-FR-028-01 | W-001 |
| AC-FR-028-02 | F-006/W-001 |
| AC-FR-029-01 | W-001 |
| AC-FR-029-02 | W-001/R-001 |
| AC-FR-030-01 | W-001 |
| AC-FR-030-02 | W-001 |
| AC-FR-031-01 | S-001 |
| AC-FR-031-02 | S-001 |
| AC-FR-033-01 | W-001/S-001/T-003 |
| AC-FR-034-01 | W-001/S-001/R-001 |
| AC-QR-010-01 | O-001/O-002 |
| AC-QR-012-01 | O-002 |
| AC-FR-022-02 | F-002/F-004 |
| AC-FR-022-03 | F-002/F-004 |

Completion requires successful Q-006 followed by Q-005, every task's acceptance evidence,
source-manifest conformance, active production controls, strict success/failure
schemas, and preserved approved-specification and reference-tree integrity.
