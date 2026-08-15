# Current implementation reference

This document describes the executable architecture of `codemie-gitops` 0.1.5.
It is intended to be sufficient, together with the closed contracts and tests,
to recreate a behaviorally equivalent implementation. It does not require the
same private names or source layout.

## Product boundary

The binary processes one entity per invocation. Supported commands are `lint`,
`apply`, `save`, and `login`. Supported declaration kinds are `Assistant`,
`Workflow`, `Skill`, and `Datasource`. There is no batch operation, repository
discovery, plan, delete, state database, cache, automatic retry of a modifying
request, rollback, or compensation.

Every declaration uses `apiVersion: codemie.epam.com/v1alpha1`. The checked-in
JSON Schema is closed: unknown fields fail. YAML is an untrusted boundary and
must pass restricted parsing, JSON Schema validation, generated DTO decoding,
and semantic validation before business logic receives it.

## Build and generated types

`Cargo.toml` fixes Rust 1.95 and edition 2024. `Cargo.lock` is required for
reproducible builds. `build.rs` reads
`specs/codemie-cicd-tool/contracts/declaration-v1alpha1.schema.json`, removes
conditionals unsupported by the type generator, disambiguates the wire field
`indexType`, and uses `typify` to generate declaration DTOs in `OUT_DIR`.
Runtime schema validation still uses the original unmodified embedded schema;
generated DTOs do not widen accepted YAML.

The library exposes one public entry point, `codemie_gitops::run(args)`. The
binary creates the Tokio runtime, initializes the environment-filtered tracing
subscriber, and passes `std::env::args_os()` to that function. HTTP client
construction installs the Rustls ring provider.

## Module responsibilities

| Module | Required behavior |
|---|---|
| `cli` | Parse the exact command surface, reject unknown/secret flags, convert raw arguments into validated commands, dispatch, and preserve stdout/stderr rules. |
| `config` | Validate target/auth URLs. Accept HTTPS; accept HTTP target URLs only for loopback. Resolve flags over environment values through Clap. No configuration file is loaded. |
| `auth` | Select bearer-token, Keycloak client-credentials, Keycloak ROPC, or local-auth mode. Keep secrets in `secrecy::SecretString`; disable redirects; enforce connection/request timeouts. |
| `input` | Open exactly one selected regular file, resolve only explicit Skill and File Datasource auxiliaries, reject symlinks/escapes/non-files, and enforce byte budgets. |
| `parse` | Reject duplicate keys, multiple documents, anchors, aliases, tags, merge keys, excessive depth/scalars/collections, then run JSON Schema and typed DTO decoding. |
| `validate` | Validate natural identities and Workflow-local graph relationships. Cross-entity existence is not checked offline. |
| `lint` | Run input/parse/semantic validation without network access and emit `valid` plus deterministic warnings. |
| `coordinator` | Enforce the five-minute apply deadline, validate Workflow adoption use, dispatch the kind adapter, perform exactly one planned mutation, and post-write verify identity. |
| `adapters` | Implement kind-specific identity resolution, reference resolution, authorization evidence, routes, mutation dispatch, and verification. |
| `projection` | Convert validated declarations and resolved natural references into exact server JSON, query, or multipart requests. Materialize applicable omitted nullable fields as JSON null. |
| `http` | Build redirect-disabled Rustls clients, encode routes/queries, add bearer auth, cap bodies, strictly decode consumed JSON, retry bounded GETs only, and never retry writes. |
| `pagination` | Require consistent zero-based pagination with page size 100, at most 1,000 pages and 100,000 items; reject drift and cycles. |
| `strict_json` | Reject duplicate JSON object keys recursively while allowing unknown unconsumed response fields at typed boundaries that explicitly permit them. |
| `save` | Read one selected server entity, reject unsafe/unrepresentable state, reverse-project canonical YAML, validate it in memory, and publish with create-new semantics. |
| `output` / `render` | Serialize the closed success/warning/error forms. Success uses stdout. Every failure uses stderr and leaves stdout empty. Before emitting the closed diagnostic, emit the full internal error chain at `DEBUG` level via `tracing` (opt-in via `RUST_LOG=debug`; stderr only; not part of the machine-readable contract). |
| `error` | Preserve typed layer errors and map reconciliation/server write failures to exit 1; usage, local, auth, connectivity, compatibility, timeout, and internal failures to exit 2. |

## Command behavior

### `lint --file PATH [--output text|json]`

`lint` performs no network requests and reads no neighboring declarations. It
may read one explicit Skill `contentFrom` Markdown file or the explicit paths
listed by a File Datasource. Success action is `valid`. Failed lint emits no
warnings.

### `apply --file PATH [--url URL] [--adopt-workflow-id UUID] [--output text|json]`

Target precedence is `--url` over `CODEMIE_URL`; a target is mandatory.
`CODEMIE_TOKEN` is the only bearer-token input. Apply loads and validates input,
proves project membership, resolves identity and natural references, proves
`write` on an existing target, seals a write plan, sends one POST or PUT, then
re-resolves the exact authored identity. A failed post-write check never causes
rollback, deletion, or a second write.

`--adopt-workflow-id` is valid only for Workflow. It selects one unmarked legacy
Workflow by canonical UUID after normal identity scans and adds the reserved v2
identity marker during its one update. UUIDs never enter declaration YAML.

### `save --kind KIND --project PROJECT --file PATH ...`

Selectors are `--slug` for Assistant, `--slug` plus optional `--id` for
Workflow, `--name` for Skill, and `--repo-name` for Datasource. Exactly the
selector set for the chosen kind is accepted. Save uses GET requests only.

The output path and every File Datasource placeholder path must not exist.
Parents must already exist and contain no symlink component. Writes use
create-new semantics and no temporary rename. Skill main content is emitted
inline. File Datasource server filenames are preserved in `uploaded_files` and
corresponding zero-byte local placeholders are created before YAML publication.

### `login [--url URL] [--auth-url URL] [--client-id ID] [--email EMAIL]`

Secret flags do not exist. Secrets come only from `CODEMIE_TOKEN`,
`CODEMIE_CLIENT_SECRET`, and `CODEMIE_PASSWORD`. Non-secret environment inputs
are `CODEMIE_URL`, `CODEMIE_AUTH_URL`, `CODEMIE_CLIENT_ID`, and `CODEMIE_EMAIL`.

Mode priority and requirements:

1. Non-empty `CODEMIE_TOKEN`: return bearer token without auth request.
2. Client credentials: client secret + HTTPS auth URL. Client ID is optional;
   when absent, the form omits `client_id`.
3. ROPC: email + password + HTTPS auth URL; client ID defaults to
   `codemie-sdk`.
4. Local development: email + password without auth URL; POST to
   `{CODEMIE_URL}/v1/local-auth/login` and require a target URL.
5. Any other combination: configuration failure before network access.

Successful login prints only the token line. Auth and API clients disable
redirects, use a 10-second connect timeout and 60-second request timeout, and
use Rustls rather than native TLS.

## Input and parsing limits

| Resource | Limit |
|---|---:|
| Selected YAML bytes | 1 MiB |
| YAML depth | 32 |
| Bytes per YAML scalar | 128 KiB |
| Members per YAML collection | 10,000 |
| Skill sidecar bytes | 128 KiB; resulting content must meet schema character bounds |
| File Datasource parts | 1–10 |
| File Datasource bytes per file | 32 MiB |
| File Datasource aggregate bytes | 128 MiB |
| Whole lint/apply/save invocation | 300 seconds |
| HTTP response body | 8 MiB |
| JSON response depth | 64 |
| Server list page size | 100 |
| Server list pages/items | 1,000 / 100,000 |

Selected declarations and auxiliaries must be regular files. Symlinks are
rejected. Auxiliary paths must be relative, remain beneath the declaration's
canonical parent, and contain no parent traversal. Skill sidecars end in `.md`.
File multipart names are safe UTF-8 basenames without controls or separators.

## Reconciliation and authorization

All write paths first call `GET /v1/user`. Exact membership in the authored
project permits create. Updates additionally require `write` in the exact
resolved entity's `user_abilities`.

- Assistant: resolve exact project + slug using the direct lookup. Resolve
  Datasource context, sub-assistant, and Skill references before projection.
- Skill: enumerate page 0 onward and match exact project + authenticated creator
  ID + name. A create HTTP 409 permits one bounded read-only re-resolution, but
  the invocation remains conflict and sends no PUT.
- Workflow: scan both required scopes, accept only a strict reserved v2 marker
  matching creator + project + slug, and reject malformed or ambiguous markers.
  Resolve persisted actors, Skill refs, and Datasource refs before projection.
- Datasource: exhaust visible project-scoped pages for update selection. A
  visible miss allows one create; server HTTP 409 is authoritative collision
  evidence. File Datasource uses multipart; other kinds use the pinned JSON or
  query projection.

The reserved Workflow metadata key is
`codemie.epam.com/gitops/workflow-identity`. Current markers contain version 2,
authenticated creator user ID, project, and slug. Authored `spec.name` is a
display value and never replaces the natural identity.

## Transport invariants

API routes must begin with `/v1/` and are joined beneath the validated base URL.
Route segments and query values are percent-encoded by helpers. Redirects are
disabled. Authorization headers are marked sensitive. Response bodies are
bounded before decoding. Consumed JSON is duplicate-key safe. GET has at most
three attempts with bounded delay; POST, PUT, form login, and multipart writes
are never retried.

HTTP success alone does not prove a write. Coordinator post-write verification
must resolve the returned route identity to the authored natural identity.
Connectivity or compatibility loss after mutation is classified separately
because the write may have committed.

## Output and errors

Text success is one line: `valid`, `created`, or `updated` plus the stable
identity fields defined by the output contract. JSON success follows
`contracts/outcome.schema.json`. Warnings and diagnostics follow their closed
schemas. Raw payloads, response bodies, credentials, headers, cookies, secret
values, and sidecar/file bytes are prohibited from output.

The closed diagnostic written to stderr contains only the `errorCode`,
`category`, and `exitCode` from a fixed enum (SEC-005). The full internal error
chain is additionally emitted at `DEBUG` level via `tracing` immediately before
the closed diagnostic line. Enable it with `RUST_LOG=debug`. This does not
weaken SEC-005: tracing output is opt-in, goes only to stderr, and is never
part of the machine-readable stdout contract. It is safe to enable in
development and in CI pipelines that do not parse stderr.

Exit 0 means success. Exit 1 means valid local input reached reconciliation or
a server modifying-request failure. Exit 2 means CLI usage, local input,
schema, configuration, authentication/authorization, visibility, connectivity,
compatibility, timeout, output, or internal failure. Clap usage errors also use
exit 2.

## Test reconstruction map

Most behavioral tests are colocated under `#[cfg(test)]` in the owning module.
Keep this ownership when recreating coverage:

- parsing/schema/resource attacks: `parse`, `input`, `schema`;
- domain conversions and invariants: `domain`, command DTO conversions;
- transport, retries, response limits, strict decoding: `http`, `strict_json`,
  `pagination`;
- identity, authorization, request counts, races: each kind adapter plus
  `coordinator`;
- forward/reverse wire mappings: `projection`, `save::reverse`,
  `save::snapshot`;
- exclusive publication and placeholders: `save::publication`;
- safe text/JSON output and exit mapping: `render`, `output`, `error`;
- executable CLI behavior: `tests/cli_lint.rs`;
- pinned OpenAPI compatibility: `tests/openapi_contract.rs`.

Required local gate sequence:

```sh
make format
make lint
make test
cargo build --locked --release
```

`make test` runs `cargo test --locked --all-targets`. Reference-only `codemie/`
and `codemie-ui/` are never formatted, migrated, or treated as workspace code.

## Reconstruction checklist

1. Restore the exact locked dependency graph and Rust toolchain.
2. Generate declaration DTOs from the closed schema during build.
3. Implement boundary parsing and typed conversions before networking.
4. Implement output/error contracts before entity adapters so failures remain
   safe during development.
5. Implement redirect-disabled strict HTTP and pagination primitives.
6. Implement kind adapters and projections independently, including exact
   request-count and no-write tests.
7. Implement coordinator sealing and post-write verification.
8. Implement GET-only save reverse projection and exclusive publication.
9. Run unit, CLI, OpenAPI, format, lint, and locked-build gates.

Behavior not specified here must be taken from the closed contracts and their
tests. Historical verification prose may explain intent but cannot widen the
current executable contract.
