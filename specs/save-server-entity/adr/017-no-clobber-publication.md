# ADR-017: Publish complete artifacts with native no-replace operations

## Status

Superseded by ADR-018 and specification v3.

## Context

This ADR previously required staging, native no-replace rename, and Skill
sidecar-first/YAML-last publication. Specification v3 explicitly removes
staging, temporary files, rename publication, multi-file output, and atomic or
complete-visibility guarantees.

## Superseding decision

Save writes one canonical YAML directly to a create-new final path. Existing
targets remain protected, but a failed direct write may leave an incomplete new
file and must be reported as `E_OUTPUT_WRITE` without `saved` or cleanup.
ADR-018 and `contracts/publication-v1.md` are normative.
