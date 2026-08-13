# Behavior-Preservation Inventory: v33/v3 rebaseline

## Status and rule

Current implementation tests are evidence of behavior, not authority. Preserve
only behavior still approved by the v33/v3 specifications. Superseded tests
must be deleted or rewritten as negative-boundary evidence.

## Preserve

| Behavior | Current evidence area | Required action |
|---|---|---|
| Closed stdout success and one safe stderr diagnostic on failure | `tests/cli_lint.rs`, render/output unit tests | Retain and expand to save partial-write failures. |
| Strict marked YAML, duplicate-key/schema/resource limits | parse/declaration tests | Retain for one selected file. |
| Environment-only secrets, validated URL/TLS/redirect policy | config/auth/http tests | Retain; remove config-file source cases. |
| Pinned compatibility, bounded HTTP/pagination, zero blind mutation retry | HTTP/adapter tests | Retain. |
| Kind-specific identity, authorization, race, preservation, and post-write verification | adapter/coordinator tests | Retain. |
| Strict reverse DTOs, secret/mask rejection, canonical YAML | save tests/contracts | Retain; change Skill expected output to inline content. |
| Workflow-local actor/state reference validation | validate tests | Retain within selected declaration. |

## Replace with new evidence

| Superseded behavior | New required evidence |
|---|---|
| Repository discovery, exclusions, ordering, 10,000-file cap | Only selected `--file` is opened; parent/neighbors are not enumerated. |
| Repository config/default project | Config file is never opened; explicit project/flags/environment behavior. |
| Skill `contentFrom` through repository-root machinery | Retain the authoring form through one 128 KiB direct declaration-relative safe read; remove root/walking coupling. |
| Cross-file duplicate/missing/wrong-kind graph validation | Lint accepts structurally valid refs without local declarations; apply resolves/fails online before write. |
| File Datasource path streaming | Retain 1–10 explicit direct reads with 32 MiB/file and 128 MiB aggregate bounds; remove root/walking/temp coupling. |
| `--follow-symlinks` and `--repo-root` behavior | Both are unknown options before filesystem/network access. |
| Cancellation checkpoints in parser/sidecar/file loops | Invocation timeout and bounded one-file read tests; no token API assertion. |
| Save overlay/prospective repository validity | Generated declaration validates in memory with zero filesystem opens. |
| Staged atomic no-replace and orphan-sidecar behavior | Direct create-new write, untouched existing target, and diagnosed permitted partial final file. |

## Blocking pre-implementation gaps

1. Compiled-process filesystem-open tracing for one-file lint/apply/save.
2. Online-reference resolution tests covering missing, ambiguous, inaccessible,
   incompatible, and successful evidence with mutation capture.
3. Direct-write fault injection after final creation.
4. Updated CLI/schema/help goldens for removed options and inline Skill content.
5. Independent confirmation that no old preservation test is misclassified as
   a regression requirement.
