# Verification report: v28 compatibility and zero-based pagination corrections

## Status

```text
Verification status: VERIFIED — READY FOR RELEASE REVIEW
```

Independent stable-worktree re-verification result: **PASS**. The delivered
T-003/R-001 and Q-008/W-001/S-001 corrections converge with the approved v28
specification, contracts, ADRs, plan, data model, and tasks for the scope below.
All three findings from the earlier Q-007 post-implementation reviews are now
resolved, and all independently executed focused and full repository gates
pass.

This verdict means sufficient implementation evidence exists for release
review. It does not authorize deployment or release, and it does not replace
V-000 live-target qualification, remote O-001 activation, or V-001/V-002
production-enablement evidence.

> **Q-008 pagination supersession:** The original Q-007 review's statement
> that Workflow and Skill pagination was one-indexed is permanently
> superseded. Pinned backend tag `2.42.0`, commit
> `2a481c290c99bf30ef80aadafa03d876a7f5f732`, and the verified implementation
> prove both are zero-indexed. Page 0 is the mandatory first request. See
> `Q-008-verification-report.md`.

## Scope verified

- Reopened T-003/R-001 v28 compatibility correction.
- Q-008/W-001/S-001 zero-based Workflow and Skill pagination correction.
- No runtime `/v1/info` compatibility gate or contact.
- Assistant strict direct lookup without `/v1/user`.
- Workflow, Skill, and Datasource exact-effective-project preflight.
- Strict consumed DTO fields, required-nullable members, write-ability
  classification, additive unconsumed fields, and pagination invariants.
- Actual completed-resolution evidence owned by `PreparedWrite`, the sole
  modifying dispatcher boundary, and zero-write ordering.
- Workflow exact marker, ordinary display-name guard, and explicit-adoption
  boundary.
- Initial, post-write, Workflow two-pass/adoption, and Skill create-409 scanner
  reuse.
- Reference-only tree integrity.

## Sources of truth

- Specification: `specs/codemie-cicd-tool.md` v28.
- Requirements: SC-021, FR-005/006/011/021/028–034/036, IR-011/012,
  PA-005, and VR-007–010/016.
- Acceptance criteria: AC-IR-011-01, AC-IR-012-01,
  AC-FR-028-01/02, AC-FR-029-01/02, AC-FR-030-01/02,
  AC-FR-031-01/02, AC-FR-033-01, and AC-FR-034-01.
- Plan: `plan.md`, especially sections 5, 6, 10, 13, and 16.
- Data model: `data-model.md`, especially sections 4, 5, and 7.
- Contracts: `contracts/adapter-manifest-v2.42.0.json`,
  `contracts/http-adapter.md`, and `contracts/source-baseline.md`.
- ADRs: ADR-004, ADR-007, ADR-008, and ADR-012.
- Tasks: Q-007/Q-008, reopened T-003, W-001, S-001, and R-001.
- Prior reviews: `Q-007-verification-report.md`,
  `Q-007-security-review.md`, and `Q-008-verification-report.md`.
- Provided Jira material: none.
- Provided Confluence material: none.

## Change set inspected

In-scope production and test files:

- `src/adapters/mod.rs`
- `src/adapters/assistant.rs`
- `src/adapters/workflow.rs`
- `src/adapters/skill.rs`
- `src/adapters/datasource.rs`
- `src/coordinator/mod.rs`
- `src/http/mod.rs`
- `src/main.rs`
- deleted `src/preflight/mod.rs`

In-scope specification and architecture files were the v28 specification,
ADR-007/008/012, adapter manifest, HTTP/source contracts, data model, plan,
research, tasks, and Q-007/Q-008 verification artifacts. Unrelated `Makefile`,
`ops/dev/`, and development-dependency helper changes were not used as proof.

## Validation performed

```text
cargo test --locked adapters::tests:: -- --nocapture
  PASS — 3 passed; 0 failed

cargo test --locked adapters::workflow::tests:: -- --nocapture
  PASS — 23 passed; 0 failed

cargo test --locked adapters::skill::tests:: -- --nocapture
  PASS — 14 passed; 0 failed

cargo test --locked adapters::datasource::tests::datasource_response_requires_consumed_fields_and_allows_additions -- --exact --nocapture
  PASS — 1 passed; 0 failed

cargo test --locked adapters:: -- --nocapture
  PASS — 66 passed; 0 failed

cargo test --locked coordinator::tests:: -- --nocapture
  PASS — 11 passed; 0 failed

cargo test --locked http::tests::preflight_visibility -- --nocapture
  PASS — 9 passed; 0 failed

cargo fmt --all -- --check
  PASS

make lint
  PASS — cargo clippy --all-targets -- -D warnings

cargo check --locked --all-targets
  PASS

make test
  PASS — 343 Rust unit tests, 9 CLI integration tests, and 16 Python
  operational-control tests

jq -e <additive policy, capability applicability/project predicate,
       Workflow/Skill page-base/page-size assertions>
       contracts/adapter-manifest-v2.42.0.json
  PASS — true

git diff --check
  PASS

git -C codemie{,-ui} rev-parse/describe/diff checks
  PASS — backend 2.42.0 at 2a481c290c99bf30ef80aadafa03d876a7f5f732;
         UI 2.42.0 at 55945d075d82e771c4a2f4238afec1eb4c79d1e1;
         no tracked reference-only edits
```

The backend reference checkout contains an unrelated untracked `mise.toml`.
It was not used as normative source evidence, modified, or treated as a
tracked reference-tree change.

## Acceptance-criteria matrix

| Criterion | Status | Code evidence | Executed test evidence |
|---|---|---|---|
| AC-IR-011-01 | SATISFIED | Coordinator does not call `/v1/info`; apply enters the kind adapter directly | Coordinator success/failure mocks require zero info contact |
| AC-IR-012-01 | SATISFIED | Strict DTOs, invariant checks, kind-specific evidence, private dispatcher | Isolated consumed-field/type matrices, exact-project preflight, additive/outbound equality, pagination and zero-write matrices pass |
| AC-FR-028-01/02 | SATISFIED | Strict marker codec and metadata merge/canonicalization | Marker, DTO, and projection tests pass |
| AC-FR-029-01/02 | SATISFIED | Exact target-project reserved-marker filtering; invalid/duplicate evidence fails closed | Both-scope marker, repeated-ID, ambiguity, drift, and exact-marker tests pass |
| AC-FR-030-01 | SATISFIED | Both scans precede by-ID adoption detail/ability validation and one PUT | Explicit-adoption success, candidate-fault, exact-marker veto, and same-name non-veto tests pass |
| AC-FR-030-02 | SATISFIED | Authored `spec.name` and marker slug are separate scanner inputs; display name never selects | `spec.name != slug` regression passes in project and marketplace scope with zero POST/PUT/PATCH/DELETE |
| AC-FR-031-01/02 | SATISFIED | Shared exact `(project,name)` zero-based Skill enumerator with no tie-break | Empty, one, multiple, 101-item, drift, repeat-ID, post-write, and 409 tests pass |
| AC-FR-033-01 | SATISFIED | Workflow/Skill/Datasource prepared writes own exact-project evidence; Assistant is excluded | Positive/other-project and strict preflight tests pass |
| AC-FR-034-01 | SATISFIED | Coordinator uses kind-specific post-write verifier; Workflow/Skill verifiers reuse the initial scanners | Direct page-0 post-write tests and coordinator success/may-have-committed request-count tests pass |
| R-001 sealed transition | SATISFIED | Non-zero-sized, kind-specific completed evidence is owned by private `PreparedWrite`; raw methods are private | Non-ZST, sole-boundary, all-method zero-write, adapter, and coordinator tests pass |

## Requirements coverage

| Requirement | Status | Evidence |
|---|---|---|
| IR-011 | SATISFIED | No runtime info/SHA gate or contact; matching and mismatching info cannot influence apply |
| IR-012 | SATISFIED | Strict full consumed DTO and pagination matrices, additive-only tolerance, and sealed no-write ordering |
| FR-029 | SATISFIED | Exact marker/project resolution and strict fail-closed parsing across both scopes |
| FR-030 | SATISFIED | Authored-name ordinary guard and exact by-ID explicit adoption semantics |
| FR-031 | SATISFIED | Zero-based exhaustive Skill resolution and exact filtering |
| FR-033 | SATISFIED | Exact-project visibility plus item/detail write ability evidence |
| FR-034 | SATISFIED | Shared zero-based post-write scanners and safe may-have-committed taxonomy |

## Architecture conformance

- `PreparedWrite` contains private `PrewriteEvidence` with kind-specific
  `CompletedResolution`; the values carry actual project/key/target,
  scan/reference/detail/ability evidence, and File bytes where applicable.
- All completed-resolution types and `PreparedWrite` are non-zero-sized.
- Plan, resolution kind/project/target, visibility proof, and server ID must
  agree before a request is released.
- Target API JSON and multipart POST/PUT primitives are private inside the HTTP
  module. `dispatch_prepared` is the sole production modifying boundary.
- Workflow and Skill scanners start at page 0. `pages == 0` stops after the
  one origin request; otherwise traversal is exactly `0..pages-1`.
- Workflow initial resolution, both passes, adoption precheck, and post-write
  verification call `enumerate_all`. Skill initial resolution, post-write
  verification, and create-409 re-resolution call `enumerate`.
- Workflow marker classification uses only `(project, slug)`. The ordinary
  legacy guard separately uses exact authored `spec.name`; explicit adoption
  ignores unrelated unmarked same-name rows and vetoes an existing exact marker.
- Strict consumed fields have no Serde defaults. Workflow `meta_config` and
  Skill `created_by` preserve required-member/nullable-value semantics.
  Additive unknown response members do not widen projected requests.

No undocumented deviation, architecture violation, migration, or new
operational obligation was found in the verified scope.

## Test quality assessment

The final suite provides mutation-relevant evidence for the corrected risks:

- each consumed Workflow/Skill/Datasource entity and pagination member is
  independently removed and type-corrupted from an otherwise-valid DTO;
- required-nullable members are present with null baselines and fail when
  removed or changed to a non-null invalid shape;
- additive response members decode while captured projected request bodies
  remain exactly equal;
- Workflow verifies a 101-item page-0/page-1 sequence and exactly one empty
  marketplace page-0 request; Skill verifies both multi-page initial and
  single-page post-write traversal;
- invalid post-write origins make no modifying call; Skill 409 performs one
  POST, reuses the page-0 scanner, and performs no second write on failure;
- coordinator tests prove page-0 origin on initial and post-write reads,
  no `/v1/info` contact, exact one-write success, and may-have-committed failure
  without a second write; and
- boundary source tests would fail if raw modifying APIs became visible or an
  adapter bypassed the evidence-bearing dispatcher.

No relevant skipped or flaky test was observed. The unexecuted live-target
probe remains a separately owned deployment-verification obligation.

## Findings

### Finding ID: V28-POST-001

Severity: HIGH
Status: RESOLVED

Title: `PreparedWrite` owns actual kind-specific completed evidence and is the
sole production modifying boundary.

Evidence: private non-zero-sized completed-resolution/evidence types,
kind/project/target checks, private raw target modifying primitives, and the
passing non-ZST/sole-boundary tests.

Expected: only a sealed aggregate containing operation-specific preflight,
completed reads, and projection can reach POST/PUT.

Actual: the type and module boundaries now enforce that transition.

Impact: the earlier fabricated/empty seal risk is closed.

Required action: none; retain the focused boundary tests.

Owner: implementation-engineer

Verification: rerun the boundary, Clippy, and full-suite gates on future HTTP
boundary changes.

### Finding ID: V28-POST-002

Severity: MEDIUM
Status: RESOLVED

Title: the mandatory compatibility, pagination-reuse, and zero-write evidence
matrix is now sufficient for the v28 correction.

Evidence: isolated DTO mutations, additive/outbound equality, zero-based empty
and 101-item sequences, direct Workflow/Skill post-write tests, Skill 409
reuse/fault tests, coordinator page-origin/no-info/request-count tests, and
all-method zero-write assertions all pass.

Expected: removal or corruption of a required field, page-0 origin, reuse
path, ability check, or write prerequisite must be detected before an
unauthorized modifying transition or success.

Actual: the final tests exercise those controls independently at their owning
boundaries, while the private dispatcher provides the cross-adapter backstop.

Impact: the material evidence gaps from the earlier review are closed.

Required action: none for this finding; preserve the focused matrices.

Owner: implementation-engineer

Verification: rerun focused adapters/coordinator tests and the full gates after
contract, DTO, pagination, or dispatcher changes.

### Finding ID: V28-POST-003

Severity: HIGH
Status: RESOLVED

Title: Workflow ordinary reconciliation now compares unmarked display name
with authored `spec.name`, not slug.

Evidence: the resolver passes authored `spec.name` separately from slug;
marker classification consumes the slug only, while the unmarked guard
consumes the authored display name only. The two-scope regression passes with
`spec.name != metadata.slug`, exit 1, and zero POST/PUT/PATCH/DELETE. Explicit
adoption same-name non-veto and existing-exact-marker veto tests also pass.

Expected: an unmarked target-project row matching exact authored display name
blocks ordinary create without selecting it; explicit adoption remains by ID.

Actual: implementation and regression evidence now match that requirement.

Impact: the duplicate-Workflow/adoption-ceremony bypass is closed.

Required action: none; retain both-scope and adoption-boundary regressions.

Owner: implementation-engineer

Verification: rerun the Workflow focused suite on marker/name/adoption changes.

## Unverified areas

- Live target V-000 non-empty page-0 compatibility probes.
- Remote O-001 controls, external writer governance, and deployment-specific
  configuration.
- Release packaging, deployment, rollback, and production monitoring evidence.

These areas are outside the bounded implementation-verification verdict and
remain owned by their downstream lifecycle tasks.

## Files changed during verification

- Updated this verification-owned report only.
- No production Rust, reference-only source, contract, or architecture source
  was modified by the verifier.

## Release-review focus

The release engineer should confirm the downstream dependency state rather
than reinterpret this code verdict: V-000 live target qualification, O-001
remote activation/writer serialization, V-001/V-002 production-enablement
evidence, reproducible locked packaging, deployment configuration, monitoring,
and rollback readiness. No release, publish, tag, merge, or deploy action is
authorized by this report.

## Final verdict rationale

The two previous HIGH implementation defects are resolved in both code and
mutation-relevant tests, and the previous MEDIUM evidence gap is closed for
the approved v28 correction. The sealed boundary owns real evidence; strict
DTO, visibility, ability, pagination, marker, adoption, post-write, and 409
behaviors converge with the specification; all focused and full gates pass;
and no tracked reference-only edit exists. The implementation is therefore
**VERIFIED — READY FOR RELEASE REVIEW** for this bounded scope.
