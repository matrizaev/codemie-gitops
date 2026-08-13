# Architecture traceability: save server entity

Status: complete architecture-to-implementation/verification matrix for the
approved product specification v2. The specification remains authoritative;
this matrix assigns each requirement to its governing architecture artifact,
implementation work package, and independent evidence gate.

Artifact abbreviations: `PL` = [`plan.md`](plan.md), `DM` =
[`data-model.md`](data-model.md), `RM` =
[`contracts/save-read-reverse-v2.42.0-v1.json`](contracts/save-read-reverse-v2.42.0-v1.json),
`CY` = [`contracts/canonical-yaml-v1.md`](contracts/canonical-yaml-v1.md),
`PV` = [`contracts/prospective-validation-v1.md`](contracts/prospective-validation-v1.md),
`SS` = [`contracts/skill-snapshot-v1.md`](contracts/skill-snapshot-v1.md), and
`PUB` = [`contracts/publication-v1.md`](contracts/publication-v1.md).
Task and gate definitions are in [`tasks.md`](tasks.md).

The corrected Datasource decision C-SAVE-005 is governed by ADR-013 and the
RM discriminator rules, implemented by D-SAVE-001/R-SAVE-001, and independently
checked by Q-SAVE-001/V-SAVE-002. It is the source decision traced further by
VR-SAVE-013 and AC-SAVE-025/026 below.

## Functional requirements

| Requirement | Governing architecture | Implementation owner | Evidence gate |
|---|---|---|---|
| FR-SAVE-001 | PL §13; CLI contract | F-SAVE-001 | V-SAVE-002 |
| FR-SAVE-002 | PL §§11–13; DM §2 | F-SAVE-001, C-SAVE-001 | V-SAVE-002 |
| FR-SAVE-003 | PL §§11–13; RM | A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-004 | PL §§11/13/14 | F-SAVE-002, C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| FR-SAVE-005 | PL §13; CLI contract | F-SAVE-001 | V-SAVE-002 |
| FR-SAVE-006 | PL §§13/14; CLI contract | F-SAVE-001/002 | S-SAVE-SEC-002, V-SAVE-002 |
| FR-SAVE-007 | RM Assistant selection/routes | A-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-008 | RM Workflow selection/routes | W-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-009 | RM Workflow selection; DM §§2/4 | W-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-010 | PL §13; output v2 schema | W-SAVE-001, C-SAVE-001 | V-SAVE-002 |
| FR-SAVE-011 | RM Skill selection/routes | S-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-012 | RM Datasource selection/routes | D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-013 | PL §12; DM §§4/6; RM | R-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| FR-SAVE-014 | RM field rules; ADR-013 | R-SAVE-001, A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-015 | RM exclusions; ADR-013 | F-SAVE-003, R-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| FR-SAVE-016 | RM reference/ID rules; DM §5 | R-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| FR-SAVE-017 | RM reference routes/rules | A/W-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-018 | ADR-014; PV | V-SAVE-001 | V-SAVE-002 |
| FR-SAVE-019 | SS; CY; DM §§4/6 | S-SAVE-001, Y-SAVE-001 | V-SAVE-002 |
| FR-SAVE-020 | RM Skill routes; SS | S-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-021 | RM Workflow rules | W-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| FR-SAVE-022 | RM Datasource classes/predicates | D-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| FR-SAVE-023 | RM secret/mask rules | F-SAVE-003, D-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| FR-SAVE-024 | ADR-015; CY | Y-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| FR-SAVE-025 | ADR-017; PUB | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002, L-SAVE-001 |
| FR-SAVE-026 | PL §11; PUB | C-SAVE-001, P-SAVE-001 | V-SAVE-002 |
| FR-SAVE-027 | ADR-017; PUB | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002, L-SAVE-001 |
| FR-SAVE-028 | PL §13; output/diagnostic v2 | F-SAVE-001, C-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| FR-SAVE-029 | PL §14; output v2 | F-SAVE-001, C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| FR-SAVE-030 | ADR-013; RM | F-SAVE-002/003, A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |

## Data requirements

| Requirement | Governing architecture | Implementation owner | Evidence gate |
|---|---|---|---|
| DR-SAVE-001 | PL §12; DM §§4/6; declaration schema | R-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| DR-SAVE-002 | ADR-013; RM field rules | F-SAVE-003, R-SAVE-001 | Q-SAVE-001, S-SAVE-SEC-002 |
| DR-SAVE-003 | RM per-kind transforms | A/W/S/D-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| DR-SAVE-004 | RM reference rules; DM §5 | R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| DR-SAVE-005 | RM Workflow rules | W-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| DR-SAVE-006 | ADR-016; SS; CY | S-SAVE-001, Y-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| DR-SAVE-007 | ADR-015; CY | Y-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| DR-SAVE-008 | RM Datasource predicates/mapping | D-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| DR-SAVE-009 | ADR-017; PUB; DM §§6/8 | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002, L-SAVE-001 |

## Integration requirements

| Requirement | Governing architecture | Implementation owner | Evidence gate |
|---|---|---|---|
| IR-SAVE-001 | PL §13; RM routes | F-SAVE-002, A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| IR-SAVE-002 | RM selection/detail routes | A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| IR-SAVE-003 | RM reference routes | A/W-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| IR-SAVE-004 | PL §§13/14; CLI contract | F-SAVE-002 | S-SAVE-SEC-002, V-SAVE-002 |
| IR-SAVE-005 | ADR-013; RM source identity | F-SAVE-002/003 | Q-SAVE-001, V-SAVE-002 |
| IR-SAVE-006 | ADR-013; RM compatibility policy | F-SAVE-003, A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |

## Quality requirements

| Requirement | Governing architecture | Implementation owner | Evidence gate |
|---|---|---|---|
| QR-SAVE-001 | ADR-015; CY | Y-SAVE-001 | V-SAVE-002 |
| QR-SAVE-002 | ADR-014; PV | V-SAVE-001, C-SAVE-001 | V-SAVE-002 |
| QR-SAVE-003 | PL §§13/14 | F-SAVE-002, C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| QR-SAVE-004 | ADR-017; PUB | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002, L-SAVE-001 |
| QR-SAVE-005 | ADR-013/016/017; RM/SS/PUB | F-SAVE-003, S/D-SAVE-001, P-SAVE-001 | S-SAVE-SEC-002 |
| QR-SAVE-006 | PL §13; RM pagination | F-SAVE-002, A/W/S/D-SAVE-001 | V-SAVE-002 |
| QR-SAVE-007 | ADR-014/015/017; CY/PV/PUB | Y/V/P-SAVE-001 | V-SAVE-002, L-SAVE-001 |
| QR-SAVE-008 | PL §14; output v2 | C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| QR-SAVE-009 | ADR-013; RM/schema | Q-SAVE-001/002, O-SAVE-001 | V-SAVE-002 |

## Validation requirements

| Requirement | Governing architecture | Implementation owner | Evidence gate |
|---|---|---|---|
| VR-SAVE-001 | CLI contract; DM §2 | F-SAVE-001 | V-SAVE-002 |
| VR-SAVE-002 | CLI contract; DM §2 | F-SAVE-001 | V-SAVE-002 |
| VR-SAVE-003 | CLI contract; DM §2 | F-SAVE-001 | V-SAVE-002 |
| VR-SAVE-004 | ADR-017; PUB | F-SAVE-001, P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| VR-SAVE-005 | ADR-014; PV | V-SAVE-001 | V-SAVE-002 |
| VR-SAVE-006 | RM selection/pagination rules | A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| VR-SAVE-007 | RM Workflow rules | W-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| VR-SAVE-008 | RM reference rules | A/W-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| VR-SAVE-009 | ADR-013; RM; declaration schema | R-SAVE-001 | Q-SAVE-001/002, V-SAVE-002 |
| VR-SAVE-010 | RM secret/mask rules | F-SAVE-003, D-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| VR-SAVE-011 | ADR-014; PV | V-SAVE-001 | V-SAVE-002 |
| VR-SAVE-012 | ADR-017; PUB | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002, L-SAVE-001 |
| VR-SAVE-013 | RM Datasource discriminator rules | D-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |

## Acceptance criteria

| Requirement | Governing architecture | Primary task/test owner | Independent gate |
|---|---|---|---|
| AC-SAVE-001 | RM Assistant; CY/PUB | A-SAVE-001, C-SAVE-001 | V-SAVE-002 |
| AC-SAVE-002 | RM Workflow; CY/PUB | W-SAVE-001, C-SAVE-001 | V-SAVE-002 |
| AC-SAVE-003 | RM Workflow ID mode; output v2 | W-SAVE-001, C-SAVE-001 | V-SAVE-002 |
| AC-SAVE-004 | RM Workflow selection | W-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-005 | RM Workflow selection | W-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-006 | RM Skill selection | S-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-007 | ADR-016; SS/CY | S-SAVE-001, Y-SAVE-001 | V-SAVE-002 |
| AC-SAVE-008 | ADR-016; RM/SS | S-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-009 | ADR-017; PUB | S-SAVE-001, P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-010 | RM reference rules | A/W-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-011 | ADR-014; PV | V-SAVE-001 | V-SAVE-002 |
| AC-SAVE-012 | ADR-014; PV | V-SAVE-001, C-SAVE-001 | V-SAVE-002 |
| AC-SAVE-013 | RM Datasource predicates | D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-014 | RM secret classes; ADR-013 | F-SAVE-003, D-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-015 | RM mask rules | D-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-016 | ADR-017; PUB | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-017 | ADR-017; PUB | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002, L-SAVE-001 |
| AC-SAVE-018 | ADR-017; PUB | P-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-019 | ADR-015; CY | Y-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| AC-SAVE-020 | ADR-013; RM source identity | F-SAVE-003, A/W/S/D-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-021 | ADR-013; RM compatibility policy | F-SAVE-003, C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-022 | PL §§13/14 | F-SAVE-002, C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-023 | output/diagnostic v2 | F-SAVE-001, C-SAVE-001 | Q-SAVE-002, V-SAVE-002 |
| AC-SAVE-024 | output/diagnostic v2; PL §14 | F-SAVE-001, C-SAVE-001 | S-SAVE-SEC-002, V-SAVE-002 |
| AC-SAVE-025 | RM Datasource discriminator rules | D-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |
| AC-SAVE-026 | RM Datasource discriminator rules | D-SAVE-001, R-SAVE-001 | Q-SAVE-001, V-SAVE-002 |

Every row has an implementation owner and an independent gate. Proposed ADRs
013–017 must be accepted by the authorized review process before their
dependent implementation tasks start.
