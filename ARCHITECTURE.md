# Architecture

`codemie-gitops` is a stateless CLI tool for validating, saving, and applying CodeMie declarations. It acts as a client to the CodeMie server API without maintaining local state, plans, or batch operations.

## Design Philosophy

- **Stateless**: No local database or state files; each invocation is independent.
- **Single-entity**: Process one declaration per command invocation.
- **Specification-driven**: Validate against JSON Schema before semantic operations.
- **Type-safe**: Use Rust's type system to enforce domain invariants.
- **Observable**: Structured logging with `tracing` for production debugging.

## Core Commands

| Command | Purpose |
|---------|---------|
| `lint` | Validate declaration against schema; produces schema-validation errors |
| `apply` | POST/PATCH valid declaration to CodeMie; produces server response errors |
| `save` | READ entity from CodeMie; produce declaration in `codemie.epam.com/v1alpha1` format |
| `login` | Exchange OAuth credentials for access token |

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
├── cli/              # Command-line parsing and invocation
├── config/           # Configuration loading and validation
├── parse/            # Schema validation and YAML/JSON parsing
├── http/             # CodeMie API client and request/response handling
├── adapters/         # Entity-specific API adapters (Assistant, Workflow, Skill, Datasource)
├── projection/       # Convert domain types → server API requests
├── save/             # Read server entities → declaration format
├── render/           # Output formatting (text, JSON, YAML)
├── validate/         # Post-parse semantic validation
├── output/           # Result serialization
└── error.rs          # Error types and diagnostics
```

## Data Flow

### Lint

```
YAML/JSON File
    ↓
[parse] → Validate against JSON Schema
    ↓
[validate] → Semantic validation
    ↓
Text/JSON Error Report or Success
```

### Apply

```
YAML/JSON File
    ↓
[parse] → Validate against JSON Schema
    ↓
[validate] → Semantic validation
    ↓
[projection] → Convert to server API request
    ↓
[http] → POST/PATCH to CodeMie
    ↓
Text/JSON Response
```

### Save

```
CLI Selectors (--kind, --project, --slug, etc.)
    ↓
[http] → GET from CodeMie by identity
    ↓
[save] → Convert server entity → Declaration snapshot
    ↓
[render] → Serialize to YAML/JSON
    ↓
Declaration File or stdout
```

## Key Design Decisions

### 1. **Stateless Operations**
Each invocation is independent. Authentication via `--url` and bearer token in `Authorization` header. No session state or local cache.

### 2. **Snapshot-Based Save**
The `save` command reads a server entity and produces a snapshot containing:
- Resolved references (nested Assistant/Workflow by ID → slug-based declaration references)
- Declared-form selectors (identity fields in `metadata`)
- Rendered content (YAML/JSON serialization)

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
- Identity resolution
- Natural update route detection (e.g., is this an existing datasource update or a create?)

### 5. **Type-Driven Identity**
Identity is modeled with strong types:
- `ProjectName(String)` — validated project identifier
- `Slug(String)` — URL-safe slug
- `ServerId(Uuid)` — server-assigned entity ID
- `RepoName(String)` — datasource repository identifier

### 6. **Error Handling**
Errors are structured using `thiserror`:
- Parse errors → schema violation details
- Validation errors → semantic invariant violation
- HTTP errors → server response with typed classification
- Auth errors → credential/token issues

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
- **Live tests** (manual): Against local dev server instances

Run tests with:
```sh
make test
```

## Dependencies

- **Tokio**: Async runtime
- **Reqwest + Rustls**: HTTP client with TLS
- **Serde/serde_json**: Serialization
- **Clap**: CLI parsing
- **Tracing**: Structured logging
- **Thiserror**: Error types
- **Typify + Schemars**: JSON Schema generation and type generation

## Future Considerations

- Batch operations (multiple entities per invocation)
- Plan/preview mode (show changes without applying)
- State tracking and rollback
- Local entity caching for performance
- Additional output formats (Kustomize, Helm, Terraform)

---

For more information, see [README.md](README.md) for commands and [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
