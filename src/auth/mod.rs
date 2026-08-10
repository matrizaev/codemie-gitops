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
use crate::config::ValidatedUrl;
use crate::error::AppError;

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
pub fn select_auth_mode(
    credentials: &Credentials,
    auth_url: Option<&ValidatedUrl>,
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
    use crate::config::ValidatedUrl;

    fn make_auth_url() -> ValidatedUrl {
        ValidatedUrl::try_from("https://auth.example.com/token")
            .expect("test auth URL must be valid")
    }

    // --- Existing ---

    #[test]
    fn credentials_from_env_does_not_panic() {
        let creds = Credentials::from_env(None, None);
        let _ = creds.bearer_token;
        let _ = creds.client_secret;
        let _ = creds.password;
    }

    #[test]
    fn auth_mode_variants_are_distinct() {
        assert_ne!(AuthMode::KeycloakClientCredentials, AuthMode::LocalAuth);
        assert_ne!(AuthMode::KeycloakRopc, AuthMode::BearerToken);
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
        assert_eq!(
            select_auth_mode(&creds, None).unwrap(),
            AuthMode::LocalAuth
        );
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
        assert!(!msg.contains("CODEMIE_"), "error must not echo env var names");
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
        assert!(
            id.is_none(),
            "default must not be applied for Mode (a)"
        );
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
        assert!(id.is_none(), "default must not be applied for BearerToken mode");
    }
}
