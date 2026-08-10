# ADR-005: Modular monolith distributed as one Rust binary

## Status

Proposed

## Context

The product is a three-command CLI that processes one entity per invocation.
It must be self-contained on Linux x86_64 and has no requirement for a daemon,
database, plugin host, or shared network service.

## Decision drivers

- Self-contained CI distribution
- Low operational complexity
- Clear kind adapter boundaries
- Testability
- No speculative infrastructure

## Options considered

### A. One Rust workspace/binary with internal modules and adapter traits

Compile all commands, schemas, and kind adapters into one release artifact.

### B. Separate per-kind executables

Build assistant/workflow/datasource/skill tools independently.

### C. Client plus a reconciliation service

Run a persistent controller/daemon and make the CLI submit jobs.

## Decision

Choose option A. Use one binary crate initially, with modules for CLI/config,
source parsing, schemas, repository index, reference resolution, HTTP,
compatibility, kind adapters, request projection, success output, and the
closed safe-diagnostic builder.
Module boundaries are internal; no dynamic plugin ABI is introduced.

Build the release for `x86_64-unknown-linux-musl` with rustls so the artifact has
no Python/OpenSSL runtime dependency. Release verification must prove the
binary runs on the supported runner baseline.

## Consequences

### Positive

- One artifact to version, scan, promote, and invoke.
- Shared validation/output/security behavior cannot drift between kinds.
- Internal adapter boundaries retain evolvability without service overhead.
- Local unit and contract tests can exercise all behavior in process.

### Negative

- Any kind update releases the whole binary.
- Embedded schemas/OpenAPI fixture increase artifact size.
- A musl target can expose compatibility issues in native dependencies; avoid
  such dependencies and verify the build.

### Risks

- An over-general adapter framework could become speculative. Keep the trait
  narrow and allow kind-specific implementations.
- Static linking claims can be invalidated by future native dependencies;
  release checks must inspect dynamic dependencies.

## Follow-up actions

- Establish the Rust workspace only after pre-implementation verification.
- Add artifact inspection, SBOM, checksum, signature, and runner smoke tests.
- Revisit crate splitting only when compile time or ownership evidence demands
  it.

## References

- Product specification v24: QR-002/006/008 and section 24 packaging constraint
- FR-005 one-entity boundary
- `plan.md`, target architecture

## Amendment: Supply chain and CI controls (SEC-006, v25)

The following requirements are normative for the dependency, CI, and release
lifecycle of the `codemie-gitops` binary. They supplement the release controls
already planned in this ADR (artifact inspection, SBOM, checksum, signature,
locked build, same-artifact promotion).

### Dependency integrity

- `Cargo.lock` must be committed to the repository and checked in at every
  dependency change
- All CI and release builds use `cargo build --locked` or `--frozen`; unlocked
  builds are not permitted in release pipelines
- Default Cargo registry checksum verification must not be disabled
- Any git or path dependency requires explicit reviewer approval and a
  documented justification; git dependencies must be pinned to a specific
  commit SHA; path dependencies are allowed only for workspace-internal crates
- RustSec-compatible vulnerability scanning (`cargo audit` or equivalent) must
  run in CI on every build; all advisories require documented triage within the
  sprint; waivers must name the advisory ID, assessed risk, and an expiry date
- Dependency features that grant access to HTTP, filesystem, serialization, or
  secret-handling must be explicitly reviewed and minimized at initial adoption
  and on each change

### CI trust controls

- Third-party CI actions and includes (e.g., GitHub Actions) must be pinned
  to immutable commit SHAs, not mutable tag or branch references
  (e.g., `actions/checkout@<full-sha>` not `actions/checkout@v4`)
- Every workflow job must carry an explicit `permissions:` block granting only
  the minimum scopes required for that job; no job defaults to unscoped access
- Secrets (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, `CODEMIE_PASSWORD`, and
  release signing credentials) must not be available to fork-triggered jobs or
  untrusted pull-request workflows
- Apply and release jobs must execute in protected deployment environments
  requiring human approval or named-reviewer sign-off
- Build and test runners must be isolated from production environments, secret
  stores, and deployment credentials
- Branch protection must be active on the default and all release branches;
  force-pushes and unsigned commits must be blocked

### Release and artifact integrity

- Release artifacts are promoted from the exact same binary that passed all CI
  gates; no re-build from source at release time
- SHA-256 checksum and signature verification must be performed and logged
  before executing or distributing the release artifact
- SBOM and provenance metadata must be retained per the adopting organization's
  release policy; minimum required artifact attributes: tool name, version,
  Git commit SHA, and build timestamp
- Repository secret scanning must be active on the repository; a confirmed
  secret exposure triggers immediate rotation and containment per the adopting
  organization's incident response runbook

### References for this amendment

- SEC-006 remediation, v25
- Product specification v25: QR-006/007/008
- Tasks: F-001, O-002, V-002, L-001
