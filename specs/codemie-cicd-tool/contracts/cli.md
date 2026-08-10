# CLI contract

Source: `specs/codemie-cicd-tool.md` v26, FR-001–006, FR-009,
FR-011/012, FR-014–017, FR-021–036, and VR-001–016.

Status: NORMATIVE ARCHITECTURE CONTRACT.

## 1. Command surface

```text
codemie-gitops lint --file <path> [--repo-root <path>]
                    [--follow-symlinks] [--output text|json]
codemie-gitops apply --file <path> [--repo-root <path>] [--url <url>]
                     [--follow-symlinks]
                     [--adopt-workflow-id <uuid>] [--output text|json]
codemie-gitops login [--url <url>] [--auth-url <url>]
                     [--client-id <id> |
                      --email <email>]
```

Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
`CODEMIE_PASSWORD`) are accepted **only through environment variables**.
The corresponding `--token`, `--client-secret`, and `--password` flags
are **not part of this command surface** and MUST NOT be accepted.
Supplying any of these as a flag is `E_USAGE`, exit 2, before network
access. The non-secret selectors `--client-id` and `--email` MAY be
supplied as flags. (SEC-001 remediation, v25.)

The executable is exactly `codemie-gitops`. A packaging/help snapshot proves
that no phase-1 command is published under another executable name.

No plan, delete, batch, generic-adoption, or dedicated Datasource lifecycle
option/command exists. An unknown or forbidden option fails as `E_USAGE`, exit
2, before network access. Ordinary Datasource request fields remain valid.

`--adopt-workflow-id` accepts a canonical UUID only for a Workflow declaration.
It is an invocation selector, never YAML, local state, or output. It selects
only that UUID and invokes ADR-008 checks; display name never selects or vetoes
the explicit candidate.

## 2. Configuration and project resolution

`.codemie/config.yaml` is a closed object governed by
[`repository-config.schema.json`](repository-config.schema.json). Its only
keys are non-secret `url`, `auth_url`, and `project`. Unknown keys, including
credential-like keys, are rejected locally.

**URL validation (SEC-002, ADR-011)**: Every URL value supplied via flag,
environment variable, or repository config must be a `ValidatedUrl` (absolute
`http`/`https`; no userinfo; no fragment; no control characters). URL userinfo
is rejected at schema-validation time for `url` and `auth_url` in config.
`auth_url` must be `https` scheme; an `http` `auth_url` is `E_CONFIGURATION`,
exit 2. `url` allows `http` only when the resolved destination is loopback
(runtime-enforced; schema accepts `http://localhost/...` forms). An invalid
higher-precedence value is `E_CONFIGURATION`, exit 2; lower-precedence values
are not used as fallbacks.

Resolution is per field:

- target API: `--url` > `CODEMIE_URL` > config `url`;
- Keycloak endpoint: `--auth-url` > `CODEMIE_AUTH_URL` > config `auth_url`;
- bearer token: `CODEMIE_TOKEN` (environment only — `--token` flag not accepted);
- client ID: `--client-id` > `CODEMIE_CLIENT_ID` (non-secret selector; flag permitted);
- client secret: `CODEMIE_CLIENT_SECRET` (environment only — `--client-secret` flag not accepted);
- local email: `--email` > `CODEMIE_EMAIL`;
- local password: `CODEMIE_PASSWORD` (environment only — `--password` flag not accepted); and
- effective project: declaration `metadata.project` > config `project`.

Secret credentials (bearer token, client secret, password) have no flag source
and no repository-config source or built-in default. Supplying any of them as a
flag is `E_USAGE`, exit 2, before network access. Non-secret selectors
(`--client-id`, `--email`) follow the standard flag-over-environment precedence.
An invalid higher-precedence value is not bypassed. For declarations,
`metadata.project` overrides repository `project`; omission is valid only when
the config supplies a non-empty default. Effective-project resolution occurs
before semantic/cross-reference validation and is offline. Missing in both
places is `E_SCHEMA`, exit 2.

## 3. Discovery and safe local reads

The repository root is explicit `--repo-root`, otherwise the nearest ancestor
with `.git`. Discovery reads regular YAML files in deterministic bytewise path
order, excluding `.git/**` and `.codemie/config.yaml`. Directory names such as
`codemie` and `codemie-ui` have no runtime exclusion semantics. Symlinks are
rejected unless `--follow-symlinks`; an allowed canonical target must remain in
the repository and cycles or escapes fail.

`contentFrom` resolves relative to the declaring YAML's directory. It receives
the same root-containment, symlink, regular-file, and size checks as other local
reads. File, path, and content values never enter diagnostics.

## 4. `lint`

Lint performs no network access. It parses bounded YAML, validates the exact
closed declaration schema, computes effective project, applies semantic and
secret-field rules, builds the repository symbol table, and validates natural
and Workflow-local references. It never inserts server defaults. Omitted
optional-null fields are valid; omitted/null authoring-required fields are
`E_SCHEMA`, exit 2, before network access.

Success writes one `valid` record to stdout. A text warning uses a fixed safe
stderr line; a JSON warning uses one compact object conforming to
[`warning.schema.json`](warning.schema.json). Warnings do not change exit 0 and
never enter the success object. Failure leaves stdout empty and uses the
diagnostic contract.

## 5. `apply`

Before a write, apply completes local validation, authentication,
compatibility, exhaustive identity, visibility, write, preservation, and
reference checks. It then runs:

```text
enumerate/resolve -> exact identity filter
-> absent: project POST request -> POST -> verify identity -> created
-> present: read only required detail -> project PUT request -> PUT
            -> verify identity -> updated
```

No field-state test suppresses the selected write. Every valid invocation
performs one modifying request.

Projection enumerates the source-pinned presence classes. Omitted and explicit
null optional-null fields become explicit JSON null in each applicable JSON
POST/PUT. Authoring-only, operation-inapplicable, read-only, and tool/mixed-owned
members do not receive fabricated nulls. Workflow `meta_config` is assembled by
its strict decode/preserve/overlay/reserved-member/canonical-string rule. File
uses the exact singular multipart/query route and encodings in the HTTP
contract.

- Assistant uses exact slug lookup.
- Workflow uses the reserved `meta_config` identity; optional adoption follows
  ADR-008 and performs PUT.
- Skill uses the exhaustive ADR-007 resolver and no first/newest/owner tie-break.
- Datasource exhausts visible pages and uses the selected peer kind's ordinary
  create or update projection. `find_id` is a hint, never uniqueness proof.
- External integration references are pre-existing configuration. The CLI
  sends only selected-schema opaque references; CodeMie is authoritative for
  validity and access.
- Dedicated Datasource lifecycle routes are outside the command/API surface.
- POST/PUT is never blindly retried. Only the bounded Skill create-409
  re-resolution in ADR-007 is permitted, and it never repeats POST.

## 6. `login`

Exactly one of three modes is selected before any network access:

- **Mode (a) — Keycloak `client_credentials`**: client ID (`--client-id` /
  `CODEMIE_CLIENT_ID`; non-secret selector), client secret
  (`CODEMIE_CLIENT_SECRET`; environment only), and an explicit token endpoint
  resolved as `--auth-url` > `CODEMIE_AUTH_URL` > config `auth_url`. Selected
  when `CODEMIE_CLIENT_SECRET` is set AND `auth_url` is configured.
- **Mode (c) — Keycloak ROPC (v26)**: client ID (`--client-id` /
  `CODEMIE_CLIENT_ID`; non-secret selector; defaults to `codemie-sdk` if
  unset), email (`--email` / `CODEMIE_EMAIL`), and password (`CODEMIE_PASSWORD`;
  environment only), sent to the same `auth_url` endpoint as Mode (a). No
  `CODEMIE_CLIENT_SECRET` is required or sent. Selected when
  `CODEMIE_CLIENT_SECRET` is **not** set, AND `CODEMIE_EMAIL` +
  `CODEMIE_PASSWORD` are set, AND `auth_url` is configured.
- **Mode (b) — Local-auth**: email (`--email` / `CODEMIE_EMAIL`) and password
  (`CODEMIE_PASSWORD`; environment only) at `POST /v1/local-auth/login`
  (relative to `target_url`). Selected when `CODEMIE_CLIENT_SECRET` is **not**
  set, AND `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` are set, AND `auth_url` is
  **not** configured.

The `--client-secret` and `--password` flags are not part of this command surface.
Supplying either as a flag is `E_USAGE`, exit 2, before any network access.

Mode (a) sends `grant_type=client_credentials`; Mode (c) sends
`grant_type=password` (ROPC). Both send only to the selected `auth_url`
endpoint. Neither derives or probes an endpoint from `--url`, `CODEMIE_URL`,
config `url`, a hostname, realm/path convention, or any other value. A missing
or invalid `auth_url` is `E_CONFIGURATION`, exit 2, before any network access;
lower-precedence values are not contacted. Any combination that does not match
exactly one of the three modes above is `E_CONFIGURATION`, exit 2.

Successful login writes exactly the bearer token plus newline to stdout and
nothing to stderr. It is never persisted or repeated. Failure writes no stdout
and uses a fixed allowlisted diagnostic; server text and body are discarded.

## 7. Exit and stream contract

| Exit | Categories | Examples |
|---|---|---|
| 0 | success | `valid`, `created`, `updated`; warnings allowed |
| 1 | reconciliation, server rejection | ambiguity, invalid marker, adoption required, resolution instability, locally valid target rejection, uncertain write |
| 2 | usage/local/config/auth/authz/compatibility/connectivity/internal | local files/schema/refs, incomplete visibility, forbidden write, incompatible API, unavailable target |

On every failure, stdout is empty. Text mode emits a fixed safe line to stderr;
JSON mode emits exactly one compact object conforming to
[`diagnostic.schema.json`](diagnostic.schema.json). The closed schema enforces
the error-code/category/exit matrix.

## 8. Successful output

JSON success is exactly [`outcome.schema.json`](outcome.schema.json):

```json
{"action":"updated","kind":"Workflow","project":"example","slug":"flow"}
```

The text record carries the same approved fields:

```text
updated Workflow example/flow
```

Identity is only project plus the kind key. No server ID, integration ID,
target URL, request content, payload, or server text appears.

## 9. Diagnostic data boundary

The only admissible failure fields are those in
[`diagnostic.schema.json`](diagnostic.schema.json): stable code/category/exit;
local file/line/column/field path; HTTP status/method/route template; tool
request ID; and validated dedicated server correlation ID.

Correlation IDs match `[A-Za-z0-9._:-]{1,128}`. No raw/full URL, request or
response body, server-provided text or payload, arbitrary header, declaration
or sidecar value, credential, token, cookie, authorization header,
secret-bearing field, or secret-like value is accepted by the renderer. Debug,
verbose, trace, panic, and internal paths have no broader API.

## 10. Safe output rendering (SEC-005)

All output rendering (text and JSON) must satisfy these rules:

**One physical record per line**: Each outcome, warning, or diagnostic produces
exactly one physical output line. No untrusted value may include an unescaped
newline, carriage return, or other record separator that would extend or forge
an adjacent record.

**Identifier constraints**: Identity values (`project`, `slug`, `name`,
`repo_name`) written to output are governed by `outcome.schema.json` and must
have already passed schema validation. The schema enforces `maxLength` and
rejects C0/C1 controls and bidi formatting characters. The renderer must not
bypass schema validation.

**Field path canonicality**: `source.fieldPath` values in diagnostics and
warnings are generated by the parser from the YAML AST structure, not echoed
from arbitrary input key strings. Source file paths are abbreviated under the
4,096-byte `source.file` limit.

**JSON output**: JSON output must be produced by a serializer (not string
concatenation). The serializer must JSON-escape control characters in all
string values, including identity fields.

**Text output**: Text mode for outcomes uses only the fixed template
`<action> <kind> <project>/<key>`. Values come from schema-validated fields
only. If a rendered value contains a character that would misrepresent the
record (after schema validation has already passed), it is displayed as
produced by the serializer — the schema constraints already exclude controls
and bidi characters from the identifier fields.

**Route and query encoding**: HTTP route parameters and query parameter values
must be percent-encoded using a URL-safe encoder. No route or query value is
constructed by string concatenation of untrusted input.

**Multipart basenames**: See HTTP adapter contract §7 for the basename
rejection and encoding rules.

**Allowed values come from fixed enum vocabulary**: `action`, `kind`,
`category`, `errorCode`, `warningCode` values come from fixed enums and are
never derived from untrusted input.
