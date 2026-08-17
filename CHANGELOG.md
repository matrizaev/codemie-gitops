# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.11] - 2026-08-16

### Added
- File Datasource save now emits canonical YAML with editable zero-byte local
  placeholders when CodeMie exposes filenames but not source bytes.

### Changed
- Workflow save reverses managed Assistant, Skill, and Datasource IDs into
  portable natural references while preserving graph-local state IDs.
- Save documentation and architecture now define File Datasource YAML-last
  multi-artifact publication and orphan-placeholder recovery.
- Documentation restructure: removed historical SDD process artifacts (feature
  specs, plans, task lists, verification/security-review reports, and the
  completed Rust architecture remediation) from the active tree; consolidated
  executable contracts under `contracts/`; centralized durable architecture
  rationale in `docs/adr/`; and updated `AGENTS.md` to a temporary-artifact
  lifecycle with a Git-ignored `.work/` workspace.
- Shared exhaustive pagination scanner (`scan_pages`) now enforces the
  traversal invariants once for every kind adapter; the Workflow adapter was
  split into `workflow/mod.rs` (apply/adoption/verification/snapshots) and
  `workflow/enumeration.rs` (page scan and marker codec).
- Update resolution targets now carry the write-ability proof in the type, so
  an update without proven `write` evidence is unrepresentable.

### Fixed
- Response bodies are now bounded during streaming reads (8 MiB cap enforced
  per chunk) on both the API and authentication clients; the auth token
  response is strictly decoded (duplicate JSON keys rejected, depth bounded).
- Datasource identity matching and post-write verification now include
  `index_type` (kind is part of identity), and Datasource JSON-kind optional
  fields materialize omission/null as explicit JSON null per the manifest.
- Path segments are percent-encoded with `%20` (not `+`) for slugs, projects,
  and server ids; the workflow `meta_config` merge decodes strictly.
- Workflow offline validation now rejects duplicate state ids and transition
  targets (`next.state_id`/`state_ids`, condition, switch) that name unknown
  states.
- `save` no longer rejects valid Skills whose MCP server sets a numeric token
  size limit; the Skill stability fingerprint ignores audit/counter fields;
  File Datasource save preserves `guardrail_assignments`; placeholder names
  use sequential `replace-content-N.txt` numbering.
- CLI parse failures emit the closed usage diagnostic instead of clap's raw
  text; `login` no longer panics on a closed stdout; output-write failures
  emit a closed diagnostic.
- Reconciliation errors now map to their specific closed codes
  (`E_ADOPTION_REQUIRED`, `E_IDENTITY_MARKER_INVALID`, `E_RESOLUTION_UNSTABLE`,
  `E_MISSING_REFERENCE`), timeouts map to `E_TIMEOUT`, and post-write
  verification failures are always framed as write-uncertainty.
- Domain identity newtypes enforce the schema's length/character constraints;
  `--id`/`--adopt-workflow-id` require a canonical hyphenated UUID; secrets
  that are not valid UTF-8 are explicit configuration errors; `localhost`
  HTTP is permitted only after runtime loopback resolution confirms it.

---

## [0.1.3] - 2026-08-14

### Added
- **Reverse-projection normalization**: Implemented specification v3.4 normalization for `save` in `src/save/reverse.rs`, converting server API responses into canonical `codemie.epam.com/v1alpha1` declarations:
  - Context references normalized to same-project Datasource references.
  - Enriched category objects normalized to string names.
  - MCP servers, toolkits, tools, and integration settings stripped of internal/credential metadata.
  - Materialization of OpenAPI-declared defaults for schema-required fields.
- **Release workflow enhancements**: Updated GitHub Actions release workflow for musl Linux targets and `musl-tools`.

### Changed
- **Architecture alignment**: Aligned internal module structure and removed obsolete repository-discovery and multi-file walking code in favor of single-file boundaries.

---

## [0.1.2] - 2026-08-13

### Added
- **Open source documentation**: Added comprehensive documentation suite including `ARCHITECTURE.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `AGENTS.md`, and `examples/README.md`.

### Changed
- **Version bump**: Bumped version to 0.1.2 for OSS documentation release.

### Fixed
- **Release workflow**: Fixed GitHub Actions `publish` job by adding checkout action to provide repository context (`.git` directory) for `gh release` commands.
- **Build consistency**: Verified `Cargo.lock` integrity across all platforms.

### Changed
- **Documentation**: Added comprehensive OSS documentation (Architecture, Contributing, Security Policy, Code of Conduct, Changelog).
- **Internal refactoring**: Maintained backward-compatible APIs and data structures.

### Verified
- All four entity types (Assistant, Workflow, Skill, Datasource) verified with full round-trip testing: `apply → save → lint → re-apply`.
- Cross-platform release binaries confirmed for Windows x86_64, macOS aarch64, Linux x86_64, and Linux aarch64.

---

## [0.1.0] - 2026-08-11

### Added
- **Core CLI**: `lint`, `apply`, `save`, and `login` commands.
- **Schema validation**: Full JSON Schema validation of declarations before processing.
- **Entity support**: Complete implementation for four CodeMie entity types:
  - **Assistant**: Create/update with slug-based identity resolution and sub-assistant/skill nesting.
  - **Workflow**: Create/update with marker-based adoption support (v2 workflow marker).
  - **Skill**: Create/update with name-based identity and creator scoping.
  - **Datasource**: Create/update for Git, Azure, Confluence, Jira, and other backend types with natural identity matching (project + repo_name).
  
- **Save operation**: Read entities from CodeMie and produce apply-able declarations in `codemie.epam.com/v1alpha1` format.
  - Managed-ID resolution: Convert server IDs to slug-based declaration references.
  - Snapshot serialization: YAML/JSON output with structured declaration format.
  
- **HTTP client**: Stateless API communication via bearer token authentication (OAuth).
  - TLS certificate validation with `rustls`.
  - Structured error handling with typed server responses.
  
- **Configuration**: Support for configuration via environment variables and `.env` files.
  - `CODEMIE_URL`: CodeMie server base URL
  - `CODEMIE_TOKEN`: Bearer token for authentication
  
- **Authentication**: `login` command for OAuth token exchange.
  
- **Error handling**: Comprehensive error types with structured diagnostics.
  - Schema validation errors with field paths.
  - Semantic validation errors with context.
  - HTTP errors with server response details.
  
- **Output formats**: Text, JSON, and YAML output rendering.
  
- **Integration tests**: 
  - 6 CLI integration tests validating command-line parsing and end-to-end behavior.
  - Full round-trip validation for each entity type against live CodeMie server.
  
- **CI/CD Pipeline**:
  - GitHub Actions with 4 required checks (quality, test, audit, version bump).
  - Automated release workflow building binaries for 4 platforms.
  - Protected main branch with review and CI requirements.
  
- **Development tooling**:
  - Makefile with `format`, `lint`, `test`, and `build-release` targets.
  - `rust-toolchain.toml` pinned to Rust 1.95, Edition 2024.
  - `pytest.ini` and Python test scripts for CLI validation.

### Technical Details
- **Language**: Rust 1.95, Edition 2024
- **Runtime**: Tokio async runtime
- **HTTP**: Reqwest with Rustls for TLS
- **Serialization**: Serde with JSON/YAML support
- **Schema**: JSON Schema validation with typify code generation
- **Error types**: Structured errors with `thiserror`
- **Logging**: Structured tracing with configurable levels

### Known Limitations
- **Stateless**: No local state or batch operations; one entity per command invocation.
- **Single-file**: Declarations are single-entity files; multi-entity declarations not supported.
- **No caching**: Each operation re-validates and re-queries the server.

### Verified With
- CodeMie Server v2.42.0
- Rust 1.95 (Edition 2024)
- Ubuntu 24.04 LTS, macOS Sonoma, Windows Server 2022

---

## Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes to CLI interface or declaration format
- **MINOR**: Backward-compatible feature additions
- **PATCH**: Bug fixes and maintenance updates

---

## Release Process

Releases are automated via GitHub Actions:

1. Version bump in `Cargo.toml` merged to main
2. Create annotated git tag: `git tag v0.1.1`
3. Push tag: `git push origin v0.1.1`
4. GitHub Actions automatically:
   - Builds binaries for 4 platforms
   - Generates SHA-256 checksums
   - Creates GitHub Release with artifacts

---

## Contributors

Thanks to all contributors who have helped improve this project!

---

**Latest Release**: [v0.1.3](https://github.com/matrizaev/codemie-gitops/releases/tag/v0.1.3)

For detailed commit history, see [Git Commits](https://github.com/matrizaev/codemie-gitops/commits/main).
