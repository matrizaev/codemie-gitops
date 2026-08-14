# Architecture research: v33 single-file revision

## Status

Historical migration evidence captured on 2026-08-13. This document records
the pre-v33 implementation inspected during planning, not current source.

## Approved evidence

- `../codemie-cicd-tool.md` v33 requires exactly one `--file` for lint/apply,
  offline reference-shape validation, and online reference existence during
  apply. It removes repository walking/root/closure/order and repository config,
  while v33.3 retains only explicit Skill/File auxiliary paths.
- `../save-server-entity/spec.md` v3 requires one inline-content YAML output,
  validation of only that declaration, and a direct no-overwrite final-path
  write whose failure may leave a partial file.

## Pre-migration implementation evidence

- `src/cli/mod.rs` still exposes repository and symlink flags and resolves a
  repository root/config before lint/apply/save.
- `src/repository.rs` defines disk/overlay repository views, enumerates YAML,
  opens sidecars, and validates a graph closure.
- `src/discovery/mod.rs` owns walking, containment, and sidecar budgets.
- `src/parse/mod.rs` expands Skill `contentFrom`.
- `src/cancellation.rs` and call sites make a cancellation token part of local
  parsing/loading APIs.
- `src/save/publication.rs` stages through `tempfile` and publishes with
  `rustix` no-replace rename; `src/save/mod.rs` creates a Skill sidecar and
  validates through the overlay.

These were implementation facts that conflicted with the newer approved specs.
The migration is complete; use `../../docs/implementation-reference.md` for
current source structure and behavior.

## Retained server-contract evidence

The pinned reference-only CodeMie 2.42.0 source, adapter manifest, OpenAPI
snapshot, and ADRs 007/008/014–018 continue to support kind-specific target and
reference resolution, authorization, compatibility, write projection, and race
handling. No server API change is required by the local-boundary revision.

## Conclusions

1. A single declaration loader can replace repository discovery for both lint
   and apply; bounded reads plus the invocation timeout meet v33.
2. Offline graph existence checks must be deleted, not emulated. Existing safe
   read primitives should be narrowed to selected-declaration-relative explicit
   paths without root discovery or enumeration.
3. Natural-reference types remain useful because apply resolves them online.
4. Repository config is not a valid fallback. URL/auth URL/project sources must
   be explicit per the revised CLI contracts.
5. `tempfile`, `rustix`, discovery code, repository overlays, and cooperative
   cancellation may become unused; dependency removal is an implementation task
   after call-site migration, not an architectural prerequisite.
6. Reference-only `codemie/` and `codemie-ui/` remain unmodified and outside
   the product architecture.
