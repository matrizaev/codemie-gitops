# Verification report: v31 personal-project owner qualification (T-004/V-000A)

## Status

```text
Verification status: NOT VERIFIED
```

The implemented personal-owner predicate is substantially present and the
focused local suites pass. Two approved v31 boundaries are nevertheless not
implemented: Rust dispatch can replace the qualified API capability/origin,
and V-000A binds the manifest actor to email rather than authenticated
`/v1/user.user_id`. The current tests encode or omit those defects, so green
results are insufficient for convergence.

## Scope and sources of truth

- Product: `specs/codemie-cicd-tool.md`, DRAFT v31; SC-022, FR-037, DR-013,
  IR-013, PA-008, VR-017, AC-FR-037-01/02/03.
- Architecture: `plan.md`, `data-model.md`, `tasks.md`, ADR-013 and amended
  ADR-007/008/009/012.
- Contracts: `contracts/http-adapter.md`,
  `contracts/adapter-manifest-v2.42.0.json`, and `contracts/source-baseline.md`.
- Implementation/evidence: Rust HTTP/adapters and tests; `scripts/v000_target.py`
  and `tests/test_v000_target.py`.
- Excluded: live V-000B/V-003 evidence, network, `.env`, reference-tree edits,
  deployment, and release authorization. No Jira/Confluence material was
  provided.

## Acceptance and control matrix

| Control | Status | Evidence |
|---|---|---|
| Workflow/Skill/Datasource applicability; Assistant exclusion | SATISFIED | The three applicable adapters invoke preflight; Assistant does not. |
| Exact user ID/membership and exactly one total detail member | SATISFIED | `preflight_visibility` enforces exact membership count and sole-member equality; focused Rust/Python cases pass. |
| Strict duplicate-aware field decoding | PARTIALLY SATISFIED | Both decoders reject duplicate keys before DTO conversion, but the required exhaustive field/category mutation matrix is absent. |
| One percent-encoded route segment | SATISFIED | Rust byte encoding and Python `quote(..., safe="-_~")` plus dot encoding preserve one segment; adversarial route tests pass. |
| Exact incompatibility/visibility taxonomy and ordering | PARTIALLY SATISFIED | Rust ordering is before resolution and modification. Python uses harness labels such as `actor-binding-invalid`/`role-binding-invalid`, and new cardinality tests assert only an exception rather than the required exact category and zero resolver count. |
| Same client/origin/token/session through final revalidation and dispatch | NOT SATISFIED | Rust call sites reuse the local variables, but the required invariant is not sealed; see Q009-POST-001. Python reuses one transport/session within the probe. |
| Zero writes on failed qualification | SATISFIED for inspected paths | Rust failures return before dispatch; V-000A transport is GET-only. Matrix evidence remains incomplete for every required mutation. |
| Separate entity write ability | SATISFIED | Applicable adapters establish resolution/write evidence separately from visibility before sealing. |
| Safe evidence and live-evidence reset | PARTIALLY SATISFIED | Output is sanitized and task reset labels now consistently invalidate pre-v31 records. V-000B/V-003 remain fresh external evidence requirements; see Q009-POST-003 for version labeling. |

## Findings

### Q009-POST-001

Finding ID: Q009-POST-001  
Severity: HIGH  
Status: OPEN

Title: Rust `PreparedWrite` does not carry the qualified invocation capability

Evidence:

- `src/adapters/mod.rs`: `PreparedWrite` contains only plan, prewrite evidence,
  and optional file bytes.
- `src/http/mod.rs`: `ApiClient::dispatch_prepared(&self, url, prepared)` accepts
  a separately supplied client and base URL.
- Workflow, Skill, Datasource, and Assistant call sites supply those values
  separately.
- `contracts/http-adapter.md` requires dispatch to take the capability from the
  seal and accept no separately supplied client, origin/base URL, token, or
  session; `data-model.md` states the same invariant.

Expected: the same opaque capability used for qualification, resolution,
write-ability proof, and final revalidation is carried by `PreparedWrite` and
is the only capability dispatch can consume.

Actual: current call sites happen to reuse their variables, but the type/API
permits a replacement client or URL after evidence is sealed.

Impact: the security-critical same-origin/principal/session property is not
enforced by the approved boundary and cannot be mutation-tested as specified.

Required action: implementation engineer must make the capability part of the
sealed write boundary and remove separately supplied dispatch replacements.

Verification: compile-time/API inspection plus substitution mutations proving
no alternate client/origin/token/session can reach dispatch.

### Q009-POST-002

Finding ID: Q009-POST-002  
Severity: HIGH  
Status: OPEN

Title: V-000A authorizes the manifest actor by email instead of `user_id`

Evidence:

- `scripts/v000_target.py:943` compares `user["email"]` with
  `authorization["actor"]`.
- `tasks.md` requires the authenticated actor ID from `/v1/user.user_id` to
  equal `authorization.actor`; `plan.md` uses the same rule.
- Python fixtures use an email actor, so the test suite validates the obsolete
  behavior rather than detecting it.

Expected: exact actor equality is based on the authenticated `user_id`.

Actual: an email match qualifies and a correct user-ID actor does not.

Impact: V-000A evidence is bound to the wrong principal identifier and cannot
support the refreshed V-000B/V-003 authorization gate.

Required action: implementation engineer must bind to `user_id`, refresh the
manifest fixture/schema expectations, and add positive and mismatch tests.

Verification: a user-ID actor passes, an email-only actor fails before entity
resolution, and the failure emits only the approved safe category.

### Q009-POST-003

Finding ID: Q009-POST-003  
Severity: MEDIUM  
Status: OPEN

Title: V-000A evidence still labels its manifest version as 2

Evidence:

- `scripts/v000_target.py:41` defines `MANIFEST_VERSION = 2` and persists it.
- `contracts/adapter-manifest-v2.42.0.json` declares `manifestVersion: 3`.
- V-000A/V-000B require a refreshed manifest-version-bearing record and reject
  pre-v31 evidence.

Expected: generated evidence unambiguously identifies the refreshed v31
manifest contract.

Actual: successful evidence is labeled version 2.

Impact: operators and lifecycle checks cannot reliably distinguish refreshed
v31 qualification evidence from the prior format.

Required action: solution architect should confirm the evidence-version
namespace if intentionally independent; otherwise implementation engineer
must align it to version 3 and test the persisted value.

Verification: generated sanitized evidence reports the approved version and
all stale-version fixtures fail closed.

### Q009-POST-004

Finding ID: Q009-POST-004  
Severity: MEDIUM  
Status: OPEN

Title: Required Rust/Python mutation matrices are incomplete

Evidence: the focused suites pass, but do not prove every missing/null/empty/
wrong-type consumed field, every nested duplicate-key mutation, exact category
and zero-resolution behavior for each qualification negative, Assistant
exclusion, and client/origin/token/session substitution. Several new Python
tests assert only `QualificationError` without its category or request counts.

Expected: T-004/V-000A matrices in `tasks.md` explicitly cover each mutation
and exact fail-closed observations.

Actual: important branches are implemented, but the evidence does not meet the
specified mutation-oriented standard.

Impact: regressions in taxonomy, ordering, or capability binding can remain
green; Q009-POST-001 and Q009-POST-002 demonstrate this weakness.

Required action: implementation engineer adds the enumerated negative and
substitution cases without weakening existing assertions.

Verification: each named mutation independently asserts exact error, exit 2
where applicable, empty stdout/safe stderr, zero resolver calls before
qualification, and zero modifying calls.

## Validation performed

```text
cargo test --locked preflight_visibility -- --nocapture             PASS (11)
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests.test_v000_target PASS (29)
make format                                                         PASS
make lint                                                           PASS
```

The coordinating verification run also reported the stable-tree full gates as
passing (346 Rust, 9 CLI integration, and 51 Python tests); that report is
corroborating evidence, not a substitute for this agent's focused reproduction.

## Data, security, and operations assessment

- Data/migrations: not applicable; all proof state is invocation-local.
- Security: Q009-POST-001 and Q009-POST-002 are unresolved authorization-boundary
  defects and block verification.
- Operations: no live target was contacted. Existing live evidence is reset;
  V-000B/V-003 must be rerun only after corrected V-000A/T-004 verification.

## Files changed during verification

- `specs/codemie-cicd-tool/Q-009-verification-report.md` only.

## Final verdict rationale

The implementation correctly excludes Assistant, enforces exact personal
project/member cardinality, uses strict duplicate-aware decoding and a single
encoded project segment, orders the observed qualification before writes, and
keeps V-000A GET-only. It is nevertheless **NOT VERIFIED** because the approved
opaque capability seal is absent and the refreshed probe identifies the actor
using the wrong field. Return Q009-POST-001/002/004 to the implementation
engineer and Q009-POST-003's namespace decision to the solution architect;
repeat convergence verification before any fresh live evidence.
