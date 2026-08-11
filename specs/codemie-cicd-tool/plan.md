# Architecture plan: CodeMie declarative CI/CD CLI

## 1. Status

**Architecture status: READY FOR INDEPENDENT POST-IMPLEMENTATION VERIFICATION
AND O-001 HANDOFF**

Product specification v27 is the approved source. The architecture now pins
presence, null, applicability, ownership, and transport classes for every
admitted field, requires an explicit token endpoint with deterministic
precedence, incorporates all architect-owned security remediation from the
pre-implementation security review, and synchronizes the v27 per-file lint
warning lifecycle (`security-review-preimplementation.md`):

- **ADR-003 handoff**: ADR-003 superseded by ADR-011 (credential input,
  ValidatedUrl, TLS, redirect policy).
- **SEC-001** (CLOSED by product-spec-owner, v25): secret credentials
  env-only, corresponding flags `E_USAGE` exit 2.
- **SEC-002** (CLOSED): ValidatedUrl type, HTTPS/loopback policy, redirect
  disable for auth POSTs, URL userinfo rejection in all schemas.
- **SEC-003** (CLOSED): versioned resource budget defaults for all 18 resource
  dimensions in `http-adapter.md` §2.4 and `data-model.md` §11.
- **SEC-004** (CLOSED): ADR-012 accepted Option A; a project-admin visibility
  preflight is required for Datasource reconciliation and D-001 is complete.
- **SEC-005** (CLOSED): identifier maxLength and control/bidi pattern in all
  schemas; output rendering rules in `cli.md` §10.
- **SEC-006** (CLOSED): supply-chain and CI controls amendment in ADR-005.

The implementation baseline, including D-001, is complete. Independent review
of the v27 warning-contract delta found only stale lifecycle text in this plan;
this synchronization resolves that documentation finding. O-001 is the next
production-enablement handoff. V-001, V-002, and L-001 remain downstream and
are not complete.

## 2. Executive summary

The implementation baseline is one self-contained Linux x86_64 Rust executable,
`codemie-gitops`, with
offline `lint`, single-entity `apply`, and stateless `login`. Checked-in closed
schemas define the authoring surface for exactly four entities: Assistant,
Workflow, Datasource, and Skill. Direct REST adapters resolve an exact identity,
project the declaration into the pinned operation request, issue POST when the
identity is absent or PUT when it is present, and verify identity after the
write. Every valid apply writes once and reports only `created` or `updated`.

Workflow uses its reserved `meta_config` identity and explicit by-UUID legacy
adoption. Skill and Datasource use exhaustive, ambiguity-refusing list
resolution because their server persistence does not enforce the approved
natural key. Datasource kinds are peer request projections within one adapter;
file/source/content fields use ordinary write-through CRUD. There is no delete,
local state, generic marker, interpolation, or dedicated Datasource lifecycle
surface.

## 3. Sources consulted

- Approved product source: `specs/codemie-cicd-tool.md` v27, all sections.
- Security review input: `security-review-preimplementation.md` (read-only;
  findings SEC-001–SEC-006 used as remediation drivers).
- Verification sources: `Q-006-verification-report.md` (read-only v26 baseline)
  and the current-task independent v27 delta result supplied by the verifier
  (warning behavior passed; this plan cleanup was the remaining finding).
- Backend reference: tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- UI reference: tag `2.42.0`, commit
  `55945d075d82e771c4a2f4238afec1eb4c79d1e1`.
- Exact evidence and paths: `research.md`, `contracts/source-baseline.md`, and
  `contracts/adapter-manifest-v2.42.0.json`.
- Jira/Confluence: none available.

The two source trees are reference-only evidence, never implementation targets
or runtime dependencies.

## 4. Scope and constraints

In scope: marked YAML, deterministic repository discovery, effective project
configuration, closed schema/semantic/cross-reference validation, Skill
sidecars, exact REST reconciliation, ordinary per-kind Datasource CRUD,
Keycloak/local-development login, safe human/JSONL output, and GitHub/GitLab
operational patterns.

Out of scope: plan/batch/delete, overlays/templates/interpolation, local state,
generic ownership/adoption, credential or external-integration provisioning,
live provider-schema discovery, dedicated Datasource lifecycle controls, server
changes, and changes to either reference repository.

Quality constraints include one static binary, no reference Python dependency,
offline lint, fail-closed identity, serialized production apply, strict
allowlist output, and deployment contract testing.

## 5. Current architecture

The implementation baseline exists in the product area under `src/`, with the
binary entry point and command dispatch, offline repository loading/lint,
single-entity apply coordination, per-kind adapters, authentication/HTTP
boundaries, and typed output rendering. `tests/cli_lint.rs` supplies CLI lint
integration coverage. D-001's Datasource adapter baseline is implemented in
`src/adapters/datasource.rs`. External CodeMie remains the remote system of
record and exposes the Assistant, Workflow, Skill, and ordinary per-kind
Datasource contracts consumed by the CLI; it owns remote records,
authorization, and external integrations.

Important limits are: Workflow identity metadata has no database uniqueness
constraint; Skill uniqueness includes creator; Datasource natural uniqueness is
not database-enforced and `find_id` returns a first match; writes lack a common
conditional-update contract; `/v1/info` does not reliably identify source
compatibility. Exact exhaustive reads, privilege proof, serialized writers,
post-write identity verification, manual ambiguity remediation, and pinned
deployment tests contain these limits.

## 6. Requirements-to-architecture map

| Requirements | Architecture response |
|---|---|
| FR-001–004, FR-022/023/025/027, VR-016 | closed bundled schema; effective project; source-pinned required/optional-null classes; scalar YAML-relative sidecars; deterministic offline index |
| FR-005/006/008/012/015/021, DR-012 | resolve identity; POST if absent; PUT if present; optional omission/null to explicit null; bounded applicability/ownership transforms; no delete |
| FR-009/011/014/016/024/026 | stateless auth; exact exit union; stdout success/stderr failure; repository-closure validation before target-only lint warnings; bytewise warning-code/canonical-field-path ordering; no warnings on failure; allowlist renderer |
| FR-017 | exact per-field precedence; closed non-secret repository config; secret credentials from environment only; non-secret selectors from flag or environment (SEC-001, v25) |
| FR-019/020 | no generic adoption or ownership marker; Workflow-only identity exception |
| FR-028–030, FR-032–035 | Workflow reserved record, exhaustive resolver, explicit adoption, exact local actor/reference projection, race visibility |
| FR-031–034 | exhaustive Skill resolver, privileged visibility/write proof, bounded 409 recovery, no tie-break/delete |
| FR-036, IR-008 | one Datasource adapter, peer exact create/update mappings, ordinary write-through CRUD, no dedicated lifecycle surface |
| PA-005/006, QR-009–011 | privileged resolver identity, serialized CI, writer governance, inventory/remediation |
| SEC-001 (v25) | env-only secret credentials; `--token`/`--client-secret`/`--password` flags are E_USAGE, exit 2, before network access; ADR-011 |
| SEC-002 (v25) | ValidatedUrl type; HTTPS-required auth_url; loopback exception for target_url; userinfo/fragment/control-char rejection in all URL schema patterns; redirect disabled for auth POSTs; ADR-011 |
| SEC-003 (v25) | 18 versioned resource budget defaults; YAML, file, network, response, pagination, concurrency, deadline dimensions; `http-adapter.md` §2.4; `data-model.md` §11 |
| SEC-004 (closed) | ADR-012 accepted Option A; project-admin complete-visibility preflight; D-001 implementation baseline complete |
| SEC-005 (v25) | identifier maxLength and control/bidi pattern rejection in all schemas; safe output rendering rules (one line/record, JSON serializer, canonical field paths, multipart basename safety); `cli.md` §10 |
| SEC-006 (v25) | Cargo.lock committed; --locked builds; RustSec scanning; SHA-pinned CI actions; permissions blocks; secret isolation for fork/PR; protected deployment environments; same-artifact promotion; SBOM; ADR-005 amendment |

## 7. Decisions and alternatives

### Author/schema contract

- Selected: closed checked-in v1alpha1 schema plus a source-pinned adapter
  manifest. This gives offline determinism and prevents target drift from
  widening authoring.
- Rejected: live OpenAPI as the author schema, because source fields/defaults
  and the exposed version are not a stable product contract.
- Rejected: permissive YAML forwarded to the server, because it violates local
  unknown, secret, and runtime-field rejection.

### Reconciliation identity

- Assistant: direct exact slug/project API.
- Workflow: reserved `meta_config` record with explicit by-ID legacy adoption;
  display name is never an implicit selector.
- Skill: exhaustive exact list resolution because transport IDs cannot adopt
  existing rows and server uniqueness is creator-scoped.
- Datasource: exhaustive exact list resolution because `find_id` is first-match
  and database natural uniqueness is absent.

Server-side natural-key endpoints would improve atomicity but are outside the
approved scope. Client state, UUIDs in declarations, and implicit display-name
selection conflict with the approved portability and identity boundaries.

### Write policy

- Selected: operation-driven write-through. Resolution produces either
  `Create(request)` or `Update(server_id, request)`. Existing entities receive
  PUT on every valid invocation.
- Rejected: field-state-dependent write suppression, because v24 explicitly
  defines repeat apply as update.

### Presence and null policy

- Selected: classify every field from the pinned request contracts. Omission
  and YAML null produce an explicit null for optional-null JSON properties on
  create and update. Null-rejecting/defaulted fields remain authoring-required.
- Selected: do not fabricate properties for authoring-only,
  operation-inapplicable, read-only, or mixed/tool-owned structures. Workflow
  `meta_config` retains its preservation merge.
- Rejected: retain omission or materialize a concrete server default; both
  violate v24. Target-existence-dependent validation is also rejected because
  a field rejected as null in either operation must be required before
  resolution.

### Packaging

- Selected: modular library plus one Rust binary, minimizing distribution and
  OS dependencies while retaining testable boundaries.
- Rejected: Python/server-package reuse and multi-service deployment because the
  product requires a self-contained Rust CLI.

### Authentication endpoint

- Selected: Keycloak receives one exact endpoint resolved as `--auth-url` >
  `CODEMIE_AUTH_URL` > repository config `auth_url`. The highest-precedence
  value is validated and used as-is.
- Rejected: deriving or probing an identity-provider URL from the CodeMie API
  URL, hostname, realm, path, or other convention. No such contract exists and
  v24 prohibits it.
- Missing explicit endpoint is a local exit-2 failure before any network call.
  Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, `CODEMIE_PASSWORD`)
  are accepted from environment only; the `--token`, `--client-secret`, and
  `--password` flags are not accepted. Non-secret selectors (`--client-id`,
  `--email`) use flag-over-environment precedence. No credential is loaded from
  repository config. (SEC-001 remediation, v25.)
- Mode (c) Keycloak ROPC (v26): uses the same `auth_url` as Mode (a) but sends
  `grant_type=password` with `client_id` (defaulting to `codemie-sdk` if unset),
  `username` (`CODEMIE_EMAIL`), and `password` (`CODEMIE_PASSWORD`). No
  `client_secret` is used or sent. Selected when `CODEMIE_CLIENT_SECRET` is
  unset, `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` are set, and `auth_url` is
  configured. Same HTTPS, redirect-disabled, and SEC-001 env-only rules apply as
  for Mode (a).

## 8. Target components

| Component | Responsibility | Security/failure boundary |
|---|---|---|
| command shell | exact commands, flags, config, and exits | rejects usage before network |
| discovery/loader | deterministic YAML and sidecar reads | bounds, containment, source coordinates only |
| schema/semantic validator | closed v1alpha1, effective project, and field-presence classes | no live schema/default insertion |
| repository index | offline natural and graph-local reference closure | no server access |
| lint warning evaluator | after complete repository-closure validation, evaluate only the `--file` declaration and sort by fixed warning code then canonical field path | never emits on a failed lint or for closure-only declarations |
| request projector | typed create/update requests from authored intent | only approved transforms; no equality branch |
| resolver/adapters | four entity protocols and peer Datasource mappings | exact identity, visibility, and write proof |
| auth/transport | bearer/login, TLS/proxy/CA, retries/timeouts/strict decode | secrets isolated; bodies discarded |
| output boundary | typed stdout outcome, typed stderr warning, and typed stderr diagnostic | warnings only on successful lint; failure is exactly one diagnostic; no generic message/body API |

```mermaid
flowchart LR
  Git[Git YAML + sidecars] --> Local[Discovery + closed validation]
  Config[Repository config] --> Local
  Local --> Index[Repository and graph-local indexes]
  Index --> Resolve[Identity and reference resolution]
  Resolve --> Project[Create or update projection]
  Auth[Stateless auth] --> HTTP[Safe HTTP]
  Project --> HTTP
  HTTP --> API[External CodeMie API]
  HTTP --> Verify[Post-write identity verification]
  Verify --> Out[Success stdout]
  Local --> Err[Safe failure stderr]
  HTTP --> Err
```

## 9. Data and consistency

Git is the desired-state source; CodeMie owns remote records. The CLI stores no
state. `metadata.project` may be omitted when repository config provides a
non-empty default. Skill `contentFrom` is a scalar path relative to its
declaring YAML. Natural keys contain effective project plus the kind key and
never server IDs.

Request projection includes authored fields plus only FR-021 transformations.
It never inserts server defaults. Required/null-rejecting values must be
authored. An omitted or explicit-null optional authorable property becomes a
present JSON null in each applicable JSON request. Authoring-only,
operation-inapplicable, read-only, and mixed/tool-owned members receive no
fabricated null. Operation-specific contracts decide which fields exist on
create and update; create-only fields are absent from update. Existing server
reads are used only where needed for identity,
authorization, Workflow metadata preservation/adoption, server-shape decoding,
reference mapping, or post-write identity verification.

One POST or PUT is the modifying transaction boundary. There is no blind write
retry, delete, or rollback. The one Skill create-409 path performs one bounded
full resolution and never repeats POST. An uncertain result reports a safe
failure and requires re-resolution on a later invocation.

## 10. API and Datasource mapping

The checked-in adapter manifest is authoritative for exact routes, operation
fields, pagination, response fields consumed by resolution, and representation
transforms. Runtime preflight strictly decodes fields it consumes and fails
incompatible before writes where detectable. Live OpenAPI and `/v1/info` cannot
expand the contract.

Datasource uses one exhaustive resolver and peer per-kind create/update
projections. File, source, content, scheduling, and configuration fields in the
selected ordinary operation contract are transmitted on each valid apply.
File uses the singular multipart route, byte-preserving UploadFile parts, and
the pinned JSON-string/query encodings. Workflow `meta_config` is decoded from
and encoded to its source-pinned string form.
External integration references are opaque pre-existing values; CodeMie is
authoritative for validation and authorization. The CLI exposes no dedicated
Datasource lifecycle command, flag, or endpoint.

## 11. Security and output

Use one least-privilege invocation principal. Workflow and Skill writes require
complete project visibility plus write proof. Credentials come only from the
approved environment or flags and never repository config/YAML. HTTPS is
required outside explicit local development; proxy/CA support, disabled
cross-origin credential redirects, bounded retries/body drains, and typed
response decoding protect the network boundary.

**URL and TLS policy (SEC-002, ADR-011)**: Every URL input is a `ValidatedUrl`
(absolute http/https, no userinfo, no fragment, no C0/C1 controls). `auth_url`
requires HTTPS unconditionally; `target_url` requires HTTPS except for resolved
loopback addresses (127.0.0.0/8 or ::1). URL userinfo is rejected at schema
validation time. An invalid higher-precedence URL is `E_CONFIGURATION`, exit 2;
lower-precedence values are not consulted. Redirects are disabled for Keycloak
`POST .../token` and local-auth `POST /v1/local-auth/login`; a 3xx response is
`E_AUTHENTICATION`, exit 2. For API calls, redirects are preferred-disabled; if
retained, must be method-aware, same-origin (same scheme+host+port), max 3
hops, with no Authorization header forwarded to a different authority.

**Resource budgets (SEC-003)**: Versioned defaults for 18 resource dimensions
are defined in `contracts/http-adapter.md` §2.4 and `data-model.md` §11,
including YAML parsing limits (1 MiB/file, depth 32, alias 1,000, scalar 128
KiB), file/sidecar (32 MiB per file, 128 MiB aggregate), response body (8 MiB),
pagination (1,000 pages / 100,000 items), retries (3 GET), timeouts (60s
request, 300s invocation deadline), and concurrency (1 sequential request). The
YAML-per-file and aggregate-upload limits are flagged for product-spec-owner
confirmation if any currently-deployed declaration exceeds them.

**Identifier constraints (SEC-005)**: Identity-bearing fields in output and
schemas (`project`, `slug`, `name`, `repo_name`, `fieldPath`) have maxLength
bounds and reject C0/C1 control characters (U+0000–U+001F, U+007F–U+009F) and
bidi formatting characters (U+202A–U+202E, U+2066–U+2069). Output rendering
rules are in `cli.md` §10: one physical record per line, JSON via serializer
(not concatenation), field paths generated canonically from YAML AST,
percent-encoded route/query parameters, multipart basename safety checks.

**Supply-chain controls (SEC-006)**: Cargo.lock is committed; all builds use
`--locked`; RustSec advisory scanning and license review are CI gates; CI
action references are SHA-pinned; permissions blocks are set on all workflows;
secrets are isolated from fork/PR contexts; deployment environments are
protected; same-artifact promotion is required; SBOM and provenance accompany
releases. See ADR-005 amendment.

Success validates against `outcome.schema.json` and contains only action, kind,
project, and the kind natural-key member.
Apply actions are `created` or `updated`; lint uses `valid`. A successful lint
evaluates suspected-plaintext-secret and deprecated-value warnings only for the
`--file` declaration after the complete repository closure validates, then
emits them by bytewise fixed warning code and canonical field path. If any
closure declaration fails, lint emits no warnings: stdout is empty and stderr
contains exactly the closed `diagnostic.schema.json` failure record. Bodies,
server text, payloads, declaration/sidecar values, secrets, arbitrary headers,
raw URLs, and exception strings never enter output or logs.

Architecture-remediation security review items are closed (see §1 status).
Post-implementation security review (V-002) remains a downstream lifecycle
task.

## 12. Operations, deployment, and recovery

Build once and promote the same static Linux x86_64 artifact. Configuration and
secrets are environment-specific inputs. Pre-implementation verification must
approve spec/schema/manifest/task convergence. Deployment verification runs
non-mutating contract and behavior fixtures against a target; drift blocks that
deployment or release, not architecture readiness.

Production applies serialize per target environment. Teams govern other
Workflow/Skill identity writers and maintain duplicate/marker inventory and
manual remediation. Logs and metrics use only safe action/category/status/
latency/request identifiers. Rollback is the prior binary plus Git revert and a
new apply; existing remote writes are not automatically reversed.

## 13. Implementation stages

1. **Complete** — the pre-implementation architecture convergence and security
   review that gated baseline implementation; these do not replace V-001 or
   V-002.
2. **Complete** — binary, closed repository config, discovery, exact
   declaration schema, and offline references.
3. **Complete** — request projection plus typed success/warning/diagnostic
   boundaries, including the v27 lint-warning contract.
4. **Complete** — authentication, safe transport, and source-pinned
   compatibility preflight.
5. **Complete** — Assistant, Workflow, Skill, and Datasource adapters/resolvers,
   including D-001.
6. **Complete** — write-through coordinator and implementation test baseline.
7. **Next / incomplete** — O-001 serialization and identity-writer governance,
   followed by O-002 and V-000; V-001, V-002, and L-001 remain downstream in
   their declared dependency order.

Detailed bounded work and exact dependencies are in `tasks.md`.

## 14. ADRs

Statuses below mirror the status section in each ADR file.

| ADR | Status | Decision |
|---|---|---|
| 001 | Proposed | bundled closed schema and marked YAML |
| 002 | Proposed | resolve-project-write reconciliation without default insertion |
| 003 | Superseded by ADR-011 | stateless auth and safe HTTP (historical; credential input and URL/TLS/redirect policy superseded by ADR-011) |
| 004 | Proposed | source-pinned manifest compatibility gate |
| 005 | Proposed | modular single Rust binary; v25 amendment adds supply-chain and CI controls (SEC-006) |
| 006 | Superseded by ADR-008 | derived Workflow UUID rejected |
| 007 | Accepted | exhaustive Skill resolver and unconditional existing-entity PUT |
| 008 | Accepted | Workflow marker, explicit adoption, and unconditional existing-entity PUT |
| 009 | Accepted — amended by ADR-012 (visibility precondition) | uniform Datasource ordinary write-through CRUD boundary |
| 010 | Proposed | separate closed success/failure records |
| 011 | Proposed | URL validation, credential input (env-only secrets), TLS/HTTPS policy, redirect policy; supersedes ADR-003 on these topics |
| 012 | Accepted — Option A selected, 2026-08-10 | project-admin complete-visibility preflight; D-001 complete |

## 15. Risks and open questions

| ID | Type | Risk/question | Treatment/owner |
|---|---|---|---|
| R-01 | non-blocking | deployment differs from pinned source | target contract suite; verification/release owner |
| R-02 | non-blocking | resolver snapshots churn or external writers race | fail closed, serialization, governed writers, remediation |
| R-03 | non-blocking | provider schema varies by deployment | reject until an exact reviewed schema is bundled |
| R-04 | non-blocking | QR-005 has no latency threshold | measure without inventing a release SLO; product owner may later define |
No open entity, projection, configuration, or authentication-endpoint decision
remains. Deployment verification and independent security review are bounded
downstream lifecycle work, not missing architecture inputs.

## 16. Handoff

O-001 is the next production-enablement task: activate per-environment
serialization and identity-writer governance using the completed implementation
baseline. O-002 and V-000 follow it. V-001 must still perform independent full
post-implementation specification-to-code convergence verification; V-002 and
L-001 remain dependent downstream tasks. This plan does not mark O-001, V-001,
V-002, or L-001 complete.
