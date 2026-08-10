# Verification Report: Q-004 — Explicit-Only Authentication Configuration and SEC-002 URL Policy

**Date**: 2026-08-10
**Engineer**: verification-engineer

---

## Status

```
Verification status: CONDITIONALLY VERIFIED
```

One MEDIUM schema-level gap (VER-001) and two MEDIUM secondary findings (VER-002, VER-003) are open.
No BLOCKER or HIGH findings. All six primary acceptance areas satisfy their runtime-enforcement
requirements. Resolution of VER-001 and VER-002 before production release is recommended.

---

## Scope Verified

Feature F-002 implementation in:

- `src/auth/mod.rs` — `AuthMode`, `Credentials`, `select_auth_mode`, `effective_client_id`,
  `build_auth_client`, `login`
- `src/config/mod.rs` — `ValidatedUrl`, `ValidatedAuthUrl`, `RepositoryConfig`,
  `resolve_validated_url`, `resolve_validated_auth_url`, `resolve_config`
- `src/cli/mod.rs` — `Command::Login` struct, `run()` login dispatch

Exclusions: `lint`/`apply` dispatch paths (unimplemented, behind `todo!`), adapter layer,
and any future tasks beyond T-001/T-002 scaffold.

---

## Sources of Truth

| Artifact | Path |
|---|---|
| Architecture plan v25 | `specs/codemie-cicd-tool/plan.md` |
| ADR-011 (URL validation, credential channels) | `specs/codemie-cicd-tool/adr/011-url-validation-and-credential-input.md` |
| CLI contract | `specs/codemie-cicd-tool/contracts/cli.md` |
| HTTP adapter contract | `specs/codemie-cicd-tool/contracts/http-adapter.md` |
| Repository config schema | `specs/codemie-cicd-tool/contracts/repository-config.schema.json` |
| Task definition | `specs/codemie-cicd-tool/tasks.md` (Q-004) |
| Prior Q-001 report | `specs/codemie-cicd-tool/Q-001-verification-report.md` |
| Cargo.toml | `Cargo.toml` |

---

## Change Set Inspected

| File | Relevance |
|---|---|
| `src/auth/mod.rs` | Auth modes, credential resolution, HTTP client construction, login dispatch |
| `src/config/mod.rs` | ValidatedUrl, ValidatedAuthUrl, RepositoryConfig schema enforcement, config resolution |
| `src/cli/mod.rs` | CLI struct (missing flags), env attribute precedence, login handler |
| `src/http/mod.rs` | ApiClient redirect policy, TLS, timeout; HttpClient scaffold |
| `src/error.rs` | Exit code mapping |
| `Cargo.toml` | reqwest feature selection |
| `specs/codemie-cicd-tool/contracts/repository-config.schema.json` | Schema-level URL and credential enforcement |

---

## Validation Performed

```
cargo test --locked     240 passed, 0 failed, 0 ignored   PASS
```

Command run on 2026-08-10 in `/home/matrizaev/codemie-gitops`. Output included 18 dead-code
warnings (all relate to unimplemented adapter stubs, not to the F-002 scope). No test failures.

---

## Acceptance-Criteria Matrix

| Criterion | Status | Code evidence | Test evidence | Notes |
|---|---|---|---|---|
| AC-FR-017-01: `--auth-url` > `CODEMIE_AUTH_URL` > config `auth_url` precedence; invalid selected value is exit 2 before network | SATISFIED | `src/config/mod.rs` `resolve_validated_auth_url` uses `flag.or(config)` then validates; clap `env = "CODEMIE_AUTH_URL"` handles flag > env; `ValidatedAuthUrl::try_from` returns `AppError::Configuration` (exit 2) on failure | `resolve_auth_url_invalid_flag_does_not_fallback_to_config`, `resolve_auth_url_both_none_gives_none`, `resolve_url_flag_used_when_present` | |
| AC-FR-017-01 (url): `--url` > `CODEMIE_URL` > config `url` | SATISFIED | Same pattern via `resolve_validated_url`; clap `env = "CODEMIE_URL"` | `resolve_url_flag_used_when_present`, `resolve_url_config_used_when_flag_absent`, `resolve_url_invalid_flag_does_not_fallback_to_config` | |
| AC-FR-024-01 / SEC-001: `--token`, `--client-secret`, `--password` flags absent; E_USAGE exit 2 when supplied | SATISFIED | `Command::Login` struct defines no secret fields; clap rejects unknown flags at parse time | `cli_does_not_have_token_flag_on_login`, `login_rejects_unknown_token_flag`, `login_rejects_unknown_client_secret_flag`, `login_rejects_unknown_password_flag` | |
| AC-FR-024-01 / SEC-001: secrets resolved from env only; no flag source | SATISFIED | `Credentials::from_env` uses `std::env::var()` exclusively for `CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, `CODEMIE_PASSWORD` | `credentials_from_env_does_not_panic`; non-secret flag tests | |
| AC-FR-024-01 / SEC-001: credential keys rejected from repository config | SATISFIED | `RepositoryConfig` uses `#[serde(deny_unknown_fields)]` | `repository_config_rejects_credential_keys`, `repository_config_rejects_client_secret_key`, `repository_config_rejects_password_key` | |
| AC-FR-024-03 / SEC-002: `auth_url` schema pattern rejects `http://`, userinfo, fragment, C0 controls | SATISFIED | Schema pattern `^https://[^@#\x00-\x1f\x7f]+$` enforces HTTPS, no userinfo, no fragment, rejects C0+DEL | Q-001 N14–N16 probes; `validated_auth_url_rejects_http_loopback`, `_http_non_loopback`, `_http_localhost`, `_userinfo`, `_fragment` | C1 (0x80–0x9F) not covered at schema level; see VER-001 |
| AC-FR-024-03 / SEC-002: runtime `ValidatedUrl` rejects C0/C1 controls, schemes other than http/https, userinfo, fragment, empty host | SATISFIED | `ValidatedUrl::try_from` checks `n <= 0x1F \|\| (0x7F..=0x9F).contains(&n)`, scheme whitelist, URL parse, fragment, authority `@` scan, `host_str()` non-empty, loopback for http | `validated_url_rejects_c0_control_char`, `_c0_tab`, `_c1_control_char`, `_ftp_scheme`, `_userinfo`, `_userinfo_username_only`, `_fragment`, `_http_non_loopback` | |
| AC-FR-024-03 / SEC-002: `ValidatedAuthUrl` rejects http:// unconditionally including loopback | SATISFIED | `ValidatedAuthUrl::try_from` checks scheme == "https" before delegating; no loopback exception | `validated_auth_url_rejects_http_loopback`, `_http_localhost`, `_http_non_loopback` | Correct: spec and ADR-011 §3 require HTTPS for auth_url without loopback exception |
| AC-FR-024-03 / SEC-002: `http://` target URL rejected for non-loopback; `http://127.x.x.x` accepted | SATISFIED | `ValidatedUrl::try_from` calls `is_loopback_http()`; http is accepted only when host is loopback | `validated_url_rejects_http_non_loopback`, `validated_url_accepts_http_loopback_ipv4`, `_ipv4_range`, `_localhost` | |
| AC-FR-024-07 / AC-FR-024-08: five-case auth mode selection, all correct | SATISFIED | `select_auth_mode` cases 1–5 in `src/auth/mod.rs` lines 111–142; see detailed trace below | `select_auth_mode_bearer_token_highest_priority`, `_keycloak_client_credentials`, `_keycloak_ropc`, `_local_auth`, `_client_secret_without_auth_url_is_invalid`, `_no_valid_combination_returns_configuration_error`, `_bearer_token_no_auth_url` | BearerToken (case 1) not in cli.md §6; see VER-OBS-001 |
| ADR-011 §4: redirect disabled for auth POST; 3xx → E_AUTHENTICATION exit 2 | SATISFIED | `build_auth_client()` uses `.redirect(Policy::none())`; `extract_token_from_response` maps `is_redirection()` → `AppError::Authentication` | `login_3xx_response_is_authentication_error`, `login_3xx_on_local_auth_is_authentication_error` | |
| ADR-011 §4: redirect disabled for API transport | SATISFIED | `ApiClient::new()` uses `.redirect(Policy::none())`; `classify_error_status(status, true)` maps 3xx on modifying requests → `AppError::Authentication` | `post_3xx_returns_authentication_error`, `delete_3xx_returns_authentication_error` | |
| SEC-002 / Cargo.toml: rustls-tls enforced, no native-tls | SATISFIED | `reqwest = { default-features = false, features = ["rustls-tls", "json"] }` in `Cargo.toml`; both `build_auth_client` and `ApiClient::new` call `.use_rustls_tls()` | `api_client_constructs_with_loopback_url` (compile-time enforcement) | |
| Error message must not expose credential values (SEC-001 / SEC-005) | SATISFIED | `select_auth_mode_error_message_does_not_contain_credential_values` uses non-empty secrets and asserts error message is fixed | `select_auth_mode_error_message_does_not_contain_credential_values` | |

---

## Requirements Coverage

| Requirement | Status | Evidence | Findings |
|---|---|---|---|
| FR-017: Config resolution precedence (flag > env > config) | SATISFIED | clap `env =` + `flag.or(config)` in `resolve_validated_url`/`resolve_validated_auth_url` | None |
| FR-024 / SEC-001: Forbidden CLI flags cause E_USAGE exit 2 | SATISFIED | clap parse-time rejection; no secret fields in `Command::Login` | None |
| FR-024 / SEC-001: Secrets from env only | SATISFIED | `Credentials::from_env` uses `std::env::var()` exclusively | None |
| FR-024 / SEC-001: Repo config rejects credential keys | SATISFIED | `#[serde(deny_unknown_fields)]` on `RepositoryConfig` | None |
| FR-024 / SEC-002: ValidatedUrl full enforcement | SATISFIED | All checks present in `ValidatedUrl::try_from` | None |
| FR-024 / SEC-002: Schema patterns enforce HTTPS for auth_url | SATISFIED for HTTPS/userinfo/fragment; see note | Schema pattern `^https://...` | VER-001 (C1 not rejected at schema level) |
| FR-024 / SEC-002: auth_url HTTPS unconditional | SATISFIED | `ValidatedAuthUrl::try_from` checks scheme == "https" first | None |
| FR-024 / ADR-011 §4: Redirect policy | SATISFIED | `Policy::none()` on both `build_auth_client` and `ApiClient::new`; 3xx → `AppError::Authentication` | VER-003 (HttpClient scaffold only) |
| TLS enforcement | SATISFIED | `default-features = false, features = ["rustls-tls"]`; `.use_rustls_tls()` on both clients | None |
| IR-006: Five auth mode cases | SATISFIED | `select_auth_mode` cases 1–5 | VER-OBS-001 (BearerToken not in cli.md §6) |
| SEC-003: Per-request timeout 60 s | PARTIALLY SATISFIED | `ApiClient` sets 60 s timeout; `build_auth_client` does not | VER-002 |

---

## Five-Case Auth Mode Trace (AC-FR-024-07/08)

| Case | Guard in `select_auth_mode` | Result | Test |
|---|---|---|---|
| 1. BearerToken | `credentials.bearer_token.is_some()` | `AuthMode::BearerToken`; `login()` returns token directly, no network call | `select_auth_mode_bearer_token_highest_priority`, `login_bearer_token_returns_token_without_network_call` |
| 2. Mode (a) — Keycloak client_credentials | `client_secret.is_some() && auth_url.is_some()` | `AuthMode::KeycloakClientCredentials`; POST `grant_type=client_credentials` to auth_url | `select_auth_mode_keycloak_client_credentials`, `login_mode_a_sends_correct_body_and_returns_token` |
| 3. Mode (c) — Keycloak ROPC (v26) | `client_secret.is_none() && email.is_some() && password.is_some() && auth_url.is_some()` | `AuthMode::KeycloakRopc`; POST `grant_type=password`, NO `client_secret`, client_id defaults to `codemie-sdk` | `select_auth_mode_keycloak_ropc`, `login_mode_c_sends_correct_body_no_client_secret`, `effective_client_id_mode_c_defaults_to_codemie_sdk` |
| 4. Mode (b) — Local-auth | `client_secret.is_none() && email.is_some() && password.is_some() && auth_url.is_none()` | `AuthMode::LocalAuth`; POST JSON to `{url}/v1/local-auth/login` | `select_auth_mode_local_auth`, `login_mode_b_sends_correct_json_body_and_returns_token` |
| 5. Unmatched | All conditions false (e.g. client_secret+no auth_url; nothing set) | `AppError::Configuration` exit 2; no network call | `select_auth_mode_client_secret_without_auth_url_is_invalid`, `select_auth_mode_no_valid_combination_returns_configuration_error` |

The Mode (c) body is exactly `grant_type=password&client_id={id}&username={email}&password={password}`.
The `client_secret` field is absent. Test `login_mode_c_sends_correct_body_no_client_secret` uses
`mockito::Matcher::Exact(...)` to assert the exact form body, proving the omission is enforced.

---

## Architecture Conformance

**Conforming elements**

- Config resolution uses `flag.or(config)` as specified in plan.md §7.
- `ValidatedUrl` and `ValidatedAuthUrl` are distinct types per plan.md §11.
- `#[serde(deny_unknown_fields)]` on `RepositoryConfig` enforces closed schema per ADR-011 §1.
- `reqwest` TLS uses rustls-only feature selection; no OpenSSL runtime dependency.
- `ApiClient` satisfies ADR-011 §4 redirect policy.
- `build_auth_client` satisfies ADR-011 §4 redirect policy for auth POSTs.
- Authentication errors use fixed messages; credential values do not appear in error text (SEC-005).
- `exit_code()` on `AppError` maps `Reconciliation` → 1, all others → 2 (contracts/cli.md §7).

**Undocumented deviation**

- `select_auth_mode` adds `BearerToken` as a 4th highest-priority case. cli.md §6 says "exactly one of three modes"; ADR-011 §1b says "any combination outside these three cases is E_CONFIGURATION". The BearerToken case is explicitly tested in Q-004 acceptance evidence, so this is expected behavior, but the contracts do not document it. See VER-OBS-001.

**Approved deviations**

None recorded.

**Violations**

- `HttpClient` scaffold (src/http/mod.rs lines 60–78) does not call `.redirect(Policy::none())`. See VER-003.

---

## Test Quality Assessment

**Coverage relevance**: High. Tests in all three modules use a combination of clap `try_parse_from`,
serde deserialization, direct function calls, and mockito mock HTTP server tests. Auth mode tests
cover all five cases including edge cases (bearer+secret simultaneous, client_secret without
auth_url).

**Negative paths**: Well covered. `login_3xx_response_is_authentication_error`,
`login_4xx_response_is_authentication_error`, `login_5xx_response_is_authentication_error`,
`login_connection_error_is_connectivity_error`, `validated_url_rejects_*` series.

**Boundaries**: `is_loopback_http_true_for_127_range` tests the full 127.x.x.x range. C0, DEL,
and C1 character rejection each have dedicated tests.

**Integration realism**: mockito tests verify exact HTTP request bodies (especially Mode (c)
`Matcher::Exact`). This is strong evidence that no `client_secret` field is sent in ROPC
requests.

**Flakiness**: None observed. All 240 tests passed deterministically.

**Gap — Mode (c) 3xx redirect test**: There is no dedicated test for a 3xx response to a
Keycloak ROPC POST. The function-level coverage via `login_3xx_response_is_authentication_error`
(Mode a) covers the same `extract_token_from_response` path, but an explicit Mode (c) 3xx
test would be ideal. This is LOW severity because the code path is identical.

**Gap — `HttpClient` redirect behavior**: `http_client_constructs` only verifies that the
struct builds without error. It does not verify redirect behavior. Given the scaffold nature
of `HttpClient`, this is acceptable for the current scope.

---

## Data and Migration Assessment

Not applicable. F-002 introduces no data model changes, no database migrations, and no
persistent state beyond process-level environment variable reads.

---

## Security Assessment

**Authentication credential isolation**: Secret credentials are resolved exclusively from
environment variables. No flag source, no config file source. Enforced structurally by the
absence of secret fields in `Command::Login` and `RepositoryConfig`.

**URL injection prevention**: Both `ValidatedUrl` and `ValidatedAuthUrl` enforce the full
ADR-011 §2 character and scheme rules before any URL value reaches the network. The schema
patterns provide defense-in-depth for values sourced from repository config, with the caveat
that C1 controls (0x80–0x9F) are only enforced at the runtime layer (see VER-001).

**Credential replay via redirect**: Disabled at the reqwest client level for both auth
(`build_auth_client`) and API (`ApiClient`) clients. 3xx responses on auth POSTs are
classified as `AppError::Authentication` before the token is extracted, preventing credential
forwarding.

**TLS enforcement**: `default-features = false, features = ["rustls-tls"]` in Cargo.toml
is the only accepted build configuration. Both HTTP clients call `.use_rustls_tls()`.

**Unverified area**: No dynamic test of TLS certificate validation behavior (e.g., self-signed
cert rejection). This is a gap in evidence but not a code gap; rustls defaults enforce
certificate validation.

---

## Operational Assessment

**Tracing**: The `select_auth_mode` error message does not expose credential values (verified
by dedicated test). No credential values appear in fixed error strings throughout `auth/mod.rs`
and `config/mod.rs`.

**Logging**: `tracing_subscriber` is initialized in `main.rs` before dispatch. Structured
logging fields are present in later modules; the auth module uses fixed string errors without
structured fields. No audit of `CODEMIE_TOKEN` use is emitted (acceptable for current scope).

**Rollback**: Not applicable for a CLI tool without persistent server-side state in this scope.

**Timeout**: See VER-002. Auth client lacks per-request timeout. This is an operational concern
for the `login` subcommand in environments with network issues.

---

## Findings

### VER-001

```
Finding ID: VER-001
Severity: MEDIUM
Status: OPEN

Title:
Schema patterns do not reject C1 control characters (U+0080–U+009F)

Evidence:
- specs/codemie-cicd-tool/contracts/repository-config.schema.json
  Pattern for url:      ^https?://[^@#\x00-\x1f\x7f]+$
  Pattern for auth_url: ^https://[^@#\x00-\x1f\x7f]+$
- Character class [^\x00-\x1f\x7f] excludes C0 (0x00–0x1F) and DEL (0x7F)
  but does NOT exclude C1 range 0x80–0x9F
- Runtime ValidatedUrl::try_from (src/config/mod.rs): checks
  n <= 0x1F || (0x7F..=0x9F).contains(&n), correctly covering the full
  ADR-011 §2 requirement

Expected:
Q-004 acceptance evidence: "url and auth_url schema patterns reject @, #, and
C0/C1 control characters"
ADR-011 §2: "No C0/C1 control characters (U+0000–U+001F, U+007F–U+009F)"

Actual:
A repository config YAML value such as url: "https://example.com/"
would pass JSON schema validation but fail at runtime ValidatedUrl::try_from.
The runtime boundary holds; the schema layer is incomplete.

Impact:
Defense-in-depth gap. The authoritative runtime enforcement is correct.
A consumer relying solely on schema validation for input sanitization would
miss C1 control characters. The Q-004 acceptance criterion is not fully
satisfied at the schema level.

Required action:
Update both url and auth_url patterns in
specs/codemie-cicd-tool/contracts/repository-config.schema.json to:
  [^@#\x00-\x1f\x7f-\x9f]
or equivalently:
  [^@#\x00-\x9f]
This aligns the schema layer with the runtime enforcement already present in
ValidatedUrl::try_from.

Owner:
solution-architect

Verification:
1. Update schema pattern.
2. Re-run Q-001 probe N13 family with a C1 character in url value and confirm
   schema validation fails.
3. Confirm runtime test validated_url_rejects_c1_control_char still passes.
```

### VER-002

```
Finding ID: VER-002
Severity: MEDIUM
Status: OPEN

Title:
Authentication POST requests lack the 60-second per-request timeout

Evidence:
- src/auth/mod.rs build_auth_client() (lines 175–181):
  reqwest::Client::builder()
      .use_rustls_tls()
      .redirect(reqwest::redirect::Policy::none())
      .build()  // No .timeout() call
- src/http/mod.rs ApiClient::new(): correctly sets
  .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS)) where REQUEST_TIMEOUT_SECS = 60
- specs/codemie-cicd-tool/contracts/http-adapter.md §2.4:
  "Total request timeout: 60 seconds — Per individual HTTP request"

Expected:
All HTTP requests, including authentication POSTs (Modes a, b, c), must observe
a 60-second per-request timeout per SEC-003 and http-adapter.md §2.4.

Actual:
build_auth_client() builds a reqwest::Client without a timeout. Authentication
POST requests in login() for Modes a, b, c can block indefinitely on a slow or
stalling authentication endpoint.

Impact:
The login subcommand may hang indefinitely in environments with network issues,
consuming process resources and preventing operator intervention via normal exit
paths. This is an operational correctness gap; it does not affect security
invariants.

Required action:
Add .timeout(Duration::from_secs(60)) to the build_auth_client builder chain.
Consider sharing the REQUEST_TIMEOUT_SECS constant from http/mod.rs.

Owner:
implementation-engineer

Verification:
1. Add a mockito test that delays the response past the timeout and asserts
   AppError::Connectivity is returned within the expected window.
2. Confirm auth client construction and timeout behavior match ApiClient.
```

### VER-003

```
Finding ID: VER-003
Severity: MEDIUM
Status: OPEN

Title:
HttpClient scaffold does not disable redirects (ADR-011 §4)

Evidence:
- src/http/mod.rs HttpClient::new() (lines 64–71):
  reqwest::Client::builder()
      .use_rustls_tls()
      .build()  // No .redirect(Policy::none()) call
- ADR-011 §4: "Preferred: disable all redirects for simplicity"
- ApiClient::new() correctly includes .redirect(reqwest::redirect::Policy::none())
- Comment in HttpClient: "scaffold placeholder from before T-002;
  for authenticated API calls use ApiClient below"

Expected:
All exported HTTP client types used in authenticated API contexts must disable
redirects per ADR-011 §4. The HttpClient type is pub and could be misused by
future implementation tasks.

Actual:
HttpClient::new() builds a client that will follow redirects by default (reqwest
default policy). While no production call sites of HttpClient are visible outside
tests, the type's public visibility means it is available to any future code in
the crate.

Impact:
Latent policy violation. No active code path misuses HttpClient today. If a
future task uses HttpClient instead of ApiClient for authenticated calls, it
would silently allow redirect following. The scaffold comment discourages this,
but there is no structural enforcement.

Required action:
Either:
(a) Add .redirect(reqwest::redirect::Policy::none()) to HttpClient::new(),
    matching ApiClient; or
(b) Restrict HttpClient visibility (pub(crate) or remove) if no external
    consumers exist; or
(c) Add a compile-time doc comment prohibiting use for authenticated calls
    and confirm no callers exist.

Owner:
implementation-engineer

Verification:
Confirm HttpClient::new includes redirect policy, or confirm no pub callers
outside tests remain. Add redirect behavior test matching ApiClient pattern.
```

### VER-OBS-001

```
Finding ID: VER-OBS-001
Severity: NOTE
Status: OPEN

Title:
cli.md §6 and ADR-011 §1b do not document the BearerToken auth mode

Evidence:
- src/auth/mod.rs select_auth_mode: case 1 (line 111) returns BearerToken when
  CODEMIE_TOKEN is set, at highest priority, before any network call
- src/auth/mod.rs login(): BearerToken arm returns credentials.bearer_token
  without network access (confirmed by login_bearer_token_returns_token_without_network_call)
- specs/codemie-cicd-tool/contracts/cli.md §6: "Exactly one of three modes is
  selected before any network access: (a), (c), (b)"
- specs/codemie-cicd-tool/adr/011-url-validation-and-credential-input.md §1b:
  "Any combination outside these three cases is E_CONFIGURATION, exit 2"
- specs/codemie-cicd-tool/tasks.md Q-004 acceptance evidence explicitly lists:
  "BearerToken: CODEMIE_TOKEN set → BearerToken, no network call"

Impact:
The behavior is explicitly required by the Q-004 task. However, the normative
contracts (cli.md §6, ADR-011 §1b) describe exactly three modes and classify
all others as E_CONFIGURATION. This inconsistency can mislead future engineers
consulting the contracts.

Required action:
solution-architect to update cli.md §6 and ADR-011 §1b to document BearerToken
as a fourth pre-network case with highest selection priority.

Owner:
solution-architect
```

### VER-OBS-002

```
Finding ID: VER-OBS-002
Severity: NOTE
Status: OPEN (carries forward Q-001 OBS-002)

Title:
adapter-manifest authUrlRequiredFor does not list Mode (c) ROPC

Evidence:
- specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json:
  "authUrlRequiredFor": "keycloak_client_credentials"
- Mode (c) KeycloakRopc also requires auth_url (line 124 in select_auth_mode)

Impact:
No correctness gap; cli.md §6 is normative. The manifest field is
informational. A consumer reading only the manifest would not know that
Mode (c) also requires auth_url.

Required action:
solution-architect to update authUrlRequiredFor to name both modes, e.g.
"keycloak_client_credentials,keycloak_ropc" or an array.

Owner:
solution-architect
```

---

## Unverified Areas

- **Dynamic TLS certificate validation**: No test verifies that self-signed or expired
  certificates are rejected. This is rustls default behavior and outside Q-004 scope,
  but is not directly evidenced.

- **CODEMIE_AUTH_URL env-only path**: The clap `env = "CODEMIE_AUTH_URL"` attribute
  guarantees flag > env precedence, but no test exercises the case where `CODEMIE_AUTH_URL`
  is set in the environment and `--auth-url` is absent. The mechanism is clap's own `env=`
  implementation; it is reliable, but there is no integration test for this exact path.

- **Mode (c) 3xx redirect behavior**: Covered transitively through shared
  `extract_token_from_response` path tested by Mode (a) redirect test, but no dedicated
  Mode (c) 3xx test exists.

- **Timeout on auth client**: Not tested (VER-002). Absence of `.timeout()` verified by
  code inspection; the negative case (no hang) is untestable without a deliberate stall server.

---

## Files Changed During Verification

None. This report was written as a new file; no implementation or test files were modified.

---

## Release-Review Focus

The following items require attention before production release:

1. **VER-002 (auth client timeout)**: The `login` subcommand can hang indefinitely on a
   stalling authentication endpoint. Fix before release.

2. **VER-001 (schema C1 gap)**: A YAML config file with C1 control characters in a URL value
   passes schema validation but fails at runtime. The security boundary holds but the schema
   layer should be corrected to match ADR-011 §2 exactly.

3. **VER-003 (HttpClient redirect)**: Remediate or restrict before any future task uses
   `HttpClient` for authenticated calls. Confirm no production call sites exist.

4. **VER-OBS-001 (cli.md §6 documentation)**: Update before Q-005 or any task that generates
   contract documentation, so engineers and tooling have a consistent view of auth modes.

5. **Deployment ordering**: No deployment concerns for this CLI binary.

6. **Environment-specific behavior**: `CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
   `CODEMIE_PASSWORD` must be set as environment variables in the CI runtime. No default
   values exist; missing env vars produce `E_CONFIGURATION`.

---

## Final Verdict Rationale

All six primary acceptance areas — config resolution precedence, credential source restriction,
URL validation, auth mode selection, redirect policy, and TLS enforcement — are implemented
correctly and supported by 240 passing tests. The runtime security boundaries (ValidatedUrl,
ValidatedAuthUrl, build_auth_client redirect policy, ApiClient redirect policy, clap parse-time
flag rejection, env-only secret resolution, deny_unknown_fields schema closure) are all present
and verified by dedicated tests.

The CONDITIONALLY VERIFIED status reflects three MEDIUM findings:

- VER-001: The schema patterns incompletely enforce C1 control character rejection.
  Runtime enforcement is correct and the security boundary holds, but the acceptance
  criterion for schema-level enforcement is not fully met.

- VER-002: Authentication POST requests can hang indefinitely; the 60-second timeout
  required by SEC-003 and http-adapter.md §2.4 is missing from build_auth_client.

- VER-003: The HttpClient scaffold type does not disable redirects, violating ADR-011 §4
  in any context where it is used for authenticated API calls. No active production caller
  exists, but the latent violation should be resolved.

None of these findings affect the correctness of the core authentication security properties
(no credential exposure, no redirect following during auth, HTTPS enforcement for auth URLs,
TLS-only transport). The conditions for clearing the CONDITIONALLY VERIFIED status are listed
in the Release-Review Focus section.
