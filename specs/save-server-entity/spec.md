# Feature specification: Save a server entity as a local declaration

## 1. Document status

- **Status:** APPROVED — v3.3; downstream artifact refresh required
- **Specification owner:** Product Specification Owner (named owner pending)
- **Source request:** User request, 2026-08-11: add a command that saves an
  entity YAML from the CodeMie server locally so resources created through the
  UI can begin to be owned through GitOps.
- **Related Jira issues:** None provided.
- **Related Confluence pages:** None provided.
- **Parent product specification:**
  [`../codemie-cicd-tool.md`](../codemie-cicd-tool.md), v33.3.
- **Pinned target baseline:** CodeMie backend tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- **Last reviewed:** 2026-08-13.

This specification approves the bounded server-to-declaration capability that
the parent specification deferred under OQ-14 and “bidirectional sync.” It does
not approve continuous bidirectional synchronization, remote ownership
markers, recursive export, or server writes by the new command.

Version 2 corrects Datasource reverse discrimination for the pinned backend.
For a code Datasource, persisted `vcs_type` selects the declaration's Git or
SVN branch, while persisted `index_type` supplies the declaration's code
strategy field. This correction does not change the command surface, natural
identity, supported entity kinds, or non-exportability boundary.

Version 3 aligns save output with the parent specification's single-file
declaration model. Save emits exactly one YAML file, with Skill content inline;
it does not construct or validate a repository view. It writes the final file
directly without a staging file, temporary file, rename-based publication, or
cross-file transaction. Existing targets remain a visible refusal condition.

Version 3.2 aligns explicit auxiliary-input terminology with parent v33.3.
Save continues to emit inline Skill content, although lint/apply also accept
bounded `contentFrom`. File Datasource remains non-exportable because a valid
path-based declaration requires original local source files and the pinned
read contract exposes filenames but not those original bytes.

Version 3.3 resolves the save-success output conflict in favor of the existing
per-entity outcome contract. Save reports `action`, `kind`, `project`, and the
kind's natural selector in both output modes; ID-selected unmarked Workflow
save additionally reports `adoptionRequired: true`. It continues to exclude
paths, URLs, server IDs, content, timestamps, users, and external provenance.

## 2. Executive summary

Platform authors can create CodeMie Assistants, Workflows, Skills, and
Datasources through the UI, but `codemie-gitops` currently requires them to
manually reconstruct those resources as local declarations before Git can
become their desired-state record. Manual reconstruction is error-prone,
especially for server-generated IDs, Workflow identity metadata, Skill
content, and per-kind Datasource fields.

This feature adds one command, `codemie-gitops save`, which reads one existing
server entity, converts it into the existing
`codemie.epam.com/v1alpha1` declaration language, validates that one generated
declaration offline, and writes the requested local YAML without intentionally
overwriting an existing file. The command makes no modifying server request.

The initial release supports the four product entity kinds: Assistant,
Workflow, Skill, and Datasource. Deprecated Autonomous Workflows are
non-exportable in v1 because the pinned server's exhaustive Workflow list
routes exclude them. It reverses managed-entity server references
to natural keys, removes server-owned IDs and runtime fields, stores a Skill's
main instructions inline in YAML, and emits deterministic canonical YAML. A
known secret, masked value, unsupported subtype, or required state that cannot
be reconstructed fails before the output write begins.

An unmarked UI-created Workflow is selected only by a reviewed server UUID plus
a caller-chosen slug. `save` does not mark or adopt it. The successful outcome
states that adoption is required for a later `apply`; only that later `apply`
may use the existing
`--adopt-workflow-id` ceremony with that same reviewed UUID. No server UUID is
written to YAML, a sidecar, local state, or command output.

The most important scope boundary is that `save` is one-entity and
non-recursive. It validates only its one generated declaration; it does not
inspect neighboring declarations or require a locally complete dependency
closure. References must still be exactly recoverable from the server.

## 3. Evidence consulted

### Jira

No Jira content was provided or available locally.

### Confluence

No Confluence content was provided or available locally.

### Current product repository

- [`../codemie-cicd-tool.md`](../codemie-cicd-tool.md) — authoritative current
  product behavior, four-entity scope, natural keys, reference forms,
  Workflow-only identity/adoption, omission/null behavior, output safety, and
  OQ-14's previous export deferral.
- [`../codemie-cicd-tool/contracts/cli.md`](../codemie-cicd-tool/contracts/cli.md)
  — current `lint`, `apply`, and `login` command surface, configuration
  precedence, exit taxonomy, and stream contract.
- [`../codemie-cicd-tool/contracts/declaration-v1alpha1.md`](../codemie-cicd-tool/contracts/declaration-v1alpha1.md)
  and
  [`../codemie-cicd-tool/contracts/declaration-v1alpha1.schema.json`](../codemie-cicd-tool/contracts/declaration-v1alpha1.schema.json)
  — exact closed local declaration language, effective project, Skill
  `contentFrom`, Workflow reference forms, and supported Datasource union; its
  code branches use `spec.index_type` (`git` or `svn`) as the union
  discriminator and `spec.indexType` (`code`, `summary`, or `chunk-summary`)
  as the code strategy.
- [`../codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json`](../codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json)
  — pinned routes, field classes, pagination, identity, and request/read shapes
  for all four entities and Datasource kinds. Its existing Datasource
  resolution field list includes persisted `index_type` but not `vcs_type`;
  this is an apply-oriented contract gap that architecture must correct for
  save rather than treating as product evidence that one discriminator is
  sufficient.
- [`../codemie-cicd-tool/contracts/http-adapter.md`](../codemie-cicd-tool/contracts/http-adapter.md)
  — target URL, authentication, redirect, response-budget, compatibility,
  exhaustive resolution, and sensitive-diagnostic boundaries.
- [`../codemie-cicd-tool/contracts/source-baseline.md`](../codemie-cicd-tool/contracts/source-baseline.md)
  — reference-only source identity and the rule that `/v1/info.version` is not
  source/API identity.
- [`../codemie-cicd-tool/adr/007-skill-exhaustive-list-resolution.md`](../codemie-cicd-tool/adr/007-skill-exhaustive-list-resolution.md)
  — Skill creator-scoped server uniqueness and ambiguity-refusing natural-key
  resolution.
- [`../codemie-cicd-tool/adr/008-workflow-meta-config-identity-and-adoption.md`](../codemie-cicd-tool/adr/008-workflow-meta-config-identity-and-adoption.md)
  and
  [`../../ops/o002/WORKFLOW_ADOPTION.md`](../../ops/o002/WORKFLOW_ADOPTION.md)
  — reserved Workflow identity, exact-ID adoption, metadata preservation, and
  the prohibition on persisting or reporting the adoption UUID.
- [`../codemie-cicd-tool/adr/009-datasource-ordinary-crud-boundary.md`](../codemie-cicd-tool/adr/009-datasource-ordinary-crud-boundary.md)
  and
  [`../codemie-cicd-tool/adr/012-datasource-visibility.md`](../codemie-cicd-tool/adr/012-datasource-visibility.md)
  — per-kind Datasource formats, exhaustive identity resolution, and the
  project-admin complete-visibility requirement.
- [`../codemie-cicd-tool/contracts/outcome.schema.json`](../codemie-cicd-tool/contracts/outcome.schema.json)
  and
  [`../codemie-cicd-tool/contracts/diagnostic.schema.json`](../codemie-cicd-tool/contracts/diagnostic.schema.json)
  — current closed success and safe-diagnostic formats, which this feature
  requires architecture to extend without weakening their boundaries.
- [`../../src/cli/mod.rs`](../../src/cli/mod.rs) and
  [`../../README.md`](../../README.md) — implemented command naming and current
  user-visible behavior.

### External/reference-only CodeMie backend

The following evidence was inspected from the reference-only `codemie/`
checkout at tag `2.42.0`, commit
`2a481c290c99bf30ef80aadafa03d876a7f5f732`. These files are evidence only and
are not part of the product architecture or an authorized edit surface.

- `codemie/src/codemie/rest_api/routers/assistant.py` and
  `models/assistant.py` — project+slug detail read, full Assistant fields,
  nested Assistant enrichment, Skill enrichment, server-owned fields, and
  masking of sensitive prompt-variable defaults.
- `codemie/src/codemie/rest_api/routers/workflow.py`,
  `core/workflow_models/workflow_config.py`, and
  `core/workflow_models/workflow_models.py` — paginated Workflow reads,
  by-ID detail, string-encoded `meta_config`, `yaml_config`, and server-ID
  positions in execution configuration.
- `codemie/src/codemie/rest_api/routers/skill.py` and
  `models/skill.py` — paginated list, by-ID detail including main content,
  companion-file metadata, and per-file content reads.
- `codemie/src/codemie/rest_api/routers/index.py` and
  `models/index.py` — paginated Datasource list, by-ID detail, per-kind stored
  fields, the `index_type` code strategy, the separate `vcs_type` Git/SVN
  discriminator, server/runtime state, SharePoint token-bearing fields, File
  Datasource filename/status data, and the unrelated status-Markdown export
  endpoint.
- `codemie/src/codemie/datasource/code/code_datasource_processor.py`,
  `datasource/svn/svn_datasource_processor.py`, and
  `external/alembic/versions/p0q1r2s3t4u5_add_vcs_type_to_index_info.py` — Git
  uses the model's default `vcs_type` of `git`; SVN persists the selected code
  strategy in `index_type` and `svn` in `vcs_type`; the pinned migration
  normalizes legacy rows that stored `svn` in `index_type`.

### User-provided decisions

- A new command must save an existing server entity as local YAML.
- The purpose is to begin managing resources created through the UI through
  GitOps.

No Jira, Confluence, stakeholder deadline, telemetry target, or server-change
commitment was provided.

## 4. Problem statement

The current tool can validate and apply a declaration but cannot derive one
from a resource that already exists on the server. Authors must manually copy
UI-visible fields, infer required values, convert references, and avoid copying
server IDs or secrets. For Workflows, manual conversion can also confuse the
server UUID with the GitOps slug or accidentally bypass the explicit adoption
boundary. For Skills and Datasources, content and per-kind fields make manual
reconstruction especially costly.

Without a safe import path, teams either leave UI-created resources outside
GitOps control or create declarations that do not faithfully represent the
server resource and may duplicate, overwrite, or expose data when applied.

## 5. Business objective

Reduce the cost and risk of onboarding existing UI-created resources into a
reviewed Git repository while preserving the current identity, validation,
authorization, compatibility, and secret-handling contracts.

## 6. Success measures

### Product success

- A supported, representable server entity can be saved without manually
  copying its authorable fields.
- Every successful output is accepted by the current offline lint contract in
  the resulting repository.
- A later `apply` selects the same existing natural identity; an unmarked
  Workflow does so only through the explicit adoption invocation.

### User success

- The author supplies one kind, one project, one natural key (or the approved
  Workflow ID selector), and one local YAML path.
- The author receives either a complete local declaration set or no new local
  output.
- The outcome tells an unmarked Workflow author that adoption is still
  required without persisting or echoing the server UUID.

### Operational success

- `save` sends zero server-modifying requests.
- It never overwrites an existing local file.
- It never publishes a raw server response, server-owned managed-entity ID,
  known secret field, masked secret substitute, or partial declaration.

No adoption-rate or duration target is approved. Performance thresholds are
defined only by the inherited resource budgets and invocation deadline.

## 7. Actors and stakeholders

| Actor or stakeholder | Goal | Permissions and responsibilities |
|---|---|---|
| Platform asset author | Onboard an existing UI-created entity into Git | Read the selected entity and its references; choose a local path; review the generated diff; save dependencies first |
| Project administrator | Safely resolve Workflow, Skill, or Datasource identity over a complete project view | Provide the project-complete visibility required for ambiguity detection; no server write permission is required by `save` itself |
| Reviewer / approver | Confirm that generated YAML is suitable desired state | Review generated YAML and Skill content; verify intended ownership/governance change before merge or apply |
| CI runner | Optionally verify saved declarations | Run `lint`; `save` is not required in a credentialed deployment job |
| Platform administrator | Govern later adoption and out-of-band writers | Review the exact Workflow UUID and authorize the separate adoption apply; remediate duplicate identities |
| Security reviewer | Confirm that server reads cannot become secret export | Review reverse field classifications, output paths, temporary data, and diagnostic allowlists |
| CodeMie platform owner | Confirm pinned read-contract compatibility | Supply target/API evidence when the pinned read shape is insufficient or changes |

Creating a local declaration does not grant server permission and is not proof
that the invoking user or repository owns the server entity.

## 8. Goals

1. Save one existing Assistant, Workflow, Skill, or Datasource as the current
   declaration API version.
2. Select the entity using the approved natural identity, with an exact-ID
   exception only for an unmarked Workflow.
3. Produce deterministic, reviewable, offline-valid local artifacts.
4. Reverse managed-entity server references to portable natural-key
   references.
5. Keep server-owned IDs, runtime state, credentials, masked values, and
   non-authorable fields out of local output.
6. Preserve the existing Workflow adoption safety boundary.
7. Fail without server mutation, overwrite, or partial local publication when
   faithful reconstruction is not possible.

## 9. Non-goals

- Continuous pull, watch, drift detection, or bidirectional synchronization.
- Recursive or batch saving of dependencies.
- Choosing output directories or filenames automatically beyond the Skill
  main-content sidecar derived from the caller's YAML path.
- Overwriting, merging into, or patching an existing declaration.
- A `--force`, `--replace`, or stdout-YAML mode.
- Server create, update, delete, marker mutation, adoption, permission change,
  or ownership transfer by `save`.
- A generic ownership marker or generic adoption command.
- Selecting an Assistant, Skill, or Datasource by server UUID.
- Resolving duplicate Skill or Datasource natural identities by creator,
  newest row, list order, or supplied UUID.
- Exporting File Datasource source bytes when the server does not expose the
  original files through the pinned ordinary read contract.
- Exporting provider-defined or Bedrock Datasources that are outside the
  current declaration schema.
- Exporting platform integration credentials, encrypted settings, tokens, or
  other secret material.
- Changing the existing declaration schema, null semantics, apply behavior, or
  later adoption procedure as part of this product specification.

## 10. Scope

### In scope

- A `save` subcommand on the existing `codemie-gitops` executable.
- Assistant selected by effective project and slug.
- Marked Workflow selected by effective project and reserved-marker slug.
- Unmarked Workflow selected by exact reviewed UUID, effective project, and a
  caller-chosen future slug.
- Skill selected by effective project and name with exhaustive ambiguity
  checks.
- Datasource selected by effective project and `repo_name` with exhaustive
  ambiguity checks; the selected server row supplies the source-pinned
  discriminator fields required by DR-SAVE-008.
- The existing ten authorable Datasource schema branches when their current
  state is faithfully reconstructable; File is a known non-exportable branch
  in this release because required source bytes are not available.
- Reverse projection to `codemie.epam.com/v1alpha1`.
- Adjacent main-content Markdown sidecar for Skill.
- Retrieval and inline declaration representation of Skill companion files as
  already required by the declaration schema.
- Prospective offline repository-closure validation before publication.
- Text and JSON outcomes plus safe failure diagnostics.

### Out of scope

- Any source-code or API change under `codemie/` or `codemie-ui/`.
- Any server endpoint intended for status/report downloads rather than the
  pinned authoring/read model, including `/v1/index/{id}/export` status
  Markdown.
- Automatic Git add, commit, branch, PR, merge, or push.
- Automatic `apply` after save.
- Persisting the source server URL, server UUID, export time, user identity,
  Git identity, or other provenance in YAML or command output.
- Saving to a path outside the selected repository.

### Deferred

- Recursive dependency closure and atomic multi-entity onboarding.
- A reviewed override for replacing an existing local declaration.
- User-selected Skill main-content sidecar paths or inline Skill output.
- A File Datasource package format that includes original source files.
- Export of a deployment-specific provider Datasource after a closed schema is
  approved.
- A machine-readable dependency discovery/report command.
- Crash-recovery UX for abandoned internal staging artifacts; architecture
  must still satisfy the observable publication rules in QR-SAVE-004.

## 11. Terminology

| Term | Definition |
|---|---|
| Save | Read one server entity, reverse-project it into local declaration artifacts, validate the prospective repository, and publish those artifacts without modifying the server |
| Reverse projection | The pinned conversion from server read fields to the existing authoring envelope, including natural-reference recovery and exclusion of server-owned state |
| Representable entity | An entity whose complete current authorable state can be expressed in the current declaration schema without a secret, masked substitute, unsupported type, guessed value, or server-managed ID |
| Prospective repository | The current repository plus the not-yet-published YAML and Skill sidecar, evaluated in memory under the same offline rules as `lint` |
| Marked Workflow | A Workflow with one valid reserved `codemie.epam.com/gitops/workflow-identity` record |
| Unmarked Workflow | A Workflow with no reserved Workflow identity member; malformed members are invalid rather than unmarked |
| Adoption-required save | A successful read-only save of an unmarked Workflow selected by ID; it still requires a later explicit `apply --adopt-workflow-id` |
| Managed-entity ID | A server ID for one of Assistant, Workflow, Skill, or Datasource; it is never declaration identity |
| Opaque configuration ID | A schema-approved reference to configuration outside the four managed entity kinds, such as `setting_id`; it is preserved when classified non-secret and is not treated as a managed-entity ID |
| Publication | Making the final YAML and, for Skill, the required final Markdown sidecar visible at the requested paths after all validation passes |

Workflow-local `execution_config.assistants[].id` and
`states[].assistant_id` values are graph identifiers, not Workflow server IDs,
and remain in the declaration.

## 12. Current behavior

- `codemie-gitops` exposes `lint`, `apply`, and `login`; it has no server-to-YAML
  command.
- The declaration schema already supports all four entity kinds and natural
  references.
- `lint` validates only the declaration selected by `--file` offline.
- `apply` always sends a create or update after safe resolution; it does not
  establish generic ownership.
- Assistant identity is project+slug and has a direct read route.
- Workflow has no native slug. GitOps identity lives in a reserved
  string-encoded `meta_config` member. An unmarked Workflow can be adopted only
  by the explicit invocation-only UUID flag on `apply`.
- Skill server uniqueness includes creator, so project+name must be resolved
  exhaustively and can be ambiguous.
- Datasource list visibility is complete only for the approved project-admin,
  global-admin, or global-maintainer principal, and no natural-key uniqueness
  constraint is evidenced.
- For a persisted code Datasource, backend `index_type` is the code strategy
  (`code`, `summary`, or `chunk-summary`) and backend `vcs_type` is the branch
  discriminator (`git` or `svn`). These map to distinct declaration properties
  named `spec.indexType` and `spec.index_type`, respectively.
- Skill detail exposes main content and companion-file metadata; companion
  content requires additional reads.
- File Datasource detail exposes filenames and processing state, not the
  original local source-file byte set required by the declaration's `files`.
- SharePoint and provider-related server models contain token/OTP-bearing
  fields that are prohibited from declarations.
- Assistant detail masks sensitive prompt-variable defaults. A masked value is
  not faithful authorable state and cannot be saved as though it were the
  original value.

## 13. User and system scenarios

### SC-SAVE-001 — Save a leaf Skill

- **Actor:** Platform asset author.
- **Trigger:** The author wants to onboard an existing project Skill.
- **Preconditions:** The target YAML and derived Markdown paths do not exist;
  the author has complete project visibility and read access.
- **Main flow:** The author invokes `save` with kind Skill, project, name, and
  YAML path. The tool resolves exactly one Skill, reads its detail and all
  companion content, creates a prospective declaration using `contentFrom`,
  validates it, then publishes the Markdown and YAML.
- **Expected outcome:** Both artifacts are complete; the YAML names only the
  natural identity; immediate offline lint succeeds; stdout reports `saved`.
- **Failure flow:** Any duplicate identity, missing companion content, unsafe
  content, output collision, or validation error leaves both final paths
  absent.

### SC-SAVE-002 — Save a marked Workflow by slug

- **Actor:** Platform asset author.
- **Trigger:** A Workflow already contains the valid GitOps identity record.
- **Main flow:** The tool exhausts the required scopes, resolves exactly one
  project+slug marker, reads detail, removes the reserved member from authored
  `spec.meta_config`, converts resource IDs to natural references, validates,
  and saves the declaration.
- **Expected outcome:** A later ordinary `apply` resolves the same Workflow by
  the retained server marker. The saved outcome does not say adoption is
  required.

### SC-SAVE-003 — Save an unmarked UI-created Workflow

- **Actor:** Platform administrator or authorized asset owner.
- **Trigger:** The actor has reviewed the exact server UUID and chosen a stable
  GitOps slug.
- **Preconditions:** The exact-ID candidate is in the effective project, is
  readable, contains no reserved identity member, and no valid or invalid
  marker conflicts with the intended identity.
- **Main flow:** The actor invokes Workflow save with `--id` and `--slug`. The
  tool selects only that ID, ignores display name as identity, performs the
  required full marker/visibility checks, and saves a UUID-free declaration.
- **Expected outcome:** The outcome sets `adoptionRequired` to true. The server
  remains unmodified. The actor retains the reviewed UUID out of band and later
  invokes `apply --adopt-workflow-id` under the existing runbook.
- **Failure flow:** A marked candidate, malformed marker, conflicting marker,
  wrong project, inaccessible candidate, or unrepresentable reference leaves
  no local output.

### SC-SAVE-004 — Save an Assistant with dependencies

- **Actor:** Platform asset author.
- **Trigger:** The selected Assistant refers to Datasources, Skills, or
  sub-Assistants.
- **Preconditions:** Exact local declarations for every resulting natural
  reference already exist and the server references are readable and
  unambiguous.
- **Main flow:** The tool resolves the Assistant by project+slug, reverses each
  reference to its natural key, constructs the prospective repository, and
  runs the same offline closure checks as lint.
- **Expected outcome:** The saved Assistant contains no managed-entity server
  IDs and the resulting repository lints successfully.
- **Alternate flow:** If a dependency has not been saved, the author saves the
  dependency first and retries.

### SC-SAVE-005 — Refuse a non-exportable Datasource

- **Actor:** Platform asset author.
- **Trigger:** The selected Datasource is File, provider-defined, Bedrock, or
  contains another required state that cannot be represented safely.
- **Expected outcome:** The command exits 1 with a fixed non-exportable-entity
  diagnostic, writes nothing to stdout or final files, does not print the
  sensitive or missing value, and makes no server modification.

### SC-SAVE-006 — Refuse overwrite

- **Actor:** Platform asset author.
- **Trigger:** The requested YAML path, or the derived Skill sidecar path,
  already exists.
- **Expected outcome:** The command fails locally with exit 2 before any server
  request and leaves every existing byte unchanged. There is no force option.

### SC-SAVE-007 — Target read is incompatible

- **Actor:** Platform asset author.
- **Trigger:** A required read field, pagination invariant, reference detail,
  or reverse-projection classification does not match the pinned contract.
- **Expected outcome:** The command exits 2 with `E_API_INCOMPATIBLE`, leaves
  stdout and final output paths empty/absent, and emits no raw server content.
  `/v1/info.version` does not independently accept or reject the target.

### SC-SAVE-008 — Local publication fails

- **Actor:** Platform asset author.
- **Trigger:** Validation passed but publication cannot complete.
- **Expected outcome:** The command exits 2, stdout is empty, pre-existing files
  remain byte-identical, and no final declaration is visible without all of its
  required sidecars.

## 14. Functional requirements

| ID | Requirement | Rationale | Scenarios |
|---|---|---|---|
| **FR-SAVE-001** | The tool MUST provide the exact `save` command surface in §14.1 on the `codemie-gitops` executable. | Gives the feature one explicit, non-conflicting name. | All |
| **FR-SAVE-002** | Each invocation MUST target exactly one server entity and one YAML declaration. | Preserves the existing one-entity orchestration model. | SC-SAVE-001–005 |
| **FR-SAVE-003** | `save` MUST support Assistant, Workflow, Skill, and Datasource, subject only to the representability rules in this specification. | Matches the product's four-entity model. | SC-SAVE-001–005 |
| **FR-SAVE-004** | `save` MUST make only non-mutating CodeMie requests. It MUST NOT send POST, PUT, PATCH, DELETE, adopt, update a marker, or call `apply`. | Saving local state must not change the source server. | All |
| **FR-SAVE-005** | `--project <project>` MUST supply the effective project; an absent or invalid value MUST fail before network access. Save MUST NOT discover project from a repository configuration file. | There is no source declaration from which to obtain project. | SC-SAVE-001–007 |
| **FR-SAVE-006** | Target URL MUST resolve as `--url` > `CODEMIE_URL`, and bearer token MUST use environment-only `CODEMIE_TOKEN`. Secret-bearing CLI flags MUST remain unsupported. | Reuses the approved flag/environment secret boundary without repository config. | All online scenarios |
| **FR-SAVE-007** | Assistant MUST be selected only by exact effective project+`--slug` through the pinned direct lookup. A null-slug Assistant is not representable and MUST NOT be selected by ID or display name. | Keeps Assistant adoption on its approved natural identity. | SC-SAVE-004 |
| **FR-SAVE-008** | Without `--id`, Workflow MUST be resolved by exhaustive exact effective project+valid reserved-marker `--slug`. Display name MUST NOT select or break ties. | Preserves Workflow identity. | SC-SAVE-002 |
| **FR-SAVE-009** | Workflow alone MAY accept `--id <canonical-uuid>` together with `--slug <desired-natural-key>` for an unmarked candidate. The tool MUST select only that ID, prove exact project, prove complete marker visibility, require no reserved member on the candidate, and require zero valid or invalid marker conflicts for the desired project+slug. A marked candidate MUST fail and direct the actor to natural-key selection without echoing the ID. | Safely covers UI-created unmarked Workflows. | SC-SAVE-003 |
| **FR-SAVE-010** | ID-selected Workflow save MUST NOT alter or adopt the server Workflow. Its successful outcome MUST include `adoptionRequired: true`; only a later explicit `apply --adopt-workflow-id` operation may adopt it using the same reviewed UUID out of band. | Separates local conversion from server adoption. | SC-SAVE-003 |
| **FR-SAVE-011** | Skill MUST be selected by exhaustive, complete-visibility, exact effective project+`--name` resolution. More than one exact result MUST fail; current creator, newest, first, and ID MUST NOT break the tie. | Server uniqueness is creator-scoped. | SC-SAVE-001 |
| **FR-SAVE-012** | Datasource MUST be selected by exhaustive, complete-visibility, exact effective project+`--repo-name` resolution. More than one exact result MUST fail, including rows with different persisted discriminator values under the same approved natural key. Discriminator fields MUST NOT participate in natural-key filtering or break a tie. The unique row's source-pinned discriminator combination selects the reverse projection under DR-SAVE-008. | Prevents arbitrary adoption of duplicate Datasources and preserves the backend's separate VCS/strategy meanings. | SC-SAVE-005 |
| **FR-SAVE-013** | The command MUST reverse-project every selected entity into exactly one `codemie.epam.com/v1alpha1` declaration using the current closed schema. It MUST emit the explicit `--project` value as `metadata.project`. | Makes saved output self-contained and portable. | SC-SAVE-001–005 |
| **FR-SAVE-014** | The reverse projection MUST include every current authorable value required to reproduce the selected entity's declarative state, including concrete server-selected defaults and explicit nulls. It MUST NOT guess, invent, or substitute a value. | Avoids incomplete or misleading ownership. | SC-SAVE-005, 007 |
| **FR-SAVE-015** | The tool MUST exclude every field classified as server-owned, audit, usage, reaction, status, processing, history, transport-only, or otherwise prohibited by the declaration contract. | Keeps declarations focused on authored desired state. | All |
| **FR-SAVE-016** | Every managed-entity server ID MUST either be converted to the exact approved natural-reference position or excluded. No managed-entity server ID may appear in YAML, Skill sidecars, persistent client state, success output, warnings, or diagnostics. Workflow-local graph IDs and schema-approved opaque configuration IDs are not managed-entity IDs and MUST retain their existing semantics. | Preserves portability without corrupting local graph/configuration identifiers. | SC-SAVE-002–004 |
| **FR-SAVE-017** | Assistant, Workflow, and inline Workflow resource references MUST be converted to the natural-key forms already defined by DR-003 and FR-035 of the parent specification. Every referenced target MUST resolve exactly and expose the fields needed for a valid natural key. Missing, inaccessible, null-key, or ambiguous targets MUST fail without guessing. | Reverses the apply-time transformation safely. | SC-SAVE-002, 004 |
| **FR-SAVE-018** | `save` MUST NOT recursively save referenced entities. It MUST validate the generated declaration's syntax, schema, semantics, and local reference shapes without inspecting other local files. | Preserves one-entity scope and single-file validation. | SC-SAVE-004 |
| **FR-SAVE-019** | For Skill, the main server content MUST be written inline to `spec.content` in the YAML. Save MUST NOT emit or reference a sidecar. | Preserves one-file output. | SC-SAVE-001 |
| **FR-SAVE-020** | Skill companion-file metadata and content MUST be fully read and represented in the existing `spec.companion_files` declaration form. Missing, duplicate-path, incompatible, over-budget, or inaccessible content MUST fail the whole save. | Detail alone exposes only metadata; partial Skill export is unsafe. | SC-SAVE-001 |
| **FR-SAVE-021** | Workflow reverse projection MUST strictly decode `yaml_config` and `meta_config`, remove only the valid reserved identity member from authored `spec.meta_config`, preserve all non-reserved authorable values, and reverse the server resource-ID positions defined by the declaration contract. Malformed or conflicting metadata MUST fail and MUST NOT be rewritten. | Preserves mixed ownership and adoption safety. | SC-SAVE-002, 003 |
| **FR-SAVE-022** | A Datasource MUST use only the current per-kind authorable fields. Secret fields, encrypted settings, server/runtime fields, and cross-kind fields MUST never be written. File, provider-defined, and Bedrock Datasources MUST fail as non-exportable in this release. | The current server cannot provide a safe complete declaration for those cases. | SC-SAVE-005 |
| **FR-SAVE-023** | If a required authorable value is masked, redacted, encrypted, secret-classified, omitted from the pinned read contract, or otherwise not faithfully reconstructable, the command MUST fail as non-exportable. It MUST NOT write a mask token, placeholder, empty value, discovered default, or partial declaration. | Prevents silent state loss and secret export. | SC-SAVE-005 |
| **FR-SAVE-024** | The tool MUST render deterministic canonical YAML. Two saves of the same normalized server state to equivalent empty destinations MUST produce byte-identical YAML. | Produces stable reviewable diffs. | SC-SAVE-001–004 |
| **FR-SAVE-025** | The command MUST reject an existing YAML target before network access and MUST NOT offer a force/replace path in this release. | Protects an already present local declaration. | SC-SAVE-006 |
| **FR-SAVE-026** | The command MUST not begin the final-file write until server resolution, complete reverse projection, secret/non-exportable checks, compatibility checks, and generated-declaration validation all pass. | Prevents knowingly invalid output while acknowledging direct-write failures. | All |
| **FR-SAVE-027** | A successful invocation MUST have completed the direct write of exactly one final YAML file and MUST NOT replace an existing path. It MUST NOT use staging, temporary files, rename-based publication, or a cross-file transaction. | Defines the single-file direct-write boundary. | SC-SAVE-001, 006, 008 |
| **FR-SAVE-028** | Success MUST emit exactly one identity-bearing outcome line to stdout and nothing else. Text mode MUST use `saved <Kind> <project>/<natural-key>`; an ID-selected unmarked Workflow MUST append ` (adoption required on apply)`. JSON mode MUST emit one compact object containing exactly `action: "saved"`, `kind`, `project`, and the applicable `slug`, `name`, or `repo_name`; the ID-selected unmarked Workflow object MUST additionally contain `adoptionRequired: true`. `adoptionRequired` MUST be absent for every other success. Failures MUST leave stdout empty and emit exactly one safe diagnostic line to stderr. | Preserves the existing structured per-entity output contract and makes the saved identity unambiguous. | All |
| **FR-SAVE-029** | `save` MUST NOT emit YAML or content to stdout, and MUST NOT include target URL, output path, server ID, response content, user identity, timestamp, Git/CI provenance, adoption UUID, or any field beyond FR-SAVE-028 in the successful outcome. The explicit project and natural selector required by FR-SAVE-028 are approved non-sensitive identity fields. | Avoids content leakage while preserving the current per-entity identity and provenance boundary. | All |
| **FR-SAVE-030** | `save` MUST apply the compatibility and response-budget rules in IR-SAVE-001–006 before local publication. | Fails closed against drift and resource abuse. | SC-SAVE-007 |
| **FR-SAVE-031** | Save MUST validate only the one generated declaration. It MUST NOT walk or scan a repository, discover or validate a repository closure, or require referenced declarations to exist locally. `--repo-root` and `--follow-symlinks` MUST be rejected as unknown options. | Aligns output with the parent single-file processing boundary. | SC-SAVE-001–008 |
| **FR-SAVE-032** | Skill main content MUST be emitted inline as `spec.content`; save MUST produce no generated sidecar. | Ensures every save has one declaration output. | SC-SAVE-001 |
| **FR-SAVE-033** | After all server reads, reverse projection, confidentiality checks, and single-declaration validation pass, save MUST write directly to the requested final YAML path. It MUST NOT use a staging file, temporary file, rename-based publication, or atomic/multi-file publication protocol. It MUST refuse a target that is already present and MUST NOT offer force/replace. A write failure MAY leave an incomplete new final file; the command MUST exit 2, leave stdout empty, emit `E_OUTPUT_WRITE`, and MUST NOT report `saved`. | Defines the intentionally simple one-file output behavior and its observable failure mode. | SC-SAVE-001, 006, 008 |

**v3 supersession rule:** FR-SAVE-031–033 supersede the repository/config,
prospective-closure, Skill-sidecar, staging, atomicity, publication ordering,
rollback, and temporary-file clauses retained in historical FR-SAVE-005/006,
FR-SAVE-013/016/018–020, FR-SAVE-024–027, DR-SAVE-006/007/009,
QR-SAVE-001/002/004/007, BR-SAVE-006, and their earlier acceptance text.
Those historical clauses are not approved behavior.

### 14.1 Approved command surface

```text
codemie-gitops save --kind Assistant --slug <slug> --file <yaml-path>
                     --project <project> [--url <url>] [--output text|json]

codemie-gitops save --kind Workflow --slug <slug> --file <yaml-path>
                     [--id <canonical-uuid>]
                     --project <project> [--url <url>] [--output text|json]

codemie-gitops save --kind Skill --name <name> --file <yaml-path>
                     --project <project> [--url <url>] [--output text|json]

codemie-gitops save --kind Datasource --repo-name <repo-name>
                     --file <yaml-path>
                     --project <project> [--url <url>] [--output text|json]
```

Rules:

- `--kind` values are exactly `Assistant`, `Workflow`, `Skill`, and
  `Datasource`.
- Exactly the selector associated with the selected kind is accepted.
- `--id` is accepted only for Workflow and only together with `--slug`.
- Without `--id`, Workflow `--slug` selects a valid persisted marker.
- With `--id`, Workflow `--slug` defines the future natural identity for an
  unmarked exact-ID candidate; it is not a display-name search.
- `--file` names a new YAML destination. It never means an input declaration
  on this command.
- `--output` controls only the outcome/diagnostic format, not the saved YAML.
- `--repo-root` and `--follow-symlinks` are not part of the command surface.
- Unknown flags and secret-bearing flags fail as `E_USAGE`, exit 2, before
  network access.

## 15. Data requirements

### DR-SAVE-001 — Declaration envelope

Every successful YAML MUST contain exactly one closed declaration with this
top-level order:

1. `apiVersion`
2. `kind`
3. `metadata`
4. `spec`

`apiVersion` is `codemie.epam.com/v1alpha1`. `metadata` always contains
`project` followed by the kind key: `slug`, `name`, or `repo_name`.

### DR-SAVE-002 — Reverse field classification

Before implementation, every field in each consumed pinned response MUST have
exactly one reviewed classification:

1. direct authorable field;
2. authorable field requiring a bounded reverse transformation;
3. managed-entity reference requiring natural-key recovery;
4. mixed-owned Workflow metadata;
5. known safely excluded server/audit/runtime field;
6. known secret or masked field that is never persisted; or
7. required but non-exportable state that makes the entity fail.

An unclassified consumed field or unknown non-additive shape is incompatible;
it does not widen the declaration language. Known additive fields classified
as irrelevant to authored state may be ignored only when the pinned save
contract explicitly records that classification.

### DR-SAVE-003 — Current-state materialization

The output represents current readable authorable state, not the original UI
request and not the server's declaration of defaults. Therefore:

- a concrete current value is emitted even if the server originally supplied
  it as a default;
- a current authorable null is emitted as YAML null;
- required current empty lists/maps are emitted explicitly;
- create-only fields are emitted when required by the declaration and
  available from the read contract, even though a later update omits them;
- absence is not replaced by a default or placeholder; and
- lossless reconstruction is evaluated before target existence can influence a
  later apply operation.

### DR-SAVE-004 — Managed references

The only output forms for references among the four managed kinds are the
parent specification's natural keys:

- Assistant `context[].ref` -> Datasource `{project, repo_name}`;
- Assistant `sub_assistants[]` -> Assistant `{project, slug}`;
- Assistant `skills[]` -> Skill `{project, name}`;
- Workflow persisted actor `assistantRef` -> Assistant `{project, slug}`;
- Workflow inline `skillRefs[]` -> Skill `{project, name}`; and
- Workflow inline `datasourceRefs[]` -> Datasource `{project, repo_name}`.

Assistant context names are resolved within the Assistant's exact project,
consistent with the pinned runtime model. Every ID-backed Skill, Assistant, or
Datasource reference still requires exact detail plus ambiguity/visibility
checks appropriate to that kind.

### DR-SAVE-005 — Workflow metadata and IDs

- The reserved Workflow identity member is never authored in `spec.meta_config`.
- A valid reserved member supplies identity for natural Workflow selection and
  is removed from the authored non-reserved map.
- For an ID-selected unmarked Workflow, `metadata.slug` is the caller's
  validated `--slug`.
- Workflow server UUID, creator/update IDs, and history IDs are excluded.
- Workflow-local actor/state IDs remain unchanged.
- No local adoption-state file is created.

### DR-SAVE-006 — Skill content and companion files

- Let the requested YAML basename be `B`.
- If `B` ends in `.yaml` or `.yml`, the default main-content sidecar basename
  is `B` with that final suffix replaced by `.md`.
- Otherwise, the sidecar basename is `B.md`.
- The sidecar is in the YAML's same parent directory.
- `spec.contentFrom` contains only that relative basename using its UTF-8 name;
  it is not an absolute path.
- The sidecar bytes are the exact UTF-8 encoding of the decoded server content,
  without BOM insertion, newline conversion, or an added trailing newline.
- `spec.content` is absent.
- Companion files remain entries in `spec.companion_files`, ordered
  deterministically by their normalized `path`; their content is not silently
  replaced by extra filesystem sidecars.
- Duplicate companion paths or a path/content/encoding/size mismatch makes the
  Skill non-exportable.

### DR-SAVE-007 — Canonical serialization

Canonical YAML MUST satisfy all of the following observable rules:

- UTF-8 without BOM;
- LF line endings and exactly one final newline for the YAML document;
- no document marker, tags, anchors, aliases, or merge keys;
- envelope and metadata order from DR-SAVE-001;
- known `spec` and closed nested-object properties in the checked-in declaration
  schema's canonical property order;
- free-form map keys ordered recursively by Unicode scalar value;
- ordered domain lists preserve their server order;
- fields with set semantics use the pinned save-contract ordering and reject
  duplicates rather than producing unstable order;
- scalar values preserve type and value; non-finite numbers are rejected; and
- a canonical serializer golden contract MUST make identical normalized input
  byte-identical across supported platforms.

The solution architect owns the exact quoting and block-scalar representation,
provided these observable guarantees and byte-stability goldens are met.

### DR-SAVE-008 — Datasource exportability

- Datasource branch selection MUST use this closed reverse mapping from the
  pinned server fields to the current declaration fields:

  | Persisted server `index_type` | Persisted server `vcs_type` | Declaration `spec.index_type` | Declaration `spec.indexType` |
  |---|---|---|---|
  | `code`, `summary`, or `chunk-summary` | `git` | `git` | Exact persisted server `index_type` value |
  | `code`, `summary`, or `chunk-summary` | `svn` | `svn` | Exact persisted server `index_type` value |
  | `knowledge_base_confluence` | Not a branch discriminator | `confluence` | Absent |
  | `knowledge_base_jira` | Not a branch discriminator | `jira` | Absent |
  | `knowledge_base_xray` | Not a branch discriminator | `xray` | Absent |
  | `knowledge_base_azure_devops_wiki` | Not a branch discriminator | `azure_devops_wiki` | Absent |
  | `knowledge_base_azure_devops_work_item` | Not a branch discriminator | `azure_devops_work_item` | Absent |
  | `knowledge_base_sharepoint` | Not a branch discriminator | `sharepoint` | Absent |
  | `knowledge_base_file` | Not a branch discriminator | `file` | Absent |
  | `llm_routing_google` | Not a branch discriminator | `google` | Absent |

- A code row is classifiable only when persisted `index_type` is exactly one
  of `code`, `summary`, or `chunk-summary` and persisted `vcs_type` is exactly
  `git` or `svn`. A missing, differently typed, unknown, or legacy
  discriminator combination, including persisted `index_type: svn`, is
  incompatible with the pinned baseline and MUST fail with
  `E_API_INCOMPATIBLE` before publication. The command MUST NOT infer the VCS
  from the repository URL, default `vcs_type`, or interchange `index_type` and
  `vcs_type`.
- `vcs_type` MUST NOT select or alter a non-code declaration branch. Provider,
  Bedrock, and any other persisted `index_type` absent from the closed mapping
  remain non-exportable or incompatible as classified below; a live response
  cannot widen the declaration union.
- `setting_id` and other schema-approved opaque non-secret configuration IDs may
  be retained.
- Credentials, `access_token`, `expires_at`, provider OTP, encrypted settings,
  processed content, runtime states, usage, and server IDs are excluded.
- The command MUST NOT call `/v1/index/{id}/settings` or treat encrypted
  settings as declaration input.
- SharePoint may be saved only when every declaration-required non-secret field
  is reconstructable without its stored token material.
- File is non-exportable because parent v33.3 requires paths to original local
  source files, while the pinned detail read exposes only filenames/processing
  state, not the original source bytes from which to create those files.
- Provider-defined and Bedrock rows are non-exportable because no matching
  current declaration branch exists.

### DR-SAVE-009 — Local path and retention

- Save does not resolve or inspect a repository root.
- The YAML parent directory MUST already exist.
- The final output path MUST be non-existing.
- New final output paths MUST NOT be symlinks, traverse a symlinked output path,
  or alias another target.
- `save` MUST NOT retain a response cache, ID map, adoption map, backup,
  intermediate declaration, staging file, or secret-bearing temporary file after the
  invocation.
- The requested YAML is the only intentional retained artifact. A failed
  direct write may leave that final path incomplete under FR-SAVE-033.

## 16. Integration requirements

### IR-SAVE-001 — Server protocol

`save` is a direct authenticated REST reader against the pinned baseline. It
uses only the operation-applicable GET routes required for identity,
visibility, detail, references, Skill companion content, and compatibility. It
does not use direct database access, UI scraping, or a modifying route.

### IR-SAVE-002 — Identity and visibility reads

- Assistant uses the pinned exact project+slug detail route and requires read
  ability.
- Workflow natural selection uses both required exhaustive list scopes and
  detail.
- Workflow ID selection uses exact by-ID detail plus the same complete marker
  scans needed to prove adoption eligibility.
- Skill uses exhaustive paginated list then exact by-ID detail.
- Datasource uses exhaustive paginated `full_response=true` list then exact
  by-ID detail; `find_id` is never uniqueness proof.
- Workflow, Skill, and Datasource resolution require global admin, global
  maintainer, or project-admin evidence for the exact effective project.
- `save` requires read access, not server write access. A saved declaration is
  not an authorization claim.

### IR-SAVE-003 — Reference reads

Reference resolution may read Assistant, Skill, or Datasource detail and the
complete list/visibility evidence required to prove the referenced natural key
is portable and unambiguous. A reference route failure fails the selected
entity; partial reference conversion is prohibited.

### IR-SAVE-004 — Authentication and transport

The command inherits the approved target-URL validation, HTTPS/loopback rule,
system trust, redirect restriction, environment-only token input, request
timeout, invocation deadline, response-header/body limits, JSON depth limit,
pagination budgets, and bounded GET retry behavior. It does not acquire a token
implicitly.

### IR-SAVE-005 — Compatibility identity

The source-derived target baseline is tag `2.42.0`, commit
`2a481c290c99bf30ef80aadafa03d876a7f5f732`. `GET /v1/info.version` is semantic
observability only. The exact pinned backend is compatible when every
operation-applicable save read and reverse-projection invariant passes, even
though it reports `APP_VERSION=0.16.0`. That semantic value cannot approve a
different shape or reject the exact pinned source.

### IR-SAVE-006 — Fail-closed response handling

Every consumed response must satisfy a source-pinned save-read contract before
publication. Missing required fields, unexpected type changes, invalid page
origin/echo/count, repeated IDs, unclassified non-additive fields, truncated
content, invalid metadata encoding, or response-budget exhaustion fails closed.
An otherwise compatible but unstable identity snapshot is a reconciliation
failure. Raw bodies and server text are discarded after bounded decoding and
never enter diagnostics or non-target persistence.

## 17. Quality requirements

| ID | Requirement |
|---|---|
| **QR-SAVE-001 — Determinism** | Given the same normalized server state, same target basename, and same current schema, saved YAML MUST be byte-identical across repeated runs and supported platforms. |
| **QR-SAVE-002 — Offline validity** | Immediately after success, `codemie-gitops lint --file <saved-yaml>` MUST exit 0 without network access, neighboring-file reads, or file modification. |
| **QR-SAVE-003 — Read-only server safety** | Instrumented acceptance evidence MUST show zero POST, PUT, PATCH, and DELETE requests for success and every failure path. |
| **QR-SAVE-004 — Direct-write integrity** | Success MUST mean the one final YAML write completed. Save MUST refuse an existing target. A write failure or crash MAY leave an incomplete newly created YAML path and MUST never be reported as success. |
| **QR-SAVE-005 — Confidentiality** | Known credentials, secret-classified fields, masked substitutes, auth headers, cookies, raw bodies, managed-entity IDs, and source content outside the intentional declaration fields MUST NOT be printed, logged, cached, or retained. There is no debug/verbose exception. |
| **QR-SAVE-006 — Bounded operation** | The inherited 300-second invocation deadline, 60-second per-request timeout, response limits, pagination limits, and single-invocation concurrency limit apply. |
| **QR-SAVE-007 — Portability** | The YAML output MUST be usable as the sole `--file` input to lint/apply on every supported platform. |
| **QR-SAVE-008 — Audit separation** | The tool MUST NOT add timestamps, source environment, server UUID, current user, Git author, commit, CI-run identity, or replacement provenance to declaration or outcome. Git, CI, and platform systems retain their own audit records. |
| **QR-SAVE-009 — Maintainability** | Any backend or declaration-schema change that affects reverse projection MUST update a reviewed save-read/reverse-projection contract and positive/negative goldens before the target is accepted. |

## 18. Business rules

- **BR-SAVE-001:** Local YAML is the desired-state record only after the team
  reviews/governs it; `save` itself does not transfer platform ownership.
- **BR-SAVE-002:** A successful save never implies that the invoking token can
  later update the entity.
- **BR-SAVE-003:** Assistant, marked Workflow, unique Skill, and unique
  Datasource are reconciled later by their existing natural-key contracts; no
  generic adoption step is added.
- **BR-SAVE-004:** An unmarked Workflow remains unmarked after save and must be
  explicitly adopted once under the existing runbook.
- **BR-SAVE-005:** Workflow identity metadata is not an ownership marker.
- **BR-SAVE-006:** Save does not enforce local dependency ordering or inspect
  other declarations. It must still recover every emitted natural reference
  exactly from server evidence.
- **BR-SAVE-007:** A duplicate server natural identity is an operator
  remediation condition, not a selector choice.
- **BR-SAVE-008:** A value that cannot be distinguished from a mask or secret
  is not exportable desired state.
- **BR-SAVE-009:** No local path is overwritten even when its bytes happen to
  equal the generated bytes.

## 19. Permissions and authorization

- Any invocation requires a valid bearer token with read access to the selected
  entity and every referenced entity whose detail is needed.
- Assistant direct natural lookup does not require project-admin visibility
  merely to read a unique exact slug.
- Workflow, Skill, and Datasource natural resolution requires a complete
  project view proven by global-admin, global-maintainer, or exact-project
  project-admin evidence, consistent with the existing identity contracts.
- Workflow ID selection also requires complete marker visibility because the
  result is intended for later adoption under that slug.
- `save` does not require server write permission and MUST NOT describe read
  access as write permission or ownership.
- A later `apply` independently rechecks its existing write, visibility,
  identity, and adoption preconditions.
- A 401/expired token is authentication, exit 2. A 403/incomplete visibility is
  authorization, exit 2. Neither falls back to a less complete selector.

## 20. Validation rules

| ID | Rule |
|---|---|
| **VR-SAVE-001** | `--kind`, selector combination, selector syntax, project, URL, repository root, and output mode are validated before network access. |
| **VR-SAVE-002** | The effective project is non-empty and satisfies the existing project-property constraints. |
| **VR-SAVE-003** | Natural selector values satisfy the current metadata-key schema; Workflow `--id` is a canonical UUID. |
| **VR-SAVE-004** | YAML and derived Skill sidecar final paths are distinct, absent, contained under the repository, and have an existing safe parent. |
| **VR-SAVE-005** | A local declaration with the same natural identity at another path makes the prospective repository ambiguous and blocks save. |
| **VR-SAVE-006** | Server identity resolution uses the same pagination and complete-visibility invariants as the corresponding apply resolver. |
| **VR-SAVE-007** | A Workflow reserved record is strictly decoded; malformed, duplicate-key, non-object, mismatched, or conflicting content is never treated as unmarked. |
| **VR-SAVE-008** | Every server reference is transformed only after exact target detail and natural-key validity are proven. |
| **VR-SAVE-009** | Every output field and omission conforms to the current declaration schema and the reviewed reverse-field classification. |
| **VR-SAVE-010** | Known secret fields and masked values are rejected or safely excluded according to their classification before any final file is opened for publication. |
| **VR-SAVE-011** | The prospective repository is run through the same complete offline validation closure as `lint`, including generated Skill sidecar content. |
| **VR-SAVE-012** | Final non-existence is checked again at publication; a race-created file is never replaced. |
| **VR-SAVE-013** | A Datasource reverse projection validates and maps the complete discriminator combination exactly as DR-SAVE-008 before projecting any per-kind authorable field. |

## 21. Acceptance criteria

### AC-SAVE-001 — Save a simple Assistant

```gherkin
Given a readable codemie Assistant with exact project "demo" and slug "helper"
And the target path does not exist
And the prospective repository is valid
When the user runs save for Assistant project "demo" slug "helper"
Then the command makes only non-mutating server requests
And writes one canonical v1alpha1 Assistant declaration
And the declaration contains metadata.project "demo" and metadata.slug "helper"
And no managed-entity server ID appears in the file or output
And stdout contains one saved outcome
And the command exits 0
```

### AC-SAVE-002 — Save a marked Workflow

```gherkin
Given exactly one visible Workflow has a valid marker for project "demo" and slug "flow"
When the user saves Workflow project "demo" slug "flow" without --id
Then the Workflow is selected only by that exact marker
And the reserved marker is absent from authored spec.meta_config
And non-reserved authorable metadata is preserved
And the outcome does not set adoptionRequired
And no server modification occurs
```

### AC-SAVE-003 — Save an unmarked Workflow for later adoption

```gherkin
Given a reviewed canonical Workflow UUID identifies a readable unmarked Workflow in project "demo"
And no valid or invalid marker conflicts with desired slug "flow"
When the user saves Workflow with that --id and --slug "flow"
Then metadata.slug is "flow"
And the UUID is absent from YAML, local state, stdout, and stderr
And the JSON success outcome contains adoptionRequired true
And the server Workflow remains unmarked
And the command exits 0
```

### AC-SAVE-004 — ID cannot bypass a marked Workflow identity

```gherkin
Given the Workflow selected by --id already contains any reserved identity member
When save is invoked with --id
Then the command exits 1 with a safe reconciliation diagnostic
And tells the user to use natural-key selection without echoing the ID or marker value
And writes no local artifact
And makes no server modification
```

### AC-SAVE-005 — Workflow display name never selects

```gherkin
Given two unmarked Workflows share a display name equal to the desired slug
And one exact reviewed UUID is supplied
When save validates the ID-selected unmarked candidate
Then display name neither selects nor vetoes that candidate
And all marker, project, visibility, reference, and representability checks still apply
```

### AC-SAVE-006 — Ambiguous Skill fails

```gherkin
Given two visible Skills have the same exact project and name under different creators
When save is invoked for that project and name
Then neither Skill is selected
And the command exits 1 with E_AMBIGUOUS_IDENTITY
And stdout and all final output paths remain empty or absent
```

### AC-SAVE-007 — Skill main content uses a sidecar

```gherkin
Given exactly one exportable Skill named "triage-skill"
And the requested YAML path is "skills/triage-skill.yaml"
When save succeeds
Then "skills/triage-skill.md" contains the exact UTF-8 server content bytes
And the YAML contains contentFrom "triage-skill.md"
And the YAML does not contain spec.content
And immediate offline lint succeeds
```

### AC-SAVE-008 — Skill companion files are complete

```gherkin
Given a Skill detail lists two companion files
When save succeeds
Then both companion contents were read and validated
And both appear in spec.companion_files in canonical path order
And no companion content is missing or replaced by metadata-only output
```

### AC-SAVE-009 — Skill sidecar collision is all-or-none

```gherkin
Given the YAML target is absent
And the derived Markdown target already exists
When save is invoked
Then the command exits 2 before any network request
And the existing Markdown bytes are unchanged
And no YAML is created
```

### AC-SAVE-010 — Reverse managed references

```gherkin
Given a Workflow detail contains one persisted Assistant server ID
And one inline actor contains Skill and Datasource server IDs
And each referenced entity resolves to exactly one valid natural key
And matching dependency declarations already exist locally
When save succeeds
Then the persisted actor contains assistantRef only
And the inline actor contains skillRefs and datasourceRefs only
And no assistant_id, skill_ids, or datasource_ids appear in those server-resource positions
And workflow-local actor IDs and state assistant_id values are preserved
```

### AC-SAVE-011 — Missing local dependency blocks publication

```gherkin
Given the server entity references an exact readable Skill
But no matching local Skill declaration exists in the prospective repository
When save reaches prospective offline validation
Then it exits 2 with E_REFERENCE
And identifies only the canonical reference field path
And writes no YAML or sidecar
```

### AC-SAVE-012 — Result is immediately lint-valid

```gherkin
Given save reports success
When lint is immediately run on the saved YAML using the same repository and config
Then lint exits 0 without network access
And lint does not modify the saved artifacts
```

### AC-SAVE-013 — File Datasource is refused

```gherkin
Given the unique selected Datasource has persisted index_type "knowledge_base_file"
When save reads its detail
Then it classifies the declaration branch as spec.index_type "file"
And it does not treat uploaded or processed filenames as source file content
And exits 1 with E_ENTITY_NOT_EXPORTABLE
And no final file is written
And no content or filename value appears in the diagnostic
```

### AC-SAVE-014 — Secret Datasource fields never leave the server response boundary

```gherkin
Given a SharePoint detail includes stored access-token state
And all non-secret declaration-required fields are reconstructable
When save succeeds
Then access_token, expires_at, cookies, and encrypted settings are absent from YAML, stdout, stderr, logs, and retained temporary data
And schema-approved non-secret configuration references retain their exact values
```

### AC-SAVE-015 — Required masked state fails

```gherkin
Given an Assistant has a sensitive prompt-variable default returned only as a masked substitute
When save attempts reverse projection
Then it exits 1 with E_ENTITY_NOT_EXPORTABLE
And does not write the mask, an empty value, or a placeholder
And the diagnostic does not reproduce the variable value or mask token
```

### AC-SAVE-016 — Existing YAML is never overwritten

```gherkin
Given the requested YAML path already exists
When save is invoked
Then it exits 2 with E_OUTPUT_EXISTS before network access
And the existing file remains byte-identical
And no backup or replacement file is created
```

### AC-SAVE-017 — Publication race does not clobber

```gherkin
Given the target was absent during initial validation
But another process creates it before publication
When save publishes
Then save does not replace or truncate the raced file
And exits 2 with stdout empty
And removes any other final artifact created by this invocation
```

### AC-SAVE-018 — Failure has no partial output

```gherkin
Given a Skill requires YAML and Markdown output
And an ordinary publication error occurs
When save returns failure
Then neither invocation-created final path remains
And every pre-existing path is byte-identical
And stdout is empty
```

### AC-SAVE-019 — Canonical output is stable

```gherkin
Given two equivalent empty repositories with the same config and dependencies
And the normalized server entity state is identical
And the requested target basenames are identical
When save succeeds once in each repository
Then corresponding YAML bytes are identical
And corresponding Skill sidecar bytes, when applicable, are identical
```

### AC-SAVE-020 — Exact pinned clone is compatible

```gherkin
Given the target is built from backend tag 2.42.0 commit 2a481c290c99bf30ef80aadafa03d876a7f5f732
And GET /v1/info reports version 0.16.0
And every operation-applicable save read contract passes
When save runs
Then the version string does not independently reject the target
And save proceeds to normal resolution and publication
```

### AC-SAVE-021 — Incompatible response fails before local writes

```gherkin
Given a required detail field or pagination invariant does not match the pinned save-read contract
When save runs
Then it exits 2 with E_API_INCOMPATIBLE
And no final output path is created
And stdout is empty
And stderr contains no raw response or server text
```

### AC-SAVE-022 — Server remains read-only on all paths

```gherkin
Given an instrumented target records every HTTP method
When each successful and failing save scenario is exercised
Then the record contains no POST, PUT, PATCH, or DELETE from save
And an unmarked Workflow remains byte-for-byte unchanged on the server
```

### AC-SAVE-023 — Text and JSON success output

```gherkin
Given a Skill save succeeds for project "demo" and name "triage-skill"
When output mode is text
Then stdout is exactly one line equivalent to "saved Skill demo/triage-skill"
And an ID-selected unmarked Workflow uses one line equivalent to
"saved Workflow demo/flow (adoption required on apply)"
When output mode is json
Then stdout is exactly one compact object with action, kind, project, and name
And neither form contains a path, URL, ID, content, timestamp, or user

Given an ID-selected unmarked Workflow save succeeds for project "demo" and slug "flow"
When output mode is json
Then stdout is exactly one compact object with action "saved", kind "Workflow", project "demo", slug "flow", and adoptionRequired true
And adoptionRequired is absent from every other save success outcome
```

### AC-SAVE-024 — Failure stream and diagnostic safety

```gherkin
Given any save failure under any supported verbosity or panic boundary
When the command terminates
Then stdout is empty
And stderr contains exactly one selected-mode diagnostic line
And the diagnostic contains only approved allowlisted fields
And it contains no selector value copied from a response body, server text, URL, token, ID, content, or secret
```

### AC-SAVE-025 — Code Datasource uses separate VCS and strategy fields

```gherkin
Given the unique selected Datasource has persisted index_type "summary"
And it has persisted vcs_type "svn"
And every required authorable field is faithfully reconstructable
When save reverse-projects the Datasource
Then the declaration contains spec.index_type "svn"
And the declaration contains spec.indexType "summary"
And changing only persisted vcs_type to "git" changes spec.index_type to "git"
And does not change spec.indexType
And the persisted fields are neither interchanged nor inferred from the repository URL
```

### AC-SAVE-026 — Invalid code discriminator combination is incompatible

```gherkin
Given the unique selected Datasource has persisted index_type "chunk-summary"
And persisted vcs_type is missing, not a string, or not exactly "git" or "svn"
When save evaluates the pinned reverse-projection contract
Then it exits 2 with E_API_INCOMPATIBLE
And no final output path is created
And stdout is empty
And the diagnostic does not contain the invalid value or raw response
```

### AC-SAVE-031 — Save does not inspect a repository

```gherkin
Given save can resolve and reverse-project one server entity
And directories around the requested output contain declarations, invalid YAML, and symlinks
When save is invoked
Then it does not enumerate or open those surrounding files
And it validates only the generated declaration
And --repo-root or --follow-symlinks would instead fail as E_USAGE before network access
```

### AC-SAVE-032 — Skill output is one YAML file

```gherkin
Given a representable Skill with main content
When save succeeds
Then exactly one new YAML file is retained
And its spec.content contains the main content inline
And no Markdown sidecar is created
```

### AC-SAVE-033 — Direct write failure is visible

```gherkin
Given all reads and generated-declaration validation have succeeded
And the requested final path did not exist at the preflight check
When the direct final-file write fails after creating the path
Then save exits code 2
And stdout is empty
And stderr contains the safe E_OUTPUT_WRITE diagnostic
And an incomplete final file may remain
And save does not use a temporary file, rename, rollback, or cleanup of that final path
```

## 22. Edge cases and failure scenarios

- Missing effective project: exit 2 before network; no config fallback after an
  invalid explicit `--project`.
- Wrong selector for kind or multiple selector flags: `E_USAGE`, exit 2 before
  network.
- Workflow `--id` is non-canonical: `E_USAGE`, exit 2 before network.
- Natural target absent: `E_ENTITY_NOT_FOUND`, exit 1; no file.
- ID-selected Workflow absent or wrong project: reconciliation failure, exit 1;
  the diagnostic does not echo the ID.
- Marked Workflow selected by ID: fail; natural-key selection is required.
- Malformed/duplicate/conflicting Workflow marker: exit 1; never treated as
  unmarked and never copied into YAML.
- Snapshot churn with otherwise valid shapes: `E_RESOLUTION_UNSTABLE`, exit 1.
- Incomplete project visibility: `E_VISIBILITY_UNPROVEN`, exit 2.
- Referenced Assistant has no slug: selected entity is non-exportable; no
  fallback to name or ID.
- Referenced Skill/Datasource natural key is ambiguous: missing/ambiguous server
  reference, exit 1; no output.
- Referenced dependency has no local declaration: offline `E_REFERENCE`, exit
  2; save dependency first.
- Cyclic dependency group with no lint-valid one-at-a-time ordering: unsupported
  by this initial command; no partial group is saved. Manual reviewed authoring
  or a future atomic batch feature is required.
- Unsupported Assistant type: non-exportable under the current codemie-only
  declaration schema.
- Autonomous/deprecated Workflow state is non-exportable in v1 because the
  pinned exhaustive read routes exclude it.
- Skill content outside schema limits or non-UTF-8-representable companion
  content: non-exportable; no sidecar or YAML.
- Skill companion metadata/content changes during reads: unstable resolution,
  exit 1; no mixed snapshot.
- A suspected secret in a field classified as author content but not a known
  secret position is subject to the existing lint heuristic. `save` cannot
  prove arbitrary prose contains no secret; reviewers remain responsible for
  content review. Known secret/masked positions always fail or are excluded as
  specified.
- Datasource kind is File, provider, or Bedrock: known non-exportable, exit 1.
- Datasource has a code-strategy `index_type` with missing or invalid
  `vcs_type`, or a legacy/inconsistent persisted discriminator combination:
  incompatible, exit 2; do not infer from URL, storage defaults, or other
  fields.
- Datasource response contains an unclassified authoring-relevant field:
  incompatible, exit 2, rather than silent omission.
- Output parent absent, unsafe, unreadable, or unwritable: local output failure,
  exit 2 before the direct write when detectable.
- Existing output path is a directory, special file, hard-link alias, or
  symlink: reject; no network when detectable locally.
- Disk-full, permission failure, cancellation, or timeout during the direct
  write: exit 2 with `E_OUTPUT_WRITE`; an incomplete newly created final YAML
  may remain and is not automatically removed.

## 23. Dependencies

| Dependency | Owner | Why it matters | Required evidence | Blocking status |
|---|---|---|---|---|
| Approved parent declaration schema | Product specification owner / architect | Save output must be valid input to current lint/apply | Exact checked-in v1alpha1 schema and field classifications | Available |
| Pinned save-read/reverse-projection contract | Solution architect | Current adapter manifest is write-oriented and does not yet fully classify every read field for save | Reviewed per-kind response inventory, reference routes, exclusion/secret classes, and goldens | Blocks implementation, not architecture planning |
| Complete project visibility | Adopting platform administrator | Workflow/Skill/Datasource natural identity must be exhaustive | Existing exact-project admin/global predicate and deployment evidence | Operational prerequisite |
| Skill companion content reads | CodeMie platform owner / architect | A detail response contains metadata only | Pinned route, query shape, content integrity, budgets, and snapshot consistency evidence | Blocks Skill implementation until verified |
| Filesystem publication contract | Solution architect / security reviewer | YAML+Skill sidecar must be no-overwrite, complete-file, and YAML-last for Skills; no rollback | Cross-platform threat model and failure/cancellation tests | Blocks implementation |
| Extended outcome/diagnostic contracts | Solution architect | Current schemas do not include `saved`, `adoptionRequired`, or save-specific error codes | Versioned closed schema updates and render tests | Blocks implementation |
| Security review | Security reviewer | Server responses can contain credentials, masked values, and proprietary content | Pre-implementation review of reverse field allowlists, paths, temporary data, logs, and response handling | Required before implementation begins |

## 24. Constraints

- The product is implemented outside reference-only `codemie/` and
  `codemie-ui/`; this feature does not authorize edits there.
- The executable remains `codemie-gitops` and the declaration API remains
  `codemie.epam.com/v1alpha1` for this release.
- The command uses documented REST reads, never direct database access.
- The target baseline remains backend tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732` until an approved compatibility
  update.
- Tokens remain environment-only and target transport keeps the approved TLS,
  redirect, and safe-diagnostic controls.
- Managed-entity server IDs remain absent from declarations, retained local
  state, and normal output.
- Existing files are never overwritten.
- Successful output must pass the existing offline declaration contract; save
  cannot widen schemas from live server responses.
- No server write, adoption, or ownership marker is part of `save`.

## 25. Assumptions

| ID | Assumption | Evidence | Risk if false | Confirmation owner |
|---|---|---|---|---|
| **A-SAVE-001** | The pinned detail and companion-content reads can reconstruct every current authorable field for Assistant, marked/unmarked Workflow, Skill, and the non-File authorable Datasource branches except explicitly non-exportable cases. | Direct reference-source inspection and current adapter manifest, but no save-specific reverse contract yet. | Additional entity states or entire branches may need to fail as non-exportable; scope cannot silently widen. | Solution architect + verification engineer |
| **A-SAVE-002** | Assistant context names are scoped to the Assistant's exact project in the pinned runtime model. | Reference server lookups use the Assistant/request project with context name. | Cross-project context would be ambiguous because persisted context lacks project; save must fail rather than infer. | CodeMie platform owner |
| **A-SAVE-003** | The existing validator can validate one generated in-memory declaration without reading neighboring files. | Parent v33.3 requires this boundary; save emits inline Skill content and no File Datasource output. | If false, architecture must isolate the one-file validation behavior before save can conform. | Solution architect |
| **A-SAVE-004** | The repository's existing governance can keep UI/API writers controlled after onboarding. | Parent specification already requires governed writers for identity safety. | Saved state may drift immediately; continuous sync remains out of scope. | Adopting team |

## 26. Conflicts and inconsistencies

### C-SAVE-001 — Export was deferred in the parent specification

- Parent spec §10 and OQ-14 defer server-to-YAML export/bidirectional sync.
- The 2026-08-11 user request explicitly asks for the bounded save command.

**Resolution:** This approved feature supersedes that deferral only for the
one-entity, read-only `save` behavior defined here. Continuous sync and broader
bidirectional behavior remain deferred.

### C-SAVE-002 — Generic adoption remains prohibited

- The goal language says teams should begin “owning” UI-created resources.
- Parent FR-019/020 prohibit generic adoption and ownership markers.

**Resolution:** Saving YAML is local onboarding, not ownership transfer or an
authorization marker. Assistant, Skill, and Datasource later reconcile through
their existing natural identities. Workflow retains the sole explicit ID-based
adoption ceremony, which `save` does not execute.

### C-SAVE-003 — All four kinds versus non-exportable Datasource states

- The requested feature should cover all supported kinds.
- File Datasource requires original local source files for its explicit path
  inputs, but the pinned ordinary detail read does not expose their bytes; provider and Bedrock lack an approved
  declaration branch.

**Resolution:** Datasource is an in-scope entity kind. Known non-reconstructable
Datasource branches fail explicitly rather than being silently omitted or
represented with placeholders. This is not a removal of Datasource support.

### C-SAVE-004 — Offline validity versus one-entity scope

- Existing lint requires every managed reference to exist in the repository.
- Recursive save would expand the feature into batch orchestration.

**Resolution:** Save remains one-entity, validates the full prospective
repository, and requires dependency-first onboarding. Recursive/batch save is
deferred.

### C-SAVE-005 — Datasource branch discriminator versus code strategy

- Version 1 of FR-SAVE-012 and DR-SAVE-008 stated that persisted `index_type`
  alone selects the declaration branch.
- In pinned backend 2.42.0, a Git/SVN code row persists its code strategy in
  `IndexInfo.index_type` and its Git/SVN kind in `IndexInfo.vcs_type`.
- The current declaration schema gives those meanings to separate properties:
  `spec.indexType` is the code strategy and `spec.index_type` is the closed
  branch discriminator.
- The existing apply-oriented adapter manifest consumes `index_type` for
  Datasource resolution but does not yet consume or classify `vcs_type`.

**Resolution:** DR-SAVE-008's closed composite mapping is authoritative for
save. Persisted `vcs_type` selects `git` versus `svn`; persisted `index_type`
maps without renaming its value to declaration `spec.indexType`. Natural
identity remains exact project+`repo_name`, so neither field filters duplicate
rows or breaks ambiguity. Architecture must add `vcs_type` to the pinned
save-read/reverse contract and must not carry forward the single-discriminator
statement.

## 27. Open questions

No product question blocks architecture planning.

The following are architecture-owned questions and MUST preserve the approved
observable behavior:

1. What versioned artifact will hold the exhaustive save-read and reverse-field
   classification alongside the current write adapter manifest?
2. How will the implementation make YAML+Skill-sidecar publication
   no-overwrite and all-or-none for every reported failure and cancellation on
   supported filesystems?
3. How will a prospective in-memory repository view reuse the same validator
   as `lint` without changing lint behavior?
4. What canonical YAML quoting/block rules satisfy DR-SAVE-007 and stable
   cross-platform goldens?
5. How will companion-file snapshot consistency be proven across metadata and
   content reads within the invocation deadline?
6. Which exact pinned read fields are harmless additive server/runtime state,
   and which must make an unknown response incompatible to avoid silent loss?

If architecture evidence disproves A-SAVE-001 for a non-File currently
authorable Datasource branch or another supported entity state, it must report
the gap to the product specification owner. It must not invent a default,
placeholder, secret export, or partial declaration.

## 28. Traceability matrix

| Source | Scenario | Requirements | Acceptance criteria |
|---|---|---|---|
| User request: save server YAML locally | SC-SAVE-001–004 | FR-SAVE-001–004, 013–014 | AC-SAVE-001–003, 007 |
| Parent four-kind scope and declaration schema | SC-SAVE-001–005 | FR-SAVE-003, 013, 022 | AC-SAVE-001, 007, 013–014 |
| Parent natural identity/ref decisions | SC-SAVE-002–004 | FR-SAVE-007–012, 016–018, 021 | AC-SAVE-002–006, 010–011 |
| Parent Workflow adoption decision | SC-SAVE-003 | FR-SAVE-008–010, 021 | AC-SAVE-003–005 |
| User decision 2026-08-13: one YAML output, direct write | SC-SAVE-001/006/008 | FR-SAVE-031–033 | AC-SAVE-031–033 |
| Parent Datasource security/schema boundary | SC-SAVE-005 | FR-SAVE-012, 022–023; DR-SAVE-008; VR-SAVE-013 | AC-SAVE-013–015, 025–026 |
| Parent compatibility identity | SC-SAVE-007 | FR-SAVE-030; IR-SAVE-005/006 | AC-SAVE-020–021 |
| Parent output/diagnostic boundary | All | FR-SAVE-028–029 | AC-SAVE-023–024 |
| Retained visible no-overwrite policy | SC-SAVE-006, 008 | FR-SAVE-025, 033 | AC-SAVE-033 |
| Reference-only CodeMie 2.42.0 reads | SC-SAVE-001–007 | FR-SAVE-007–023; DR-SAVE-008; IR-SAVE-001–006 | AC-SAVE-001–015, 020–022, 025–026 |

## 29. Handoff to solution architect

### Approved product behavior

- Add one `save` command with the exact kind-specific selector surface in
  §14.1.
- Save one entity per invocation; do not recurse.
- Read the server only; never adopt or modify it.
- Resolve project as `--project` then repository config project.
- Select Assistant/Skill/Datasource by natural key and marked Workflow by its
  valid reserved marker.
- Permit server ID only for an unmarked Workflow, with caller-supplied slug and
  full adoption-eligibility checks.
- Keep the reviewed Workflow ID entirely out of retained/output data and mark
  the success outcome `adoptionRequired: true`.
- Reverse managed IDs to existing natural-key references; preserve only
  workflow-local IDs and schema-approved opaque non-secret configuration IDs.
- Materialize current readable authorable state without defaults, guesses,
  placeholders, or masked values.
- Put main Skill content in the deterministic adjacent Markdown sidecar and
  include complete companion-file content through the current schema.
- Refuse File/provider/Bedrock Datasources and any other entity whose required
  authorable state is not safely reconstructable.
- For code Datasources, use persisted `vcs_type` to select declaration
  `spec.index_type` (`git` or `svn`) and map persisted `index_type` unchanged to
  declaration `spec.indexType` (`code`, `summary`, or `chunk-summary`).
- Require the prospective full repository to pass the same offline closure as
  lint before publication.
- Produce deterministic canonical YAML, never overwrite, and publish no
  partial declaration set.
- Emit a safe `saved` outcome or one safe failure diagnostic under the existing
  stream/exit principles.

### Hard constraints

- Do not modify or derive runtime architecture from reference-only `codemie/`
  or `codemie-ui/`.
- Do not add server writes, direct database reads, generic adoption, generic
  ownership markers, recursive save, force overwrite, stdout YAML, or a local
  ID/adoption map.
- Do not weaken the existing token, URL, TLS, redirect, response-budget,
  diagnostic, or provenance boundaries.
- Do not use `/v1/info.version` as source/API identity.
- Do not let a live response widen the declaration schema.
- Do not store known secrets, masks, raw responses, or managed-entity IDs.
- Do not publish output that fails current offline lint.

### Architecture decisions required

- Define and version the pinned save-read/reverse-projection contract for every
  entity, supported Datasource branch, field, and reference route.
- Correct the current apply-oriented Datasource read contract so save consumes
  and validates `vcs_type` for code rows separately from `index_type`; include
  positive Git/SVN strategy cross-product and invalid-combination goldens.
- Extend the closed outcome schema for `saved` and optional
  `adoptionRequired: true` without changing existing outcome shapes.
- Extend the closed diagnostic schema with at least:
  `E_ENTITY_NOT_FOUND` and `E_ENTITY_NOT_EXPORTABLE` as exit-1 reconciliation
  errors, and `E_OUTPUT_EXISTS`, `E_OUTPUT_PATH`, and `E_OUTPUT_WRITE` as exit-2
  local-output errors.
- Define direct final-path creation and write behavior that preserves the
  existing-target refusal, reports partial-file write failures as specified,
  and introduces no staging, temporary-file, rename, or atomic publication
  mechanism.
- Define canonical YAML scalar representation and cross-platform goldens.
- Reuse the single-declaration validation behavior for generated save output;
  do not construct a prospective repository view.
- Define companion-content consistency and bounded-read evidence.

### Decisions the architect must not reinterpret

- `save` is not `apply`, adoption, ownership transfer, or drift sync.
- No ID selector exists for Assistant, Skill, or Datasource.
- No natural-identity ambiguity may be resolved by an ID, owner, newest, first,
  name, or list order.
- An ID-selected Workflow must be unmarked; the command does not add the
  marker.
- File Datasource filenames are not substitutes for original source bytes.
- Datasource `index_type` alone does not select Git versus SVN, and neither
  `index_type` nor `vcs_type` participates in the Datasource natural key.
- A mask, redaction token, empty value, server default, or omitted field is not
  a substitute for unavailable authorable state.
- Missing local dependencies fail before publication; the command does not
  save a broken reference graph.
- No existing output path is replaced, even when byte-identical.
- Successful output does not include path, URL, UUID, user, time, Git, or CI
  provenance.

### Required lifecycle

1. Solution architecture adds the read/reverse contracts, updated closed
   schemas, security design, and implementation tasks.
2. Verification independently checks this specification against those
   artifacts before implementation.
3. Security review approves response-field allowlists, secret/mask handling,
   direct-output failure handling, and diagnostics.
4. Implementation proceeds only after those reviews converge.

## 30. Readiness assessment

```text
Specification status: READY FOR ARCHITECTURE PLANNING
```

The product decisions that materially affect architecture are resolved: exact
command name and selectors, all-four-kind boundary, read-only server behavior,
effective project, unmarked Workflow handling, natural-reference reversal,
single-generated-declaration validation, inline Skill content, Datasource
non-exportability and secret rules, canonical output, direct single-file write
with visible partial-file failure behavior, exit/output behavior, and
compatibility gating. The remaining questions are
architecture and verification questions about satisfying these approved
observable requirements, not unresolved product scope.
