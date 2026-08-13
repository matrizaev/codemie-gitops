# Security Architecture Review: Q-005

**Status**: APPROVED FOR NEXT STAGE
**Date**: 2026-08-10
**Reviewer**: security-reviewer agent
**Review mode**: Mode C — Implementation security review

---

## Verdict

All six security boundaries reviewed contain no CRITICAL, HIGH, or MEDIUM findings.
Three NOTE-level observations are recorded but none block progression. Features
F-006, T-003, W-001, S-001, D-001, A-001, and R-001 may proceed to implementation
under the existing architecture. Two items from prior verification rounds are
carried forward unchanged (Q-002 VER-001, Q-004 VER-001). The single supply-chain
gap (NOTE: no `--locked` at the Cargo config level) should be addressed before
the first release build but does not block development-stage tasks.

---

## Boundary Evidence

### 1. Input (SEC-003)

All four YAML resource budgets are present as named public constants and are
asserted by constant-value tests: `MAX_YAML_FILE_BYTES = 1 MiB`
(`src/parse/mod.rs:36`), `MAX_YAML_DEPTH = 32` (line 39), `MAX_YAML_SCALAR_BYTES
= 128 KiB` (line 42), `MAX_YAML_COLLECTION_MEMBERS = 10 000` (line 46). Budget
enforcement is ordered correctly: byte limit fires before AST allocation; depth,
scalar, and collection limits fire during the tree walk after parsing. The
injection scanner uses two independent layers — a pre-parse raw-byte scan
(anchors, aliases, tags) plus a post-parse `check_yaml_tree` tree walk (tagged
values, merge keys) — providing defense in depth against YAML deserialization
attacks (ADR-001). Quoted-string boundaries are correctly tracked in the scanner
so that `&`/`*`/`!` inside string literals are not misclassified. The only
documented false-positive path (block-scalar content containing `&word`) is
acceptable given the closed v1alpha1 schema.

### 2. Authentication (SEC-001)

`src/cli/mod.rs` defines no `--token`, `--client-secret`, or `--password` flags;
six dedicated clap rejection tests confirm that supplying these flags at
parse-time exits with code 2. `Credentials::from_env()` reads all secret fields
(`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`, `CODEMIE_PASSWORD`) from environment
variables only. `build_auth_client()` sets `redirect::Policy::none()`,
`connect_timeout(10 s)`, and `timeout(60 s)` with rustls TLS. Mock tests confirm
that a 3xx response on any auth POST is mapped to `AppError::Authentication`
rather than being followed. The Mode (c) ROPC body is verified by an exact
`Matcher::Exact` mock that fails if any field other than `grant_type`, `client_id`,
`username`, and `password` appears — confirming the absence of `client_secret`.
Auth-URL HTTPS enforcement is applied at `ValidatedAuthUrl::try_from()` before any
network call; however the `login()` function accepts `Option<&str>` rather than
`Option<&ValidatedAuthUrl>`, meaning the guarantee is not expressed at the type
boundary (see SEC-Q005-002).

### 3. Transport (SEC-002)

`ApiClient::new()` sets `redirect::Policy::none()`, `timeout(60 s)`, and rustls.
The `bounded_body()` helper enforces an 8 MiB body limit both via early
`Content-Length` rejection and a post-read byte-count check. JSON depth is bounded
at 64 levels by `json_max_depth`. GET retries up to three times on 5xx/429/connect
error; POST, PUT, and DELETE are never retried. Bearer tokens are placed only in
the `Authorization` header and are never logged (`SEC-001`/`SEC-005`). The legacy
`HttpClient` scaffold also sets `redirect::Policy::none()` and rustls but lacks
any timeout (see SEC-Q005-001); all live call sites in the current codebase use
`ApiClient`.

### 4. Output (SEC-005)

`src/render/mod.rs` uses closed Rust enums for every output category:
`EntityKind`, `EntityKey`, `ErrorCode`, `DiagnosticCategory`, `WarningCode`,
`WarningCategory`, and `HttpMethod`. The `diagnostic_from_app_error()` mapper
discards the `AppError` message string entirely and emits only the closed `as_str()`
code. Two injection tests confirm: (a) `AppError::Usage("malicious\nnewline …")`
produces exactly `"error: E_USAGE\n"` in text mode; (b) a config error whose
message contains `http://evil.com` does not appear anywhere in JSON output. All
JSON records are produced by `serde_json`, never by string concatenation. Stdout
and stderr separation is enforced at the `Renderer` boundary and tested.

### 5. Privileged Resolution (SEC-004/ADR-012)

`src/discovery/mod.rs` mitigates the TOCTOU race via `load_sidecar_file`: the
file is opened once and `File::metadata()` is called on the same open file
descriptor before reading, avoiding a stat-then-open window. Path containment is
enforced by `check_containment`, which calls `fs::canonicalize` (resolving all
symlinks) and then verifies `starts_with(canonical_root)`. Symlink rejection
(`check_no_symlink_in_path`) walks every `Component::Normal` prefix with
`symlink_metadata` and returns an error on the first symlink component, so
symlinks are caught before canonicalization. When `--follow-symlinks` is set, the
canonical target is still subject to the containment check; escape attempts return
`AppError::Schema`. Discovery caps files at 10 000 before further processing.
Sidecar aggregate cap (128 MiB) and per-file cap (32 MiB) are constants tested to
their specified values.

### 6. Supply Chain (SEC-006)

`Cargo.lock` is present at the repository root. The `.cargo/config.toml` sets the
default build target to `x86_64-unknown-linux-musl` and a fallback C compiler
environment variable, but contains no `build.locked = true` or equivalent
directive (see SEC-Q005-003). Rustls is used exclusively; no OpenSSL runtime
dependency is introduced. The musl static build target reduces the attack surface
of the distributed binary. No credentials, tokens, or hard-coded secrets were
found in the reviewed source files or the Cargo configuration.

---

## Findings

| ID | Severity | Area | Description | Owner |
|---|---|---|---|---|
| SEC-Q005-001 | NOTE | Transport | `HttpClient` legacy scaffold omits `.timeout()` and `.connect_timeout()`. Currently unreachable from live code (all callers use `ApiClient`), but creates a hang risk if used for future real requests. | implementation-engineer |
| SEC-Q005-002 | NOTE | Authentication | `login(…, auth_url: Option<&str>)` accepts raw strings, so HTTPS enforcement for auth URLs depends solely on `ValidatedAuthUrl` being used upstream. The type boundary does not express the invariant. Future call sites that bypass config resolution could send credentials over HTTP. | implementation-engineer |
| SEC-Q005-003 | NOTE | Supply Chain | `.cargo/config.toml` does not set `build.locked = true` or equivalent. `Cargo.lock` is committed, but `cargo build` without `--locked` will not error if Cargo.lock drifts. CI release builds must pass `--locked` explicitly. | release-engineer |

---

## Open Items (Carried Forward)

| ID | Severity | Description | Owner | Blocks |
|---|---|---|---|---|
| Q-002 VER-001 | MEDIUM | `{scope}` placeholder in manifest Workflow enumerate URL | solution-architect | W-001 |
| Q-004 VER-001 | NOTE | Schema patterns for `url`/`auth_url` missing C1 range `\x80-\x9f`; runtime `ValidatedUrl`/`ValidatedAuthUrl` covers it | solution-architect | none |
