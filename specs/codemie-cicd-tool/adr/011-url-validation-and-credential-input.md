# ADR-011: Credential input channels, ValidatedUrl, TLS enforcement, and redirect policy

## Status

Proposed

Supersedes ADR-003 for credential input and URL/TLS/redirect policy.
ADR-003 remains historical rationale for stateless-auth and allowlist-
diagnostics decisions, which are unchanged.

## Context

The pre-implementation security review identified two high-severity gaps
in the credential handling and transport design (SEC-001, SEC-002):

1. **SEC-001 (v25 remediation, product-spec-owner closed)**: ADR-003 permitted
   secret credentials to be supplied via CLI flags. Flags appear in `argv`,
   which is readable by process monitors, shell traces, CI diagnostics, and
   same-host processes. Product specification v25 replaced every secret-value
   flag with environment-only input. The architecture ADR must now reflect
   this normative change.

2. **SEC-002 (solution-architect, this ADR)**: URL and transport policy was
   incomplete:
   - Repository config accepted HTTP (remote plaintext) and URL userinfo
   - The "local development" HTTP exception was not mechanically defined
   - Redirect behavior for authentication POSTs was not specified
   - The CI environment was not required to override repository endpoint values

This ADR closes both gaps. It governs all credential input channels, the
`ValidatedUrl` type, HTTPS requirements, loopback exceptions, redirect
behavior, and CI endpoint protection.

## Decision drivers

- Secret credentials must not appear in `argv`, shell history, or CI traces
- Every credential-bearing HTTP request must reach a validated, TLS-protected
  destination
- The loopback HTTP exception must be mechanically narrow and runtime-enforced
- Redirect behavior for credential endpoints must be fail-closed
- URL userinfo embedded in any authorable non-secret field must be rejected
- CI configuration must prevent repository config from selecting the production
  credential destination

## Options considered

### A. Retain flag-over-env for all credentials, add masking guidance

Rejected. CI log masking does not remove the value from `argv` or process
monitoring. The security reviewer did not accept risk acceptance for
process-visible secrets.

### B. Environment-only secrets; accept but deprecate the secret flags

Rejected. A deprecated flag still places the value in `argv`. Fail-closed
before argument parsing is the only safe option.

### C. Environment-only secrets; secret flags are `E_USAGE` exit 2 before network

**Selected**. Non-secret selectors retain flag-over-environment precedence.

### D. Interactive stdin or file-descriptor input for secrets

Deferred. Not required for phase 1; may be added as an enhancement.

### E. Accept HTTP for any URL, rely on HTTPS as a recommendation

Rejected. No business requirement for non-loopback plaintext HTTP to
credential destinations.

### F. Define loopback HTTP by DNS resolution at schema validation time

Rejected. JSON Schema cannot resolve DNS. Schema validation rejects
non-loopback HTTP patterns where detectable; runtime enforces the resolved-
address loopback restriction.

### G. Allow redirects on authentication POSTs with same-origin enforcement

Rejected. An authentication POST is a one-shot credential delivery; a 3xx
response signals misconfiguration or an active attack. Disabling redirects
entirely is simpler and more secure.

## Decision

### 1. Credential input channels

Secret credentials resolve **only** from their named environment variable:

| Credential | Accepted source | Not accepted |
|---|---|---|
| Bearer token | `CODEMIE_TOKEN` | `--token` flag |
| Keycloak client secret | `CODEMIE_CLIENT_SECRET` | `--client-secret` flag |
| Local-auth password | `CODEMIE_PASSWORD` | `--password` flag |

The CLI parser must **not** register `--token`, `--client-secret`, or
`--password` as valid options. Supplying any of these as a CLI argument is
`E_USAGE`, exit 2, before any network access, with empty stdout.

Non-secret selectors retain flag-over-environment precedence:

| Selector | Sources (highest to lowest) |
|---|---|
| Keycloak client ID | `--client-id` flag, `CODEMIE_CLIENT_ID` |
| Local-auth email | `--email` flag, `CODEMIE_EMAIL` |

No credential has a repository-config source, built-in default, or derived
value. Unknown or credential-like keys in `.codemie/config.yaml` fail locally
before any network access.

### 1a. Keycloak ROPC credential channels (Mode (c), v26)

Mode (c) — Keycloak ROPC — uses the following credential channels:

| Credential | Accepted source | Not accepted | Notes |
|---|---|---|---|
| Client ID | `--client-id` flag, `CODEMIE_CLIENT_ID` | — | Defaults to `codemie-sdk` if unset |
| Email (username) | `--email` flag, `CODEMIE_EMAIL` | — | Non-secret; flag permitted |
| Password | `CODEMIE_PASSWORD` | `--password` flag | Secret; env-only under SEC-001 |

Mode (c) does **not** use `CODEMIE_CLIENT_SECRET`. No `client_secret` field
appears in the ROPC request body. `auth_url` is required and subject to the
same HTTPS requirement as Mode (a) (§3). The redirect-disabled rule (§4)
applies to the Mode (c) ROPC `POST <auth_url>` request.

### 1b. Login mode selection

The `login` command selects exactly one authentication mode before any network
access:

1. **Mode (a) — Keycloak `client_credentials`**: `CODEMIE_CLIENT_SECRET` is
   set AND `auth_url` is configured.
2. **Mode (c) — Keycloak ROPC**: `CODEMIE_CLIENT_SECRET` is **not** set, AND
   `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` are set, AND `auth_url` is configured.
3. **Mode (b) — Local-auth**: `CODEMIE_CLIENT_SECRET` is **not** set, AND
   `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` are set, AND `auth_url` is **not**
   configured.

Any combination outside these three cases is `E_CONFIGURATION`, exit 2, before
network access.

### 2. ValidatedUrl type

A `ValidatedUrl` is an absolute URL satisfying all of:

- Scheme is `http` or `https` only
- No userinfo component — the authority contains no `@` character
- No fragment identifier — the URL contains no `#` character
- No C0/C1 control characters (U+0000–U+001F, U+007F–U+009F) anywhere
- Non-empty, syntactically valid host
- Port, if present, is a valid decimal integer

`ValidatedUrl` is the type for:
- `target_url` and `auth_url` in `ResolvedConfig`
- `url` and `auth_url` in `.codemie/config.yaml`
- Every URL-valued authorable field in declarations that is transmitted to the
  server or could affect a credential destination (Datasource `link`,
  `site_url`, MCP `mcp_connect_url`, and similar)

URL userinfo in any of these fields is rejected at schema validation time
before any parse of the full declaration. Reliance on prose alone is not
sufficient; `contracts/repository-config.schema.json` and
`contracts/declaration-v1alpha1.schema.json` enforce this at the schema level.

### 3. HTTPS requirement and loopback exception

**`auth_url` (Keycloak token endpoint)**: HTTPS required unconditionally.
There is no loopback exception for the identity provider endpoint.

**`target_url` (CodeMie API endpoint)**: HTTPS required for remote hosts.
HTTP is permitted only when the resolved destination is loopback:
- IPv4 loopback: any address in 127.0.0.0/8
- IPv6 loopback: ::1
- Hostname `localhost`: accepted at schema level; the runtime must confirm
  the resolved address is loopback before issuing any HTTP request

**No `--insecure` flag or TLS-verification bypass** is provided. CA/proxy
trust is configured through standard system/CA/proxy environment variables
(`HTTPS_PROXY`, `SSL_CERT_FILE`, and similar).

An invalid higher-precedence URL is exit 2 before network access; lower-
precedence values are not consulted.

### 4. Redirect policy

**Keycloak `POST /protocol/openid-connect/token`** (Mode (a) and Mode (c) ROPC):
redirects are **disabled**. If the server returns a 3xx status, the CLI exits
`E_AUTHENTICATION`, exit 2. No credential is replayed to any redirected URL.

**Local-auth `POST /v1/local-auth/login`**: redirects are **disabled**. Same
treatment as Keycloak.

**Authenticated API calls (GET/POST/PUT)**:
- Preferred: disable all redirects for simplicity.
- If any redirect is retained, it must satisfy all of:
  1. Method-aware: POST and PUT are never automatically redirected to GET
  2. Same-origin: redirect target has the same scheme, host, and port as the
     original request
  3. Bounded: maximum 3 redirect hops
  4. No credential replay: the `Authorization` header must never be forwarded
     to a different authority

### 5. CI endpoint protection

Production CI examples must:

- Inject `CODEMIE_URL` and `CODEMIE_AUTH_URL` from protected CI/CD environment
  configuration at higher precedence than repository config (`--url`/`--auth-url`
  flags or protected environment variables take precedence over `.codemie/config.yaml`)
- Run PR and lint jobs with **no secrets** — `CODEMIE_TOKEN`,
  `CODEMIE_CLIENT_SECRET`, and `CODEMIE_PASSWORD` must not be available to
  fork-triggered or untrusted pull-request workflows
- Use protected deployment environments and human approval gates for apply jobs
- Require host/endpoint ownership review for changes to `.codemie/config.yaml`
  `url` or `auth_url` fields when repository endpoint fallback is used operationally

## Consequences

### Positive

- Secret credentials cannot appear in process argument vectors or shell history
- URL userinfo is rejected at schema time; no credential form can enter Git
  under a non-secret field
- Authentication POST redirects are disabled; exfiltration via redirect chain
  is blocked
- Loopback HTTP exception is runtime-enforced and mechanically narrow
- CI examples require protected endpoint injection, preventing repository config
  from selecting the production credential destination
- Mode (c) Keycloak ROPC allows user-credential login against Keycloak without
  a machine `client_secret`; the same SEC-001 env-only, HTTPS, and
  redirect-disabled rules apply

### Negative

- Teams who previously passed `--token` or `--client-secret` must migrate to
  environment variables before using phase 1
- The loopback HTTP exception requires the runtime to resolve the hostname at
  connection time, not only at config parsing time
- Redirect-disabled behavior may affect deployments behind redirect-based
  reverse proxies; those must be configured to present a direct TLS endpoint

### Risks

- Hostname `localhost` may resolve to non-loopback addresses in unusual DNS
  configurations; the runtime must validate the resolved address, not the string
- A future interactive override (stdin/fd) may be needed for local developer
  workflows; this is out of scope for phase 1 and does not affect SEC-001 closure

## Follow-up actions

- Update `contracts/repository-config.schema.json` URL patterns to enforce
  no-userinfo, no-fragment, no-controls, and HTTPS-only for `auth_url`
- Update `contracts/declaration-v1alpha1.schema.json` URL-valued fields to
  reject userinfo (Datasource link, site_url, MCP mcp_connect_url)
- Implementations must enforce HTTPS/loopback at runtime for `target_url`
- T-001 acceptance evidence: mock IdP redirect-rejection and adversarial URL corpus
- T-002 acceptance evidence: redirect-disabled behavior and same-origin-only
  redirect if retained
- O-002 acceptance evidence: protected endpoint injection and secret-free PR jobs

## References

- Supersedes: ADR-003
- Product specification v26: FR-009/017/024 (Mode (c) Keycloak ROPC added in v26), QR-007, SC-012, IR-003/006
- SEC-001 (closed, product-spec-owner), SEC-002 (this remediation)
- Tasks: F-002, T-001, T-002, O-002
- `contracts/repository-config.schema.json`, `contracts/http-adapter.md`
