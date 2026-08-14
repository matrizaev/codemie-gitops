# Pre-implementation security review: save server entity

## Status

**APPROVED WITH NON-BLOCKING FINDINGS**

Review date: 2026-08-12.

This review covers the approved save design and the pinned CodeMie read
contract. It does not approve implementation or release. Implementation may
start after Q-SAVE-001/Q-SAVE-002 evidence is refreshed against the updated
manifest.

### Current implementation addendum — 2026-08-14

The implementation now applies the reviewed API-to-declaration boundary:
toolkit, tool, MCP, and integration-settings responses are allowlisted into
declaration fields, while nested API config and credential-bearing values are
excluded. This addendum records implementation alignment only; it does not
replace the historical pre-implementation security decision or constitute a
release approval.

## Scope and evidence

Reviewed:

- `spec.md`, `plan.md`, `data-model.md`, `tasks.md`;
- accepted ADR-013 through ADR-017;
- all contracts under `contracts/`;
- the pinned reverse manifest and declaration schema; and
- the reference-only `codemie/` checkout at tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.

The `codemie/` and `codemie-ui/` trees are reference-only and were not
modified.

## Security decision

The design has no unresolved high or critical security finding. The primary
controls are adequate in principle:

- save receives only GET-capable transport capability;
- response DTOs are closed and fail closed on unknown contracted fields;
- secret, masked, managed-ID, audit, and runtime fields do not cross the
  projection boundary;
- MCP custom configuration or auth-token states are non-exportable;
- generated YAML is validated through the existing repository closure before
  publication;
- paths are repository-relative, parent traversal does not follow symlinks,
  and existing leaves are rejected;
- files are fully staged before publication;
- final publication uses atomic no-replace operations;
- Skill YAML is published after its complete main-content sidecar; and
- no rollback operation removes a final path after publication.

## Threat review

| Area | Finding | Disposition |
|---|---|---|
| Server response injection | Malicious or changed response fields could bypass projection | **Controlled** by closed DTOs, manifest classification, bounded decoding, and fail-closed compatibility errors. Mutation tests remain required. |
| Secrets and masks | Prompt defaults, MCP custom config/auth tokens, integration credentials, SharePoint/provider secrets, or masked substitutes could be emitted | **Controlled** by non-retaining secret sinks and explicit non-exportability predicates. No debug output path is allowed. |
| Managed IDs | Server UUIDs could leak through YAML, sidecars, logs, diagnostics, or transient reference maps | **Controlled by design**: IDs are transient typed values and are excluded before AST construction. Recursive canary tests remain required. |
| Workflow adoption | Save could mutate or mark a Workflow, or expose its UUID | **Controlled**: save has no modifying route; adoption remains a later explicit apply operation; output omits the UUID. |
| Path traversal | User-selected YAML or derived sidecar path could escape the repository | **Controlled** by normalized repository-relative paths, no-follow parent traversal, containment checks, and pre-network validation. |
| Symlink/reparse attacks | Final paths or parents could redirect writes | **Controlled** by no-follow traversal and rejection of existing final leaves. Platform-specific tests remain required. |
| Existing-file clobber | A check-then-write race could replace repository content | **Controlled** by native same-directory atomic no-replace publication; fallback is forbidden. |
| Partial files | Readers could observe truncated YAML or sidecar | **Controlled** by complete staging and atomic final rename. YAML is published last for Skills. |
| Orphan sidecar | A crash or failed YAML rename could leave a complete Markdown sidecar | **Accepted residual behavior**. It is not a declaration, cannot cause YAML-without-sidecar, and is documented for manual review/removal. Save never removes final paths. |
| Cancellation/deadline | Cancellation could leave staging entries or change a committed result | **Controlled** by cleanup before final rename and deferred cancellation through the publication attempt. Post-YAML cancellation cannot downgrade success. |
| Logs/diagnostics | Raw URLs, paths, bodies, tokens, content, or OS errors could leak | **Controlled** by typed safe renderers and bounded error classification. Panic/backtrace and tracing tests remain required. |
| Resource exhaustion | Large pages, content, or companion payloads could exhaust memory or deadline | **Controlled** by inherited body, page, depth, aggregate-content, retry, and invocation budgets. Sequential Skill reads are bounded. |

## Required implementation evidence

Before implementation is considered complete, tests must demonstrate:

1. zero POST/PUT/PATCH/DELETE requests on every success and failure path;
2. secret and managed-ID canaries absent from artifacts, stdout, stderr, and
   logs;
3. unknown-field, missing-field, duplicate-key, invalid-encoding, and response
   budget failures are fail-closed;
4. path traversal, symlink/reparse, hard-link, destination-race, and
   independent-process no-replace cases preserve existing files;
5. staging files are owner-only and are removed on pre-publication failure;
6. a post-sidecar/pre-YAML failure can leave only a complete orphan sidecar and
   never a YAML without its sidecar;
7. cancellation at each checkpoint does not remove an existing or
   race-created path;
8. diagnostics contain only approved safe fields and no raw OS/server values;
9. explicit server nulls either appear as YAML `null` and pass the declaration
   schema or fail before publication; and
10. all claimed OS/filesystem combinations pass the native no-replace
    qualification suite.

## Non-blocking findings

- The orphan-sidecar recovery is manual and should be included in operator
  documentation and help text.
- Platform qualification must name the supported OS/filesystem pairs; network
  and userspace filesystems remain unsupported until tested.
- Q-SAVE-001 must refresh its field and route evidence after the manifest
  corrections. This is a verification gate, not an unresolved security
  design defect.

## Verdict

**APPROVED WITH NON-BLOCKING FINDINGS.** No high or critical security finding
blocks implementation. The implementation must satisfy the evidence list above
and must not weaken the accepted no-clobber, no-secret, no-managed-ID, or
apply-only adoption boundaries.
