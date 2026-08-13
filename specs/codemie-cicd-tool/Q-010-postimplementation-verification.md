# Verification report: v32.2 T-005 replacement implementation

## Status

```text
Verification status: VERIFIED — READY FOR RELEASE REVIEW
```

## Scope verified

Post-implementation convergence of T-005 against product v32.2, the refreshed
plan/model/contracts/ADRs, Q-010, and SEC-010. Inspected the changed Rust HTTP,
coordinator, projection, and four adapter modules plus their tests. The pinned
`codemie/` trees were reference-only. V-000C, live V-000B/V-003, network,
`.env`, deployment, and release approval were excluded by their later task
gates.

## Executive assessment

The core v32 replacement is substantially implemented. Rust authorization uses
strict exact membership without project detail or administration, exact row
`write` gates updates/adoption, Assistant remains direct, Workflow uses the v2
creator marker and explicit adoption, Skill target/reference filtering is
creator-scoped, Datasource JSON and multipart creation treat 409 as terminal,
and all successful writes pass through a sealed same-client dispatcher followed
by exact verification.

V32.2 resolves Skill create 409 consistently: one POST may be followed by
exactly one page-0-origin same-creator read-only classification scan, never a
second write. The implementation and focused zero/one/multiple/incompatible
tests conform. The multipart Datasource 409 test also proves one POST and no
PUT/PATCH/DELETE.

The final SEC-010 evidence gap is closed by executable target and inline
reference matrices covering foreign-only, foreign-plus-own, own-only, and
multiple-own results with exact POST/PUT/PATCH/DELETE counts and selected IDs.
No open blocking, high, or medium finding remains in the T-005 scope.

## Change set inspected

- `src/http/mod.rs`
- `src/adapters/{mod,assistant,workflow,skill,datasource}.rs`
- `src/coordinator/mod.rs`
- `src/projection/mod.rs`
- relevant Rust unit/integration tests and `tests/test_v000_target.py`
- v32.2 specification, plan, model, contracts, ADR-014–018, tasks, Q-010, and
  `SEC-010-security-review.md`

## Validation performed

```text
cargo test --locked (coordinating root run)              PASS (348 unit + 9 integration)
Python V-000/O-002 gates (coordinating root run)          PASS (49/O-002)
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  tests.test_v000_target                                 PASS (27 reproduced)
make format                                              PASS
make lint                                                PASS
git diff --check                                         PASS
make test                                                NOT REPRODUCIBLE in second run
```

This verifier's focused Rust rerun reached 11 passes and 13 failures because mockito
could not create local servers (`Operation not permitted`). This is an
environment/resource failure: the immediately preceding identical Rust suite
completed all 353 tests successfully. No product assertion failure was
observed, but the failed command is recorded rather than reported as passing.

## Acceptance and control matrix

| Criterion/control | Status | Evidence |
|---|---|---|
| No v31 project-detail/admin gate in Rust apply | SATISFIED | `UserResponse` consumes only `user_id` and `projects[].name`; no `/v1/projects` production route; exact membership tests pass. |
| Membership-qualified create for all kinds | SATISFIED | All four adapters call the same preflight before resolution/seal; create tests pass. |
| Exact entity `write` for update/adoption | SATISFIED | `prove_write` is retained in concrete resolution evidence; negative Assistant/Workflow/Skill/Datasource tests assert no PUT. |
| Assistant exact lookup and post-write read | SATISFIED | Direct slug/project endpoint only; consumed-field and write tests pass. |
| Skill creator-scoped target/reference | SATISFIED | `target_creator_scope_request_count_matrix` and `inline_skill_reference_creator_scope_request_count_matrix` cover foreign-only, foreign-plus-own, own-only, and duplicate-own cases; they assert exact selected IDs/actions and zero unintended mutations. |
| Workflow v2 and legacy adoption | SATISFIED | Exact current-creator v2 decoding, marker insertion, zero-v2 adoption check, same creator/project/write, preservation, and ambiguity tests pass. |
| Datasource visible miss and authoritative 409 | SATISFIED | JSON behavior and `file_multipart_409_is_one_post_with_no_followup_request` prove terminal collision, body discard, and no later mutation. |
| Strict consumed decoding | SATISFIED | Duplicate-aware JSON decoding plus missing/null/empty/wrong-type tests; additive fields remain ignored. |
| Same client/session and zero pre-gate writes | SATISFIED | `PreparedWrite` privately owns the `ApiClient` reference, proof, resolution, plan, and multipart bytes; dispatch accepts only this aggregate. |
| Post-write ambiguity/uncertainty | SATISFIED | Coordinator always exact-verifies after success and converts compatibility/connectivity/reconciliation failures to may-have-committed classes without compensation. |
| Safe diagnostics/evidence | SATISFIED for T-005 Rust boundary | Closed renderer and empty-stdout tests pass; response bodies are drained/discarded on collisions. V-000C remains a later reset task. |

## Findings

### Q010-POST-001

Finding ID: Q010-POST-001  
Severity: BLOCKER  
Status: CLOSED

Title: Skill create-409 behavior converged in v32.2

Evidence:

- Product IR-007/AC-IR-007, HTTP contract, plan/model, ADR-007/017, and tasks
  now require one read-only creator-scoped classification scan.
- `create_with_reresolution` implements zero/one/multiple classification with
  no second POST or PUT.
- Focused tests cover one match, zero/multiple, and incompatible re-resolution.

Closure: source, architecture, implementation, and tests now agree.

### Q010-POST-002

Finding ID: Q010-POST-002  
Severity: MEDIUM  
Status: CLOSED

Title: SEC-010 Skill creator-scope matrices are executable

Evidence:

- `target_creator_scope_request_count_matrix` proves foreign-only target create,
  foreign-plus-own and own-only exact update, duplicate-own ambiguity, exact
  target route, and zero unintended PATCH/DELETE.
- `inline_skill_reference_creator_scope_request_count_matrix` proves
  foreign-only unresolved, foreign-plus-own/own-only exact own-ID resolution,
  duplicate-own ambiguity, and zero POST/PUT/PATCH/DELETE.
- `file_multipart_409_is_one_post_with_no_followup_request` separately closes
  the multipart portion of the original finding.

Closure: SEC-010 control 4 and T-005's inline `skillRefs` matrix now have direct
deterministic evidence.

## Data, architecture, security, and operations

- Data migration: no database migration. Workflow v1/unmarked replacement is
  explicit adoption, not bulk rewrite.
- Architecture: conforming. Several test names/comments
  still mention roles/ADR-012, but observed production logic uses membership.
- Security: SEC-010's required T-005 matrices are now represented by executable
  negative/request-count evidence; no authorization bypass was observed.
- Operations: V-000C and fresh live/serialization evidence remain downstream;
  prior v31 evidence is not accepted by this verification.

## Files changed during verification

- `specs/codemie-cicd-tool/Q-010-postimplementation-verification.md` only.

## Final verdict rationale

V32.2 closes the Skill-409 conflict and both final creator-scope matrices now
directly prove the SEC-010 target/reference boundary. The implementation,
tests, contracts, migration model, and safe operational handoff converge for
T-005. This verdict permits release review; it does not approve deployment or
replace the fresh V-000C/V-000B/V-003 and operational evidence gates.
