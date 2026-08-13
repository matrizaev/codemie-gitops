# Security review report: O-002A/V-000A implementation and enterprise-smoke gates

## Status

```text
Security review status: APPROVED FOR NEXT STAGE
Threat-model status: ACCEPTABLE WITH REQUIRED CONTROLS
Local O-002A implementation: SECURITY-READY
Local V-000A implementation: SECURITY-READY
Enterprise Assistant/Workflow/Skill write smoke: NOT YET AUTHORIZED
V-003 Datasource write path: OUT OF SCOPE AND REJECTED PRE-NETWORK
Release/deployment authorization: NOT GRANTED
```

- Review date: 2026-08-11
- Review mode: Mode C — post-implementation security review
- Reviewed baseline: commit `1d5df7c3dc55761e60e424943cf6a200f9e47d97`
  plus the uncommitted v29 architecture and O-002A/V-000A implementation
  snapshot reviewed on that date

The architecture and stable post-remediation implementation are suitable for
the next local lifecycle stage. SEC-O002-003/005/006/007 are resolved. The
V-000A harness now copies the component-by-component `O_NOFOLLOW`-opened,
expected-digest executable into a sealed `memfd`, retains that executable
capability through lint, login, qualification, and the future apply handoff,
and never resolves a bare input through `PATH`. The provider checker now
enforces a closed command and credential surface, both provider checksum
handoffs are cwd-correct, and every operator entry point states the v29
provider split.

The checked-in provider examples implement the approved v29 split exactly:
GitHub captures one fresh token and invokes native `add-mask` on the
immediately following command before any apply, while GitLab invokes no login
and uses only the pre-supplied protected+masked, environment-scoped
`CODEMIE_TOKEN` process environment. The full mutation matrix and independent
reproduction of the prior bypasses now fail closed.

The user authorized create/update testing against
`https://codemie.lab.epam.com/` and configured an opaque
`CODEMIE_TEST_PROJECT` in the local credential file. The value was not read,
printed, or copied into this report. The live authorization still has to be
validated in memory against the concrete declarations, authenticated actor,
complete-visibility/write evidence, unique identity namespace, and exclusive
writer window. No enterprise write is authorized by this review. V-003 is a
closed Assistant/Workflow/Skill procedure:
Datasource members, paths, kinds, selectors, and authorization exceptions are
invalid locally and fail before authentication or any other network request.

No network request or remote mutation was made during this review. Credential
values were neither read nor printed.

## Scope

In scope:

- root and example READMEs for build, lint, login, apply, and recovery;
- portable GitHub Actions and GitLab CI examples;
- Git-revert/new-apply, Workflow adoption, and uncertain-write runbooks;
- local structural policy checks and mutation-negative tests for the examples;
- the local, non-mutating V-000 qualification harness contract;
- safe local consumption of an ignored `.env` for a later explicitly approved
  enterprise smoke;
- the proposed serialized Assistant/Workflow/Skill create-then-update smoke
  procedure;
- the closed pre-network Datasource write exclusion from V-003; and
- retained read-only V-000 Datasource compatibility coverage.

Explicit exclusions:

- remote provider configuration or O-001 activation;
- execution of V-000B or any live CodeMie request;
- enterprise entity creation/update/deletion;
- deployment, release, publication, tagging, pushing, or merging;
- production code remediation; and
- changes to reference-only `codemie/` or `codemie-ui/`.

## Sources of truth

- Product specification: `specs/codemie-cicd-tool.md` v29, especially
  FR-002/008/009/017/024/026/027/033/034/036, IR-006/008/011/012,
  PA-003/005/006/007, QR-007/009/010/011/012, VR-011/012, and
  AC-QR-012-01.
- Architecture: `plan.md`, `data-model.md`, and ADR-005/007/008/009/011/012.
- Contracts: `contracts/cli.md`, `contracts/http-adapter.md`,
  `contracts/declaration-v1alpha1.md`, and
  `contracts/adapter-manifest-v2.42.0.json`.
- Tasks: O-001, O-002/O-002A/O-002B, V-000/V-000A/V-000B, and V-003 in
  `tasks.md`.
- Existing operational evidence: `O-001-security-review.md`,
  `O-001-verification-report.md`, `.github/workflows/ci.yml`,
  `.github/workflows/codemie-gitops-apply.yml`, `.gitlab-ci.yml`, and
  `ops/o001/CHECKLIST.md`.
- Pre-implementation verification:
  `O-002A-V-000A-preimplementation-verification.md`.
- Prior post-implementation verification:
  `O-002A-V-000A-postimplementation-verification.md`; its reproduced findings
  describe the pre-remediation snapshot and were retested directly here.
- Implementation boundary: `README.md`, `examples/**`, `ops/o002/**`,
  `ops/v000/**`, `scripts/check_o002_examples.py`,
  `scripts/v000_target.py`, `tests/test_o002_examples.py`, and
  `tests/test_v000_target.py`.
- User-provided context: an ignored local `.env` contains credentials for the
  enterprise lab deployment. Its contents were not inspected.
- Jira/Confluence material: none was provided or available locally.

## Executive assessment

The local examples remain inert and their actual credential branches are
provider-correct. GitHub restricts apply to manual execution on `main`, names a
protected environment, scopes secrets to the final step, immediately masks its
fresh login token, and keeps fork/PR build work secret-free. GitLab restricts
apply to a manual protected-default-ref job on a protected runner and
environment, uses a static resource group, does not invoke login, and consumes
the pre-supplied token only through the job environment. Remote approval,
environment protection, protected/masked variable scope, runner protection,
and variable-override policy remain O-002B evidence rather than facts proved by
the YAML.

Both protected examples verify the promoted artifact from inside
`o002-artifact` before changing to `examples/repository`. A direct checksum
handoff test accepts the exact bytes and rejects replacement bytes. The
checker requires the full build/test/release/checker/checksum sequence, exact
provider credential commands, artifact paths, protected-job ordering, and
closed provider-specific token lifetime. The full v29 mutation matrix and
independent reruns reject pre-mask commands/output, token persistence or
transfer, build-job CodeMie secrets, GitLab login/assignment/simulated masking,
and weakened or omitted build/checker/checksum gates.

The V-000A dotenv parser is non-evaluating and closed to the documented key
allowlist; its tests demonstrate literal handling of command-substitution text
and rejection of duplicate, unknown, exported, malformed, and unsafe-permission
inputs. This review did not access the workspace `.env` or any of its values.
CI examples do not load `.env`.

The GET-only V-000 transport, strict consumed-field and pagination decoder,
body/page/item/deadline budgets, redirect rejection, exact-origin header
attachment, sanitized evidence, closed Assistant/Workflow/Skill manifest, and
actor/project/role/window/digest helper gates are present and their focused
tests pass. The exact-binary boundary now opens every authored path component
with `O_NOFOLLOW`, copies and hashes the opened regular non-writable executable
into a sealed `memfd`, and invokes only `/proc/self/fd/<retained-fd>`. Tests and
direct probes cover bare, `./`, nested-relative, `PATH`-shadow, rename,
replacement, and opened-source in-place mutation cases without credential
canary exposure.

The final live-readiness delta also closes natural-key absence before proof
creation. Skill uses a non-search `project_with_marketplace` compatibility scan
with a required non-empty page 0, followed by a separately exhaustive pinned
search-hint scan whose results are client-filtered to exact `(project,name)`.
Assistant requires the exact project-and-slug lookup to return not found.
Workflow exhausts project-visible and marketplace scopes and rejects an exact
reserved marker, a malformed target-project marker, or an unmarked
target-project row colliding with the concrete declaration's authored
`spec.name`. Only after all three absence gates and the runtime binding gate
pass is the process-local proof constructed. All observed requests are GET,
and evidence stores only fixed absence categories and bounded counts, never
the natural keys or display name.

The enterprise smoke is a privileged, durable write to a shared external
system. Its minimum safe scope is one authorized project, one named actor, one
exclusive window, and unique run-scoped natural keys. Assistant, Workflow, and
Skill may proceed only after non-mutating target qualification succeeds.
V-003 has no Datasource opt-in, selector, exception, or execution branch.
Datasource remains in V-000's non-mutating GET compatibility contract under
IR-008, but that evidence cannot construct a declaration or authorize a write.
Any future Datasource exercise would require a separate approved task and
security review outside V-003.

## Assets

| Asset | Confidentiality | Integrity | Availability | Owner |
|---|---|---|---|---|
| Enterprise bearer token/client secret/password | Critical | High | Medium | User / CodeMie identity owner |
| Authorized CodeMie project and entity records | Internal | High | High | CodeMie platform/project owner |
| Workflow reserved identity record | Internal | High | Medium | CodeMie platform identity administrator |
| CI protected environments and runners | Internal | High | High | Release Engineering |
| Tested `codemie-gitops` binary and checksum | Low | High | Medium | Release Engineering |
| Example declarations and runbooks | Public/internal | High | Low | Repository maintainers |
| Datasource inputs, external integrations, indexed content | Potentially sensitive | High | Medium | Data owner / platform owner |
| Sanitized smoke and qualification evidence | Internal | High | Low | Verification / Release Engineering |

## Actors and trust boundaries

Relevant actors are repository contributors, untrusted fork/PR authors,
trusted maintainers, CI runners, protected deployment approvers, the local
operator, the user-provided enterprise identity, the CodeMie platform/project
owner, the data owner, external integration providers, and a network attacker.

Material trust crossings are:

1. repository source and example declarations to an unprivileged build/lint
   runner;
2. the built binary and checksum through CI artifact storage to a protected
   deployment runner;
3. protected environment credentials to the in-memory login/apply process;
4. the local ignored `.env` to the manual smoke harness;
5. the tested binary over TLS to the enterprise CodeMie API;
6. V-000 read-only Datasource responses to the bounded qualification decoder;
   and
7. live Assistant/Workflow/Skill command output to persisted verification
   evidence.

## Attack surface and threat model

| Threat | Asset/boundary | Required control | Residual risk |
|---|---|---|---|
| Secret exposed in argv, shell trace, logs, artifacts, or cache | Credentials crossing local/CI process boundary | Environment-only secret input; GitHub fresh login immediately followed by native masking; GitLab pre-supplied protected+masked token with no login; no trace, echo, file, output, cache, artifact, or dotenv transfer; closed checker mutation matrix | Remote provider log/persistence settings remain O-002B evidence |
| Fork or PR code receives deployment secrets | Untrusted source to CI runner | Secret-free lint/test jobs; no `pull_request_target`; deployment only from trusted protected refs into approval-gated environments | Remote provider settings still require activation evidence |
| Repository config redirects credentials to an attacker endpoint | Repo source to network destination | Inject/check the exact protected HTTPS target and explicit auth URL at higher precedence; reject redirects/TLS bypass; endpoint ownership review | Compromised provider configuration remains external risk |
| Modified or untested executable performs the write | Build artifact to protected runner | Locked test/build; cwd-correct checksum; protected job consumes without rebuild; V-000 executes a retained sealed copy of the exactly opened and hashed staged binary | Future V-003 must retain the same capability and bind every apply to the process-local proof and digest |
| Wrong project or pre-existing identity is modified | Declaration to shared enterprise state | Explicit project/actor authorization, run-scoped unique natural keys, prove absence, exact identity, one declaration per invocation | Natural-key uniqueness is not server-enforced for every kind |
| Concurrent writer creates ambiguity or overwrites state | Manual/provider writer boundary | Exclusive window; per-target serialization; create then update sequentially; stop on any uncertainty; clean inventory before resume | O-001 remote mutex and freeze are not yet evidenced |
| Response or diagnostic leaks token/body/entity data | Enterprise response to local/CI output | Existing allowlist renderer; qualification harness stores no bodies, raw URLs, arbitrary headers, payloads, integration IDs, or credentials | Approved project/key identity remains visible in successful CLI output |
| Datasource triggers source access, integration use, indexing, compute, storage, or retention | V-003 manifest/selector to a potential durable write | Closed executable allowlist is exactly Assistant/Workflow/Skill; reject every Datasource member, path, kind, selector, or exception before authentication/network; V-000 may issue bounded GET probes only | A future Datasource exercise would require a new task and security review; it is not a V-003 option |
| Operator attempts cleanup after an unexpected write | Uncertain remote state | No automatic delete, rollback, or blind retry; inventory and manual platform remediation under named owner | Test entities are durable until an external owner removes them |

## Required implementation controls

### README and local examples

1. Document the exact command surface and environment-only secret variables.
   `--token`, `--client-secret`, and `--password` must not appear except as
   explicitly rejected negative examples that contain no value.
2. Document that `.codemie/config.yaml` is non-secret and that protected
   `CODEMIE_URL`/`CODEMIE_AUTH_URL` input must override repository endpoint
   fallback for credentialed jobs.
3. Keep all examples inert. Merely checking out the repository or running the
   default test suite must not contact CodeMie or apply an entity.
4. Show offline `lint` before login/apply and make the one-entity,
   always-write behavior explicit. A repeat apply is an update, not a no-op.
5. GitHub captures login stdout directly into an in-memory shell variable and
   immediately invokes native `add-mask`. GitLab does not invoke login and uses
   only its pre-supplied environment-scoped protected+masked `CODEMIE_TOKEN`.
   Neither path prints, persists, transfers, or re-emits the token.
6. Disable shell tracing and application/HTTP debug output before credentials
   are loaded. CI debug-trace modes are incompatible with a credentialed job.
7. Use the exact checksummed binary built and tested without deployment
   credentials. The protected job may verify/execute it but must not rebuild.
8. Link the Workflow adoption, Git-revert/new-apply, and uncertain-write
   runbooks from both operator entry points.

### CI provider examples

1. GitHub PR/fork jobs must have read-only permissions and no environment or
   secret references. `pull_request_target` is prohibited.
2. GitHub apply must be manual/trusted-ref only, name a protected environment,
   require approval remotely, use a static per-environment concurrency key,
   and restrict secrets to the final credentialed step.
3. A generated GitHub token must be masked before any later command can emit
   it; it must remain within the same step rather than enter `$GITHUB_ENV` or a
   job output.
4. GitLab apply must require a protected default ref, manual approval, a static
   protected environment and `resource_group`, a protected runner, and
   protected/masked environment-scoped variables. Pipeline variable overrides
   must be restricted remotely.
5. GitLab `CI_DEBUG_TRACE` must be forbidden for the credentialed job. The job
   consumes its pre-supplied environment-scoped protected+masked
   `CODEMIE_TOKEN` directly, never invokes `login`, never assigns a replacement
   token, and never uses simulated masking, an artifact, dotenv report, cache,
   file, or downstream variable for token transfer.
6. Neither provider example may use local-auth Mode (b), `.env`, `--insecure`,
   disabled certificate verification, `curl -v`, HTTP/body logging, or a
   privileged rebuild/test command.
7. A structural checker must validate provider semantics and include
   mutation-negative tests. Plain substring presence is insufficient proof.

### Local `.env` consumption

Before a live test, the harness must fail closed unless all checks pass:

- `.env` is the exact workspace-root path, a regular non-symlink file, owned by
  the current user, and not group/world readable;
- Git reports it ignored and untracked, and no tracked/history copy exists;
- a non-evaluating parser accepts only documented `CODEMIE_*` credential and
  selector keys plus the dedicated test-project key, rejects duplicate,
  unknown, malformed, exported, or multiline entries, and never prints a
  value;
- `source`, `.`, `eval`, `env $(...)`, and equivalent shell evaluation are not
  used;
- `CODEMIE_URL` is forced or compared in memory to the authorized exact HTTPS
  origin, without printing it with credentials;
- shell tracing is disabled before parsing, and no environment dump, process
  debug log, or diagnostic bundle is collected; and
- client secret/password variables are unset after a token is acquired, and
  the token is unset after the final apply.

The earlier architecture review recorded that the workspace file passed
non-content checks. This post-implementation retest did not reopen or inspect
the file and did not rely on its values.

### V-000A read-only transport

1. The harness verifies the staged binary checksum before authentication. It
   opens the authored path without following any symlink component, copies the
   expected-digest bytes into a sealed in-memory executable, retains that exact
   executable capability, invokes its `codemie-gitops login`, or consumes an already
   supplied token; it does not implement a credential POST. The fixed
   non-secret SHA-256 is persisted in the sanitized V-000B handoff for the
   later V-003 equality check.
2. The probe transport exposes a GET-only type/allowlist below CLI parsing.
   POST, PUT, PATCH, and DELETE must be unconstructable in preflight mode.
3. HTTPS certificate verification is mandatory. Userinfo, fragments, TLS
   bypass, proxy downgrade, and redirects are rejected. The exact authorized
   scheme/host/port is checked before construction and after URL resolution;
   Authorization is attached only to that origin.
4. Same-origin and cross-origin 3xx responses are not followed. Credentials
   never reach a redirect target.
5. Connect/read timeouts and the 300-second invocation deadline apply; response
   bodies are capped at 8 MiB; pagination is capped at 1,000 pages and 100,000
   items.
6. Consumed JSON fields are strictly decoded. Missing/wrong-type fields fail
   closed; additive unconsumed fields cannot widen the contract.
7. Failures use fixed sanitized categories. Raw URLs, response bodies,
   exception strings, credentials, entities, and payloads never enter output
   or persisted evidence.
8. Fake-server tests must cover both redirect classes, missing credential at
   redirect targets, the write-method negative surface, all budgets, strict
   JSON faults, and token/body/exception canaries across stdout, stderr, files,
   and evidence.
9. Read-only qualification retains the complete pinned GET contract, including
   Datasource compatibility under IR-008. The probe cannot emit a Datasource
   declaration, selector, authorization exception, or modifying request.

## Enterprise smoke execution gate

The later live smoke must follow this exact order:

1. Before authentication or any network access, strictly validate the closed
   smoke manifest. It must contain exactly one Assistant, one Workflow, and one
   Skill. Every Datasource or other member, path, kind, selector, or
   authorization escape hatch is invalid.
2. The user or CodeMie platform/project owner records the exact authorized test
   origin, project, actor, three allowed kinds, durable-record owner,
   collision-resistant run prefix, and a bounded `exclusiveWriter` record.
   Credential possession alone is not authorization.
3. Run locked formatting/lint/tests, build the release binary once, smoke its
   version/help output, compute its SHA-256, make the staged binary read-only,
   and use that exact path for every qualification and apply command.
4. Load credentials through the safe `.env` boundary, capture/reuse one token
   without output, and validate the exact authorized HTTPS origin.
5. Offline-lint every concrete declaration and resolve its effective project.
   Declarations and sidecars contain only approved non-sensitive test content.
6. In the same controlled execution and with the same token/session that will
   perform apply, rerun the complete non-mutating V-000B probe. This includes
   the bounded read-only Datasource GET contract but cannot enable a Datasource
   write. A prior, stale, differently targeted/project-scoped/principaled, or
   differently digested record cannot satisfy the gate.
7. Before the first possible modifying request, strictly decode the
   authenticated `GET /v1/user.email` actor and role/project fields and prove:

   ```text
   authorization.project
     == CODEMIE_TEST_PROJECT
     == every declaration's resolved effective project
     == the exact projects[].name entry used by the fresh role proof
   authenticated actor == authorization.actor
   ```

   The qualifying role is global administrator/maintainer or
   `is_project_admin=true` on that exact project entry. An admin role for any
   other accessible project is insufficient. Every mismatch yields zero writes.
8. Validate `exclusiveWriter.confirmed=true`, the confirmer, start/end times,
   and exact run prefix. The current time and complete create/update sequence
   must fit inside the bounded window, and the confirmer must attest that no
   other writer can use that prefix. Missing, false, expired, future,
   differently scoped, or non-covering confirmation yields zero writes.
9. Require the V-000B handoff SHA-256 to equal the staged binary digest and the
   V-003 evidence digest. Recompute it immediately before every apply; a change
   stops before the next modifying request.
10. Prove all three reviewed run-scoped natural keys absent under complete
    visibility before the first write. Skill separates the non-search
    `project_with_marketplace` compatibility observation from an exhaustive
    pinned search-hint scan client-filtered to exact `(project,name)`.
    Assistant uses its exact project-and-slug lookup. Workflow exhausts both
    project-visible and marketplace scopes and rejects the exact reserved
    marker, malformed target-project markers, and unmarked target-project
    authored-display-name collisions. Any existing or ambiguous identity
    aborts the entire smoke; do not adopt or select it.
11. Execute only serialized Assistant, Workflow, and Skill operations: one
    create per declaration followed by one repeat apply of that same declaration
    requiring `updated`. Any nonzero exit, timeout, signal, malformed output,
    verification gap, or uncertainty stops the whole sequence without retry.
12. Stop after the required updates. Do not delete, roll back, rename/move,
    automatically clean up, or issue additional repeat applies. Durable-record
    disposition remains assigned to the named manual owner.
13. Persist only sanitized evidence: the staged-binary SHA-256 (or immutable
    record reference), approved action/kind/project/natural-key outcomes, fixed
    binding/window status, safe request IDs, and reviewer outcome. Do not
    persist `.env`, token, actor value, URLs, bodies, payloads, arbitrary
    headers, integration IDs, declaration values, or server error text.

On any uncertain write, immediately unset credentials, hold the identity
prefix, record only safe status metadata, run complete non-mutating inventory,
and route to the O-001 uncertain-write procedure. Automatic retry, delete,
rollback, or tie-breaking is prohibited.

## Closed Datasource write exclusion

V-003 cannot express or execute a Datasource write. Its manifest and executable
allowlist are closed to exactly Assistant, Workflow, and Skill. A Datasource
member, declaration path, kind, selector, or authorization exception fails
locally before authentication or any other network access; there is no opt-in
or escape hatch. V-000 still verifies the pinned Datasource GET contract as a
bounded read-only compatibility probe under IR-008. That read coverage cannot
construct a Datasource declaration or reach POST, PUT, PATCH, or DELETE. Any
future live Datasource exercise requires a separately approved task and
security review outside V-003.

## Findings

No implementation security finding remains open for the reviewed local
O-002A/V-000A boundary. The prior architecture findings remain correctly
resolved at the design level, the closed V-003 entity set plus local
equality/window helpers are implemented, and SEC-O002-003/005/006/007 are
resolved by direct post-remediation evidence. No live write is authorized.

### SEC-O002-001 — Enterprise write authorization requires runtime project binding

- Severity: HIGH
- Confidence: HIGH
- Classification: Confirmed control gap, resolved in architecture and local
  gate helpers
- Status: READY FOR END-TO-END AND LIVE RETEST
- Affected assets: enterprise project/entity integrity and authorized identity
- Evidence: `plan.md` and `tasks.md` require a fresh V-000B pass in the same
  controlled execution and token/session as apply. `validate_runtime_gate` and
  its mutation tests prove equality among `authorization.project`, the opaque
  `CODEMIE_TEST_PROJECT`, every declaration's resolved effective project, and
  the exact `projects[].name` entry used for role evidence; strictly decoded
  `GET /v1/user.email` must equal `authorization.actor`. The role must be global
  administrator/maintainer or project-admin on that exact entry. The closed
  manifest also requires the confirmer, start/end/sequence-end values, run
  prefix, and affirmative no-other-writer attestation. `run_probes` now
  completes exact Assistant, Workflow, and Skill absence checks under the
  pinned visibility/search contracts before it constructs `QualificationProof`.
- Attack/failure scenario: valid credentials and a declaration naming an
  unintended accessible project cause a durable create/update outside the
  user's intended scope.
- Required remediation: retain the local gates. Before any V-003 write, bind
  them end to end to the fresh probe and the same apply token/session, and
  obtain the named user/platform-project owner's concrete authorization.
- Owner: implementation-engineer for the harness; user / CodeMie
  platform-project owner for live authorization
- Verification: the local mutation tests pass. The later end-to-end driver and
  live sanitized evidence must prove zero writes for every mismatch and show
  only closed binding/window status.

### SEC-O002-002 — Datasource smoke has unbounded external side effects

- Severity: HIGH
- Confidence: HIGH
- Classification: Potential risk resolved by closed scope exclusion
- Status: RESOLVED BY SCOPE EXCLUSION
- Affected assets: external systems, indexed data, compute budget, storage,
  retention, and project integrity
- Evidence: ADR-009 and the pinned adapter manifest admit source/content/file,
  integration, and scheduling fields, while the product contract does not
  govern server-internal processing and exposes no delete/rollback. Corrected
  `plan.md` and `tasks.md` make V-003's entity set exactly
  `{Assistant, Workflow, Skill}` and require every Datasource member, path,
  kind, selector, or exception to fail before authentication/network access.
- Attack/failure scenario: a seemingly small Datasource create contacts an
  external or internal URL/integration, ingests sensitive content, triggers
  embedding/indexing cost, persists data, or schedules recurring processing.
- Required remediation: retain the non-extensible V-003 exclusion. Any future
  Datasource exercise must be defined and reviewed as a separate task, not a
  V-003 option.
- Owner: solution-architect / implementation-engineer
- Verification: implemented schema/manifest mutation tests prove Datasource
  member, kind, and path forms plus missing required kinds fail before transport,
  while static transport inspection and tests prove V-000's Datasource coverage
  is GET-only and cannot construct or dispatch a modifying request.

### SEC-O002-003 — Qualified staged binary can differ from the executed binary

- Severity: HIGH
- Confidence: HIGH
- Classification: Confirmed finding, remediated
- Status: RESOLVED
- Affected assets: enterprise client secret/password, bearer token, target
  integrity, and qualification evidence
- Evidence: `StagedBinary.open` now opens the authored path component by
  component using directory file descriptors and `O_NOFOLLOW`, requires a
  regular non-writable executable, copies the opened bytes while calculating
  the expected SHA-256 into a Linux `memfd`, applies and verifies
  `F_SEAL_WRITE|F_SEAL_GROW|F_SEAL_SHRINK|F_SEAL_SEAL`, and retains that
  descriptor. `StagedBinary.run` executes only `/proc/self/fd/<descriptor>` with
  the descriptor passed to the child. Lint, login, qualification evidence, and
  the process-local future-apply proof all consume this same object.
- Attack/failure scenario: an operator supplies a valid bare relative staged
  path while a different earlier `PATH` entry has the same name. The shadow
  executable passes the lint phase and receives the login environment,
  including client secret or password, even though it is not the hashed binary.
- Expected control: the binary hashed before authentication must be the exact
  executable invoked for lint, login, V-000B, and every later apply.
- Actual behavior: bare, `./`, and nested-relative inputs are resolved only by
  descriptor-safe open from the authored cwd; subprocess execution never uses
  `PATH`. Replacing the pathname or mutating the formerly opened source inode
  after staging cannot change the sealed executable capability.
- Impact: credential disclosure and qualification/write execution by an
  unreviewed artifact.
- Likelihood: the reproduced path-shadow and replacement paths are closed for
  the local harness; unsupported Linux sealing or `/proc/self/fd` support fails
  locally with `binary-platform-unsupported`.
- Required remediation: complete; retain descriptor-safe component opening,
  expected-digest sealed staging, and the same live capability/proof boundary
  when V-003 is implemented.
- Owner: implementation-engineer
- Verification: the focused suite proves bare and `./` inputs execute the
  sealed source rather than a `PATH` shadow and that the shadow receives no
  secret canary; it proves the capability continues to execute the original
  bytes after rename, pathname replacement, and source-inode in-place mutation.
  Independent probes additionally reject symlinked intermediate and final path
  components, exercise a nested-relative path, verify the complete seal set,
  and reproduce replacement/in-place mutation resistance.

TOCTOU assessment: **RESOLVED for the reviewed V-000A boundary, not accepted
risk.** The opened source remains descriptor-bound while its expected-digest
bytes are copied, and later execution uses only the fully sealed retained
`memfd`; no credentialed subprocess reopens the authored pathname. A concurrent
source mutation during copying can only influence the staged bytes by also
passing the pinned expected digest. Future V-003 must retain this same live
capability rather than reconstructing it from the persisted SHA evidence.

### SEC-O002-004 — Staged-binary qualification handoff could be ambiguous

- Severity: MEDIUM
- Confidence: HIGH
- Classification: Confirmed evidence-chain gap, resolved in architecture
- Status: PARTIALLY IMPLEMENTED; V-003 RETEST REQUIRED
- Affected assets: integrity and reproducibility of V-000B/V-003 evidence
- Evidence: the earlier verifier report found that runtime checksum checks did
  not consistently require the digest in the durable cross-task record.
  Corrected `plan.md` and `tasks.md` now require the fixed non-secret SHA-256 in
  the sanitized V-000B handoff, bind V-003 evidence to that digest or its exact
  immutable record, and recompute the staged binary immediately before every
  apply.
- Attack/failure scenario: V-000B qualifies one binary, but V-003 later invokes
  a different or modified executable while retaining a plausible pass record.
- Required remediation: retain the implemented sanitized V-000B digest field
  and process-local equality helper. The later V-003 driver must bind its
  evidence reference to that digest and accept only the same still-open sealed
  executable/session capability before every apply.
- Owner: implementation-engineer
- Verification: local pre-authentication mismatch tests pass. A future
  between-apply mutation test and completed sanitized V-000B/V-003 record must
  prove the remaining downstream link.

### SEC-O002-005 — Structural checker accepts forbidden CI credential mutations

- Severity: MEDIUM
- Confidence: HIGH
- Classification: Confirmed finding, remediated
- Status: RESOLVED
- Affected assets: GitHub client secret/token, GitLab bearer token, CI logs and
  persisted job data
- Evidence: `validate_github` and `validate_gitlab` now require exact job sets,
  build commands, artifact paths, protected-job commands and order, provider
  credential mappings, and token-related command allowlists. The GitHub login
  and native mask commands must be adjacent. GitLab's only accepted token
  operations are the nonempty check, non-assigning export, and unset; login,
  assignment, native/simulated masking, persistence, and transfer are rejected.
  Build/MR jobs reject environments, variables, and CodeMie secret references.
- Attack/failure scenario: an unsafe portable example is changed to expose or
  persist deployment credentials while the required control checker and tests
  still report success; an adopter copies the approved-looking sample.
- Expected control: structural validation must enforce a closed provider
  credential boundary and GitHub's immediate native mask adjacency, and must
  reject every GitLab login, replacement assignment, simulated mask, token
  persistence, or build/MR secret reference.
- Actual behavior: the command and credential surfaces are closed structurally;
  any extra or reordered command fails validation.
- Impact: a future regression can expose a bearer token or long-lived client
  secret in logs, files, artifacts, or an untrusted build job.
- Likelihood: the reproduced false-pass paths are closed by exact structural
  validation and mutation-negative tests.
- Required remediation: complete; retain the closed provider command surface
  and expand the mutation matrix with any future provider behavior change.
- Owner: implementation-engineer
- Verification: the checked-in matrix rejects GitHub build secrets,
  `pull_request_target`, missing protection/build/test/release/full-checker/
  checksum gates, rebuilds, pre-mask output or commands, tracing, secret flags,
  TLS bypass, token output/file/transfer; and rejects GitLab unprotected refs,
  automatic apply, runner/mutex/rebuild failures, build token variables,
  missing build/test/release/full-checker/checksum gates, dotenv reports,
  tracing, login, missing pre-supplied token, simulated masking, assignment,
  environment dump, token file, and token transfer. All focused tests and six
  independent reproductions passed fail-closed.

### SEC-O002-006 — Portable checksum verification resolves the artifact path from the wrong directory

- Severity: MEDIUM
- Confidence: HIGH
- Classification: Confirmed finding, remediated
- Status: RESOLVED
- Affected assets: tested-artifact integrity and protected-job availability
- Evidence: both build jobs run checksum generation from inside
  `o002-artifact`, recording `codemie-gitops`; both protected jobs run strict
  checksum verification from that same directory before changing to
  `examples/repository`. The checker requires those exact commands and cwd
  ordering.
- Attack/failure scenario: the provider sample reaches its protected job but
  fails before login/apply, so the promised same-artifact control cannot be
  demonstrated or adopted from the checked-in example.
- Expected control: verify the exact promoted binary successfully before any
  credential acquisition or apply.
- Actual behavior: the checksum entry resolves to the promoted binary and the
  protected command fails before credentials if its bytes differ.
- Impact: deployment-example availability and absence of usable exact-artifact
  evidence. The prior failure was fail-closed and did not itself expose a
  secret.
- Likelihood: the prior certain availability failure is removed.
- Required remediation: complete; retain artifact-local generation and strict
  verification before the credential boundary.
- Owner: implementation-engineer
- Verification: the direct temporary-directory handoff accepts the exact
  promoted bytes and rejects replacement bytes. Both provider mutation suites
  reject omitted generation/verification and moved verification; full checker
  and offline lint pass.

### SEC-O002-007 — Operator index contradicts the GitLab no-login rule

- Severity: LOW
- Confidence: HIGH
- Classification: Confirmed documentation finding, remediated
- Status: RESOLVED
- Affected assets: GitLab bearer-token confidentiality
- Evidence: `README.md`, `examples/README.md`, and `ops/o002/README.md` now all
  distinguish the two providers: GitHub performs one fresh login followed
  immediately by native masking; GitLab consumes a pre-supplied
  environment-scoped protected+masked `CODEMIE_TOKEN` and never invokes login.
- Attack/failure scenario: a GitLab adopter follows the operator index instead
  of the provider example and creates a fresh token without a native runtime
  masking primitive.
- Expected control: every operator-facing entry point states the same
  provider-specific split.
- Actual behavior: every reviewed operator entry point states the v29 split and
  the no-persistence, no-transfer, no-re-emission, no-simulated-mask rules.
- Impact: confusing guidance can lead to unmasked fresh-token handling.
- Likelihood: the conflicting guidance has been removed.
- Required remediation: complete; retain provider-specific wording.
- Owner: implementation-engineer
- Verification: direct inspection plus the checked-in structure/runbook test
  and full checker pass.

No credential exposure was confirmed. No security risk is accepted on behalf
of the user.

## Security requirements coverage

| Requirement/control | Status | Evidence / required proof |
|---|---|---|
| FR-017 / SEC-001 environment-only secrets | SATISFIED LOCALLY | Provider examples and closed mutation matrix reject argv, build-job, output, persistence, transfer, cache/artifact/dotenv, and trace exposure paths |
| FR-024 / IR-006 / QR-012 provider-safe token delivery | SATISFIED LOCALLY | GitHub one fresh login plus immediately adjacent native mask; GitLab pre-supplied protected+masked environment token with no login/assignment/simulated mask; consistent provider docs |
| SEC-006 fork/PR isolation and protected environment | SATISFIED IN CHECKED-IN YAML; REMOTE PROOF REQUIRED | Manual trusted-ref protected jobs and secret-free actual build jobs; remote settings remain O-002B |
| Same tested artifact | SATISFIED LOCALLY | Cwd-correct provider checksum handoff; V-000 component-safe open, expected-digest sealed executable, retained capability, and path/PATH/mutation tests |
| FR-034 / QR-011 uncertain-write recovery | SATISFIED LOCALLY | Runbooks prohibit retry/delete/rollback/tie-break and require complete inventory before resume |
| V-000 GET-only HTTPS/exact-origin/redirect/budget/strict/safe-evidence controls | SATISFIED LOCALLY | Closed GET-only transport plus focused tests, sealed exact-binary execution, sanitized SHA evidence, and no credential/body/exception canary persistence |
| Enterprise exact project/actor/role authorization and natural-key absence | SATISFIED IN LOCAL HELPERS; LIVE RETEST REQUIRED | Strict actor/project/role/session gates plus GET-only exact Assistant, dual-scope Workflow, and split compatibility/exact Skill absence checks complete before proof; no live authorization or write |
| Bounded exclusive writer confirmation | SATISFIED IN LOCAL MANIFEST; LIVE RETEST REQUIRED | confirmed/attested flag, confirmer, start/end/sequence-end, run prefix, current-time coverage |
| Closed V-003 entity scope | SATISFIED LOCALLY | Exactly Assistant/Workflow/Skill; Datasource member/kind/path rejected before transport, SEC-O002-002 |
| Durable V-000B to V-003 artifact binding | SATISFIED FOR V-000A; V-003 RETEST REQUIRED | Sanitized SHA evidence and unpersisted process-local binary/session proof exist; V-003 must consume the same live capability, SEC-O002-004 |

## Validation performed

The following offline, non-network validation was performed:

- inspected product v29, the current plan/tasks, relevant contracts and ADRs,
  the pre-implementation verification, the earlier review, O-001 control
  evidence, and every final file in the owned README/examples/ops/o002/ops/v000/
  scripts/tests implementation boundary;
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v
  tests.test_v000_target` — PASS, 24 tests on the final live-readiness delta;
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/check_o002_examples.py` — PASS,
  including full structural validation and offline lint of all four example
  declarations with the release binary;
- `make test` — PASS: 343 Rust unit tests, 9 CLI lint integration tests, 46
  Python tests, and the full O-002 offline checker;
- Makefile gate inspection — PASS: the default `test` target runs the complete
  Python test pattern and full O-002 checker, while the unrelated `dev-server`
  variable and recipe lines remain intact;
- `make lint` — PASS, Clippy with warnings denied;
- `cargo build --locked --release` — PASS;
- direct CI mutation probe — PASS: all six independently reproduced prior
  bypasses were rejected (GitHub build secret, pre-mask `printenv`, token file;
  GitLab build token variable, login replacement, token file);
- direct exact-binary/TOCTOU probe — PASS: intermediate and final symlink
  components were rejected; nested-relative execution used the complete sealed
  `memfd`; and rename, pathname replacement, and opened-source in-place
  mutation could not alter executed bytes;
- static request construction trace — PASS: the V-000 probe constructs only
  `urllib.request.Request(..., method="GET")`; Datasource is read-only in V-000
  and rejected from the V-003 manifest before transport construction;
- `git rev-parse HEAD` —
  `1d5df7c3dc55761e60e424943cf6a200f9e47d97`;
- `git diff --check` — PASS;
- `git -C codemie diff --exit-code -- .` plus
  `git -C codemie-ui diff --exit-code -- .` — PASS, no tracked reference-only
  changes; and
- `git status --short -- src Cargo.toml Cargo.lock tests/cli_lint.rs` — PASS,
  no Rust/reference-adjacent implementation changes in the excluded path set.

No `.env` content, credential value, token, response body, network request, or
remote mutation was read, emitted, or performed by this review.

## Remediation plan

| Finding | Owner | Required action | Verification | Status |
|---|---|---|---|---|
| SEC-O002-001 | Implementation engineer; user / platform-project owner | Retain local equality/window gates; bind them end to end and supply live values later | Zero-write mutation matrix plus sanitized live authorization record | READY FOR END-TO-END/LIVE RETEST |
| SEC-O002-002 | Solution architect / implementation engineer | Preserve closed Assistant/Workflow/Skill V-003 scope and pre-network Datasource rejection | Manifest mutation and zero-network tests; GET-only V-000 Datasource probe | RESOLVED BY SCOPE EXCLUSION |
| SEC-O002-003 | Implementation engineer | Retain component-safe open, expected-digest sealed executable, and live capability through future V-003 | Bare/`./`/relative/PATH-shadow/symlink/rename/replacement/in-place mutation tests | RESOLVED |
| SEC-O002-004 | Implementation engineer | Retain local digest evidence; implement V-003 per-apply binding later | Between-apply mutation and linked-record tests | PARTIALLY IMPLEMENTED |
| SEC-O002-005 | Implementation engineer | Retain closed structural validation and complete provider mutation matrix | Every reproduced bypass raises `ExampleError` | RESOLVED |
| SEC-O002-006 | Implementation engineer | Retain artifact-local generation/strict verification before credentials | Exact-byte handoff passes; replacement and path/order mutations fail | RESOLVED |
| SEC-O002-007 | Implementation engineer | Retain provider-specific wording in every operator entry point | Documentation assertion and review | RESOLVED |

## Unverified areas

- Contents and correctness of `.env` credentials.
- Exact enterprise project, actor, roles, and target authorization.
- Target API compatibility, exact-project visibility, and pagination behavior.
- Remote CI protected environments, runners, branch protection, secret scope,
  and mutex behavior.
- External writer freeze and live identity inventory.
- Any live Datasource integration, source, processing, storage, retention, or
  cost; these are outside V-003 rather than optional within it.
- Remote provider behavior, logs, persisted data, protected/masked variable
  scope, approvals, runner trust, branch protection, and pipeline-variable
  override restrictions.
- End-to-end V-003 fresh-probe-to-apply session binding, identity-absence proof,
  staged-digest recheck before every apply, and zero-write mutation behavior.
- End-to-end V-003 retention of the same live sealed executable and
  process-local qualification proof; persisted SHA evidence alone is not the
  retained capability.

Absence of this evidence is not treated as proof of safety or activation.

## Next action

The local O-002A/V-000A implementation may proceed to independent
post-remediation verification. That verifier must rerun the exact sealed-
capability, provider mutation, checksum, offline, and full gates rather than
relying on the earlier pre-remediation report. V-000B, V-003, O-002B, release,
and deployment remain unauthorized; any future Datasource exercise requires a
new task and security review outside V-003.
