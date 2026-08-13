# Rust Architecture Remediation Decisions

## Status

Updated for product specification v33 and save specification v3 on 2026-08-13.

## D-001 — Server contract baseline

The checked-in OpenAPI is primary for immutable wire shapes. Pinned reference-
only source and adapter/reverse manifests supply behavioral evidence that
OpenAPI cannot express. Breaking drift blocks release. Reference-only source is
never modified or included in product tasks.

## D-002 — Reconciliation boundary

Retain kind-specific natural identity, authorization, preservation, pagination,
race, compatibility, post-write verification, and exactly-one-write behavior.
Offline code validates natural-reference shape; apply adapters alone resolve
reference existence online.

Skill `contentFrom` and File Datasource `spec.files` are the only approved
auxiliary inputs. They resolve directly from the selected declaration parent
under containment/no-symlink/regular/bounded rules without walking or temporary
copies; apply inlines Skill content or streams File multipart bytes.

## D-003 — Configuration and secrets

URLs/auth URLs use flags then named environment variables. Project is explicit
in the declaration or save command as contracted. Secret credentials remain
environment-only. `.codemie/config.yaml` and repository-root discovery are
removed, not deprecated fallbacks.

## D-004 — Single-file local processing

Lint/apply open exactly `--file`, perform bounded marked parsing and one-
declaration schema/semantic checks, and inspect no unlisted neighbors. Repository
walking, ordering, closure, `--repo-root`, and `--follow-symlinks` are removed.
A command deadline and bounded reads are required; `CancellationToken` is not a
domain/public API requirement.

## D-005 — Save output

Save renders and validates one inline-content YAML, then writes it directly to
the create-new final path. No overlay, sidecar, staging/temp file, rename,
`rustix`, rollback, or atomicity promise. A partial new file may remain on
`E_OUTPUT_WRITE` and can never be reported as `saved`.

## D-006 — Library and type boundaries

Keep a thin binary over a library facade, immediate DTO-to-domain conversion,
strong invariant-carrying types, layer-owned `thiserror` enums, and structured
tracing. Remove generic repository/publication abstractions that no longer have
an approved consumer.

## D-007 — Review sequencing

Pre-implementation verification and security review precede code changes.
Post-implementation convergence/security review precede release assessment.
User-facing docs are refreshed by the implementation/documentation owner after
behavior converges.
