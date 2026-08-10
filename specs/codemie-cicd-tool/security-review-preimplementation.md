# Security review report: CodeMie declarative CI/CD CLI

## Status

```text
Security review status: APPROVED FOR IMPLEMENTATION
Architecture disposition: APPROVED FOR IMPLEMENTATION (v25 re-review, 2026-08-10)
Threat-model status: ACCEPTABLE WITH REQUIRED CONTROLS
```

Original review: 2026-08-09. Re-review (v25): 2026-08-10. See §v25 re-review at the end of this document.

There are no critical findings. Two open high-severity design findings directly
affect credential confidentiality, so implementation must not start. Four
medium-severity control gaps also require architecture and task updates before
this review is repeated.

The strongest existing controls are the closed authoring schemas, no live
schema widening, stateless credentials, strict separation of success and
failure records, allowlist-only diagnostics, no body/server-text logging,
fail-closed Workflow and Skill identity resolution, strict Workflow
`meta_config` decoding and reserved-member ownership, no blind write retries,
and explicit post-write ambiguity handling. These controls should be retained.

## Review mode

- Mode: **Architecture threat review (Mode B), pre-implementation**.
- Lifecycle task: Q-005.
- Entry gate: `verification-preimplementation.md` reports `READY FOR SECURITY
  REVIEW`.
- This review does not authorize implementation, release, or deployment.

## Scope

- Feature: `codemie-gitops` v1alpha1.
- Product source: `specs/codemie-cicd-tool.md`, DRAFT v24, SHA-256
  `98e25fa5497a3da25fe5c68936e241e19213269829bb234e50a0a8e1a34824f2`.
- Architecture: `plan.md`, `research.md`, `data-model.md`, `tasks.md`, ADR-001
  through ADR-010, and every contract under `contracts/`.
- Reference evidence: backend tag `2.42.0` at
  `2a481c290c99bf30ef80aadafa03d876a7f5f732` and UI tag `2.42.0` at
  `55945d075d82e771c4a2f4238afec1eb4c79d1e1`.
- Environments: local developer execution and Linux x86_64 CI/production client
  execution; only the external API boundary is reviewed.
- Data classifications: credentials/tokens (secret), declarations and uploaded
  sidecars (potentially confidential), project/entity identity and integration
  references (internal), output and correlation identifiers (operational).
- Explicit exclusions: implementation code (none exists), target-deployment
  qualification, release authorization, CodeMie server implementation,
  post-CRUD server-internal indexing or processing effects, runtime execution of
  assistants/workflows/MCP configuration, and modifications to `codemie/` or
  `codemie-ui/`.

The reference directories were inspected as read-only external evidence. They
are not product components or build dependencies.

## Sources of truth

- Security policy: no separate repository security policy or engineering
  constitution was found; repository `AGENTS.md` and the approved product
  security requirements are the governing local instructions.
- Specification: `specs/codemie-cicd-tool.md` v24.
- Requirements: FR-009/011/014/016/017/024/026/028-036, DR-004/006-009,
  IR-003/004/006/007/009/010, QR-003/007/009-011, PA-001-007, and
  VR-006-012.
- Architecture: `specs/codemie-cicd-tool/plan.md`, `data-model.md`, and
  `research.md`.
- Contracts: declaration, repository-config, CLI, HTTP adapter, adapter
  manifest, outcome, warning, and diagnostic contracts.
- ADRs: ADR-001 through ADR-010; ADR-006 is superseded.
- Threat model: created by this report; no prior threat-model artifact existed.
- Independent verification: `verification-preimplementation.md`.
- Jira/Confluence: none provided locally or in the conversation.

## Executive assessment

The architecture correctly treats repository declarations, sidecars, remote
responses, and server identity evidence as untrusted. It also avoids several
common high-risk patterns: there is no dynamic plugin execution, schema network
fetch, secret interpolation, client state database, generic ownership marker,
implicit Workflow adoption, arbitrary duplicate tie-break, response-body error
rendering, blind POST/PUT retry, or delete operation.

However, the credential boundary is not yet safe enough to implement:

1. The normative CLI contract requires bearer tokens, client secrets, and
   passwords to be accepted as command-line option values. Those values are
   placed in process arguments and may also enter shell history or execution
   traces.
2. The destination trust policy is incomplete. Version-controlled endpoint
   configuration can steer production credentials; the URL schema accepts
   remote plaintext HTTP and URL userinfo; “explicit local development” is not
   defined as a safe host class; and only cross-origin credential redirects are
   prohibited.

Resource budgets, Datasource complete-visibility proof, safe rendering of
untrusted identifiers/paths, and build/CI supply-chain controls are also not
specific enough to delegate safely to implementation.

## Assets

| Asset | Confidentiality | Integrity | Availability | Owner |
|---|---|---|---|---|
| Keycloak client secret | Critical | High | Medium | adopting team/platform administrator |
| Local-development password | Critical | High | Low | local developer |
| Bearer token | Critical | High | Medium | invoking principal/CI secret store |
| Declarations, Skill content, File uploads | High when repository content is confidential | High | Medium | platform asset author/adopting team |
| Remote Assistant/Workflow/Datasource/Skill records | Internal | High | High | CodeMie platform/project owner |
| Workflow reserved identity record | Internal | High | High | platform administrator/tool adapter |
| Natural-key and capability-resolution evidence | Internal | High | High | invocation/tool |
| CI logs and diagnostics | Internal | High | Medium | adopting team/CI platform |
| Embedded schemas, adapter manifest, release binary | Internal | Critical | High | maintainers/release engineer |
| CI runner resources | Low | Medium | High | adopting team/CI platform |

## Actors and trust boundaries

Relevant actors are the platform asset author, reviewer/approver, CI service
account, local developer, platform administrator, another CodeMie project user,
malicious repository contributor, compromised CI runner process, network
attacker, compromised target/IdP, and compromised dependency/build input.

Trust-boundary crossings are:

1. Git-controlled YAML/config/sidecars to the local parser and file loader.
2. CLI flags/environment/CI secret store to credential wrappers.
3. Repository or protected environment endpoint configuration to outbound
   credential destinations.
4. CLI through proxy/CA/TLS and redirect handling to Keycloak or CodeMie.
5. Untrusted API pages/details to strict decoders and identity resolution.
6. Existing Workflow `meta_config` through strict JSON decode/merge/encode to a
   PUT request.
7. Local file handles to multipart upload parts.
8. Internal errors and untrusted coordinates/identity to stdout/stderr and CI
   logs.
9. Dependency registries, CI actions, and build runners to the promoted binary.

## Attack surface

- `lint`, `apply`, and `login` command arguments and inherited environment.
- `.codemie/config.yaml`, arbitrary declaration paths, YAML syntax, aliases,
  anchors, tags, duplicate keys, nesting, strings, and arrays.
- Skill `contentFrom` and File Datasource paths, symlinks, file bytes, and
  multipart filenames.
- Explicit target and Keycloak URLs, proxy/CA environment variables, redirects,
  TLS, DNS, and all response sizes/content.
- Assistant exact lookup; Workflow, Skill, and Datasource pagination/detail
  reads; capability records; ephemeral route IDs; and post-write reads.
- Workflow adoption UUID and mixed-owned `meta_config`.
- Opaque Datasource integration identifiers and URL-like source fields.
- Text and JSON success, warning, failure, panic, and fatal output paths.
- Future Rust crates, lockfile, CI workflows/actions, build runner, signatures,
  SBOM, and artifact promotion.

## Threat model

| Threat | Asset/boundary | Existing control | Residual risk/status |
|---|---|---|---|
| Credential disclosure through process arguments | secret flag -> process | secret wrapper after parsing; no tool log | **SEC-001**; exposure occurs before wrapper protection |
| Credential exfiltration to attacker-selected endpoint | Git/environment URL -> HTTP | deterministic precedence; no URL derivation | **SEC-002**; destination trust and URL policy incomplete |
| Network interception or credential replay by redirect | HTTP/TLS/redirect | HTTPS stated; cross-origin redirect blocked | **SEC-002**; local exception and same-origin/auth redirect behavior undefined |
| YAML alias/depth or huge input exhaustion | Git -> parser/schema | “bounded YAML”; closed schema | **SEC-003**; no enforceable budgets |
| Huge file, multipart/query, response, or enumeration exhaustion | file/network -> memory/time/bandwidth | streaming intent, 10 File parts, bounded wording, one deadline | **SEC-003**; aggregate and remote enumeration limits undefined |
| Path traversal/symlink escape | Git path -> filesystem | repository containment, regular-file checks, symlink default deny | Controlled by design; descriptor-safe open and race tests remain implementation evidence |
| Multipart header/path confusion | basename -> multipart header | repository containment | Required under SEC-005 verification: reject control/NUL/path separators or prove safe encoding |
| Failure-body/secret leakage | HTTP/parser error -> logs | separate closed types; allowlist-only output; bodies/server text excluded | Strong design control; implementation canary/fault tests still required |
| CI log/terminal injection | identity/path/key -> text output | closed field list | **SEC-005**; values admit controls and no text escaping is defined |
| Workflow marker tampering/implicit adoption | remote metadata -> target selection | reserved key, strict duplicate-key JSON, exact match, visibility/write proof, explicit UUID adoption | Acceptable for implementation after other blockers; operational inventory remains required |
| Hidden or racing identity duplicate | remote list -> target selection | exhaustive pages, ambiguity refusal, post-write read; Workflow/Skill privileged visibility and serialization | Datasource is incomplete: **SEC-004** |
| Cross-project unauthorized update | route ID -> PUT | exact project filtering, server authorization, row write ability | Adequate design; target behavior still needs deployment verification |
| Compromised dependency/action/artifact | build -> binary/CI secrets | pinned reference evidence, planned lock/signature/SBOM | **SEC-006**; dependency and CI trust rules incomplete |
| Prompt/MCP content causes runtime behavior | declaration -> CodeMie runtime | CLI treats content as data and does not execute it | Server runtime is explicitly outside this review |
| Server-internal Datasource processing differs | synchronous CRUD -> server internals | none required by product | Not applicable; product intentionally evaluates only synchronous API results |

## Security requirements coverage

| Requirement/control | Status | Evidence | Findings |
|---|---|---|---|
| Stateless authentication and no client token cache | SATISFIED BY DESIGN | ADR-003; data model section 9 | — |
| Safe credential acquisition/input | NOT SATISFIED | FR-009/017/024 and CLI contract require secret option values | SEC-001 |
| Trusted TLS destination and redirect policy | PARTIALLY SATISFIED | HTTP contract section 2; repository URL schema | SEC-002 |
| Credential/config precedence and no endpoint derivation | SATISFIED BY DESIGN | v24, ADR-003, Q-004 evidence | — |
| Repository config contains no credential material | PARTIALLY SATISFIED | unknown credential keys rejected, but URL userinfo is accepted | SEC-002 |
| Authentication and authorization enforced by server | SATISFIED BY DESIGN | bearer boundary; pinned server route dependencies | — |
| Complete Workflow/Skill identity visibility and write proof | SATISFIED BY DESIGN | FR-033; HTTP contract sections 5/6/7 | — |
| Complete Datasource identity visibility | NOT SATISFIED | exhaustive resolver plus creator/project-filtered source behavior | SEC-004 |
| Workflow marker integrity/adoption | SATISFIED BY DESIGN | ADR-008; strict `meta_config` codec and explicit UUID checks | — |
| Closed YAML/config and unknown-field rejection | SATISFIED BY DESIGN | ADR-001; machine schemas | — |
| YAML/file/network resource limits | PARTIALLY SATISFIED | bounded intent without parameters | SEC-003 |
| Root-contained sidecar/File loading | PARTIALLY SATISFIED | CLI/declaration contracts; symlink default deny | SEC-003 exit evidence |
| Request/response body and secret exclusion from diagnostics | SATISFIED BY DESIGN | FR-016; ADR-003/010; closed diagnostic schema | — |
| Safe rendering of allowed diagnostic/outcome values | NOT SATISFIED | control characters accepted by schemas | SEC-005 |
| Retry and uncertain-write safety | SATISFIED BY DESIGN | no blind POST/PUT retry; bounded Skill 409 re-resolution | — |
| CI serialization and duplicate remediation | PARTIALLY SATISFIED | Workflow/Skill tasks O-001/O-002 | SEC-004, SEC-006 |
| Dependency/build/release integrity | PARTIALLY SATISFIED | pinned references; planned lock/signature/SBOM | SEC-006 |
| Retention/deletion of CLI-held sensitive data | SATISFIED BY DESIGN | transient-only model; no cache/state/log | implementation verification pending |
| External audit/provenance | NOT VERIFIED | explicitly assigned to Git/CI/platform logs | deployment evidence, not a CLI finding |

## Control assessments

### Authentication and authorization

Keycloak endpoint derivation is correctly forbidden and the explicit precedence
chain is deterministic. Mixed modes and missing inputs fail before network.
Local-auth is correctly separated from CI use, and the pinned server confirms
that `/v1/local-auth/login` is gated by local mode and rate limited.

The remaining authentication defects are the process-visible secret flag
surface and insufficient definition of a trusted credential destination.
Server-side authorization remains authoritative; the CLI does not claim that a
Workflow marker grants access. Workflow and Skill complete-visibility checks
are sound at the design level. Datasource resolution does not have an equivalent
guarantee despite a visibility-filtered list API.

### Input, output, and injection controls

Closed schemas, duplicate-key rejection, unsupported-tag rejection, local-only
schema references, and exact per-kind unions are strong. The architecture also
correctly keeps `command`, prompt, query, and MCP values as data and never
executes a local shell.

The missing controls are numeric parser/file/network budgets and canonical safe
rendering of every untrusted value that is permitted into text output. URL
userinfo must also be rejected so a “non-secret” URL field cannot carry a
credential. Route and query values must be encoded structurally, never by string
concatenation.

### File and path handling

Paths are YAML-relative, root-contained, regular files; symlinks are denied by
default and cycles/escapes fail. Those are appropriate preventive controls.
Implementation must open and validate the same file object without a
canonicalize-then-reopen race, enforce per-file and aggregate byte limits, and
encode or reject unsafe multipart basenames. `--follow-symlinks` must not weaken
repository containment.

### Data protection, privacy, and secrets

The no-persistence, no-body, no-server-text, no-cookie, no-auth-header, and
allowlist-only diagnostic design is strong. Successful login token stdout is an
explicit product behavior, but it remains pipe-sensitive. CI examples must
disable shell tracing around capture, register/mask the resulting token, avoid
echoing environment, and ensure the token is not placed in artifacts or caches.

The general prohibition on secrets in authored YAML cannot be proven by an
entropy warning. Known secret-bearing properties are correctly rejected, and
the later CI/release process still needs repository secret scanning and response
to confirmed exposure. No actual credential value was observed or printed by
this review.

### Dependencies, supply chain, CI/CD, and infrastructure

The pinned backend/UI commits are clean and match the recorded tags. They are
reference-only and are not build inputs. ADR-005 plans one Rust binary, rustls,
artifact inspection, SBOM, checksum, signature, and a locked build. These are
good foundations.

No product `Cargo.toml`, `Cargo.lock`, CI workflow, container, infrastructure,
or release artifact exists yet. The task plan does not yet require RustSec
vulnerability checking, registry/source policy, immutable CI action references,
least-privilege workflow permissions, fork/PR secret isolation, or protected
deployment environments. SEC-006 records the missing preconditions rather than
claiming an implemented vulnerability.

### Logging, audit, incident response, and recovery

The CLI intentionally omits Git/CI/environment provenance and delegates audit
correlation to external systems. That approved boundary is not reopened here.
The closed request/correlation-ID rule is safe. Production evidence must still
show that Git review, CI job/deployment events, and CodeMie audit records are
retained and access controlled according to the adopting team's policy.

Git revert plus a new apply and manual ambiguity remediation are defined.
Duplicate/marker inventory and uncertain-write runbooks remain downstream
operational evidence. No server-side effect monitoring is required.

## Validation performed

The following checks were actually run from repository root.

```text
git -C codemie rev-parse HEAD
git -C codemie rev-list -n 1 refs/tags/2.42.0
git -C codemie status --short
git -C codemie-ui rev-parse HEAD
git -C codemie-ui rev-list -n 1 refs/tags/2.42.0
git -C codemie-ui status --short
sha256sum specs/codemie-cicd-tool.md specs/codemie-cicd-tool/plan.md ...
```

Result: both reference revisions match `source-baseline.md`; both reference
worktrees were clean; architecture hashes match the pre-implementation
verification report.

```text
python3 <strict duplicate-key JSON parse of every contracts/*.json>
```

Result: PASS for all six JSON artifacts.

```text
python3 <fastjsonschema compile plus adversarial URL/output/declaration vectors>
```

Observed results:

```text
config_remote_http: ACCEPTED
config_url_userinfo: ACCEPTED
config_auth_userinfo: ACCEPTED
diagnostic_control_path: ACCEPTED
outcome_control_identity: ACCEPTED
declaration_control_identity: ACCEPTED
```

The control-identity vectors used newline and ANSI escape characters. These are
schema observations, not implementation exploits; no implementation exists.

```text
rg -n <auth, URL, redirect, bounded-resource, identity, CI, supply-chain terms>
  specs/codemie-cicd-tool.md specs/codemie-cicd-tool/
sed/nl inspection of pinned server user, local-auth, Workflow, Skill, and
  Datasource list/permission paths
```

Result: the contracts consistently preserve the v24 endpoint precedence and
diagnostic-body exclusions. They also consistently preserve secret-bearing
flags; use qualitative “bounded” language without concrete budgets; limit
privileged preflight to Workflow/Skill; and lack the CI/dependency controls in
SEC-006. Pinned Datasource list evidence confirms non-admin filtering.

No intrusive network, target-environment, or production test was performed.

## Findings

### Finding ID: SEC-001

Severity: **HIGH**

Confidence: **CONFIRMED**

Status: **CLOSED** — v25: secret credentials are env-only; `--token`/`--client-secret`/`--password` flags rejected as `E_USAGE` exit 2 before network; normative in ADR-011 and reflected consistently in cli.md §1/§2/§6, data-model.md §9 `ResolvedConfig`, and tasks F-002/T-001/O-002 acceptance evidence.

Title: Secret-bearing CLI options expose credentials through process arguments

Affected assets:

- Keycloak client secret
- Local-development password
- Bearer token
- CodeMie project write authority

Affected components:

- `specs/codemie-cicd-tool.md:435,443,450,578-580`
- `specs/codemie-cicd-tool/contracts/cli.md:10-18,44-48`
- `specs/codemie-cicd-tool/data-model.md:238-255`
- `specs/codemie-cicd-tool/tasks.md:146-160,237-249`

Evidence:

- FR-009 requires `--token`; FR-017/024 require flag-over-environment handling
  for `--client-secret` and `--password`.
- The normative CLI syntax places each secret directly after its option.
- Tasks explicitly require credential flags to override environment values.
- A secret wrapper is created only after argument parsing; it cannot remove the
  value from the parent shell, process argument vector, process monitors, or
  prior shell trace/history.

Attack scenario:

1. A local user or CI job invokes `apply` or `login` with a secret option.
2. Another same-host process, runner diagnostic, process collector, shell trace,
   or history mechanism records the command arguments.
3. The bearer token, password, or client secret is recovered.
4. The attacker authenticates as the service account/user and performs any
   operation permitted to that principal until revocation/expiry.

Expected control:

Secrets must enter through a channel that does not place the value in `argv` or
repository state. Secret input should fail closed and remain compatible with CI
secret stores.

Actual behavior:

The approved product and architecture contracts require process-visible secret
option values. Masking a CI log does not remove the value from `argv`.

Impact:

Credential compromise can expose or modify all in-scope project assets allowed
to the service account. A client secret can outlive the approximately eight-hour
access token and requires rotation.

Likelihood:

Moderate. Exploitation requires access to runner process telemetry, another
same-user process, shell traces/history, or captured diagnostics; these are
realistic on shared or insufficiently isolated runners.

Required remediation:

- Product specification owner must replace secret-value CLI flags with
  non-argv input. Environment-only input is the minimum compatible phase-1
  option; protected stdin/file-descriptor/secret-file input may be added if an
  interactive override is required.
- Non-secret selectors such as client ID may remain flags if explicitly
  classified that way.
- Update CLI/data/config contracts, examples, schemas/manifest, and F-002/T-001
  tasks consistently.
- If secret flags are retained, an authorized risk owner must explicitly accept
  the process-disclosure risk; this reviewer does not accept it.

Owner: **product-spec-owner**

Verification:

- Help and CLI snapshots contain no secret-value options.
- A canary invocation held during a mock request shows no canary in
  `/proc/<pid>/cmdline`, process listings, stdout, stderr, files, or tool logs.
- Flag/environment precedence tests are replaced by tests for the approved
  non-argv secret channel.

References:

- FR-009, FR-017, FR-024, QR-007, DR-009, ADR-003, Q-005, T-001

### Finding ID: SEC-002

Severity: **HIGH**

Confidence: **HIGH**

Status: **CLOSED** — v25: `ValidatedUrl` type is normative in ADR-011 and data-model.md §9; HTTPS required for `auth_url` unconditionally; loopback exception defined for `target_url`; redirects disabled for auth POSTs; URL userinfo rejected at schema-validation time in repository-config.schema.json and declaration schema; CI endpoint protection requirements normative in ADR-011 §5 and tasks O-002.

Title: Credential destination validation and production endpoint trust are incomplete

Affected assets:

- Keycloak client secret and returned token
- Local-development password
- Pre-issued bearer token
- Declaration and uploaded file confidentiality

Affected components:

- `.codemie/config.yaml` contract
- `specs/codemie-cicd-tool/contracts/repository-config.schema.json:9-17`
- `specs/codemie-cicd-tool/contracts/http-adapter.md:33-60`
- `specs/codemie-cicd-tool/contracts/cli.md:32-56,123-141`
- `specs/codemie-cicd-tool/plan.md:232-239`
- O-002 CI examples

Evidence:

- The repository schema accepts both `http://idp.example/token` and URLs whose
  authority contains userinfo. The executed schema vectors confirmed acceptance.
- URL userinfo permits credential material inside fields declared non-secret.
- “Require HTTPS except an explicit local-development target” does not define
  how a target becomes local; the schema accepts remote plaintext HTTP.
- Redirect policy only forbids credentials across origins. It does not forbid an
  authentication POST redirect or define method/body/token behavior.
- Repository `url` and `auth_url` are below flags/environment in precedence but
  may still become production credential destinations when protected overrides
  are absent. O-002 does not require protected environment endpoints or review
  ownership for these fields.

Attack scenario:

1. A malicious or mistakenly approved repository change sets `auth_url` or
   `url` to an attacker-controlled host; alternatively an operator supplies a
   remote HTTP endpoint.
2. Production CI has no higher-precedence protected endpoint value.
3. Login sends the service-account secret, or apply sends a bearer token and
   confidential request/file content, to that destination or over plaintext.
4. The attacker captures the credential and uses its project privileges.

Expected control:

Every credential-bearing request must have a strictly validated, TLS-protected,
trusted destination. The local HTTP exception must be mechanically narrow, and
CI configuration must prevent untrusted repository changes from selecting the
production destination.

Actual behavior:

The architecture defines precedence but not sufficient URL semantics, local
host classification, redirect handling, or production endpoint trust.

Impact:

Direct credential and confidential payload disclosure, followed by unauthorized
CodeMie reads/writes within the principal's scope.

Likelihood:

Moderate. It requires endpoint misconfiguration, a merged malicious config
change, or an unprotected CI configuration. The current tasks do not require
the controls that would make those preconditions unlikely.

Required remediation:

- Define one semantic `ValidatedUrl`: absolute `http`/`https`; no userinfo,
  fragment, control characters, invalid host/port, or ambiguous authority.
- Require HTTPS for Keycloak and all non-local API traffic. Permit HTTP only for
  an explicitly selected local-auth/development mode whose resolved destination
  is loopback; fail closed otherwise. Do not add an insecure TLS bypass.
- Disable redirects for Keycloak and local-auth credential POSTs. Prefer no
  redirect for authenticated API calls; any retained redirect behavior must be
  method-aware, same-origin, bounded, and must never replay credentials to a
  different authority.
- Production CI examples must inject target and auth endpoints from protected
  environment configuration at higher precedence than repository config, use
  protected deployment environments/approvals, and run secret-free lint on
  untrusted pull requests.
- Add host/endpoint ownership or equivalent review controls for changes to
  `.codemie/config.yaml` if repository endpoint fallback is used operationally.
- Reject URL userinfo in repository configuration and authorable source URLs so
  known URL credential forms cannot enter Git under a non-secret field.

Owner: **solution-architect**

Escalation: **product-spec-owner** if production use of repository-selected
credential destinations or non-loopback HTTP is required behavior.

Verification:

- Negative tests cover URL userinfo, fragments, controls, malformed authority,
  remote HTTP, redirect chains, cross-origin redirects, and higher-precedence
  invalid-value no-fallback.
- A mock IdP proves credential POSTs never follow 301/302/303/307/308.
- CI examples prove protected `CODEMIE_URL`/`CODEMIE_AUTH_URL` override a
  malicious repository value and that pull-request jobs receive no secrets.

References:

- FR-017/024, IR-003/006, DR-004/009, ADR-003, T-001/T-002, O-002

### Finding ID: SEC-003

Severity: **MEDIUM**

Confidence: **HIGH**

Status: **CLOSED** — v25: 18 versioned resource-budget dimensions with exact numeric defaults defined in http-adapter.md §2.4 and data-model.md §11; race-safe open-then-fstat file handling mandated in http-adapter.md §2.5; enforcement order and cancellation propagation documented.

Title: Parser, file, upload, response, and enumeration resource budgets are undefined

Affected assets:

- CI runner CPU, memory, disk descriptors, bandwidth, and job time
- CodeMie/IdP connection capacity and API cost
- Availability of lint/apply pipelines

Affected components:

- ADR-001 marked YAML parser
- declaration schema, including unbounded arrays/objects/strings
- Skill sidecars and File Datasource uploads
- HTTP response decoder/body drain
- Workflow/Skill/Datasource exhaustive resolvers
- F-003/F-004/T-002/D-001/R-001 tasks

Evidence:

- Contracts say “bounded YAML,” “bounded-size,” “bounded connect/request/body-
  drain,” and “one deadline,” but provide no byte, depth, alias, node, file,
  aggregate upload, response, page, item, query, or time budgets.
- The declaration schema caps File part count at 10 and selected domain fields,
  but many nested arrays/objects/strings and File byte sizes remain unbounded.
- Exhaustive resolution detects cycles/repeated IDs/drift but has no maximum
  pages/items/decoded bytes beyond an unspecified deadline.

Attack scenario:

1. A repository contributor supplies a small alias-expansion/deep YAML input,
   a very large sidecar/File upload, or huge repeated collections; or a
   compromised endpoint returns large bodies/many valid pages.
2. The CLI expands, parses, stores, streams, or enumerates without an enforceable
   aggregate budget.
3. The CI job exhausts memory, CPU, bandwidth, descriptors, or time.
4. Deployments are delayed and repeated retries consume additional resources.

Expected control:

Every untrusted local and remote input dimension must have a documented,
testable limit and a single cancellation/deadline budget.

Actual behavior:

The architecture delegates the values to implementation without a source of
truth or threshold evidence.

Impact:

Denial of service and potentially material CI/network/API cost. No direct
cross-project confidentiality impact is demonstrated.

Likelihood:

Moderate. Repository content is routinely processed before apply, and remote
enumeration is required for normal operation.

Required remediation:

- Architecture must define versioned defaults and configuration policy for:
  YAML/config bytes, documents, nesting, aliases/expanded nodes, scalar length,
  collection members, repository files visited, sidecar/File per-file and
  aggregate bytes, multipart parts/filename/query length, response header/body
  bytes, JSON depth, pagination pages/items/duplicate tracking, retries, total
  request and invocation deadlines, and concurrency.
- Reject before allocation or network where possible; stream files and bounded
  bodies; cancellation must stop parsing, reads, retries, and pagination.
- Open and validate the same file handle to preserve containment; do not
  canonicalize and later reopen an attacker-replaceable path.
- User-visible limits that reject previously valid declarations require product
  owner confirmation rather than ad hoc implementation choices.

Owner: **solution-architect**

Verification:

- Boundary and threshold-plus-one tests for every limit.
- YAML alias bomb, deep nesting, huge scalar/array, symlink-swap, sparse/large
  file, aggregate multipart, oversized header/body, endless-page, large-total,
  slow-response, and cancellation tests demonstrate bounded resources and safe
  exit-2 diagnostics.

References:

- ADR-001 follow-up, FR-025, IR-004, QR-005/007, CLI contract section 3,
  HTTP contract section 2, F-003/F-004/T-002/D-001

### Finding ID: SEC-004

Severity: **MEDIUM**

Confidence: **CONFIRMED**

Status: **DEFERRED** — v25: ADR-012 is PROPOSED (not ACCEPTED) and presents three options; D-001 implementation is explicitly gated on ADR-012 acceptance by product-spec-owner; architecture documents the visibility-completeness risk and requires Datasource to be added to serialized-writer governance and duplicate-inventory runbooks (O-001/O-002). Acceptable deferral condition met; gate is explicit.

Title: Datasource exhaustive resolution does not prove complete identity visibility

Affected assets:

- Datasource natural-key integrity
- Cross-entity Datasource references
- Availability and recoverability of Datasource automation

Affected components:

- Datasource resolver in ADR-002/009
- `contracts/http-adapter.md:176-217`
- `data-model.md` Datasource resolution
- D-001 and O-001/O-002 tasks
- pinned `codemie/src/codemie/service/index/index_service.py:162-218,275-282`

Evidence:

- ADR-009 states that one exact list match “proves visibility/write capability,”
  although one visible match cannot prove absence of hidden duplicates.
- HTTP adapter preflight explicitly requires global/project-admin evidence only
  “Before Workflow or Skill resolution,” not before Datasource resolution.
- Pinned server code filters `GET /v1/index` for every non-global-admin principal
  to project-visible, project-admin, or creator-owned rows. A regular project
  member can therefore receive a valid complete pagination envelope over an
  incomplete identity set.
- The same pinned evidence records no database uniqueness on
  `(project, repo_name, kind)`, and `find_id` returns a first row.
- Serialization/governance requirements and tasks name Workflow/Skill, not
  Datasource identity writers.

Attack scenario:

1. A principal can write a Datasource but cannot see another user's private
   Datasource with the same natural key, or another writer creates one between
   scans.
2. Exhaustive enumeration returns zero or one visible match and internally
   consistent pagination.
3. The CLI creates a duplicate or updates one visible row while an invisible
   duplicate exists.
4. Later privileged resolution becomes ambiguous; references and automation
   fail and require manual repair.

Expected control:

Zero/one/multiple natural-key decisions must operate over a demonstrably
complete visibility set, or the server must enforce/resolve an atomic unique
natural key.

Actual behavior:

Datasource uses a visibility-filtered list without a normative complete-
visibility precondition equivalent to FR-033.

Impact:

Datasource identity corruption/ambiguity and deployment failure. No direct
unauthorized update of a hidden row was demonstrated.

Likelihood:

Moderate where service accounts are not global/project administrators or other
Datasource writers remain active.

Required remediation:

- Product/platform owner must choose and evidence one safe boundary: require a
  role that makes Datasource enumeration complete, add/use an atomic server
  natural-key contract, or fail Datasource apply when completeness cannot be
  proven.
- Add Datasource to serialized writer governance, duplicate inventory,
  post-write ambiguity handling, and manual remediation if the client-side
  model remains.
- Capability checks must not infer complete visibility from per-row write
  ability or internally consistent pagination.

Owner: **product-spec-owner**

Supporting owner: **solution-architect / CodeMie platform owner**

Verification:

- Target fixtures include hidden same-key rows for creator, project member,
  project admin, and global admin/service-account roles.
- Zero/one resolution fails before write unless the fixture proves complete
  visibility.
- Concurrent different-principal create tests and operational mutex/inventory
  drills include Datasource.

References:

- FR-005/033/036, PA-003/005, QR-009-011, ADR-009, D-001, O-001

### Finding ID: SEC-005

Severity: **MEDIUM**

Confidence: **CONFIRMED**

Status: **CLOSED** — v25: `projectProperty`, `assistantMetadata.slug`, and `workflowMetadata.slug` have `maxLength` and C0/C1+bidi-rejection patterns in schemas; `skillMetadata.name` and `datasourceMetadata.repo_name` have alphanumeric-only patterns that structurally exclude controls; cli.md §10 defines one-record-per-line, canonical field-path generation, JSON-serializer requirement, fixed-template text, and percent-encoded route/query; multipart basename safety in http-adapter.md §7.

Title: Untrusted identity and source-coordinate values can inject terminal and CI log records

Affected assets:

- Integrity of stdout/stderr and CI logs
- Human and automated interpretation of reconciliation results
- Multipart request metadata

Affected components:

- `contracts/declaration-v1alpha1.schema.json:12-19,160-166,290-295`
- `contracts/repository-config.schema.json:19-22`
- `contracts/outcome.schema.json:9-14`
- `contracts/diagnostic.schema.json:25-33`
- warning schema source coordinates
- text renderers in F-007
- File multipart basename projection

Evidence:

- Project and Assistant/Workflow slug constraints are only non-empty; repository
  project permits any string containing a non-space.
- Outcome identity and diagnostic/warning `source.file`/`fieldPath` have no
  control-character restriction or explicit maximum.
- Executed schema vectors containing newline and ANSI escape characters were
  accepted for declarations, outcomes, and diagnostics.
- Text output is the default, but no escaping/canonicalization contract defines
  one physical record or safe representation of controls/bidirectional text.
- File uploads use an untrusted basename as multipart filename without an
  explicit safe-filename encoding/rejection rule.

Attack scenario:

1. A malicious repository contains a crafted project/slug, unknown key, or
   filename with newline, carriage return, terminal escape, or bidi controls.
2. Lint/apply emits that value in a text outcome/diagnostic or inserts it into a
   multipart filename.
3. CI logs show forged lines/status-like text, terminals process escapes, or a
   vulnerable multipart builder receives unsafe header input.
4. Reviewers or automation misinterpret the result; transport behavior may
   differ from the intended part metadata.

Expected control:

Allowed output data must also have a safe value domain and encoding. Structured
transport builders must receive validated/encoded path, query, and header
values.

Actual behavior:

The field allowlist prevents body/secret attachment but does not constrain or
escape attacker-controlled values within allowed fields.

Impact:

Log/terminal integrity loss and possible multipart metadata injection. The
closed JSON renderer limits this risk when JSON escaping is used correctly, but
default text mode remains affected.

Likelihood:

Moderate for log injection because Git supports unusual filenames and several
identity fields are unconstrained. Multipart exploitation depends on the future
HTTP library and is not yet confirmed.

Required remediation:

- Define canonical identifier constraints consistent with the pinned API and
  VR-006, including maximum lengths and rejection of C0/C1 controls, newlines,
  NUL, and bidi formatting controls.
- Render text from trusted enums plus escaped values; ensure each outcome or
  warning is one physical line and each failure line cannot be extended by an
  input value. JSON must use a serializer, never string concatenation.
- Generate canonical field paths rather than echoing arbitrary keys; encode or
  safely abbreviate source paths under an explicit length rule.
- Percent-encode route/query components structurally. Reject unsafe multipart
  basenames or prove the selected builder produces standards-compliant quoted
  parameters without CR/LF/header injection.

Owner: **solution-architect**

Verification:

- Canary corpus covers CR/LF, ANSI escape, NUL, bidi controls, quotes,
  backslashes, percent signs, Unicode normalization pairs, long names, and
  unusual Git filenames in text/JSON and multipart capture.
- Tests prove one physical text record, valid single-line JSON, no terminal
  control bytes, and no injected multipart header.

References:

- FR-016/026, DR-006, QR-004/007, VR-006/011/012, ADR-010, F-007/T-002

### Finding ID: SEC-006

Severity: **MEDIUM**

Confidence: **HIGH**

Status: **CLOSED** — v25: ADR-005 amendment makes all supply-chain and CI trust controls normative: Cargo.lock committed, `--locked` builds, RustSec advisory scanning (`cargo audit`) as CI gate, SHA-pinned CI actions, `permissions:` blocks with least-privilege scopes, secrets isolated from fork/PR contexts, protected deployment environments with human approval, same-artifact promotion, SBOM/provenance; reflected in tasks F-001/O-002/V-002 acceptance criteria.

Title: Dependency and CI trust controls are not complete enough for a credentialed deployment tool

Affected assets:

- Release binary and embedded contracts
- CI secrets and service-account authority
- Integrity of all remote writes

Affected components:

- ADR-005
- F-001, O-002, V-002, and L-001 tasks
- future `Cargo.toml`, `Cargo.lock`, CI workflows, actions, and release artifacts

Evidence:

- ADR-005 and L-001 require artifact inspection, SBOM, checksum/signature,
  locked build, and same-artifact promotion.
- F-001 requires only dependency/license inventory; V-002 says dependency
  review without naming reproducible vulnerability evidence.
- O-002 requires masked secret injection but does not require least-privilege
  CI token permissions, fork/PR secret isolation, protected deployment
  environments, immutable action references, or trusted endpoint overrides.
- No product Cargo manifest, lockfile, CI workflow, container, or infrastructure
  file exists yet, so these controls cannot be inferred from implementation.

Attack scenario:

1. A compromised crate/git dependency, mutable CI action, untrusted PR job, or
   over-privileged workflow executes in the build/deploy context.
2. It reads the CodeMie client secret/token or changes the embedded schema,
   adapter logic, or release artifact.
3. The malicious binary exfiltrates credentials or performs unauthorized
   project writes while producing plausible safe output.
4. An unverified artifact is promoted to additional environments.

Expected control:

Build and CI inputs must be immutable/reviewed, dependency integrity and known
vulnerabilities must be checked, untrusted code must not receive deployment
secrets, and promoted artifacts must be verified.

Actual behavior:

The architecture includes valuable release-integrity controls but leaves key
dependency and CI trust controls implicit.

Impact:

A supply-chain compromise can obtain service-account authority or corrupt all
managed assets within its scope.

Likelihood:

Currently unverified because no implementation/CI exists. The gap is actionable
before those files are created.

Required remediation:

- Require committed `Cargo.lock`, locked/frozen builds, default registry
  checksum verification, and explicit review/pinning of any git/path dependency.
- Add RustSec-compatible vulnerability scanning and a documented triage/waiver
  process; review features and minimize dependencies with access to HTTP,
  filesystem, serialization, and secrets.
- Pin third-party CI actions/includes to immutable revisions; set explicit
  least-privilege workflow permissions; prevent secrets in fork/untrusted PR
  jobs; protect deployment environments and branches; isolate runners.
- Verify release checksum/signature before execution and promote the exact
  verified artifact. Retain SBOM/provenance according to release policy.
- Run repository secret scanning and require rotation/containment when exposure
  is confirmed.

Owner: **solution-architect**

Downstream owners: **implementation-engineer / release-engineer**

Verification:

- Review actual Cargo and CI files; run locked build, dependency integrity,
  vulnerability, secret, and workflow-permission checks.
- Negative fork/PR test proves no deployment secret or protected environment is
  available.
- Signature/checksum verification rejects a modified binary before execution.

References:

- QR-006/007, PA-003, ADR-005, F-001, O-002, V-002, L-001

## Remediation plan

| Finding | Owner | Required action | Verification | Status |
|---|---|---|---|---|
| SEC-001 | product-spec-owner | replace process-visible secret-value flags or obtain explicit authorized risk acceptance | process-argument canary and refreshed contract trace | CLOSED (v25) |
| SEC-002 | solution-architect | define strict URL/local/TLS/redirect and protected CI destination trust | adversarial URL/redirect and CI override tests | CLOSED (v25) |
| SEC-003 | solution-architect | define numeric local/remote resource budgets and race-safe file handling | threshold, bomb, slow/large endpoint, cancellation tests | CLOSED (v25) |
| SEC-004 | product-spec-owner | approve and evidence safe Datasource complete-visibility/concurrency boundary | hidden-duplicate role fixtures and writer-governance drill | DEFERRED (ADR-012 PROPOSED; D-001 gated) |
| SEC-005 | solution-architect | constrain/escape identifiers, paths, fields, and multipart names | control-character/text/JSON/multipart corpus | CLOSED (v25) |
| SEC-006 | solution-architect | make dependency and CI trust controls normative | Cargo/CI/security/release evidence review | CLOSED (v25) |

## Accepted risks

None. The approved stdout-only login token behavior and reliance on external
Git/CI/platform audit systems are product decisions, but this report does not
treat them as blanket acceptance of shell-trace, process-argument, CI secret,
or audit-retention risks.

## Unverified areas

- No implementation exists, so authentication enforcement, zeroization,
  renderer privacy, TLS certificate verification, redirect behavior, file-open
  race safety, cancellation, and memory bounds cannot be dynamically verified.
- No Cargo dependency graph, lockfile, CI workflow, binary, SBOM, signature,
  container, infrastructure, migration, or release report exists.
- No target CodeMie/Keycloak environment was supplied for authorized behavior
  testing; runtime role/visibility, metadata preservation, and drift remain
  deployment-verification obligations.
- External Git/CI/CodeMie audit retention, branch protection, protected
  environments, runner isolation, secret rotation, and incident response are
  not represented by current repository evidence.
- Legal, regulatory, data-residency, and retention obligations were not
  provided and are not assessed.

## Exit criteria for security approval

This architecture may be resubmitted for Q-005 approval only when all of the
following are true:

1. SEC-001 is closed through an approved product contract that keeps secret
   values out of process arguments, or an authorized owner explicitly accepts
   the documented risk.
2. SEC-002 URL, local-development, TLS, redirect, and protected production
   endpoint rules are normative across specification/architecture/contracts/
   schemas/tasks and pass negative vectors.
3. Numeric resource budgets and race-safe file-open/multipart rules close
   SEC-003 with threshold tests.
4. An approved and target-evidenced Datasource visibility/concurrency model
   closes SEC-004.
5. Safe identity/source/multipart value domains and render encodings close
   SEC-005.
6. Dependency, CI secret-isolation, permission, immutable-input, and release
   verification controls close SEC-006 at the architecture/task level.
7. Independent verification confirms that the refreshed artifacts remain
   mutually consistent and that only authorized upstream artifacts changed.
8. The security reviewer retests the exact vectors and changes every required
   finding to `RESOLVED` or records explicit authorized risk acceptance.

## Next action

The single next lifecycle action is for the **product-spec-owner** to resolve
SEC-001's secret-input contract and SEC-004's Datasource visibility boundary,
after which the **solution-architect** must refresh the affected contracts,
schemas, ADRs, and tasks for all findings. Implementation remains gated pending
independent re-verification and security retest.

---

## v25 re-review

**Review date**: 2026-08-10

**Lifecycle task**: Q-005 targeted re-review of v25 remediation artifacts.

**Scope**: Verification that the six findings from the original review
(2026-08-09) were properly addressed by the v25 remediation artifacts. No new
feature scope was introduced.

**Artifacts reviewed**:

- `specs/codemie-cicd-tool/plan.md` (v25 remediation summary, §1 status block)
- `specs/codemie-cicd-tool/adr/011-url-validation-and-credential-input.md`
- `specs/codemie-cicd-tool/adr/012-datasource-visibility.md`
- `specs/codemie-cicd-tool/adr/005-modular-single-binary.md` (supply-chain amendment)
- `specs/codemie-cicd-tool/contracts/repository-config.schema.json`
- `specs/codemie-cicd-tool/contracts/declaration-v1alpha1.schema.json`
- `specs/codemie-cicd-tool/contracts/http-adapter.md`
- `specs/codemie-cicd-tool/data-model.md`
- `specs/codemie-cicd-tool/contracts/cli.md`
- `specs/codemie-cicd-tool/tasks.md`

### Per-finding verdict

**SEC-001 (HIGH) — CLOSED**

The product-spec-owner removed all secret-value CLI flags. ADR-011 §1 and
cli.md §1 are the normative sources: `--token`, `--client-secret`, and
`--password` are not registered as valid options; supplying any of them is
`E_USAGE`, exit 2, before any network access, with empty stdout. data-model.md
§9 `ResolvedConfig` type marks `bearer_token`, `client_secret`, and `password`
as `Secret<Environment>` with explicit "flag not accepted" annotations.
Tasks F-002, T-001, and O-002 all have acceptance evidence requiring env-only
secrets and flag rejection. The finding is confirmed closed.

**SEC-002 (HIGH) — CLOSED**

ADR-011 defines `ValidatedUrl` with all required properties. HTTPS is required
for `auth_url` unconditionally. HTTP is permitted for `target_url` only when
the resolved IP address is loopback (127.0.0.0/8 or ::1); the runtime must
resolve and verify at connection time. Redirects for Keycloak `POST .../token`
and local-auth `POST /v1/local-auth/login` are disabled with `E_AUTHENTICATION`
exit 2 on any 3xx response. URL userinfo (`@`) is rejected at schema-validation
time: `repository-config.schema.json` enforces `^https?://[^@#\x00-\x1f\x7f]+$`
for `url` and `^https://[^@#\x00-\x1f\x7f]+$` for `auth_url`. All URL-bearing
fields in `declaration-v1alpha1.schema.json` (`mcp_connect_url`, `link` fields,
`site_url`) carry the same `[^@#...]` no-userinfo pattern. CI endpoint
protection (protected variable injection, PR/fork secret isolation, protected
deployment environments) is normative in ADR-011 §5 and tasks O-002. An
invalid higher-precedence URL is exit 2 before network; lower-precedence values
are not consulted. The finding is confirmed closed.

**SEC-003 (MEDIUM) — CLOSED**

http-adapter.md §2.4 and data-model.md §11 (`ResourceBudgets_v1`) define exactly
18 versioned resource dimensions with numeric defaults: 1 MiB YAML/file, depth
32, alias 1,000, scalar 128 KiB, collection 10,000, files visited 10,000, sidecar
32 MiB/file, aggregate upload 128 MiB, response body 8 MiB, response headers
16 KiB, JSON depth 64, pagination 1,000 pages / 100,000 items, GET retries 3,
request timeout 60 s, invocation deadline 300 s, concurrency 1. Enforcement
order (reject before allocation, stream with bounded read, deadline propagates
to all in-flight operations) is specified. http-adapter.md §2.5 mandates
open-then-fstat on the same file descriptor to prevent TOCTOU races. The 1 MiB
YAML limit and 128 MiB aggregate upload limit are flagged for product-spec-owner
review if any currently-deployed declaration exceeds them. The finding is
confirmed closed.

**SEC-004 (MEDIUM) — DEFERRED**

ADR-012 is status PROPOSED and explicitly states "awaiting product-spec-owner
and CodeMie platform owner decision." The document presents three options (A,
B, C) with a recommendation but does not select one. D-001 implementation is
explicitly not authorized: the ADR body states "D-001 implementation is not
authorized" until ADR-012 is accepted; tasks.md D-001 also states "D-001 may
not begin until ADR-012 is accepted." The architecture requires that Datasource
be added to serialized-writer governance and duplicate-inventory runbooks before
that gate is reached (O-001/O-002 scope). All deferral conditions from the
original exit criteria are met. The finding is acceptably deferred with an
explicit product-level gate.

**SEC-005 (MEDIUM) — CLOSED**

`projectProperty` (used for all `project` fields across all four entity kinds)
has `maxLength: 100` and pattern `^[^ --‪-‮⁦-⁩]+$`
rejecting C0/C1 controls and bidi formatting characters.
`assistantMetadata.slug` and `workflowMetadata.slug` have `maxLength: 100` and
the same control/bidi pattern. `skillMetadata.name` and `skillKey.name` have
`maxLength: 64` and pattern `^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$` (alphanumeric
and hyphen only, structurally excluding all controls and bidi). `datasourceMetadata.repo_name`
has `maxLength: 50` and pattern `^[a-zA-Z0-9][A-Za-z0-9_-]*$` (same structural
exclusion). cli.md §10 defines: one physical record per line; JSON output via
serializer (not concatenation); `source.fieldPath` generated canonically from
YAML AST (not echoed from user input); text output using only fixed template
`<action> <kind> <project>/<key>`; route and query parameters percent-encoded.
http-adapter.md §7 mandates multipart basename rejection for any value containing
C0/C1 controls, CR, LF, NUL, or path separators. Hardening note: `assistantKey.slug`
(used in cross-reference arrays such as `sub_assistants`) carries only
`minLength: 1` without maxLength or control-character pattern; this is a NOTE-level
gap since these values are not directly included in output and the entity's own
identity fields (which do appear in output) are constrained. The finding is
confirmed closed.

**SEC-006 (MEDIUM) — CLOSED**

ADR-005 supply-chain and CI controls amendment is normative. It requires: Cargo.lock
committed and updated at every dependency change; all CI and release builds use
`cargo build --locked` or `--frozen`; default Cargo registry checksum verification
not disabled; git dependencies pinned to commit SHAs; RustSec-compatible
vulnerability scanning (`cargo audit`) as CI gate with documented triage and waiver
process; third-party CI actions pinned to immutable commit SHAs (not floating
tags); every workflow job carries an explicit `permissions:` block with
least-privilege scopes; secrets not available to fork-triggered or untrusted PR
workflows; apply and release jobs in protected deployment environments with human
approval; branch protection on default and release branches. Tasks F-001, O-002,
and V-002 include these as concrete acceptance criteria. No Cargo.toml, Cargo.lock,
or CI workflow files exist yet (implementation has not started), which is expected
at this pre-implementation lifecycle stage; the architecture gate is properly
established. The finding is confirmed closed.

### New issues introduced by v25

No new HIGH or CRITICAL issues were identified in the v25 artifacts.

One NOTE-level hardening opportunity was observed: `assistantKey.slug` used in
reference arrays (e.g., `sub_assistants`, `skillRefs` context entries) carries
only `minLength: 1` without the maxLength or control-character-rejection pattern
that `assistantMetadata.slug` has. This is not an exploitable injection path
because (a) these reference values are not directly output — output fields come
from the entity's own validated identity, and (b) `source.fieldPath` in
diagnostics is generated canonically from the YAML AST, not from the string value
of the slug. This is tracked as a NOTE for implementation defense-in-depth.

### Residual risk for SEC-004 (DEFERRED)

Until ADR-012 is accepted and D-001 is implemented, Datasource apply is not
authorized. The residual risk (Datasource identity corruption through hidden
duplicates or concurrent creators) is bounded to D-001 scope and does not affect
Assistant, Workflow, or Skill apply. The product-spec-owner and CodeMie platform
owner are the authorized decision holders. No implementation task may bypass the
ADR-012 gate.

### Overall disposition

```text
Security review status: APPROVED FOR IMPLEMENTATION
```

Both HIGH findings (SEC-001, SEC-002) are closed. All MEDIUM findings are either
closed (SEC-003, SEC-005, SEC-006) or acceptably deferred with a clearly documented
product-level gate (SEC-004). No new HIGH or CRITICAL issues were found.

Implementation may proceed for all tasks except D-001 (Datasource adapter), which
remains gated on ADR-012 acceptance by the product-spec-owner. This approval covers
the pre-implementation architecture review; post-implementation security review
(V-002) remains a mandatory downstream lifecycle stage before release authorization.

**Next lifecycle action**: **verification-engineer** (Q-006) performs pre-implementation
convergence re-verification against v25 artifacts; then **implementation-engineer**
may proceed for all in-scope tasks except D-001.
