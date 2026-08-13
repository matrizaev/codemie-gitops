# Architecture plan: Save a server entity as one local declaration

## 1. Status

Architecture status: READY FOR PRE-IMPLEMENTATION VERIFICATION

Based on `spec.md` v3 and parent specification v33. ADR-018 supersedes the
prospective-repository overlay and staged/atomic multi-artifact publication
designs in ADR-014 and ADR-017.

## 2. Executive summary

`save` performs bounded read-only server resolution, strict reverse projection,
confidentiality checks, canonical inline YAML rendering, and validation of that
one generated declaration. It then directly creates/writes the requested final
path without overwrite. It uses no repository root, config file, sidecar,
staging file, temporary file, rename, `rustix`, or atomic-publication protocol.
A write failure may leave an incomplete new final file; the command reports
`E_OUTPUT_WRITE`, emits no success, and does not remove the path.

The current implementation is non-conforming because it resolves a repository,
renders Skill `contentFrom`, validates a repository overlay, stages with
`tempfile`, and renames with `rustix`. Server read/reverse logic remains useful.

## 3. Sources consulted

- Product specification: `spec.md` v3.
- Parent product specification: `../codemie-cicd-tool.md` v33.
- Parent CLI/declaration/adapter/output contracts.
- Existing save ADRs/contracts/data model/tasks and current `src/save/**`,
  `src/repository.rs`, `src/cli`, `src/config` implementation.
- Pinned reference-only CodeMie source and save-read/reverse manifest for server
  evidence only; reference directories are not product components.

No Jira or Confluence content was available.

## 4. Problem and scope

Save must select one server Assistant, Workflow, Skill, or Datasource by the
approved natural selector, prove it exportable from strict bounded reads, and
write one canonical declaration usable directly as lint/apply `--file` input.

Excluded: mutations, secret export, inferred values, repository inspection,
local dependency existence, Skill sidecars, multiple artifacts, force/replace,
staging, rename publication, rollback/cleanup of a partially written final
path, persistence of server IDs, and server export API changes.

## 5. Current architecture

- `save` already has typed selectors, bounded HTTP reads, strict reverse DTOs,
  managed-reference recovery, canonical YAML, compatibility checks, and safe
  outcome/diagnostic integration.
- It currently derives an output and Skill companion path inside a repository,
  builds an `OverlayRepositoryView`, validates the merged repository closure,
  stages one/two artifacts, and uses native no-replace rename.
- The parent CLI currently supplies repository/config context.

Only the local construction/validation/write boundary is replaced.

## 6. Requirements and quality attributes

| Requirement | Architecture response |
|---|---|
| FR-SAVE-001–012/021–023/030 | Retain strict kind-specific read/reverse adapters and compatibility/confidentiality gates. |
| FR-SAVE-024 | Schema-aware canonical serializer emits byte-stable YAML. |
| FR-SAVE-025/033 | Preflight absence check plus direct create-new final-path open; never replace. |
| FR-SAVE-026/031 | Validate only the generated declaration in memory before beginning the final write. |
| FR-SAVE-032 | Skill `spec.content` is inline; no generated sidecar. |
| Parent FR-041/DR-014 | File Datasource declarations use explicit paths, but the current pinned read contract cannot recover original local path/source bytes; save therefore remains non-exportable for this kind. |
| FR-SAVE-028/029 | Success only after complete write; failures have empty stdout and one safe diagnostic. |
| QR-SAVE-004 | Partial final file is an acknowledged failure state, never success. |

## 7. Facts, constraints, and assumptions

- Fact: the server read/reverse contract is pinned and read-only.
- Fact: a target that exists at preflight is rejected before network access.
- Constraint: direct create/write cannot guarantee atomic complete visibility.
- Constraint: a path may appear between preflight and final create; create-new
  semantics must classify that race without replacing it.
- Assumption: the existing in-memory declaration validator can be narrowed to
  validate bytes/value without filesystem reads. If false, this is an
  implementation refactor, not permission to recreate an overlay.

No blocking product question remains.

## 8. Options considered

1. Retain staged native no-replace publication: rejected by v3.
2. Write a temporary file then rename: rejected by v3.
3. Directly create-new and write the final YAML: selected. It is simplest and
   intentionally exposes partial-file failure.

For validation, a repository overlay is rejected; the selected approach calls
the single-declaration validator with generated in-memory YAML/value.

## 9. Target architecture

| Component | State | Responsibility |
|---|---|---|
| Save CLI boundary | Modified | Exact selector, required project/file, no repository/symlink flags. |
| Read adapters | Retained | Bounded exact server selection and snapshot stability. |
| Reverse projector | Modified | Produce one declaration; Skill content inline. |
| File reverse projector | Retained fail-closed | Current pinned read contract fails File Datasource as non-exportable; no path or placeholder is emitted. |
| Canonical serializer | Retained | Deterministic YAML bytes. |
| Single-declaration validator | Shared/modified | Validate only generated output in memory. |
| Direct output writer | New/replacement | Preflight absence; direct create-new/write final path; classify partial failure. |
| Staging/overlay publisher | Removed | No production responsibility. |

## 10. Data and consistency

`SaveCommand` contains kind selector, explicit project, output path, optional
target URL, and output mode. `GeneratedDeclaration` contains typed declaration
plus canonical bytes. It has no sidecar or repository-relative identity.

`OutputWriteState = NotStarted | FinalCreated | Completed | FailedPartial`.
Server reads finish before `FinalCreated`. `Completed` alone permits `saved`.
`FailedPartial` retains the final path as operator-visible evidence; save does
not attempt cleanup because v3 forbids rollback of the direct final path.

## 11. APIs and integrations

Server routes, authentication, pagination, retries, compatibility evidence,
and Skill stable-snapshot reads remain in the existing contracts. Save sends
zero POST/PUT/PATCH/DELETE. The local write uses create-new/no-overwrite open
and ordinary write/flush semantics; no durability/atomicity guarantee is made.

## 12. Security and operations

- Validate target form/parent/symlink and detectable existence before network.
- Never render target path, server IDs, content, responses, or credentials.
- Complete server reads, projection, secret/mask checks, canonicalization, and
  generated-declaration validation before final creation.
- Detect `FailedPartial` by any failure after final creation and before complete
  write; emit `E_OUTPUT_WRITE`, exit 2, empty stdout.
- Operator recovery: inspect/remove or replace the incomplete path manually
  before retry. The CLI does not clean it up.

## 13. Migration

1. Align CLI/schema/contracts and add direct-write failure tests.
2. Change reverse Skill projection to inline content and one artifact.
3. Replace overlay validation with single-declaration in-memory validation.
4. Replace staged publisher with direct final-path writer.
5. Remove overlay/sidecar/staging call sites and unused `tempfile`/`rustix`
   dependencies after the parent v33 migration also stops using them.
6. Refresh user docs and execute independent security/convergence review.

Rollback before release is a source rollback. Do not mix the v3 CLI with an
older embedded declaration schema.

## 14. Diagrams

```mermaid
sequenceDiagram
    participant CLI
    participant API
    participant Projector
    participant Validator
    participant File
    CLI->>API: bounded read-only resolution/snapshot
    API-->>Projector: strict typed evidence
    Projector->>Validator: one inline canonical declaration
    Validator-->>CLI: valid
    CLI->>File: create-new final path; write bytes
    File-->>CLI: complete or partial failure
```

## 15. ADRs and tasks

- ADR-013, ADR-015, ADR-016 remain applicable.
- ADR-014 and ADR-017 are superseded.
- ADR-018 is accepted from the explicit v3 product decision.
- Ordered bounded implementation tasks are in `tasks.md`.

## 16. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Partial final file | Operator may mistake it for valid YAML | Never report saved; `E_OUTPUT_WRITE`; document manual recovery. |
| Check/create race | Existing file collision | Create-new open; never truncate/replace. |
| Hidden neighbor reads survive | v3 violation/security exposure | Instrument filesystem-open tests and remove overlay APIs. |
| Secret/masked server value exported | Confidentiality breach | Retain strict allowlists and fail-closed projection. |
| Drift | Incorrect declaration | Retain pinned manifest and compatibility gates. |

## 17. Handoff

Verification must prove all server interactions are read-only, generated Skill
content is inline, only one declaration is validated, no temp/staging/rename
operation occurs, existing targets are untouched, and partial writes are
diagnosed exactly. Security review must focus on direct-path races, symlinks,
partial-file confidentiality, and diagnostics. Implementation follows
`tasks.md` and does not modify reference-only directories.
