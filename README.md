# CodeMie GitOps

`codemie-gitops` validates and applies one CodeMie declaration per invocation.
The repository is the desired-state record; the CLI has no local state database,
`plan`, `delete`, batch, or automatic rollback command.

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

## Configuration and offline lint

Runtime configuration comes only from flags and environment variables. The
target uses `--url` or `CODEMIE_URL`, the authentication endpoint uses
`--auth-url` or `CODEMIE_AUTH_URL`, and every declaration contains its explicit
`metadata.project`. No repository configuration file is loaded.

Lint is offline and should run before credentials are made available:

```sh
codemie-gitops lint --file skills/example-skill.yaml
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

See [the portable examples](examples/README.md) for all four declaration kinds
and inert GitHub/GitLab samples.

## Save

`save` requires `--project` and writes one new YAML file. Skill content is
inline. The command validates the generated declaration before directly
creating the requested final path; it never overwrites, stages, renames, or
creates a sidecar. A failed write can leave an incomplete new path, reported as
`E_OUTPUT_WRITE`; inspect and remove that path manually before retrying.
