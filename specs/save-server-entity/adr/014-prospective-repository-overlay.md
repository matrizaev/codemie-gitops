# ADR-014: Validate save output through an in-memory repository overlay

## Status

Superseded by ADR-018 and specification v3.

## Context

This ADR previously selected a disk-plus-generated repository overlay to prove
whole-repository closure before publication. Product specification v3 and
parent v33 remove repository inspection, local reference existence, and Skill
sidecars from the approved behavior.

## Superseding decision

Save validates only its one generated inline declaration through
`contracts/single-declaration-validation-v1.md`. No `RepositoryView`, disk
enumeration, overlay, sidecar resolver, or closure validator is part of the
target architecture.

## Consequences

- Existing overlay implementation is migration debt, not preserved behavior.
- Immediate lint validity means validity as the sole `--file` input.
- ADR-018 and the single-declaration contract are normative.
