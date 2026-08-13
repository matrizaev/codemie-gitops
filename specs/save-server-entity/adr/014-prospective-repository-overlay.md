# ADR-014: Validate save output through an in-memory repository overlay

## Status

Accepted

## Context

The implemented repository loader discovers and opens declarations from disk,
then applies parsing, schema, semantic, duplicate-identity, sidecar, and graph
validation. `save` must run that same closure before any final artifact exists.
Writing a temporary declaration into the repository would expose invalid or
sensitive intermediate state and make lint/save behavior diverge.

## Decision drivers

- FR-SAVE-018/026 and VR-SAVE-005/011
- Zero final writes before validation
- No change to existing lint behavior
- One implementation of repository invariants
- Deterministic and failure-injectable tests

## Options considered

### A. Write hidden files and run the current disk loader

Rejected. Discovery and sidecar behavior could observe intermediate files, and
temporary content would be retained on crash.

### B. Implement a save-only validator

Rejected. Two semantic and graph engines would drift.

### C. Extract a repository-view input and overlay generated artifacts

Selected.

## Decision

The repository validation engine accepts a narrow `RepositoryView` abstraction
that can enumerate YAML entries and open bounded bytes for YAML or sidecars.
Two implementations exist:

- `DiskRepositoryView` preserves the current discovery, ordering, safe-open,
  symlink, containment, and size behavior used by `lint` and `apply`.
- `OverlayRepositoryView` wraps the disk view and adds exactly the proposed
  YAML plus the Skill main-content sidecar, keyed by validated
  repository-relative paths.

Overlay entries shadow nothing. Construction fails if either final path exists
or aliases an existing entry. Enumeration merges disk and generated YAML paths
in the same bytewise ordering used today. A `contentFrom` open resolves the
generated sidecar from immutable in-memory bytes; all other reads delegate to
the disk view.

One pure validation pipeline performs parsing, closed-schema validation,
effective-project materialization, natural validation, duplicate detection,
sidecar expansion, and graph closure. It returns the parsed target or a typed
failure. `lint` continues to compute target-only warnings after this pipeline;
`save` does not emit lint warnings and separately enforces known secret/mask
classes before the overlay is built.

Regression tests must prove that `DiskRepositoryView` produces the same
success/failure and ordering behavior as the pre-refactor loader.

## Consequences

### Positive

- Successful save implies immediate offline lint validity.
- No generated declaration or content is written before validation.
- Validation rules remain single-sourced.

### Negative

- Discovery and sidecar loading require an interface extraction.
- Diagnostics need synthetic-but-contained paths for overlay entries.

### Risks

- An overlay could accidentally allow path behavior the disk view rejects.
- Warning behavior could change during refactoring.

## Follow-up actions

- Add disk-versus-overlay equivalence tests.
- Add a guard that overlay entries cannot replace disk entries.
- Independently verify warning scope and ordering remain unchanged for lint.

## References

- Feature specification v2: FR-SAVE-018/026, DR-SAVE-009,
  QR-SAVE-002, VR-SAVE-004/005/011, AC-SAVE-011/012
- Parent ADR-001 and F-003/F-005 architecture
- `../contracts/prospective-validation-v1.md`
