# ADR-003: Keep authentication stateless and make failure output allowlist-only

## Status

Superseded by ADR-011

This document is historical rationale only. The core stateless-auth and
allowlist-diagnostics decisions remain in effect. The credential input
channel (flag-vs-environment) and URL/TLS/redirect policy are superseded
by ADR-011 (v25, SEC-001/SEC-002 remediation).

No derived-endpoint branch, credential-bearing flag, or URL without
userinfo/TLS validation from this ADR is executable. Product specification
v25 and ADR-011 govern those boundaries.

## Context

The CLI runs locally and in CI, accepts bearer credentials or obtains them via
Keycloak/local-auth, and calls the CodeMie REST API directly. FR-011/016/024/026
and QR-007 require stdout to contain successful outcomes only, all failures on
stderr, and no raw body/server text/payload/secret-like data in diagnostics or
persistent artifacts.

## Decision drivers

- CI composability
- No token cache or client state
- Least privilege and predictable credential precedence
- One enforceable output boundary covering ordinary, debug, panic, and fatal
  paths
- Useful correlation without arbitrary redaction

## Options considered

### A. Persist a CLI session/token cache

Rejected: introduces secret lifecycle/state outside the specification.

### B. Format server errors then redact known secrets

Rejected: arbitrary response text and payloads cannot be reliably classified or
redacted.

### C. Stateless credentials plus allowlist-constructed diagnostics

Selected. Success and failure use different output types and streams.

### D. Derive or discover the Keycloak endpoint from the CodeMie API URL

Rejected. The observed deployment shape is not a portable discovery contract,
and product specification v24 expressly forbids derivation and probing.

### E. Resolve an explicit endpoint from a closed precedence chain

Selected. A non-secret explicit endpoint is reproducible without coupling API
and identity-provider topology.

## Decision

Credentials come only from approved flags/environment sources. The CLI stores
no token, cookie, request/response body, or declaration/sidecar copy. Successful
`login` writes the token once to stdout and is the sole sensitive-output
exception.

For Keycloak, the token endpoint is explicit configuration resolved exactly as
`--auth-url` > `CODEMIE_AUTH_URL` > repository config `auth_url`. The selected
value is validated and contacted as-is. The CLI never derives or probes an
endpoint from `--url`, `CODEMIE_URL`, repository config `url`, a hostname,
realm, path convention, or another value. A missing endpoint or invalid higher-precedence endpoint is exit 2 before
network access. ~~Client ID, client secret, bearer token, email, and password
resolve only from their flag-over-environment pairs and are forbidden in
repository config.~~ (Superseded by ADR-011: secret credentials resolve from
environment only; secret-value flags are not accepted; see ADR-011 for the
normative credential-input and URL/TLS/redirect policy.)

All failures leave stdout empty. The transport converts failures to an internal
classification containing only HTTP status/method/route template, a locally
generated request ID, and a validated dedicated server
correlation ID. It discards response bodies without formatting them. The
diagnostic builder adds only stable error code/category/exit code and safe local
source coordinates. JSON diagnostics conform to
`contracts/diagnostic.schema.json`; successful outcomes conform separately to
`contracts/outcome.schema.json`.

No debug, verbose, trace, panic, or internal-error facility may bypass this
builder or emit request payloads, server text, arbitrary headers, credentials,
or secret-like values.

## Consequences

### Positive

- A small allowlist is auditable and testable.
- CI can treat stdout as successful machine output and stderr as failure only.
- Server-generated error content cannot exfiltrate declaration or secret data.

### Negative

- Diagnostics deliberately omit server explanations.
- Operators may need the validated correlation ID and server-side logs.
- Login remains intentionally pipe-sensitive.

### Risks

- Third-party HTTP/debug middleware may log bodies unless disabled.
- Panic formatting can leak values unless converted to fixed diagnostics.
- URL handling can leak userinfo/path if transport errors bypass typed
  diagnostics.

## Follow-up actions

- Security-review transport/middleware and every diagnostic-producing path.
- Add negative canary tests across text/JSON/debug/panic/login failures.
- Ensure bounded body draining and zero tool-created logs containing bodies.
- Document shell-safe login piping and CI secret masking.
- Test endpoint precedence, missing-endpoint no-network behavior, exact URL
  use, rejection of credential keys in repository config, and mixed-mode
  rejection.

## References

- Product specification v24: FR-009/011/014/016/017/024/026, DR-006/009,
  QR-007; AC-FR-017-01/02 and AC-FR-024-07
- ADR-010
- ADR-011 (supersedes this document for credential input and URL/TLS/redirect
  policy; v25 / SEC-001 / SEC-002 remediation)
