# CLI contract

Source: `specs/codemie-cicd-tool.md` v33.3 plus the approved Save v3.5
contract at `../../save-server-entity/contracts/cli-save-v1.md`.

Status: NORMATIVE ARCHITECTURE CONTRACT.

## 1. Command surface

```text
codemie-gitops lint --file <path> [--output text|json]
codemie-gitops apply --file <path> [--url <url>]
                     [--adopt-workflow-id <uuid>] [--output text|json]
codemie-gitops save --kind <kind> --project <project> --file <path>
                    [--slug <slug> | --name <name> | --repo-name <name>]
                    [--id <workflow-uuid>] [--url <url>]
                    [--output text|json]
codemie-gitops login [--url <url>] [--auth-url <url>]
                     [--client-id <id>] [--email <email>]
```

`--file` is required and names exactly one declaration. `--repo-root` and
`--follow-symlinks` are unknown options and fail `E_USAGE`, exit 2, before file
or network access. No plan, delete, batch, generic-adoption, or Datasource
lifecycle command exists.

Secret credentials are environment-only: `CODEMIE_TOKEN`,
`CODEMIE_CLIENT_SECRET`, and `CODEMIE_PASSWORD`. Corresponding secret-bearing
flags are unknown and fail before network access.

## 2. Configuration

There is no repository configuration file and no Git/repository-root lookup.
Resolution is per field:

- target API: `--url` > `CODEMIE_URL`;
- Keycloak endpoint: `--auth-url` > `CODEMIE_AUTH_URL`;
- bearer token: `CODEMIE_TOKEN`;
- client ID: `--client-id` > `CODEMIE_CLIENT_ID`;
- client secret: `CODEMIE_CLIENT_SECRET`;
- local email: `--email` > `CODEMIE_EMAIL`;
- local password: `CODEMIE_PASSWORD`;
- effective project for lint/apply: declaration `metadata.project`.

URLs retain ADR-011 validation, TLS, loopback, and redirect rules. Invalid
higher-precedence values fail closed and do not fall back.

## 3. Local read and validation

Lint/apply open only the selected `--file` as one ordinary bounded operation.
They do not enumerate a directory, inspect neighboring declarations, discover
a root, apply ignore rules, order files, or load configuration. The only
auxiliary reads are paths explicitly authored in Skill `contentFrom` and File
Datasource `spec.files[]`, resolved from the selected declaration's parent
under their bounded no-symlink contracts.
The command deadline governs the operation; no public cancellation-token
interface is required.

Validation covers one YAML document, duplicate keys/tags/aliases and resource
limits, the exact closed declaration schema, semantic invariants, explicit
auxiliary-input safety/content, reference shapes, and Workflow-local IDs.
Natural-reference existence is not an offline condition.

## 4. `lint`

Lint performs zero network access and zero file access beyond the selected
declaration. After that declaration validates, it emits target warning records
in fixed warning-code then field-path order and one `valid` success record.
On failure stdout is empty, no warning is emitted, and stderr contains exactly
one safe diagnostic.

## 5. `apply`

Apply first validates the selected declaration, authenticates, proves pinned
compatibility, and resolves every target/reference through bounded server reads.
Missing, ambiguous, incompatible, or unauthorized evidence fails before a
mutation. A valid invocation performs exactly one create or update, subject to
the existing kind-specific identity, preservation, authorization, post-write
verification, and Skill create-409 rules.

The tool never looks for a referenced local declaration. Workflow
`--adopt-workflow-id` remains a canonical UUID selector valid only for Workflow.

## 6. `login`

The implemented modes are bearer-token reuse, Keycloak client credentials,
Keycloak ROPC, and loopback local auth. Bearer token has highest priority and
performs no authentication request. Keycloak requires explicit `auth_url`; it
is never derived. Client-credentials mode requires `CODEMIE_CLIENT_SECRET` and
may omit client ID. ROPC requires email/password and defaults client ID to
`codemie-sdk`. Authentication POST redirects remain disabled. Success writes
only the token to stdout; every failure leaves stdout empty.

## 7. Output and diagnostics

Exit 0 is success, exit 1 is a safe reconciliation conflict, and exit 2 is
usage/configuration/local/schema/compatibility/auth/transport/write-uncertainty.
Successful lint/apply emits one closed outcome on stdout. Failures emit exactly
one closed diagnostic on stderr. Paths, declaration values/content, raw URLs,
credentials, bodies, headers, server prose, UUIDs, and Git/CI provenance are not
renderable fields.
