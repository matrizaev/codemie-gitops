# Verification report: O-001 serialization and identity-writer governance

## Status

```text
Checked-in/local controls: VERIFIED — READY FOR REMOTE ACTIVATION
Security review: APPROVED FOR NEXT STAGE
Production activation: BLOCKED ON EXTERNAL EVIDENCE
```

O-001 is not operationally complete. The repository controls converge on the
approved specification and architecture, but no remote provider, CodeMie
environment, privileged inventory, writer freeze, or production artifact
transfer was changed or verified by this work.

Blocking checked-in findings: 0

High checked-in findings: 0

Medium checked-in findings: 0

## Scope verified

- Task: `specs/codemie-cicd-tool/tasks.md` O-001
- Requirements: FR-033, FR-034, PA-005, PA-006, QR-010, QR-011
- Acceptance criterion: AC-QR-010-01
- Architecture extension: accepted ADR-012 Datasource governance obligation
- Checked-in controls:
  - GitHub Actions per-environment concurrency
  - static GitLab production/staging resource groups
  - exact-artifact promotion and credential isolation
  - Workflow, Skill, and Datasource inventory analysis
  - external-writer freeze preflight
  - concurrent-writer and uncertain-write remediation
  - provider activation evidence validation
- Exclusions:
  - remote provider activation or configuration changes
  - a live CodeMie inventory or write
  - deployment, release, publication, tagging, or commit creation
  - reference-only `codemie/` and `codemie-ui/` contents

No Jira or Confluence material was provided or consulted.

## Sources of truth

- Product specification: `specs/codemie-cicd-tool.md` v27
- Architecture plan: `specs/codemie-cicd-tool/plan.md`
- Tasks: `specs/codemie-cicd-tool/tasks.md`
- Supply-chain and CI controls: ADR-005 amendment
- Workflow identity: ADR-008
- Datasource visibility/governance extension: accepted ADR-012
- O-001 policy and runbook:
  - `ops/o001/control-policy.json`
  - `ops/o001/CHECKLIST.md`
- Provider controls:
  - `.github/workflows/codemie-gitops-apply.yml`
  - `.gitlab-ci.yml`
- Validation implementation:
  - `scripts/check_o001_controls.py`
  - `scripts/o001_inventory.py`
  - `tests/test_o001_controls.py`

Current official GitHub Actions documentation was consulted through Context7.
It confirms that `concurrency.queue: max` is valid current syntax and supports
up to 100 pending runs. The earlier claim that this key was unsupported was
retracted. Current GitLab documentation confirms that `resource_group` is a
cross-pipeline semaphore; the final pipeline uses static resource-group names
and does not depend on variable expansion for its protected apply boundary.

## Change set inspected

- `.github/workflows/codemie-gitops-apply.yml`
- `.github/workflows/ci.yml`
- `.gitlab-ci.yml`
- `.gitignore`
- `Makefile`
- `ops/o001/CHECKLIST.md`
- `ops/o001/control-policy.json`
- `ops/o001/activation-evidence.example.json`
- `ops/o001/fixtures/inventory-clean.json`
- `ops/o001/fixtures/inventory-broken.json`
- `scripts/check_o001_controls.py`
- `scripts/o001_inventory.py`
- `tests/test_o001_controls.py`

No Rust source file or tracked reference-only file changed.

## Validation performed

Final recorded validation evidence:

```text
make format       PASS
make lint         PASS — Clippy warnings denied
make test         PASS — 310 Rust unit + 9 Rust integration + 16 O-001 Python
make o001-check   PASS — local-pass; activationStatus: required; 16/16 tests
git diff --check  PASS
YAML parse        PASS
CI shell syntax   PASS — 17 script blocks
reference check   PASS — no tracked codemie/ or codemie-ui/ change
cache check       PASS — no __pycache__ or *.pyc under scripts/ or tests/
```

Focused negative evidence also passed:

- the original duplicate top-level `workflows` inventory bypass is rejected
  with exit code 2 and a generic diagnostic;
- boolean Workflow marker and schema versions are rejected;
- duplicate JSON member names at any nesting level are rejected;
- non-standard JSON `NaN` and `Infinity` values are rejected;
- duplicate provider YAML keys are rejected at root, job, and nested step
  mappings;
- quoted keys, a space before the mapping colon, explicit mappings, flow
  mappings, tags, merge keys, and comment-fragment substitutions cannot spoof
  the checked-in CI controls;
- dynamic GitLab environment aliases are rejected;
- repeated provider drill/promotion references and repeated freeze/inventory
  references are rejected;
- the incomplete activation-evidence template cannot pass.

## Acceptance-criteria and requirements matrix

| Requirement or criterion | Checked-in status | Evidence | Production status |
|---|---|---|---|
| FR-033 / PA-005 | SATISFIED | Privileged inventory prerequisite, CI preflight, named owner, fail-closed policy | NOT VERIFIED — live complete-visibility inventory absent |
| FR-034 / QR-011 | SATISFIED | Uncertain-write runbook; no automatic retry, delete, rollback, or tie-break | NOT VERIFIED — no live incident drill |
| PA-006 | SATISFIED | Strict Workflow marker inventory, invalid/duplicate gates, manual remediation ownership | NOT VERIFIED — live marker inventory absent |
| QR-010 | SATISFIED | GitHub environment concurrency; static GitLab production/staging resource groups; protected-ref rules | NOT VERIFIED — provider mutex drills absent |
| AC-QR-010-01 | SATISFIED for checked-in examples | Same-environment collision and different-environment independence encoded and tested | NOT VERIFIED — remote observations absent |
| ADR-012 extension | SATISFIED | Datasource included in writer governance, inventory, and remediation | NOT VERIFIED — live Datasource inventory absent |

## Architecture and security conformance

### Exact-artifact and credential boundary

- GitHub's unprivileged `prepare` job performs preflight, lint, tests, locked
  musl release build, smoke check, checksum generation, and immutable-SHA
  artifact upload.
- The protected GitHub job downloads that exact artifact and verifies its
  SHA-256 checksum. `CODEMIE_URL` and `CODEMIE_TOKEN` are referenced only by
  the final binary execution step.
- GitLab's unprivileged build job is pinned to the literal
  `x86_64-unknown-linux-musl` target and fails closed unless its tagged runner
  supplies that target and `musl-gcc`.
- The protected GitLab apply job consumes the same-pipeline artifact, verifies
  its checksum, and does not execute Cargo, repository Python, or tests while
  production credentials are resident.

This closes the prior same-artifact promotion and build-with-secrets findings
against ADR-005.

### Provider and identity governance

- GitLab environment and resource-group names are static, so a pipeline
  variable cannot create an alias with a separate mutex.
- Protected default-ref rules, manual approval, protected runner tags, exact
  environment-scoped secret evidence, and restricted pipeline-variable
  overrides fail closed in the checked-in contract.
- Workflow, Skill, and Datasource inventory gates require exact natural keys,
  complete visibility, zero invalid records, and zero duplicate groups.
- Concurrent-writer and uncertain-write procedures have named owners and
  prohibit automatic destructive remediation.

Security re-review status is **APPROVED FOR NEXT STAGE**. This approval applies
to the checked-in controls only and does not authorize deployment.

## Closed finding summary

| Finding | Final status | Closure evidence |
|---|---|---|
| Unsupported GitHub `queue: max` claim | RETRACTED | Current official documentation confirms support |
| Rebuild in protected apply job / secret exposure | CLOSED | Unprivileged build/test plus promoted checksummed artifact |
| GitLab dynamic target/protection bypass | CLOSED | Static environments/resource groups and protected default-ref contract |
| Boolean marker/schema version accepted | CLOSED | Strict integer type checks and negative tests |
| Reused activation references accepted | CLOSED | Pairwise-distinct references and different-environment validation |
| GitLab approval/runner/secret-scope evidence incomplete | CLOSED | Required fields and negative tests |
| Textual YAML validation spoofable | CLOSED | Canonical structural subset and duplicate/alternative-syntax rejection |
| JSON duplicate names and non-standard constants accepted | CLOSED | Strict loaders for controls, evidence, fixtures, and inventory input |
| Operational snapshots not narrowly ignored | CLOSED | Documented local-only paths in `.gitignore` |

No new checked-in blocker, high, or medium finding remains.

## Production activation blockers

Production activation remains blocked until Release Engineering and the named
CodeMie platform roles produce and validate all of the following external
evidence:

1. GitHub and GitLab protected environments exist with required approvals and
   deployment-branch restrictions.
2. The GitLab default deployment ref is protected; pipeline-variable overrides
   are restricted to the approved deployment role.
3. The tagged GitLab musl build runner provides `musl-gcc` and the installed
   `x86_64-unknown-linux-musl` Rust target.
4. The protected GitLab apply runner is restricted to protected refs and has
   only the trusted runtime/checksum utilities required by the apply boundary.
5. GitLab `CODEMIE_URL` and `CODEMIE_TOKEN` variables are protected/masked and
   scoped exactly to `production`; GitHub environment secrets are configured
   only on the corresponding protected environment.
6. Two same-environment GitHub runs and two production GitLab drill jobs prove
   that their 30-second controlled sections do not overlap.
7. A different-environment GitHub drill and the static GitLab staging drill
   prove that their mutex keys are independent.
8. A complete-visibility Workflow, Skill, and Datasource inventory reports
   zero invalid identity records and zero duplicate natural keys; every
   unmarked Workflow has an explicit reviewed disposition.
9. UI, scripts, and other API identity writers are frozen for the deployment
   window with distinct inventory and freeze evidence.
10. The first separately authorized apply records the exact promoted artifact
    reference and confirms its checksum before execution.
11. The completed external activation record passes
    `scripts/check_o001_controls.py --activation-evidence <external-path>`.

No Git remote is configured in this workspace, and the local host does not
provide `musl-gcc`. Therefore none of the remote provider settings, tagged
runner prerequisites, live mutex observations, privileged inventory, writer
freeze, or first-apply artifact transfer can be inferred from local test
success.

## Files changed during verification

- `specs/codemie-cicd-tool/O-001-verification-report.md` — this report only

## Final rationale

The checked-in O-001 controls are deterministic, fail closed, trace to the
approved requirements, include the ADR-012 Datasource extension, and have
meaningful positive and negative regression evidence. They are verified and
security-approved for the next lifecycle stage: remote activation.

O-001 must not be marked operationally complete, released, or deployed based
on this report. Production activation remains blocked until every external
requirement above is evidenced and the external activation gate passes.
