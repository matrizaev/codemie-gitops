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
use crate::error::AppError;

/// The selected authentication mode, determined before any network access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    /// Mode (a): Keycloak client_credentials.
    KeycloakClientCredentials,
    /// Mode (b): Local-auth ROPC (email + password, no auth_url).
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
pub struct Credentials {
    /// Non-null when `CODEMIE_TOKEN` is set.
    pub bearer_token: Option<String>,
    /// Non-null when `CODEMIE_CLIENT_SECRET` is set.
    pub client_secret: Option<String>,
    /// Non-null when `CODEMIE_PASSWORD` is set.
    pub password: Option<String>,
    /// Non-secret; resolved from `--client-id` > `CODEMIE_CLIENT_ID`.
    pub client_id: Option<String>,
    /// Non-secret; resolved from `--email` > `CODEMIE_EMAIL`.
    pub email: Option<String>,
}

impl Credentials {
    /// Load credentials from the current process environment.
    ///
    /// Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
    /// `CODEMIE_PASSWORD`) are read from environment only. Non-secret selectors
    /// (`client_id`, `email`) are passed in because they may be overridden by
    /// CLI flags (flag > env precedence is handled by clap).
    pub fn from_env(client_id: Option<String>, email: Option<String>) -> Self {
        Credentials {
            bearer_token: std::env::var("CODEMIE_TOKEN").ok(),
            client_secret: std::env::var("CODEMIE_CLIENT_SECRET").ok(),
            password: std::env::var("CODEMIE_PASSWORD").ok(),
            client_id,
            email,
        }
    }
}

/// Obtain a bearer token using the selected authentication mode.
///
/// This is a stub; full authentication including mock-IdP redirect rejection
/// and ROPC mode is implemented in T-001.
pub async fn login(
    _mode: AuthMode,
    _credentials: &Credentials,
    _url: Option<&str>,
    _auth_url: Option<&str>,
) -> Result<String, AppError> {
    todo!("authentication implemented in T-001")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_from_env_does_not_panic() {
        // Credentials::from_env reads env vars; verify it constructs without
        // panicking in a test environment where they may be absent.
        let creds = Credentials::from_env(None, None);
        // In a clean test environment these should all be None.
        // We don't assert their values because CI may set some of them.
        let _ = creds.bearer_token;
        let _ = creds.client_secret;
        let _ = creds.password;
    }

    #[test]
    fn auth_mode_variants_are_distinct() {
        assert_ne!(AuthMode::KeycloakClientCredentials, AuthMode::LocalAuth);
        assert_ne!(AuthMode::KeycloakRopc, AuthMode::BearerToken);
    }
}
