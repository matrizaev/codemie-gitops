# Artifact analysis report: v32 member creation and creator-scoped reconciliation

## Status

```text
Artifact analysis status: READY FOR IMPLEMENTATION
```

## Scope

- Product specification: `specs/codemie-cicd-tool.md`, v32 plus the v32.1
  Skill acceptance-criteria clarification.
- Architecture: `plan.md`, `data-model.md`, ADR-014 through ADR-018, and the
  explicitly retained portions of ADR-007 through ADR-009.
- Contracts: `contracts/http-adapter.md`, the v3 adapter manifest, and
  `contracts/source-baseline.md`.
- Work planning: `tasks.md`, section 0 v32 replacement gates.
- Pinned source: local reference-only CodeMie 2.42.0 at commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- Excluded: implementation, live/network evidence, `.env`, deployment/release
  authorization, and reference-tree changes. No Jira or Confluence material
  was provided.

## Executive assessment

The corrected artifacts now form a consistent, testable chain for exact
membership-qualified creation, exact-entity `write` authorization, Assistant
direct lookup, Workflow creator-scoped v2 identity and explicit legacy
adoption, Skill creator-scoped target and reference resolution, and Datasource
partial-list/authoritative-409 behavior. Strict decoding, capability/session
binding, zero pre-gate writes, safe diagnostics, and post-write ambiguity or
uncertainty are represented in the architecture and T-005 test obligations.

Q010-001 and Q010-002 are closed. Q-010 authorizes progression to SEC-010; it
does not independently authorize T-005, which still depends on successful
SEC-010 completion.

## Traceability coverage

| Requirement/control | Architecture, contract, and task evidence | Status |
|---|---|---|
| Membership permits create; admin optional | FR-033, ADR-015, plan/model, HTTP contract sections 1.1/7, Q-010/T-005 | SATISFIED |
| Exact selected-row `write` gates update/adoption | FR-033, PA-008, ADR-015, manifest, sealed state order, T-005 matrix | SATISFIED |
| Assistant exact `(project,slug)` | FR-033, ADR-014, manifest/HTTP contract, Q-010 matrix | SATISFIED |
| Workflow creator-scoped v2 and legacy adoption | FR-028–030/034, ADR-016, model/HTTP contract, Q-010/T-005/O-002C | SATISFIED |
| Skill target exact `(project,user_id,name)` | FR-031, AC-FR-031-01, ADR-017, model/manifest/HTTP section 6, Q-010/T-005 | SATISFIED |
| Skill creator-less reference resolution | FR-031, AC-FR-031-02, HTTP section 6, explicit inline `skillRefs` Q-010/T-005 matrix | SATISFIED |
| Datasource miss, one create, authoritative 409 | FR-037, ADR-018, model/manifest/HTTP contract, Q-010/T-005 | SATISFIED |
| Strict evidence and exact equality | DR-013/VR-017, model/HTTP contract, T-005 malformed/additive matrix | SATISFIED |
| One origin/token/principal/session capability | IR-013, ADR-014/015, sealed `PreparedWrite`, Q-010/T-005/SEC-010 | SATISFIED |
| Post-write containment and safe diagnostics | FR-016/034/037, ADR-016–018, failure table, T-005/SEC-010 | SATISFIED |
| V30/v31 replacement | spec revision, ADR-013 supersession, plan stages/risks, tasks section 0 and historical boundary | SATISFIED |

## Closed findings

### Q010-001 — CLOSED

The normative HTTP Skill algorithm now strictly decodes creator and filters
exact `(project,authenticated_user_id,name)`, excludes foreign creators,
requires exact `write` only for target update, treats administration as
optional, and defines creator-scoped 409, post-write, and reference behavior.

### Q010-002 — CLOSED

AC-FR-031-01 now distinguishes foreign rows, zero/one/multiple same-creator
target matches, and the update `write` failure. AC-FR-031-02 independently
defines creator-scoped read-only reference resolution. Q-010 and T-005 contain
the corresponding Workflow inline `skillRefs` request-count and no-ID-leakage
matrix.

## Replacement, migration, security, and operations

- V30/v31 personal-owner/admin/project-detail implementation and evidence are
  explicitly invalid for v32. The old task ledger is under a historical,
  non-normative heading; ADR-013 is superseded.
- Workflow v1/unmarked rows migrate only by explicit same-creator UUID adoption
  under a writer freeze, exact `write`, zero existing v2 match, metadata
  preservation, and post-write verification. No database migration is needed.
- Datasource performs at most one create after a visible miss. HTTP 409 is a
  terminal safe collision with no retry, guessed read, or PUT.
- SEC-010 remains the next required independent security review. Fresh O-002C,
  V-000C/V-000B, and V-003 evidence remains correctly ordered after verified
  implementation and external authorization.

## Validation performed

```text
all contract JSON parse check                         PASS (6 files)
local Markdown link existence check                  PASS
git diff --check                                      PASS
active v30/v31/admin/project-detail stale-term scan   PASS
FR-028–034/037 and AC/task trace scan                 PASS
focused Skill spec/contract/task convergence check    PASS
```

Historical v31 text remains only in explicitly superseded ADR/task/history
sections or in statements rejecting/removing that behavior. No network,
`.env`, implementation test, or live-target command was used.

## Files changed during verification

- `specs/codemie-cicd-tool/Q-010-verification-report.md` only.

## Recommended next action

Proceed to SEC-010. If it closes without open high/critical findings, T-005 may
begin under the exact matrices and boundaries recorded above.
