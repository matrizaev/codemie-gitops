# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Future enhancements to be documented

### Changed
- Future changes to be documented

### Fixed
- Future fixes to be documented

### Deprecated
- Future deprecations to be documented

### Removed
- Future removals to be documented

### Security
- Future security improvements to be documented

## [0.1.1] - 2026-08-13

### Fixed
- Fixed release workflow by adding repository checkout to publish job
- Ensured `gh release` commands have proper Git context for creating GitHub Releases

## [0.1.0] - 2026-08-13

### Added
- Initial release of `codemie-gitops` CLI tool
- `lint` command: Validate declarations against JSON Schema
- `apply` command: Apply declarations to CodeMie server
- `save` command: Read entities from CodeMie and produce declarations
- `login` command: OAuth credential exchange
- Support for four entity types: Assistant, Workflow, Skill, Datasource
- Full round-trip testing: create/apply → save → lint → re-apply
- Cross-platform binary releases (Windows x86_64, macOS aarch64, Linux x86_64, Linux aarch64)
- Protected main branch with required status checks
- Automated CI/CD pipeline with format, lint, test, and audit gates
- Security-focused design: stateless, schema-validated, type-safe
- Comprehensive documentation: README, ARCHITECTURE, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT

### Documentation
- README with command reference and local development guide
- ARCHITECTURE describing design principles and module organization
- CONTRIBUTING with development workflow and testing guidelines
- SECURITY with vulnerability reporting and best practices
- CODE_OF_CONDUCT for community standards
- Examples directory with sample declarations

---

[Unreleased]: https://github.com/matrizaev/codemie-gitops/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/matrizaev/codemie-gitops/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/matrizaev/codemie-gitops/releases/tag/v0.1.0
