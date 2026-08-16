/// Authentication and login stub.
///
/// Implements the three login modes from contracts/cli.md §6 and ADR-011:
///
/// - Mode (a) — Keycloak `client_credentials`: `CODEMIE_CLIENT_SECRET` set
///   and `auth_url` configured. Sends `grant_type=client_credentials` to
///   the selected `auth_url`.
/// - Mode (c) — Keycloak ROPC (v26): `CODEMIE_CLIENT_SECRET` not set,
///   `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` set, `auth_url` configured. Sends
///   `grant_type=password`; no `client_secret` field.
/// - Mode (b) — Local-auth: `CODEMIE_CLIENT_SECRET` not set,
///   `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` set, `auth_url` **not** configured.
///   Posts to `{url}/v1/local-auth/login`.
///
/// Security invariants (SEC-001/ADR-011):
/// - Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
///   `CODEMIE_PASSWORD`) are resolved from environment only; no flag source.
/// - `--token`, `--client-secret`, `--password` flags do not exist in the CLI.
/// - Redirect is disabled for all authentication POST requests; a 3xx response
///   is `E_AUTHENTICATION`, exit 2.
/// - `auth_url` with `http://` scheme is `E_CONFIGURATION`, exit 2, before any
///   network call.
///
/// Full authentication implementation is in T-001.
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[cfg(test)]
use crate::config::ValidatedAuthUrl;
use crate::error::AppError;
use crate::http::ensure_rustls_provider;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthError {
    #[error("failed to build authentication client")]
    ClientBuild(#[source] reqwest::Error),
    #[error("authentication endpoint is unreachable")]
    Dispatch(#[source] reqwest::Error),
    #[error("failed to read authentication response")]
    ResponseBody(#[source] reqwest::Error),
    #[error("authentication response does not match the token contract")]
    TokenResponse(#[source] serde_json::Error),
}

impl AuthError {
    pub(crate) fn is_connectivity(&self) -> bool {
        matches!(self, Self::Dispatch(_) | Self::ResponseBody(_))
    }
}

// ---------------------------------------------------------------------------
// Token response
// ---------------------------------------------------------------------------

/// Shared response shape for Keycloak token endpoint and local-auth login.
///
/// Only `access_token` is extracted; other server-supplied fields are ignored.
/// This prevents accidentally forwarding server values into the error chain or
/// output (SEC-005).
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// The selected authentication mode, determined before any network access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    /// Mode (a): Keycloak client_credentials.
    KeycloakClientCredentials,
    /// Mode (b): Local-auth (email + password, no auth_url).
    LocalAuth,
    /// Mode (c): Keycloak ROPC password grant (v26).
    KeycloakRopc,
    /// Pre-existing bearer token from `CODEMIE_TOKEN`.
    BearerToken,
}

/// The resolved set of credentials for one invocation.
///
/// Secret fields are `Option<String>` populated only from environment variables
/// at runtime; they are never stored in repository config or passed as CLI flags.
#[derive(Debug)]
pub(crate) struct SecretValue(SecretString);

impl SecretValue {
    fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl From<String> for SecretValue {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&str> for SecretValue {
    fn from(value: &str) -> Self {
        Self(value.to_owned().into())
    }
}

#[derive(Debug)]
pub struct Credentials {
    /// Non-null when `CODEMIE_TOKEN` is set.
    bearer_token: Option<SecretValue>,
    /// Non-null when `CODEMIE_CLIENT_SECRET` is set.
    client_secret: Option<SecretValue>,
    /// Non-null when `CODEMIE_PASSWORD` is set.
    password: Option<SecretValue>,
    /// Non-secret; resolved from `--client-id` > `CODEMIE_CLIENT_ID`.
    client_id: Option<String>,
    /// Non-secret; resolved from `--email` > `CODEMIE_EMAIL`.
    email: Option<String>,
}

/// Untrusted authentication selection assembled at the CLI boundary.
pub(crate) struct RawAuthSelection {
    pub(crate) credentials: Credentials,
    pub(crate) auth_url_configured: bool,
}

/// Validated authentication strategy owning exactly the credentials used by
/// its selected flow.
#[derive(Debug)]
pub(crate) enum AuthStrategy {
    BearerToken {
        token: SecretValue,
    },
    KeycloakClientCredentials {
        client_id: Option<String>,
        client_secret: SecretValue,
    },
    KeycloakRopc {
        client_id: String,
        email: String,
        password: SecretValue,
    },
    LocalAuth {
        email: String,
        password: SecretValue,
    },
}

impl TryFrom<RawAuthSelection> for AuthStrategy {
    type Error = AppError;

    fn try_from(raw: RawAuthSelection) -> Result<Self, Self::Error> {
        let Credentials {
            bearer_token,
            client_secret,
            password,
            client_id,
            email,
        } = raw.credentials;
        if let Some(token) = bearer_token {
            return Ok(Self::BearerToken { token });
        }
        if let Some(client_secret) = client_secret {
            return if raw.auth_url_configured {
                Ok(Self::KeycloakClientCredentials {
                    client_id,
                    client_secret,
                })
            } else {
                Err(AppError::Configuration(
                    "authentication configuration incomplete".into(),
                ))
            };
        }
        match (email, password, raw.auth_url_configured) {
            (Some(email), Some(password), true) => Ok(Self::KeycloakRopc {
                client_id: client_id.unwrap_or_else(|| "codemie-sdk".to_owned()),
                email,
                password,
            }),
            (Some(email), Some(password), false) => Ok(Self::LocalAuth { email, password }),
            _ => Err(AppError::Configuration(
                "authentication configuration incomplete".into(),
            )),
        }
    }
}

impl Credentials {
    /// Load credentials from the current process environment.
    ///
    /// Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
    /// `CODEMIE_PASSWORD`) are read from environment only. Non-secret selectors
    /// (`client_id`, `email`) are passed in because they may be overridden by
    /// CLI flags (flag > env precedence is handled by clap).
    /// Load credentials from the current process environment.
    ///
    /// Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
    /// `CODEMIE_PASSWORD`) are read from environment only. A secret that is
    /// not valid UTF-8 is an explicit configuration error, never a silent
    /// fallback (SEC-001).
    pub fn from_env(client_id: Option<String>, email: Option<String>) -> Result<Self, AppError> {
        fn secret(name: &str) -> Result<Option<SecretValue>, AppError> {
            match std::env::var(name) {
                Ok(value) => Ok(Some(SecretValue::from(value))),
                Err(std::env::VarError::NotUnicode(_)) => Err(AppError::Configuration(format!(
                    "{name} must be valid UTF-8"
                ))),
                Err(std::env::VarError::NotPresent) => Ok(None),
            }
        }
        Ok(Credentials {
            bearer_token: secret("CODEMIE_TOKEN")?,
            client_secret: secret("CODEMIE_CLIENT_SECRET")?,
            password: secret("CODEMIE_PASSWORD")?,
            client_id,
            email,
        })
    }
}

/// Determine the authentication mode before any network access.
///
/// Selection order (ADR-011 §1b, cli.md §6):
///
/// 1. `CODEMIE_TOKEN` set → `BearerToken` (highest precedence).
/// 2. `CODEMIE_CLIENT_SECRET` set AND `auth_url` is `Some` → `KeycloakClientCredentials` (Mode a).
/// 3. `CODEMIE_CLIENT_SECRET` not set AND `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` set
///    AND `auth_url` is `Some` → `KeycloakRopc` (Mode c).
/// 4. `CODEMIE_CLIENT_SECRET` not set AND `CODEMIE_EMAIL` + `CODEMIE_PASSWORD` set
///    AND `auth_url` is `None` → `LocalAuth` (Mode b).
/// 5. Any other combination → `E_CONFIGURATION`, exit 2.
///
/// Error messages must not expose credential values (SEC-001).
#[cfg(test)]
pub fn select_auth_mode(
    credentials: &Credentials,
    auth_url: Option<&ValidatedAuthUrl>,
) -> Result<AuthMode, AppError> {
    // 1. Pre-existing bearer token wins over all other modes.
    if credentials.bearer_token.is_some() {
        return Ok(AuthMode::BearerToken);
    }

    // 2. Keycloak client_credentials: client_secret set and auth_url configured.
    if credentials.client_secret.is_some() && auth_url.is_some() {
        return Ok(AuthMode::KeycloakClientCredentials);
    }

    // 3. Keycloak ROPC: no client_secret, email+password set, auth_url configured.
    if credentials.client_secret.is_none()
        && credentials.email.is_some()
        && credentials.password.is_some()
        && auth_url.is_some()
    {
        return Ok(AuthMode::KeycloakRopc);
    }

    // 4. Local-auth: no client_secret, email+password set, auth_url NOT configured.
    if credentials.client_secret.is_none()
        && credentials.email.is_some()
        && credentials.password.is_some()
        && auth_url.is_none()
    {
        return Ok(AuthMode::LocalAuth);
    }

    // 5. No combination matches any of the three modes.
    Err(AppError::Configuration(
        "authentication configuration incomplete".into(),
    ))
}

/// Return the effective client ID for the selected authentication mode.
///
/// For Mode (c) (`KeycloakRopc`), defaults to `"codemie-sdk"` when
/// `credentials.client_id` is `None` (ADR-011 §1a). For all other modes,
/// returns `credentials.client_id` unchanged; no default is applied.
pub fn effective_client_id(mode: &AuthMode, credentials: &Credentials) -> Option<String> {
    match mode {
        AuthMode::KeycloakRopc => Some(
            credentials
                .client_id
                .clone()
                .unwrap_or_else(|| "codemie-sdk".to_owned()),
        ),
        _ => credentials.client_id.clone(),
    }
}

// ---------------------------------------------------------------------------
// HTTP client for authentication
// ---------------------------------------------------------------------------

/// Build a reqwest client for authentication POST requests.
///
/// Security invariants (SEC-002, SEC-003, ADR-011 §4, http-adapter.md §2.4):
/// - `redirect::Policy::none()`: redirect following is disabled. Any 3xx
///   response from an authentication endpoint is returned as-is; credentials
///   are never replayed to a redirect target.
/// - `use_rustls_tls()`: enforces TLS via rustls with no OpenSSL runtime dep.
/// - `connect_timeout(10 s)`: connection establishment is bounded; the `login`
///   subcommand cannot hang indefinitely during TCP/TLS handshake.
/// - `timeout(60 s)`: per-request (send + response) timeout, consistent with
///   `ApiClient`'s `REQUEST_TIMEOUT_SECS` (http-adapter.md §2.4).
///
/// This function is `pub` so T-002 can share or extend the same base
/// configuration for authenticated API calls.
pub fn build_auth_client() -> Result<reqwest::Client, AppError> {
    ensure_rustls_provider();
    reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(AuthError::ClientBuild)
        .map_err(AppError::from)
}

/// Map a reqwest send error (network / TLS / connection) to `AppError::Connectivity`.
///
/// Only called when `send()` itself fails — i.e., no HTTP response was received.
/// A fixed message is used; the original reqwest error string is discarded to
/// prevent leaking internal URL or TLS details into the error chain (SEC-005).
fn map_reqwest_error(source: reqwest::Error) -> AppError {
    AuthError::Dispatch(source).into()
}

/// Read an authentication response body with the shared 8 MiB cap, streaming
/// so the bound is enforced during the read (SEC-003).
async fn read_bounded_body(resp: reqwest::Response) -> Result<Vec<u8>, AppError> {
    let mut resp = resp;
    if let Some(len) = resp.content_length()
        && len as usize > crate::http::RESPONSE_BODY_LIMIT
    {
        return Err(AppError::ApiIncompatible(
            "authentication response body exceeds 8 MiB limit".into(),
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(AuthError::ResponseBody)? {
        if body.len().saturating_add(chunk.len()) > crate::http::RESPONSE_BODY_LIMIT {
            return Err(AppError::ApiIncompatible(
                "authentication response body exceeds 8 MiB limit".into(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Extract the bearer token from an authentication HTTP response.
///
/// - 3xx → `AppError::Authentication`: redirect disabled; credentials must
///   never be replayed to a redirect target (ADR-011 §4).
/// - 4xx / 5xx → `AppError::Authentication`.
/// - 2xx → parse `{"access_token": "..."}` and return the token string.
///
/// The response body is bounded and strictly decoded (duplicate JSON keys are
/// rejected, nesting depth is bounded); server-supplied error bodies, status
/// text, and headers are never forwarded into the error chain or logs (SEC-005).
async fn extract_token_from_response(resp: reqwest::Response) -> Result<String, AppError> {
    let status = resp.status();

    if status.is_redirection() {
        // 3xx: redirect disabled for auth POSTs (ADR-011 §4, SEC-002).
        return Err(AppError::Authentication(
            "authentication endpoint returned a redirect".into(),
        ));
    }

    if !status.is_success() {
        // 4xx or 5xx: auth failure; server body is discarded (SEC-005).
        return Err(AppError::Authentication(
            "authentication request failed".into(),
        ));
    }

    // 2xx: read the bounded body and strictly decode access_token.
    let bytes = read_bounded_body(resp).await?;
    let value = crate::strict_json::from_slice(&bytes).map_err(AuthError::TokenResponse)?;
    if crate::http::json_max_depth(&value, 0) > crate::http::JSON_MAX_DEPTH {
        return Err(AppError::ApiIncompatible(
            "authentication response JSON nesting exceeds 64 levels".into(),
        ));
    }
    let token_resp: TokenResponse =
        serde_json::from_value(value).map_err(AuthError::TokenResponse)?;

    Ok(token_resp.access_token)
}

// ---------------------------------------------------------------------------
// login
// ---------------------------------------------------------------------------

/// Obtain a bearer token using the selected authentication mode.
///
/// # Mode (a) — Keycloak `client_credentials`
///
/// POST to `auth_url` with `Content-Type: application/x-www-form-urlencoded`:
/// `grant_type=client_credentials&client_id={client_id}&client_secret={client_secret}`
///
/// # Mode (c) — Keycloak ROPC (`grant_type=password`) — v26
///
/// POST to `auth_url` with `Content-Type: application/x-www-form-urlencoded`:
/// `grant_type=password&client_id={effective_client_id}&username={email}&password={password}`
///
/// **No `client_secret` field** is included in the request body (spec v26,
/// ADR-011 §1a).
///
/// # Mode (b) — Local-auth
///
/// POST to `{url}/v1/local-auth/login` with `Content-Type: application/json`:
/// `{"email": "{email}", "password": "{password}"}`
///
/// # Mode BearerToken
///
/// Returns the token from `credentials.bearer_token` directly; no network
/// call is made.
///
/// # Security
///
/// - All authentication POST clients use `redirect::Policy::none()` (ADR-011 §4).
/// - 3xx → `AppError::Authentication`; credentials are never replayed.
/// - 4xx / 5xx → `AppError::Authentication`.
/// - Network / TLS failures → `AppError::Connectivity`.
/// - No credential value appears in tracing events, error messages, or logs.
///
/// Callers must pass endpoints already validated by
/// `crate::config::ValidatedUrl` / `crate::config::ValidatedAuthUrl`
/// (the CLI boundary does so): `auth_url` https-only and `url` https-or-
/// loopback-http are guaranteed at that boundary.
pub async fn login(
    mode: AuthMode,
    credentials: &Credentials,
    url: Option<&str>,
    auth_url: Option<&str>,
) -> Result<String, AppError> {
    match mode {
        // ------------------------------------------------------------------
        // BearerToken: already resolved — no network call required.
        // ------------------------------------------------------------------
        AuthMode::BearerToken => credentials
            .bearer_token
            .as_ref()
            .map(|token| token.expose().to_owned())
            .ok_or_else(|| AppError::Authentication("bearer token is not set".into())),

        // ------------------------------------------------------------------
        // Mode (a): Keycloak client_credentials
        // ------------------------------------------------------------------
        AuthMode::KeycloakClientCredentials => {
            let auth_url = auth_url.ok_or_else(|| {
                AppError::Configuration(
                    "auth_url is required for Keycloak client_credentials".into(),
                )
            })?;
            let client_secret = credentials
                .client_secret
                .as_ref()
                .map(SecretValue::expose)
                .ok_or_else(|| AppError::Authentication("client_secret is not set".into()))?;

            let mut params: Vec<(String, String)> = vec![
                ("grant_type".into(), "client_credentials".into()),
                ("client_secret".into(), client_secret.to_owned()),
            ];
            // client_id is optional for Mode (a); omit when not set rather
            // than sending an empty string.
            if let Some(id) = effective_client_id(&mode, credentials) {
                params.push(("client_id".into(), id));
            }

            let client = build_auth_client()?;
            let resp = client
                .post(auth_url)
                .form(&params)
                .send()
                .await
                .map_err(map_reqwest_error)?;

            extract_token_from_response(resp).await
        }

        // ------------------------------------------------------------------
        // Mode (c): Keycloak ROPC (v26) — NO client_secret in body
        // ------------------------------------------------------------------
        AuthMode::KeycloakRopc => {
            let auth_url = auth_url.ok_or_else(|| {
                AppError::Configuration("auth_url is required for Keycloak ROPC".into())
            })?;
            // `effective_client_id` always returns Some for KeycloakRopc,
            // defaulting to "codemie-sdk" when client_id is not set (ADR-011 §1a).
            let client_id =
                effective_client_id(&mode, credentials).unwrap_or_else(|| "codemie-sdk".to_owned());
            let email = credentials.email.as_deref().ok_or_else(|| {
                AppError::Authentication("email is not set for Keycloak ROPC".into())
            })?;
            let password = credentials
                .password
                .as_ref()
                .map(SecretValue::expose)
                .ok_or_else(|| {
                    AppError::Authentication("password is not set for Keycloak ROPC".into())
                })?;

            // Mode (c) ROPC body: MUST NOT include client_secret (spec v26, ADR-011 §1a).
            let params: Vec<(String, String)> = vec![
                ("grant_type".into(), "password".into()),
                ("client_id".into(), client_id),
                ("username".into(), email.to_owned()),
                ("password".into(), password.to_owned()),
            ];

            let client = build_auth_client()?;
            let resp = client
                .post(auth_url)
                .form(&params)
                .send()
                .await
                .map_err(map_reqwest_error)?;

            extract_token_from_response(resp).await
        }

        // ------------------------------------------------------------------
        // Mode (b): Local-auth (no auth_url; uses target_url)
        // ------------------------------------------------------------------
        AuthMode::LocalAuth => {
            let url = url.ok_or_else(|| {
                AppError::Configuration("url is required for local-auth login".into())
            })?;
            let email = credentials.email.as_deref().ok_or_else(|| {
                AppError::Authentication("email is not set for local-auth".into())
            })?;
            let password = credentials
                .password
                .as_ref()
                .map(SecretValue::expose)
                .ok_or_else(|| {
                    AppError::Authentication("password is not set for local-auth".into())
                })?;

            let login_url = format!("{}/v1/local-auth/login", url.trim_end_matches('/'));

            // JSON body; serde_json ensures control characters are escaped
            // and no intermediate string concatenation is used (SEC-005).
            let body = serde_json::json!({
                "email": email,
                "password": password,
            });

            let client = build_auth_client()?;
            let resp = client
                .post(&login_url)
                .json(&body)
                .send()
                .await
                .map_err(map_reqwest_error)?;

            extract_token_from_response(resp).await
        }
    }
}

/// Execute a validated authentication strategy.
pub(crate) async fn login_with_strategy(
    strategy: AuthStrategy,
    url: Option<&str>,
    auth_url: Option<&str>,
) -> Result<String, AppError> {
    let (mode, credentials) = match strategy {
        AuthStrategy::BearerToken { token } => (
            AuthMode::BearerToken,
            Credentials {
                bearer_token: Some(token),
                client_secret: None,
                password: None,
                client_id: None,
                email: None,
            },
        ),
        AuthStrategy::KeycloakClientCredentials {
            client_id,
            client_secret,
        } => (
            AuthMode::KeycloakClientCredentials,
            Credentials {
                bearer_token: None,
                client_secret: Some(client_secret),
                password: None,
                client_id,
                email: None,
            },
        ),
        AuthStrategy::KeycloakRopc {
            client_id,
            email,
            password,
        } => (
            AuthMode::KeycloakRopc,
            Credentials {
                bearer_token: None,
                client_secret: None,
                password: Some(password),
                client_id: Some(client_id),
                email: Some(email),
            },
        ),
        AuthStrategy::LocalAuth { email, password } => (
            AuthMode::LocalAuth,
            Credentials {
                bearer_token: None,
                client_secret: None,
                password: Some(password),
                client_id: None,
                email: Some(email),
            },
        ),
    };
    login(mode, &credentials, url, auth_url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ValidatedAuthUrl;

    fn make_auth_url() -> ValidatedAuthUrl {
        ValidatedAuthUrl::try_from("https://auth.example.com/token")
            .expect("test auth URL must be valid")
    }

    // --- Existing ---

    #[test]
    fn credentials_from_env_does_not_panic() {
        let creds = Credentials::from_env(None, None).expect("env credentials must load");
        let _ = creds.bearer_token;
        let _ = creds.client_secret;
        let _ = creds.password;
    }

    #[test]
    fn auth_mode_variants_are_distinct() {
        assert_ne!(AuthMode::KeycloakClientCredentials, AuthMode::LocalAuth);
        assert_ne!(AuthMode::KeycloakRopc, AuthMode::BearerToken);
    }

    #[test]
    fn validated_auth_strategy_owns_only_selected_secret_and_redacts_debug() {
        let strategy = AuthStrategy::try_from(RawAuthSelection {
            credentials: Credentials {
                bearer_token: Some("bearer-secret".into()),
                client_secret: Some("unused-client-secret".into()),
                password: Some("unused-password".into()),
                client_id: Some("unused-client".into()),
                email: Some("unused@example.com".into()),
            },
            auth_url_configured: true,
        })
        .unwrap();

        assert!(matches!(&strategy, AuthStrategy::BearerToken { .. }));
        let debug = format!("{strategy:?}");
        assert!(!debug.contains("bearer-secret"));
        assert!(!debug.contains("unused-client-secret"));
        assert!(!debug.contains("unused-password"));
    }

    // --- select_auth_mode ---

    #[test]
    fn select_auth_mode_bearer_token_highest_priority() {
        // CODEMIE_TOKEN is set; even if client_secret is also set, BearerToken wins.
        let creds = Credentials {
            bearer_token: Some("tok".into()),
            client_secret: Some("sec".into()),
            password: None,
            client_id: None,
            email: None,
        };
        let auth_url = make_auth_url();
        assert_eq!(
            select_auth_mode(&creds, Some(&auth_url)).unwrap(),
            AuthMode::BearerToken
        );
    }

    #[test]
    fn select_auth_mode_bearer_token_no_auth_url() {
        let creds = Credentials {
            bearer_token: Some("tok".into()),
            client_secret: None,
            password: None,
            client_id: None,
            email: None,
        };
        assert_eq!(
            select_auth_mode(&creds, None).unwrap(),
            AuthMode::BearerToken
        );
    }

    #[test]
    fn select_auth_mode_keycloak_client_credentials() {
        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("sec".into()),
            password: None,
            client_id: None,
            email: None,
        };
        let auth_url = make_auth_url();
        assert_eq!(
            select_auth_mode(&creds, Some(&auth_url)).unwrap(),
            AuthMode::KeycloakClientCredentials
        );
    }

    #[test]
    fn select_auth_mode_keycloak_ropc() {
        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("pass".into()),
            client_id: None,
            email: Some("user@example.com".into()),
        };
        let auth_url = make_auth_url();
        assert_eq!(
            select_auth_mode(&creds, Some(&auth_url)).unwrap(),
            AuthMode::KeycloakRopc
        );
    }

    #[test]
    fn select_auth_mode_local_auth() {
        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("pass".into()),
            client_id: None,
            email: Some("user@example.com".into()),
        };
        assert_eq!(select_auth_mode(&creds, None).unwrap(), AuthMode::LocalAuth);
    }

    #[test]
    fn select_auth_mode_no_valid_combination_returns_configuration_error() {
        // No credentials set at all.
        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: None,
            client_id: None,
            email: None,
        };
        let result = select_auth_mode(&creds, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn select_auth_mode_client_secret_without_auth_url_is_invalid() {
        // client_secret set but no auth_url → no valid mode (not Mode a, not Mode c/b).
        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("sec".into()),
            password: None,
            client_id: None,
            email: None,
        };
        let result = select_auth_mode(&creds, None);
        assert!(result.is_err(), "client_secret without auth_url must fail");
        assert_eq!(result.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn select_auth_mode_error_message_does_not_contain_credential_values() {
        // The error message must not echo any credential value (SEC-001).
        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: None,
            client_id: None,
            email: None,
        };
        let err = select_auth_mode(&creds, None).unwrap_err();
        let msg = format!("{err}");
        // Ensure the message contains no secret-value patterns
        assert!(
            !msg.contains("CODEMIE_"),
            "error must not echo env var names"
        );
    }

    // --- effective_client_id ---

    #[test]
    fn effective_client_id_mode_c_defaults_to_codemie_sdk() {
        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("pass".into()),
            client_id: None,
            email: Some("user@example.com".into()),
        };
        let id = effective_client_id(&AuthMode::KeycloakRopc, &creds);
        assert_eq!(id.as_deref(), Some("codemie-sdk"));
    }

    #[test]
    fn effective_client_id_mode_c_uses_provided_id_when_set() {
        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("pass".into()),
            client_id: Some("my-client".into()),
            email: Some("user@example.com".into()),
        };
        let id = effective_client_id(&AuthMode::KeycloakRopc, &creds);
        assert_eq!(id.as_deref(), Some("my-client"));
    }

    #[test]
    fn effective_client_id_other_modes_no_default_applied() {
        // For Mode (a), no default is inserted when client_id is absent.
        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("sec".into()),
            password: None,
            client_id: None,
            email: None,
        };
        let id = effective_client_id(&AuthMode::KeycloakClientCredentials, &creds);
        assert!(id.is_none(), "default must not be applied for Mode (a)");
    }

    #[test]
    fn effective_client_id_bearer_mode_no_default_applied() {
        let creds = Credentials {
            bearer_token: Some("tok".into()),
            client_secret: None,
            password: None,
            client_id: None,
            email: None,
        };
        let id = effective_client_id(&AuthMode::BearerToken, &creds);
        assert!(
            id.is_none(),
            "default must not be applied for BearerToken mode"
        );
    }

    // -----------------------------------------------------------------------
    // T-001: login — BearerToken mode
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn login_bearer_token_returns_token_without_network_call() {
        // BearerToken mode must return the token immediately with no HTTP call.
        // Passing None for url/auth_url proves no network access is attempted.
        let creds = Credentials {
            bearer_token: Some("my-bearer-xyz".into()),
            client_secret: None,
            password: None,
            client_id: None,
            email: None,
        };
        let result = login(AuthMode::BearerToken, &creds, None, None).await;
        assert_eq!(result.unwrap(), "my-bearer-xyz");
    }

    // -----------------------------------------------------------------------
    // T-001: login — Mode (a) Keycloak client_credentials
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn login_mode_a_sends_correct_body_and_returns_token() {
        let mut server = mockito::Server::new_async().await;
        let auth_url = format!("{}/token", server.url());

        // Verify exact form body: grant_type, client_id, client_secret present.
        let _mock = server
            .mock("POST", "/token")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("grant_type".into(), "client_credentials".into()),
                mockito::Matcher::UrlEncoded("client_id".into(), "my-client-a".into()),
                mockito::Matcher::UrlEncoded("client_secret".into(), "my-secret".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"token-from-mode-a"}"#)
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("my-secret".into()),
            password: None,
            client_id: Some("my-client-a".into()),
            email: None,
        };
        let result = login(
            AuthMode::KeycloakClientCredentials,
            &creds,
            None,
            Some(&auth_url),
        )
        .await;

        assert_eq!(result.unwrap(), "token-from-mode-a");
        _mock.assert_async().await;
    }

    #[tokio::test]
    async fn login_mode_a_omits_client_id_when_not_set() {
        // When no client_id is configured for Mode (a), the field is omitted
        // from the request body rather than sent as empty string.
        // The exact body string is matched so that any accidental extra field
        // (like "client_id=") causes the mock to not fire and assert_async to fail.
        let mut server = mockito::Server::new_async().await;
        let auth_url = format!("{}/token", server.url());

        let _mock = server
            .mock("POST", "/token")
            .match_body(mockito::Matcher::Exact(
                "grant_type=client_credentials&client_secret=sec".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"tok-no-cid"}"#)
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("sec".into()),
            password: None,
            client_id: None, // absent
            email: None,
        };
        let result = login(
            AuthMode::KeycloakClientCredentials,
            &creds,
            None,
            Some(&auth_url),
        )
        .await;

        assert_eq!(result.unwrap(), "tok-no-cid");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-001: login — Mode (c) Keycloak ROPC
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn login_mode_c_sends_correct_body_no_client_secret() {
        // Mode (c) ROPC body MUST contain grant_type=password, client_id, username,
        // password — and MUST NOT contain client_secret (spec v26, ADR-011 §1a).
        //
        // The full body is matched exactly.  Any erroneous extra field (such as
        // "client_secret=...") causes the body to differ from the expected string,
        // the mock will not fire, and assert_async() will fail.
        let mut server = mockito::Server::new_async().await;
        let auth_url = format!("{}/token", server.url());

        // reqwest encodes '@' as '%40' in application/x-www-form-urlencoded bodies.
        let _mock = server
            .mock("POST", "/token")
            .match_body(mockito::Matcher::Exact(
                "grant_type=password&client_id=codemie-sdk&username=user%40example.com&password=mypass"
                    .into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"token-from-mode-c"}"#)
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: None, // not set for Mode (c)
            password: Some("mypass".into()),
            client_id: None, // absent → defaults to codemie-sdk
            email: Some("user@example.com".into()),
        };
        let result = login(AuthMode::KeycloakRopc, &creds, None, Some(&auth_url)).await;

        assert_eq!(result.unwrap(), "token-from-mode-c");
        _mock.assert_async().await;
    }

    #[tokio::test]
    async fn login_mode_c_uses_provided_client_id() {
        // Also verifies that no client_secret field appears when using an
        // explicit client_id: the full body is matched exactly.
        let mut server = mockito::Server::new_async().await;
        let auth_url = format!("{}/token", server.url());

        let _mock = server
            .mock("POST", "/token")
            .match_body(mockito::Matcher::Exact(
                "grant_type=password&client_id=custom-client&username=u%40x.com&password=pw".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"tok-c-custom"}"#)
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("pw".into()),
            client_id: Some("custom-client".into()),
            email: Some("u@x.com".into()),
        };
        let result = login(AuthMode::KeycloakRopc, &creds, None, Some(&auth_url)).await;

        assert_eq!(result.unwrap(), "tok-c-custom");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-001: login — Mode (b) Local-auth
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn login_mode_b_sends_correct_json_body_and_returns_token() {
        let mut server = mockito::Server::new_async().await;
        let base_url = server.url();

        let _mock = server
            .mock("POST", "/v1/local-auth/login")
            .match_header(
                "content-type",
                mockito::Matcher::Regex("application/json".into()),
            )
            .match_body(mockito::Matcher::Json(serde_json::json!({
                "email": "local@example.com",
                "password": "localpass",
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"token-from-mode-b","user":{"id":1}}"#)
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("localpass".into()),
            client_id: None,
            email: Some("local@example.com".into()),
        };
        let result = login(AuthMode::LocalAuth, &creds, Some(&base_url), None).await;

        assert_eq!(result.unwrap(), "token-from-mode-b");
        _mock.assert_async().await;
    }

    #[tokio::test]
    async fn login_mode_b_appends_path_to_base_url_with_trailing_slash() {
        // Verify that a trailing slash on the base URL doesn't produce a
        // double slash in the constructed login URL.
        let mut server = mockito::Server::new_async().await;
        let base_url = format!("{}/", server.url()); // trailing slash

        let _mock = server
            .mock("POST", "/v1/local-auth/login")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"tok-trailing"}"#)
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("pw".into()),
            client_id: None,
            email: Some("a@b.com".into()),
        };
        let result = login(AuthMode::LocalAuth, &creds, Some(&base_url), None).await;
        assert_eq!(result.unwrap(), "tok-trailing");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-001: login — 3xx / 4xx / 5xx error handling
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn login_3xx_response_is_authentication_error() {
        // A redirect response on an auth POST is E_AUTHENTICATION, exit 2
        // (ADR-011 §4: redirect disabled; credentials must not be replayed).
        let mut server = mockito::Server::new_async().await;
        let auth_url = format!("{}/token", server.url());

        let _mock = server
            .mock("POST", "/token")
            .with_status(302)
            .with_header("location", "https://other.example.com/token")
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("sec".into()),
            password: None,
            client_id: Some("cid".into()),
            email: None,
        };
        let result = login(
            AuthMode::KeycloakClientCredentials,
            &creds,
            None,
            Some(&auth_url),
        )
        .await;

        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Authentication(_)),
            "3xx must produce AppError::Authentication, got {:?}",
            err
        );
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn login_4xx_response_is_authentication_error() {
        let mut server = mockito::Server::new_async().await;
        let auth_url = format!("{}/token", server.url());

        let _mock = server
            .mock("POST", "/token")
            .with_status(401)
            .with_body(r#"{"error":"invalid_client"}"#)
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("bad-secret".into()),
            password: None,
            client_id: Some("cid".into()),
            email: None,
        };
        let result = login(
            AuthMode::KeycloakClientCredentials,
            &creds,
            None,
            Some(&auth_url),
        )
        .await;

        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Authentication(_)),
            "401 must produce AppError::Authentication, got {:?}",
            err
        );
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn login_5xx_response_is_authentication_error() {
        let mut server = mockito::Server::new_async().await;
        let auth_url = format!("{}/token", server.url());

        let _mock = server
            .mock("POST", "/token")
            .with_status(503)
            .with_body("Service Unavailable")
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("sec".into()),
            password: None,
            client_id: Some("cid".into()),
            email: None,
        };
        let result = login(
            AuthMode::KeycloakClientCredentials,
            &creds,
            None,
            Some(&auth_url),
        )
        .await;

        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Authentication(_)),
            "5xx must produce AppError::Authentication, got {:?}",
            err
        );
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn login_3xx_on_local_auth_is_authentication_error() {
        // Redirect on local-auth POST must also be E_AUTHENTICATION (ADR-011 §4).
        let mut server = mockito::Server::new_async().await;
        let base_url = server.url();

        let _mock = server
            .mock("POST", "/v1/local-auth/login")
            .with_status(301)
            .with_header("location", "https://other.example.com/login")
            .create_async()
            .await;

        let creds = Credentials {
            bearer_token: None,
            client_secret: None,
            password: Some("pw".into()),
            client_id: None,
            email: Some("u@x.com".into()),
        };
        let result = login(AuthMode::LocalAuth, &creds, Some(&base_url), None).await;

        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Authentication(_)),
            "3xx on local-auth must produce AppError::Authentication"
        );
        assert_eq!(err.exit_code(), 2);
    }

    // -----------------------------------------------------------------------
    // T-001: login — TLS / connection error → AppError::Connectivity
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn login_connection_error_is_connectivity_error() {
        // Bind a TCP listener to reserve a free OS port, record the port, then
        // immediately drop the listener.  Any subsequent connection attempt to
        // that port receives "Connection refused", which must map to
        // AppError::Connectivity (not AppError::Authentication).
        //
        // Using std::net::TcpListener (synchronous) avoids a timing race:
        // the mockito Server drop approach can race between the background task
        // shutdown and our connection attempt, occasionally returning a 501 and
        // wrongly yielding AppError::Authentication instead.
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
            // listener dropped here — port is closed before login() is called
        };
        let auth_url = format!("http://127.0.0.1:{}/token", port);

        let creds = Credentials {
            bearer_token: None,
            client_secret: Some("sec".into()),
            password: None,
            client_id: Some("cid".into()),
            email: None,
        };
        let result = login(
            AuthMode::KeycloakClientCredentials,
            &creds,
            None,
            Some(&auth_url),
        )
        .await;

        let err = result.unwrap_err();
        assert!(
            err.is_connectivity(),
            "connection error must preserve connectivity taxonomy, got {:?}",
            err
        );
        assert_eq!(err.exit_code(), 2);
    }
}
