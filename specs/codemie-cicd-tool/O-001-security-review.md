# Security review report: O-001 serialization and identity-writer governance

## Status

```text
Security review status: APPROVED FOR NEXT STAGE
Checked-in control disposition: READY FOR REMOTE ACTIVATION
Production activation status: BLOCKED ON EXTERNAL EVIDENCE
Release/deployment authorization: NOT GRANTED
```

- Review date: 2026-08-11
- Review mode: Mode C — implementation security review
- Reviewed baseline: uncommitted O-001 changes on commit
  `5c04e32c96ce050e832c941c5fbd87d7ccae6df5`

The checked-in O-001 controls contain no open critical, high, or medium security
finding. They may proceed to provider activation. This approval does not mark
O-001 complete and does not authorize a production apply, release, or
deployment. Production activation remains blocked until the external evidence
listed in this report is collected and passes the checked-in evidence gate.

## Scope

The review covered only the O-001 implementation:

- `.github/workflows/codemie-gitops-apply.yml`;
- the O-001 changes in `.github/workflows/ci.yml`;
- `.gitlab-ci.yml`;
- `ops/o001/**`;
- `scripts/check_o001_controls.py` and `scripts/o001_inventory.py`;
- `tests/test_o001_controls.py`; and
- the related `Makefile` and `.gitignore` changes.

The governing requirements were FR-033/034, PA-005/006, QR-010/011,
AC-QR-010-01, the SEC-006 CI trust boundary in ADR-005, and ADR-012's extension
of serialization, duplicate inventory, and ambiguity remediation to
Datasource.

Remote provider configuration, target CodeMie configuration, actual CI runner
provisioning, deployment authorization, O-002, and release evidence are outside
the locally verifiable scope. The reference-only `codemie/` and `codemie-ui/`
directories were not modified.

## Sources of truth and documentation check

- Product specification: `specs/codemie-cicd-tool.md` v27.
- Architecture and task ownership: `plan.md`, `tasks.md`, ADR-005, and ADR-012.
- Prior implementation security review: `Q-005-security-review.md`.
- Operational implementation: the O-001 files listed above.
- GitHub concurrency semantics: current official GitHub Actions documentation,
  checked on 2026-08-11 through Context7 library
  `/websites/github_en_actions` and a direct fetch of GitHub's official
  concurrency documentation.
- GitLab serialization semantics: current official GitLab CI/CD resource-group
  documentation used during implementation and verification.
- Jira/Confluence material: none was provided locally or in the review context.

The initial concern that GitHub did not support `concurrency.queue` was based on
stale documentation and is retracted. Current official documentation confirms
that `queue: max` is supported and retains up to 100 pending runs. The invalid
combination is `queue: max` with `cancel-in-progress: true`; the checked-in
workflow does not configure that combination.

## Executive assessment

O-001 now establishes a fail-closed checked-in control boundary for production
identity writes:

- GitHub and GitLab serialize modifying work by a target-environment mutex.
- Unprivileged jobs lint, test, build, smoke, and checksum the exact musl
  artifact later consumed by protected apply jobs.
- Protected apply jobs do not run Cargo or repository Python.
- GitHub exposes CodeMie secrets only to the final apply step.
- GitLab uses static production/staging environment and resource-group names,
  protected default-ref rules, explicit runner tags, and external evidence for
  exact environment-scoped secrets and restricted pipeline-variable overrides.
- Workflow, Skill, and Datasource inventory is strict, bounded, non-destructive,
  and designed not to retain server IDs, payloads, credentials, URLs, or bodies.
- Concurrent-writer and uncertain-write response remains manual and prohibits
  retry, delete, rollback, and arbitrary tie-breaking.

The repository deliberately reports `activationStatus: required`. It does not
claim that provider settings, runners, credentials, mutex behavior, inventory,
or writer freezes have been activated remotely.

## Assets, actors, and trust boundaries

The primary protected assets are the CodeMie production token, target URL,
remote Workflow/Skill/Datasource identity integrity, the reserved Workflow
identity record, the promoted binary, inventory evidence, and availability of
the per-environment apply queue.

Relevant actors are Release Engineering, the CodeMie Platform Administrator,
the CodeMie Platform Identity Administrator, the Operations Incident Commander,
repository contributors, CI runners, CI providers, the artifact store, and the
CodeMie service principal.

The material trust crossings are repository source to an unprivileged build
runner, build output to the CI artifact store, the exact artifact to a protected
apply runner, protected environment secrets to the final write boundary, and
complete-visibility platform inventory to external change-control evidence.

## Finding disposition

| Finding | Final status | Evidence |
|---|---|---|
| SEC-O001-001 — GitHub `queue: max` unsupported | **RETRACTED** | Current official GitHub documentation confirms the syntax and 100-pending-run bound. The workflow does not combine it with `cancel-in-progress: true`. |
| SEC-O001-002 — deployment jobs rebuild while privileged | **RESOLVED** | Separate unprivileged build/test jobs create the exact artifact. Protected jobs verify its SHA-256 checksum and execute it without Cargo or repository Python. GitHub upload/download actions are pinned to full commit SHAs. |
| SEC-O001-003 — dynamic GitLab environment/ref can bypass protection and mutex identity | **RESOLVED** | Static `production`/`staging` environments and resource groups, protected-default-ref rules, manual production apply, fixed protected runner tags, and activation evidence for exact secret scope and override restrictions. |
| SEC-O001-004 — textual or ambiguous control/evidence parsing can report a false pass | **RESOLVED** | The validator accepts only the checked-in canonical YAML subset and rejects plain, nested, quoted, quoted-space, explicit, flow, tag, merge, comment-spoof, and duplicate-key alternatives. Policy, activation evidence, and inventory JSON reject duplicate object names at every nesting level and reject `NaN`/`Infinity`. |
| SEC-O001-005 — operational snapshots can be committed accidentally | **RESOLVED** | Narrow ignored local paths exist for activation evidence and inventory snapshots and are documented in the checklist. |

Additional independently discovered cases are also closed:

- Workflow marker, policy, evidence, and gate versions require an actual integer
  `1`; Python booleans are rejected.
- Provider mutex-drill and artifact-promotion references must be pairwise
  distinct.
- The different environment must differ from production, and GitLab's
  independent drill environment is statically `staging`.
- Freeze and inventory evidence references must be distinct.
- Inventory input cannot hide an invalid earlier object through a duplicate
  top-level collection or duplicate nested marker member.

## Security requirements coverage

| Requirement | Checked-in status | Evidence |
|---|---|---|
| FR-033 / PA-005 complete-visibility prerequisite | SATISFIED LOCALLY; REMOTE PROOF REQUIRED | Fail-closed CI preflight, complete-visibility inventory procedure, activation evidence fields. |
| FR-034 / QR-011 uncertain-write behavior | SATISFIED | Named runbooks prohibit automatic retry, delete, rollback, or tie-break and require a clean re-inventory before resuming. |
| PA-006 reserved marker governance | SATISFIED LOCALLY; REMOTE INVENTORY REQUIRED | Strict Workflow marker inventory, named platform owner, manual remediation path. |
| QR-010 / AC-QR-010-01 provider serialization | SATISFIED BY CONFIGURATION; REMOTE DRILL REQUIRED | GitHub environment concurrency and static GitLab resource groups with same/different-environment drill jobs. |
| ADR-012 Datasource extension | SATISFIED LOCALLY; REMOTE INVENTORY REQUIRED | Datasource writer freeze, invalid/duplicate inventory, and uncertain-write remediation are included beside Workflow and Skill. |
| SEC-006 CI trust boundary | SATISFIED FOR O-001 CHECKED-IN SCOPE | Locked build, SHA-pinned actions, read-only permissions, secret isolation, protected apply boundary, exact-artifact promotion, and checksum-before-execution. |

## Validation performed

The final review and independent verification recorded:

- `make lint` — PASS with Clippy warnings denied;
- `make test` — PASS: 310 Rust unit tests, 9 Rust CLI integration tests, and
  16 O-001 Python tests;
- `make o001-check` — PASS with `local-pass` and
  `activationStatus: required`;
- valid GitHub/GitLab YAML syntax and valid embedded shell syntax;
- canonical YAML negative probes for plain, quoted, quoted-space, explicit,
  flow, tag, merge, commented, root, job, and nested override forms — all
  rejected;
- policy/evidence JSON duplicate-name and non-standard constant probes — all
  rejected;
- inventory stdin/file probes for a hidden invalid first `workflows` member,
  nested duplicate marker version, `NaN`, and `Infinity` — all rejected with
  exit 2, empty stdout, and the generic safe stderr diagnostic;
- no credential value detected by the focused changed-file secret scan; and
- `git diff --check` — PASS.

The existing local release artifact is a static PIE and the Rust test suite ran
for the configured musl target. The local machine does not provide `musl-gcc`,
so the external GitLab build-runner prerequisite was not reproduced locally.
The checked-in GitLab job fails closed unless its tagged runner provides both
`musl-gcc` and the literal `x86_64-unknown-linux-musl` Rust target.

## External activation blockers

O-001 production activation requires evidence for all of the following:

1. GitHub protected environments, required reviewers, and deployment-branch
   restrictions.
2. GitLab protected `production` and `staging` environments, required approval,
   protected default ref, and deployment-branch policy.
3. A provisioned `codemie-gitops-musl-build` runner and a protected
   `codemie-gitops-production-protected` apply runner.
4. Protected/masked GitLab `CODEMIE_TOKEN` and `CODEMIE_URL` variables scoped
   exactly to `production`, plus restricted pipeline-variable overrides.
5. Two same-environment mutex drills with no overlap and a different-environment
   drill proving key independence on both providers.
6. A complete-visibility Workflow, Skill, and Datasource inventory with no
   invalid marker or duplicate natural key.
7. An approved freeze of UI, scripts, and other API identity writers.
8. The first separately authorized apply's exact artifact-transfer and checksum
   evidence, using a reference distinct from all mutex-drill references.
9. Completed external activation evidence passing
   `scripts/check_o001_controls.py --activation-evidence`.

No configured Git remote or completed activation evidence was available during
the review. Absence of this evidence is not treated as proof that the controls
are active.

## Accepted risks

None. No risk was accepted on behalf of the user or an adopting organization.

## Next action

Release Engineering owns remote activation using `ops/o001/CHECKLIST.md`, with
the Platform Administrator, Identity Administrator, and Incident Commander
performing their named controls. O-001 must remain operationally incomplete
until the external evidence gate passes. A release engineer must separately
assess release readiness; this report does not authorize release or deployment.
