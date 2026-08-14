# Architecture traceability: save server entity v3.5

Status: IMPLEMENTED — automated verification passed; live-server qualification remains.

| Requirement | Architecture | Contract/ADR | Task | Evidence |
|---|---|---|---|---|
| FR-SAVE-001–012 | Strict typed read/reverse adapters | ADR-013, save-read-reverse contract | C-SAVE-002 | Four-kind read-only fixtures |
| FR-SAVE-021–023 | Natural-reference recovery and exportability | reverse contract/data model | C-SAVE-002 | Missing/ambiguous/secret negative fixtures |
| FR-SAVE-024 | Canonical deterministic YAML | canonical-yaml-v1, ADR-015 | Y-SAVE-002 | Byte goldens |
| FR-SAVE-025 | Existing target refusal | publication-v1, ADR-018 | P-SAVE-002 | Preflight/race no-clobber tests |
| FR-SAVE-026/031 | One generated declaration validation | single-declaration-validation-v1, parent ADR-019 | V-SAVE-003 | Neighbor-open trace + schema tests |
| FR-SAVE-027/033 | Direct artifact write; YAML-last for File Datasource; partial failure visible | publication-v1, ADR-020 | P-SAVE-002/F-SAVE-005 | Placeholder and YAML failure matrix |
| FR-SAVE-028/029 | Closed output/diagnostics | cli-save-v1 + output schemas | C-SAVE-002 | Stream/canary tests |
| FR-SAVE-030 | Compatibility and budgets | ADR-013/read contract | C-SAVE-002 | Drift/oversize/timeout fixtures |
| FR-SAVE-032 | Inline Skill content; no sidecar | skill-snapshot-v1, declaration schema | Y-SAVE-002 | Skill canonical golden/filesystem trace |
| Parent FR-041/DR-014 | Explicit File Datasource path authoring with unavailable-byte placeholders | parent declaration contract + save reverse manifest | F-SAVE-005 | Production export, lint, and zero-byte fixtures |
| QR-SAVE-001/002 | Determinism and immediate declaration lint validity | canonical + validation contracts | Y-SAVE-002/V-SAVE-003/F-SAVE-005 | Repeat/lint tests |
| QR-SAVE-003/005 | Read-only and confidentiality | HTTP/reverse/output contracts | C-SAVE-002, SEC-003 | Request capture/canary review |
| QR-SAVE-004 | Direct-write integrity | publication-v1 | P-SAVE-002 | Complete vs partial state tests |
| QR-SAVE-006/007 | Bounded and portable declaration artifacts | CLI/validation/publication contracts | C-SAVE-002/F-SAVE-005 | Cross-platform suite |
| v3.4 API/declaration boundary | OpenAPI response normalization and secret-safe field projection | ADR-013, reverse manifest, data model | R-SAVE-001 | Reverse-projection tests; OpenAPI contract tests |

Acceptance criteria AC-SAVE-031–033 are owned by V-SAVE-003, Y-SAVE-002, and
P-SAVE-002 respectively. Earlier acceptance evidence for sidecars, repository
closure, staging, YAML-last ordering, orphan sidecars, or atomic publication is
historical and must not be preserved as target behavior.
