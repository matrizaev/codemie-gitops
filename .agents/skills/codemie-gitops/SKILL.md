---
name: codemie-gitops
description: Working in and on the codemie-gitops repository — the stateless CodeMie declarative CI/CD CLI. Use when building, testing, or extending the CLI, authoring or validating CodeMie declarations, debugging lint/apply/save/login behavior, or running the repo's dev/release workflows.
---

# CodeMie GitOps (`codemie-gitops`)

`codemie-gitops` is a **stateless** Rust CLI that validates, saves, and applies
**one CodeMie declaration per invocation**. It is a client to the CodeMie
server API. There is deliberately **no** local state database, `plan`, `delete`,
batch, repository discovery, automatic rollback, or retry of a modifying
request.

Supported declaration kinds: `Assistant`, `Workflow`, `Skill`, `Datasource`.
Every declaration uses `apiVersion: codemie.epam.com/v1alpha1`. The checked-in
JSON Schema (`contracts/declaration-v1alpha1.schema.json`) is the **closed
machine authority for YAML** — unknown fields fail, so the schema is the
single source of truth when deciding whether YAML is valid.

## Docs authority order

Follow this order when rebuilding or changing the tool (see `docs/README.md`):

1. **Executable contracts** — `contracts/` (declaration schema, adapter
   manifest `adapter-manifest-v2.42.0.json`, pinned `openapi.json`, CLI and
   save contracts). These **override conflicting prose** for closed machine
   interfaces.
2. **Code and tests** — `src/` and `tests/` represent current behavior. When
   prose disagrees with code, update prose and tests together; never
   reinterpret a contract silently.
3. `docs/implementation-reference.md` — module boundaries, algorithms, limits.
4. `ARCHITECTURE.md` — short architectural overview.
5. `README.md`, `CONTRIBUTING.md`, `docs/yaml-reference.md` — user docs.
6. `docs/adr/` — rationale only; ADRs never override executable contracts.

Rebuild baseline: Rust 1.95, edition 2024, `cargo build --locked --release`,
required checks `make format`, `make lint`, `make test`. Backend baseline:
CodeMie tag `2.42.0` (see `contracts/source-baseline.md`). Generated
declaration DTOs come from `build.rs` + `typify` into Cargo's `OUT_DIR` and
are not committed.

## Commands

```text
codemie-gitops lint   --file <yaml-path> [--output text|json]          # offline
codemie-gitops apply  --file <yaml-path> [--url <url>]
                      [--adopt-workflow-id <uuid>] [--output text|json]
codemie-gitops save   --kind <kind> --project <project> --file <yaml-path>
                      [--slug <slug> | --name <name> | --repo-name <name>]
                      [--id <workflow-uuid>] [--url <url>]
                      [--output text|json]
codemie-gitops login  [--url <url>] [--auth-url <url>]
                      [--client-id <id>] [--email <email>]
```

- **Kind selectors** (exactly one required): `Assistant` → `--slug`;
  `Workflow` → `--slug` (plus `--id <uuid>` only for an unmarked Workflow);
  `Skill` → `--name`; `Datasource` → `--repo-name`. Every save also needs
  `--project` and `--file`.
- **Identity is natural and per kind**: `Assistant` = `project + slug`;
  `Workflow` = `project + slug` (marked) or `project + id` (unmarked
  adoption); `Skill` = `project + name`; `Datasource` = `project + repo_name`
  (the API calls `repo_name` → `name` in request bodies — a wire naming
  difference, not a second name).
- **Exit codes**: 1 = reconciliation/server rejection; 2 = usage, local
  validation, config, auth, compatibility, connectivity, or internal failure.
- **Output contract**: success is one stdout line (`valid`, `created`,
  `updated`); failures leave stdout empty, write one closed diagnostic to
  stderr (`error: E_SCHEMA` or `{errorCode, category, exitCode}`), and exit
  nonzero. `RUST_LOG=debug` adds the full internal error chain to stderr only,
  never to the machine-readable stdout contract.
- `apply` writes exactly once (POST create or PUT update); repeating the same
  declaration is another `updated`, never a no-op. Workflow adoption is
  explicit via `--adopt-workflow-id`; UUIDs never appear in YAML.
- `save` performs GETs only, writes with **create-new semantics** (never
  overwrites, stages, renames, or replaces; parent dirs must exist). A failed
  write can leave an incomplete path (`E_OUTPUT_WRITE`) or an orphan
  `<yaml-name>.files/` dir — inspect and remove manually before retrying.

## Secrets and auth — non-negotiable rules

- Secrets are **environment-only**: `CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
  `CODEMIE_PASSWORD`. The flags `--token`, `--client-secret`, `--password` do
  not exist. Never print, trace, persist, cache, artifact, or pass a secret in
  an argument.
- Login modes: client credentials (`CODEMIE_CLIENT_SECRET` + explicit HTTPS
  `CODEMIE_AUTH_URL`); Keycloak user (`CODEMIE_EMAIL` + `CODEMIE_PASSWORD` +
  HTTPS `CODEMIE_AUTH_URL`, client id default `codemie-sdk`); local-auth dev
  (email + password, no auth URL — **local development only, forbidden in
  CI**).
- Capture login stdout once in memory, disable shell tracing first, reuse the
  token in the same shell boundary, and unset it afterward:
  `CODEMIE_TOKEN="$(codemie-gitops login)"`.
- Target URL: `--url` or `CODEMIE_URL`; auth endpoint: `--auth-url` or
  `CODEMIE_AUTH_URL`. No repository config file is loaded.
- Lint is offline — run it before credentials are made available.
