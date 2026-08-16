# Architecture

`codemie-gitops` is a stateless CLI tool for validating, saving, and applying CodeMie declarations. It acts as a client to the CodeMie server API without maintaining local state, plans, or batch operations.

## Design Philosophy

- **Stateless**: No local database or state files; each invocation is independent.
- **Single-entity**: Process one declaration per command invocation.
- **Specification-driven**: Validate against JSON Schema before semantic operations.
- **Type-safe**: Use Rust's type system to enforce domain invariants.
- **Observable**: Structured logging with `tracing`. Set `RUST_LOG=debug` to emit the full internal error chain to stderr alongside the closed diagnostic. This does not weaken the output contract (SEC-005): tracing output is opt-in, goes only to stderr, and is never part of the machine-readable stdout contract.

## Core Commands

| Command | Purpose |
|---------|---------|
| `lint` | Validate declaration against schema; produces schema-validation errors |
| `apply` | POST (create) or PUT (update) valid declaration to CodeMie; produces server response errors |
| `save` | READ entity from CodeMie; write canonical declaration in `codemie.epam.com/v1alpha1` format |
| `login` | Exchange OAuth or local credentials for access token |

## Entity Types

The tool handles four CodeMie entity kinds:

| Kind | Identity | Selector |
|------|----------|----------|
| `Assistant` | `project + slug` | `--slug` |
| `Workflow` | `project + slug` (marked) or `project + id` (unmarked adoption) | `--slug [--id]` |
| `Skill` | `project + name` | `--name` |
| `Datasource` | `project + repo_name` | `--repo-name` |

## Module Organization

```
src/
├── adapters/                 # Entity adapters (Assistant, Workflow, Skill, Datasource)
│   ├── assistant.rs
│   ├── datasource.rs
│   ├── mod.rs
│   ├── skill.rs
│   └── workflow.rs
├── save/                     # Read server entities → normalized declaration format
│   ├── mod.rs
│   ├── publication.rs
│   ├── reverse.rs
│   └── snapshot.rs
├── auth.rs                   # Keycloak OAuth (client_credentials, ROPC) and local-auth login
├── cli.rs                    # Command-line parsing, DTOs, and command dispatch
├── config.rs                 # URL/auth-URL configuration resolution
├── coordinator.rs            # Orchestration of apply, identity checks, and mutations
├── declaration_schema.rs     # Generated type definitions for declarations
├── domain.rs                 # Validated domain types (ProjectName, Slug, NaturalIdentity, etc.)
├── error.rs                  # Layer-owned error types and exit code mapping
├── http.rs                   # CodeMie API client, TLS transport, and strict DTOs
├── input.rs                  # Bounded single-file loader and auxiliary input reader
├── lib.rs                    # Library facade and entry point
├── lint.rs                   # Offline lint command execution
├── main.rs                   # Minimal binary CLI driver
├── output.rs                 # Result formatting and CLI stdout/stderr writing
├── pagination.rs             # Paginated list retrieval helpers
├── parse.rs                  # YAML parsing and JSON Schema validation
├── projection.rs             # Convert domain types → server API requests
├── render.rs                 # Diagnostic and outcome rendering
├── schema.rs                 # Embedded declaration schema and validation helpers
├── strict_json.rs            # Strict JSON decoding with duplicate key rejection
└── validate.rs               # Semantic validation and reference shape checks
```

## Data Flow

### Lint

```
YAML File
    ↓
[input] → Bounded single-file read (plus explicit auxiliary sidecar if Skill contentFrom)
    ↓
[parse] → Validate against JSON Schema
    ↓
[validate] → Semantic validation & reference-shape checks
    ↓
Text/JSON Error Report (stderr) or Outcome (stdout)
```

### Apply

```
YAML File
    ↓
[input] → Bounded single-file read (plus explicit auxiliary inputs)
    ↓
[parse] → Validate against JSON Schema
    ↓
[validate] → Semantic validation
    ↓
[coordinator] → Identity resolution & online reference lookup via adapters
    ↓
[projection] → Convert domain types to server API payload
    ↓
[http] → POST (create) or PUT (update) to CodeMie
    ↓
Text/JSON Outcome (stdout)
```

### Save

```
CLI Selectors (--kind, --project, --file, etc.)
    ↓
[http] → GET from CodeMie by identity
    ↓
[save] → Reverse-project & normalize server entity → Declaration snapshot
    ↓
[validate] → In-memory validation of generated declaration
    ↓
[save::publication] → File Datasource placeholders, then direct YAML write
    ↓
Text/JSON Outcome (stdout)
```

## Key Design Decisions

Deeper rationale for each cluster below lives in [docs/adr](docs/adr/README.md).

### 1. **Stateless Operations**
Each invocation is independent. Authentication via `--url` and bearer token in `Authorization` header. No session state or local cache.

### 2. **Snapshot-Based Save**
The `save` command reads a server entity and produces a snapshot containing:
- Reverse-projected and normalized fields according to OpenAPI contract (stripping internal metadata, credentials, and reconciling API aliases)
- Inline Skill content (`spec.content`) with companion files
- File Datasource `uploaded_files` plus adjacent zero-byte explicit inputs when
  server source bytes are unavailable
- Direct write with create-new semantics (refuses to overwrite existing files, no temporary/staging rename)
- Rendered canonical YAML output validated in-memory before writing

### 3. **Schema Validation First**
`parse/mod.rs` validates incoming declarations against JSON Schema before any domain logic runs. This separates transport-level validation from semantic validation.

### 4. **Entity-Specific Adapters**
Each entity kind has an adapter module:
- `adapters/assistant.rs`
- `adapters/workflow.rs`
- `adapters/skill.rs`
- `adapters/datasource.rs`

Each adapter encapsulates:
- Server API routes
- Request/response DTO mapping
- Online identity resolution
- Natural update route detection and write planning (POST for create, PUT for update)

### 5. **Type-Driven Identity**
Identity is modeled with strong types:
- `ProjectName` — validated project identifier
- `Slug` — URL-safe slug
- `SkillName` — skill name identifier
- `RepositoryName` — datasource repository identifier (`repo_name`)
- `WorkflowId` — canonical UUID
- `ServerId` — server-assigned entity ID

### 6. **Error Handling**
Errors are structured using `thiserror`:
- Parse errors → schema violation details
- Validation errors → semantic invariant violation
- HTTP errors → server response with typed classification
- Auth errors → credential/token issues

The closed diagnostic written to stderr contains only the `errorCode`, `category`, and `exitCode` from a fixed enum — no raw server text, credentials, or user input (SEC-005). The full internal error chain is additionally emitted at `DEBUG` level via `tracing` before the closed diagnostic line. Enable it with `RUST_LOG=debug`; it goes only to stderr and is never part of the machine-readable stdout contract, so it does not weaken SEC-005.

## Boundary Types vs. Domain Types

External data (HTTP responses, CLI arguments, YAML files) are treated as untrusted:

```
Untrusted Boundary Type (serde DTO)
    ↓ TryFrom / validation
Trusted Domain Type (AssistantDeclaration, etc.)
    ↓ projection / business logic
Trusted Domain Type (ApplyRequest, etc.)
    ↓ serialization
HTTP Request Body
```

This ensures invariants are enforced at boundaries and domain logic operates only on validated types.

## Testing

- **Unit tests**: Domain logic, conversions, error handling
- **Integration tests**: CLI parsing, schema validation, end-to-end scenarios
- **Contract tests**: OpenAPI contract validation against pinned backend schemas
- **Live tests** (manual): Against local dev server instances

Run tests with:
```sh
make test
```

## Dependencies

- **Tokio**: Async runtime
- **Reqwest + Rustls**: HTTP client with TLS
- **Serde/serde_json/serde_yaml**: Serialization
- **Clap**: CLI parsing
- **Tracing**: Structured logging
- **Thiserror**: Error types
- **Typify + Schemars**: JSON Schema generation and type generation

## Explicit Scope Non-Goals

Per the product specification, the following are deliberate non-goals:
- Batch operations (multiple entities per invocation)
- Plan/preview mode (declarative always-write model)
- Local state tracking, cache database, or automatic rollback
- Repository discovery, walking, or multi-file declaration graphs

---

For exact implementation algorithms, resource limits, reconstruction order,
and test ownership, see [the current implementation reference](docs/implementation-reference.md).
See [the documentation map](docs/README.md) for the authority order (executable
contracts > code/tests > implementation reference > architecture > user docs >
ADRs), [docs/adr](docs/adr/README.md) for the rationale behind key decisions,
[README.md](README.md) for commands, and [CONTRIBUTING.md](CONTRIBUTING.md) for
development guidelines.
