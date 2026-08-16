# ADR-0009: Environment-only secrets, ValidatedUrl, TLS, and redirect policy

## Status

Accepted (originally ADR-011 superseding the credential/URL portions of
ADR-003; current behavior).

## Context

A pre-implementation security review found that secret credentials supplied via
CLI flags appear in `argv`, which is readable by process monitors, shell
traces, CI diagnostics, and same-host processes; and that URL/transport policy
was incomplete (remote HTTP, URL userinfo, unspecified redirects for
credential POSTs).

## Decision

- **Credentials are environment-only**: bearer token from `CODEMIE_TOKEN`,
  client secret from `CODEMIE_CLIENT_SECRET`, password from
  `CODEMIE_PASSWORD`. The flags `--token`, `--client-secret`, and
  `--password` do not exist; supplying them is `E_USAGE` exit 2 before
  any network access. Non-secret selectors (`--client-id`,
  `--email`) retain flag-over-environment precedence. There is no
  repository configuration file and no derived credential value.
- **ValidatedUrl**: URLs must be absolute `http`/`https`, contain no
  userinfo, fragment, or control characters, and have a valid non-empty host
  and decimal port. The type governs target/auth URLs and every URL-valued
  authorable field that reaches the server (Datasource `link`,
  `site_url`, MCP `mcp_connect_url`, ...); userinfo is rejected at
  schema validation time.
- **TLS**: `auth_url` requires HTTPS unconditionally. `target_url`
  requires HTTPS for remote hosts; HTTP is permitted only when the resolved
  destination is loopback (127.0.0.0/8, ::1, or `localhost` confirmed by
  resolution). No `--insecure` flag or TLS-verification bypass exists.
- **Redirects are disabled** for all authentication POSTs (Keycloak
  client-credentials/ROPC and local-auth). API calls also prefer redirects
  disabled; if retained, any redirect must be method-aware (never POST/PUT to
  GET), same-origin, bounded to 3 hops, and never replay credentials to a
  different authority.
- **Login modes** are selected before network access: bearer-token reuse (no
  auth request), Keycloak client-credentials (secret + explicit HTTPS
  `auth_url`; client ID optional), Keycloak ROPC (email/password + HTTPS
  `auth_url`; client ID defaults to `codemie-sdk`), or local-auth
  (email/password without `auth_url`, development only). Any other
  combination is `E_CONFIGURATION` exit 2 before network access.

## Consequences

- Secret credentials cannot appear in process argument vectors or shell
  history; URL userinfo cannot smuggle credentials into Git.
- Authentication POST redirects cannot exfiltrate credentials.
- The loopback HTTP exception is mechanically narrow and runtime-enforced.
- CI examples require protected endpoint injection with secret-free PR jobs.

## Alternatives considered

- Flag-based secrets with masking guidance: rejected (argv still exposes them).
- Accepting and deprecating secret flags: rejected (a deprecated flag still
  places the value in argv).
- Deriving/discovering the Keycloak endpoint: rejected (not a portable
  discovery contract).
- Same-origin redirects on auth POSTs: rejected (one-shot credential delivery;
  disabling redirects entirely is simpler and more secure).

## References

- [cli.md](../../contracts/cli.md)
- [http-adapter.md](../../contracts/http-adapter.md)
- `src/auth.rs`, `src/config.rs`, `src/http.rs`
- Related: ADR-0002
