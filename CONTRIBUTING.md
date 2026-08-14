# Contributing to CodeMie GitOps

Thank you for your interest in contributing to `codemie-gitops`! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project adheres to the Contributor Covenant Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the maintainers.

## Getting Started

### Prerequisites

- **Rust 1.95+** (see `rust-toolchain.toml`)
- **Podman** or **Docker** (for local dev server)
- **Make** (for convenient commands)
- **Python 3.9+** (for test scripts)

### Local Development Setup

1. **Clone and enter the workspace:**
   ```sh
   git clone https://github.com/matrizaev/codemie-gitops.git
   cd codemie-gitops
   ```

2. **Install Rust and required components:**
   ```sh
   rustup install 1.95
   rustup target add x86_64-unknown-linux-musl  # for cross-platform builds
   rustup target add aarch64-apple-darwin       # for macOS
   ```

3. **Verify the toolchain:**
   ```sh
   rustc --version
   cargo --version
   ```

4. **(Optional) Start a local CodeMie dev server:**
   ```sh
   make dev-server
   ```
   This starts the local PostgreSQL, Elasticsearch, and CodeMie backend services using Podman Compose and `ops/dev/podman-compose.yml`.

### Running Tests Locally

```sh
# Format check
make format

# Linting (Clippy)
make lint

# Run all tests
make test

# Build release
cargo build --locked --release
```

## Development Workflow

### 1. Create a Feature Branch

```sh
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

Use descriptive branch names: `feature/save-assistant`, `fix/datasource-identity`, etc.

### 2. Make Changes

Follow these principles:

- **Write code first, tests second**: For meaningful behavior changes, add or update tests.
- **Keep commits focused**: One logical change per commit.
- **Use clear commit messages**: Start with a verb (Add, Fix, Refactor, Docs).
  
  Examples:
  ```
  Add support for Datasource save operation
  Fix Assistant identity resolution in projection
  Refactor error handling in HTTP adapter
  Docs: update README with datasource selector examples
  ```

- **Format as you go:**
  ```sh
  cargo fmt --all
  ```

- **Lint and check regularly:**
  ```sh
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

### 3. Test Your Changes

```sh
# Run unit and integration tests
cargo test --workspace --all-features

# Test specific module
cargo test save::

# Test with logging
RUST_LOG=debug cargo test --workspace -- --nocapture
```

For live testing against a local server:
```bash
./scripts/wait-for-dev-dependencies.sh
cargo run -- lint --file examples/repository/assistants/example-assistant.yaml
cargo run -- apply --file examples/repository/assistants/example-assistant.yaml
cargo run -- save --kind Assistant --project demo --slug example-assistant --file saved-assistant.yaml
```

### 4. Run the Full CI Suite Before Pushing

```sh
make format   # Auto-format code
make lint     # Clippy checks
make test     # All tests
cargo check   # Type check
```

All checks must pass before opening a pull request.

### 5. Push and Open a Pull Request

```sh
git push origin feature/your-feature-name
```

Then open a pull request on GitHub. The PR will automatically:
- Run the CI workflow (format, lint, test, audit)
- Require a version bump in `Cargo.toml` (managed by release gate)
- Require 1 approving review
- Require all status checks to pass

## Code Style & Conventions

### Rust Idioms

- **Ownership**: Use `&str` over `&String`, `&[T]` over `&Vec<T>`.
- **Errors**: Use `thiserror` for error enums; propagate with `?`.
- **Types**: Prefer strong domain types over primitives.
  ```rust
  struct ProjectName(String);
  struct Slug(String);
  ```
- **Pattern matching**: Prefer exhaustive matching with `match`.
- **Documentation**: Public APIs should have doc comments.

### Naming

- Modules: lowercase, underscore-separated (`src/adapters/datasource.rs`)
- Types: PascalCase (`AssistantDeclaration`)
- Functions/methods: snake_case (`resolve_assistant_ids`)
- Constants: SCREAMING_SNAKE_CASE (`DEFAULT_TIMEOUT`)

### Imports

Group imports in this order:
```rust
// Standard library
use std::path::Path;

// External crates
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Internal modules
use crate::http::Client;
use crate::error::ParseError;
```

## Testing Guidelines

- **Unit tests**: Test in-module functions with `#[cfg(test)]` modules.
- **Integration tests**: Place in `tests/` directory for end-to-end scenarios.
- **Coverage**: Aim for meaningful behavior coverage, not line-count targets.
- **Edge cases**: Test error paths, boundary conditions, and invalid inputs.

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_validation_rejects_invalid_characters() {
        assert!(Slug::try_from("invalid slug!").is_err());
    }

    #[test]
    fn test_project_identity_combines_project_and_slug() {
        let identity = ProjectIdentity::new(
            ProjectName("prod".into()),
            Slug("my-assistant".into()),
        );
        assert_eq!(identity.to_string(), "prod/my-assistant");
    }
}
```

## Documentation

- **README.md**: User-facing command and configuration docs.
- **docs/README.md**: Authority map and historical-document rules.
- **docs/yaml-reference.md**: Complete declaration authoring language. Update
  it whenever the declaration schema changes.
- **docs/implementation-reference.md**: Current algorithms, limits, module
  ownership, and reconstruction guidance. Update it with behavioral or
  architectural changes.
- **ARCHITECTURE.md**: Short design, module organization, and data flow.
- **Closed contracts**: Update schemas/manifests and their tests with any
  accepted wire-format change. Prose cannot widen a closed contract.
- **Code comments**: Explain *why*, not *what*. Let the code speak for itself.
- **Doc comments on public APIs**: Include examples for complex behavior.

```rust
/// Resolve Assistant IDs to slug-based references in the declaration.
///
/// # Example
///
/// ```
/// let snapshot = resolve_assistant_ids(&assistant)?;
/// assert_eq!(snapshot.sub_assistants, vec!["proj/foo", "proj/bar"]);
/// ```
pub fn resolve_assistant_ids(assistant: &Assistant) -> Result<AssistantSnapshot> {
    // implementation
}
```

## Commit Message Format

Use clear, descriptive commit messages:

```
<type>: <description>

<optional body>

Fixes #<issue-number> (if applicable)
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code restructuring without behavior change
- `docs`: Documentation updates
- `test`: Test additions/updates
- `chore`: Dependency updates, build config, etc.
- `perf`: Performance improvements

Example:
```
feat: add Datasource save operation

Implement read and save for Datasource entities. Resolves identity from
project + repo_name. Handles Git/Azure/Confluence/Jira branch logic.

Fixes #42
```

## Opening a Pull Request

1. **Describe the change**: Explain what and why, not just what the code does.
2. **Link issues**: Reference any related issues (e.g., "Fixes #123").
3. **Include screenshots/examples** if the change affects user-facing behavior.
4. **Ensure CI passes**: All checks (format, lint, test, audit) must be green.
5. **Be patient**: Code reviews take time. Respond to feedback constructively.

## Versioning

This project follows [Semantic Versioning](https://semver.org/):

- `MAJOR.MINOR.PATCH` (e.g., `0.1.3`)
- `MAJOR` for breaking changes
- `MINOR` for backward-compatible features
- `PATCH` for bug fixes

Version is managed in `Cargo.toml` and enforced by the CI version-bump gate.

## Release Process

Releases are automated:

1. Merge a PR with a version bump
2. Create and push a tag (`git tag v0.1.3 && git push origin v0.1.3`)
3. GitHub Actions automatically:
   - Builds binaries for four platforms (Windows x86_64, macOS aarch64, Linux x86_64, Linux aarch64)
   - Generates checksums
   - Creates a GitHub Release with assets

## Reporting Issues

When reporting a bug:

1. **Provide a minimal reproduction** (code snippet or command)
2. **Include your environment**: Rust version, OS, CodeMie server version
3. **Describe the expected vs. actual behavior**
4. **Attach logs** (enable `RUST_LOG=debug` for more detail)

Example:
```
**Title**: `save` command fails with identity mismatch

**Environment**: Rust 1.95, Ubuntu 24.04, CodeMie server v2.42.0

**Steps to Reproduce**:
```bash
codemie-gitops save --kind Datasource --project prod \
  --repo-name my-repo --file output.yaml
```

**Error**:
```
Error: identity mismatch: expected "prod/my-repo", got "prod/different-repo"
```

**Expected**: Should read and save the Datasource with repo_name="my-repo"

**Actual**: Returns identity error instead
```

## Questions?

- Check existing issues on [GitHub](https://github.com/matrizaev/codemie-gitops/issues)
- Start with the [documentation map](docs/README.md)
- Read the [Architecture documentation](ARCHITECTURE.md)
- Review examples in [examples/](examples/)

Thank you for contributing!
