# CodeMie GitOps

`codemie-gitops` validates, saves, and applies one CodeMie declaration per
invocation. The CLI has no local state database, `plan`, `delete`, batch, or
automatic rollback command.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

## Build or install

For a source build, use the locked dependency graph, run the checks, and retain
the checksum with the binary that passed them:

```sh
make format
make lint
make test
cargo build --locked --release
sha256sum target/release/codemie-gitops > codemie-gitops.sha256
```

Alternatively, download a published prebuilt binary and verify its published
SHA-256 (and release signature, when provided) before making it executable.
Never rebuild inside a credentialed deployment job: promote and reverify the
exact tested artifact.

## Commands

```text
codemie-gitops lint   --file <yaml-path> [--output text|json]
codemie-gitops apply  --file <yaml-path> [--url <url>]
                      [--adopt-workflow-id <uuid>] [--output text|json]
codemie-gitops save   --kind <kind> --project <project> --file <yaml-path>
                      [--slug <slug> | --name <name> | --repo-name <name>]
                      [--id <workflow-uuid>] [--url <url>]
                      [--output text|json]
codemie-gitops login  [--url <url>] [--auth-url <url>]
                      [--client-id <id>] [--email <email>]
```

Save selectors are discriminated by `--kind`:

| Kind | Required selector | Optional selector |
|---|---|---|
| `Assistant` | `--slug <slug>` | none |
| `Workflow` | `--slug <slug>` | `--id <workflow-uuid>` for an unmarked Workflow |
| `Skill` | `--name <name>` | none |
| `Datasource` | `--repo-name <repo-name>` | none |

Exactly one kind-specific selector is required. `--id` is valid only with
`--kind Workflow` and never replaces the required `--slug`. It selects an
unmarked Workflow for local saving; a later `apply --adopt-workflow-id` is the
separate server adoption step.

Every save also requires `--project <project>` and `--file <yaml-path>`. The
target URL is optional when `CODEMIE_URL` is set. `--repo-root`,
`--follow-symlinks`, force/replace flags, and secret-bearing flags are
unsupported.

## Configuration and offline lint

Runtime configuration comes only from flags and environment variables. The
target uses `--url` or `CODEMIE_URL`, the authentication endpoint uses
`--auth-url` or `CODEMIE_AUTH_URL`, and `save` requires explicit `--project`.
Every declaration contains its explicit `metadata.project`. No repository
configuration file is loaded.

Lint is offline and should run before credentials are made available:

```sh
codemie-gitops lint --file examples/repository/skills/example-skill.yaml
```

## Login and token reuse

Secrets are environment-only: `CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, and
`CODEMIE_PASSWORD`. The flags `--token`, `--client-secret`, and `--password`
do not exist. Never print, trace, persist, cache, artifact, or pass a secret in
an argument.

The three login modes are:

1. Client credentials: `CODEMIE_CLIENT_ID` plus `CODEMIE_CLIENT_SECRET`, with
   an explicit HTTPS `CODEMIE_AUTH_URL`.
2. Keycloak user credentials: `CODEMIE_EMAIL` plus `CODEMIE_PASSWORD`, with an
   explicit HTTPS `CODEMIE_AUTH_URL`; client ID defaults to `codemie-sdk`.
3. Local-auth development: `CODEMIE_EMAIL` plus `CODEMIE_PASSWORD`, without an
   auth URL. This mode is for local development only and is forbidden in CI.

Capture successful login stdout once in memory, disable shell tracing first,
and reuse the token in the same shell boundary:

```sh
set +x
CODEMIE_TOKEN="$(codemie-gitops login)"
export CODEMIE_TOKEN
codemie-gitops apply --file skills/example-skill.yaml
unset CODEMIE_TOKEN CODEMIE_CLIENT_SECRET CODEMIE_PASSWORD
```

CI token delivery is provider-specific. GitHub may capture one fresh token only
inside its protected step, must immediately invoke native `add-mask` before any
later command/output, and reuses the value only in that step process. GitLab
does not invoke `login`; its protected job receives one pre-supplied,
environment-scoped protected+masked `CODEMIE_TOKEN` and reuses it only in that
job process. Neither example persists, transfers, re-emits, or simulates
masking. A provider without a safe configured token or stable runtime masking
must fail before login or a CodeMie request.

## Apply and output

`apply` accepts exactly one Assistant, Workflow, Skill, or Datasource file.
Every valid invocation writes: zero exact identities creates; one safe identity
updates. Repeating the same declaration is another `updated` operation, never
a no-op. Declarations not selected by that invocation remain untouched.

Success is one line on stdout (`valid`, `created`, or `updated`). Failures leave
stdout empty, write one allowlisted diagnostic to stderr, and exit 1 for a
reconciliation/server rejection or 2 for usage, local validation,
configuration, authentication/authorization, compatibility, connectivity, or
internal failure. `--output json` selects the closed JSON form.

Apply performs server identity resolution and creates or updates exactly one
entity. Workflow adoption is explicit: `apply --adopt-workflow-id <uuid>` is
required only for an unmarked Workflow saved by exact ID. The UUID is never
read from or written to YAML.

See [the portable examples](examples/README.md) for all four declaration kinds
and inert GitHub/GitLab samples. Validate a declaration before applying it:

```sh
codemie-gitops lint --file examples/repository/skills/example-skill.yaml
codemie-gitops apply --file examples/repository/skills/example-skill.yaml
```

## Save

`save` reads one existing Assistant, Workflow, Skill, or Datasource and writes
one new YAML declaration. It performs only GET requests. Server-managed IDs are
converted to natural references when possible; unsupported, secret, masked, or
ambiguous state fails before output.

Skill main content is emitted inline as `spec.content`; companion metadata and
content are emitted in `spec.companion_files`. No sidecar is created. The
generated declaration is validated against the closed schema before writing.

The output path must not already exist, and its parent directories must already
exist. Save writes directly with create-new semantics and never overwrites,
stages, renames, or replaces a file. A failed write can leave an incomplete new
path, reported as `E_OUTPUT_WRITE`; inspect and remove that path manually before
retrying.

Examples:

```sh
codemie-gitops save --kind Assistant --project demo \
   --slug support-assistant --file saved-assistant.yaml

codemie-gitops save --kind Skill --project demo \
   --name triage-skill --file saved-skill.yaml

codemie-gitops save --kind Workflow --project demo \
   --slug support-flow --file saved-workflow.yaml

codemie-gitops save --kind Datasource --project demo \
   --repo-name product-docs --file saved-datasource.yaml
```

An ID-selected unmarked Workflow reports `adoptionRequired: true` in JSON and
adds `(adoption required on apply)` in text output. Save itself never marks or
adopts a Workflow.

### Datasource identity

The Datasource natural identity is `project + repo_name`. In declarations and
server read responses, the value is named `repo_name`:

```yaml
metadata:
   project: demo
   repo_name: product-docs
```

CodeMie create/update request bodies call the same value `name`, so the CLI
translates `metadata.repo_name` to API `name`. There is no separate display
name. `index_type` and `vcs_type` describe the Datasource branch; they are not
part of identity and cannot disambiguate duplicate `project + repo_name` rows.

## Local development

Run the Rust checks with:

```sh
make format
make lint
make test
```

The local CodeMie server uses Podman Compose by default and requires Podman,
Podman Compose, PostgreSQL, and Elasticsearch:

```sh
make dev-server
```

The server is exposed at `http://127.0.0.1:8080` for local development. Use
local-auth credentials or a test bearer token; never use production credentials
against the development stack.
