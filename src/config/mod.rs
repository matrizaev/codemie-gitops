/// Repository configuration loading and validated config resolution.
///
/// The `.codemie/config.yaml` file is governed by
/// `contracts/repository-config.schema.json`. Its only permitted keys are the
/// non-secret `url`, `auth_url`, and `project`. Unknown keys — including any
/// credential-like keys — are rejected locally (E_CONFIGURATION, exit 2).
///
/// Full precedence logic (flag > env > config) is implemented in F-002.
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// ValidatedUrl
// ---------------------------------------------------------------------------

/// A URL that has been validated per ADR-011 §2:
///
/// - Scheme is `http` or `https` only
/// - No userinfo component (`@` in the authority)
/// - No fragment identifier (`#`)
/// - No C0/C1 control characters (U+0000–U+001F, U+007F–U+009F)
/// - Non-empty, syntactically valid host
/// - `http://` scheme is permitted only for syntactic loopback destinations
///   (127.0.0.0/8, ::1, `localhost`); runtime DNS enforcement is in T-002
#[derive(Debug, Clone)]
pub struct ValidatedUrl(String);

impl ValidatedUrl {
    /// Returns the validated URL string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ValidatedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<&str> for ValidatedUrl {
    type Error = AppError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        // 1. No C0/C1 control characters in the raw string.
        if raw.chars().any(|c| {
            let n = c as u32;
            n <= 0x1F || (0x7F..=0x9F).contains(&n)
        }) {
            return Err(AppError::Configuration(
                "URL contains control characters".into(),
            ));
        }

        // 2. Parse with the url crate for syntactic validation.
        let parsed = url::Url::parse(raw)
            .map_err(|_| AppError::Configuration("URL is not syntactically valid".into()))?;

        // 3. Scheme must be http or https.
        match parsed.scheme() {
            "http" | "https" => {}
            _ => {
                return Err(AppError::Configuration(
                    "URL scheme must be http or https".into(),
                ));
            }
        }

        // 4. No fragment — checked on the parsed representation.
        if parsed.fragment().is_some() {
            return Err(AppError::Configuration(
                "URL must not contain a fragment".into(),
            ));
        }

        // 5. No userinfo — check the authority portion of the raw string for `@`.
        //    This catches both `user@host` and the empty-userinfo `@host` form.
        {
            const SEP: &str = "://";
            if let Some(pos) = raw.find(SEP) {
                let after_sep = &raw[pos + SEP.len()..];
                let auth_end = after_sep.find(['/', '?', '#']).unwrap_or(after_sep.len());
                if after_sep[..auth_end].contains('@') {
                    return Err(AppError::Configuration(
                        "URL must not contain userinfo".into(),
                    ));
                }
            }
        }

        // 6. Non-empty host.
        match parsed.host_str() {
            Some(h) if !h.is_empty() => {}
            _ => {
                return Err(AppError::Configuration(
                    "URL must have a non-empty host".into(),
                ));
            }
        }

        // 7. For http:// scheme, the host must be a syntactic loopback marker.
        if parsed.scheme() == "http" && !is_loopback_http(raw) {
            return Err(AppError::Configuration(
                "http:// URL is only permitted for loopback destinations".into(),
            ));
        }

        Ok(ValidatedUrl(raw.to_owned()))
    }
}

impl TryFrom<String> for ValidatedUrl {
    type Error = AppError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        ValidatedUrl::try_from(raw.as_str())
    }
}

// ---------------------------------------------------------------------------
// ValidatedAuthUrl
// ---------------------------------------------------------------------------

/// A URL that satisfies all `ValidatedUrl` rules and additionally requires
/// HTTPS scheme (ADR-011 §3: no loopback exception for the identity provider).
#[derive(Debug, Clone)]
pub struct ValidatedAuthUrl(ValidatedUrl);

impl ValidatedAuthUrl {
    /// Returns the validated URL string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl std::ops::Deref for ValidatedAuthUrl {
    type Target = ValidatedUrl;

    fn deref(&self) -> &ValidatedUrl {
        &self.0
    }
}

impl std::fmt::Display for ValidatedAuthUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl TryFrom<&str> for ValidatedAuthUrl {
    type Error = AppError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        // HTTPS required unconditionally for auth endpoints (ADR-011 §3).
        // Parse once to extract the scheme before the full ValidatedUrl pass.
        let parsed = url::Url::parse(raw).map_err(|_| {
            AppError::Configuration("authentication URL is not syntactically valid".into())
        })?;
        if parsed.scheme() != "https" {
            return Err(AppError::Configuration(
                "authentication URL must use https://".into(),
            ));
        }
        // Apply all remaining ValidatedUrl rules.  The scheme is https so the
        // loopback check in ValidatedUrl::try_from is skipped.
        let validated = ValidatedUrl::try_from(raw)?;
        Ok(ValidatedAuthUrl(validated))
    }
}

impl TryFrom<String> for ValidatedAuthUrl {
    type Error = AppError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        ValidatedAuthUrl::try_from(raw.as_str())
    }
}

// ---------------------------------------------------------------------------
// is_loopback_http
// ---------------------------------------------------------------------------

/// Returns `true` iff `raw` has `http` scheme and a syntactically loopback host.
///
/// For the static validation path (no DNS resolution), the accepted loopback
/// markers are:
///
/// - IPv4 addresses in 127.0.0.0/8 (first octet == 127)
/// - IPv6 `::1`
/// - Hostname `localhost` (subject to runtime DNS confirmation in T-002)
pub fn is_loopback_http(raw: &str) -> bool {
    let parsed = match url::Url::parse(raw) {
        Ok(u) => u,
        Err(_) => return false,
    };
    if parsed.scheme() != "http" {
        return false;
    }
    match parsed.host() {
        Some(url::Host::Domain(d)) => d == "localhost",
        Some(url::Host::Ipv4(addr)) => addr.octets()[0] == 127,
        Some(url::Host::Ipv6(addr)) => addr == std::net::Ipv6Addr::LOCALHOST,
        None => false,
    }
}

// ---------------------------------------------------------------------------
// RepositoryConfig
// ---------------------------------------------------------------------------

/// The on-disk shape of `.codemie/config.yaml`.
///
/// Only the three non-secret, non-credential keys are deserialised; serde
/// `deny_unknown_fields` ensures unknown keys (including credential keys)
/// are rejected.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryConfig {
    /// Optional target API URL (non-secret; `https://` or loopback `http://`).
    pub url: Option<String>,

    /// Optional Keycloak authentication endpoint (`https://` required).
    pub auth_url: Option<String>,

    /// Optional default project for declarations that omit `metadata.project`.
    pub project: Option<String>,
}

// ---------------------------------------------------------------------------
// ResolvedConfig
// ---------------------------------------------------------------------------

/// Fully resolved, validated configuration for a single invocation.
///
/// All fields have been merged from flag > env > config precedence and
/// validated as `ValidatedUrl` / `ValidatedAuthUrl` where applicable.
/// Credential fields are never stored here — they are read from the
/// environment at the auth boundary (auth/mod.rs).
#[derive(Debug, Default)]
pub struct ResolvedConfig {
    /// Resolved target API URL. `None` means not configured.
    pub url: Option<ValidatedUrl>,

    /// Resolved Keycloak authentication endpoint. `None` means not configured.
    pub auth_url: Option<ValidatedAuthUrl>,

    /// Resolved effective default project. `None` means not configured.
    pub project: Option<String>,
}

// ---------------------------------------------------------------------------
// ResolveConfigArgs
// ---------------------------------------------------------------------------

/// Arguments supplied to `resolve_config`.
///
/// `flag_url` and `flag_auth_url` already encode the clap flag-over-env
/// resolution (clap reads `CODEMIE_URL` / `CODEMIE_AUTH_URL` via `env =`).
pub struct ResolveConfigArgs {
    /// `--url` flag value (or `CODEMIE_URL` env, resolved by clap).
    pub flag_url: Option<String>,
    /// `--auth-url` flag value (or `CODEMIE_AUTH_URL` env, resolved by clap).
    pub flag_auth_url: Option<String>,
    /// `--repo-root` flag value, or `None` to auto-detect.
    pub repo_root: Option<PathBuf>,
    /// Value of `--follow-symlinks`.
    pub follow_symlinks: bool,
}

// ---------------------------------------------------------------------------
// resolve_config
// ---------------------------------------------------------------------------

/// Resolve the final configuration for one invocation.
///
/// Precedence (per CLI contract §2):
/// - `target_url`:  flag/env (`flag_url`) > config `url`
/// - `auth_url`:    flag/env (`flag_auth_url`) > config `auth_url`
/// - `project`:     config `project` only (declaration `metadata.project`
///                  overrides at apply-time, not here)
///
/// An invalid higher-precedence value is `E_CONFIGURATION`, exit 2; the
/// lower-precedence value is NOT consulted as a fallback.
pub fn resolve_config(args: &ResolveConfigArgs) -> Result<ResolvedConfig, AppError> {
    // Determine repository root.
    let repo_root = match &args.repo_root {
        Some(r) => r.clone(),
        None => {
            let cwd = std::env::current_dir().map_err(|e| {
                AppError::Configuration(format!("cannot determine working directory: {e}"))
            })?;
            find_repo_root(&cwd).unwrap_or(cwd)
        }
    };

    // Load the repository config (absent file → default/empty).
    let config = load_repository_config(&repo_root)?;

    // Resolve each URL field with fail-fast on the highest-precedence value.
    let url = resolve_validated_url(args.flag_url.as_deref(), config.url.as_deref())?;
    let auth_url =
        resolve_validated_auth_url(args.flag_auth_url.as_deref(), config.auth_url.as_deref())?;

    Ok(ResolvedConfig {
        url,
        auth_url,
        project: config.project,
    })
}

/// Resolve and validate a target URL from flag (highest) or config (fallback).
///
/// If the highest-precedence source is present but invalid, returns
/// `E_CONFIGURATION`; the other source is not consulted.
pub(crate) fn resolve_validated_url(
    flag: Option<&str>,
    config: Option<&str>,
) -> Result<Option<ValidatedUrl>, AppError> {
    match flag.or(config) {
        None => Ok(None),
        Some(raw) => ValidatedUrl::try_from(raw).map(Some),
    }
}

/// Resolve and validate an auth URL from flag (highest) or config (fallback).
///
/// If the highest-precedence source is present but invalid, returns
/// `E_CONFIGURATION`; the other source is not consulted.
pub(crate) fn resolve_validated_auth_url(
    flag: Option<&str>,
    config: Option<&str>,
) -> Result<Option<ValidatedAuthUrl>, AppError> {
    match flag.or(config) {
        None => Ok(None),
        Some(raw) => ValidatedAuthUrl::try_from(raw).map(Some),
    }
}

// ---------------------------------------------------------------------------
// Repository root discovery
// ---------------------------------------------------------------------------

/// Locate the repository root by walking up from `start` until a `.git`
/// directory is found, or return `None` if no such ancestor exists.
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_owned();
    loop {
        if current.join(".git").is_dir() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) => current = parent.to_owned(),
            None => return None,
        }
    }
}

/// Load the `.codemie/config.yaml` file relative to `repo_root`.
///
/// Returns `Ok(RepositoryConfig::default())` if the file does not exist.
/// Returns `Err(AppError::Configuration)` if the file exists but is malformed
/// or contains unknown keys.
pub fn load_repository_config(repo_root: &Path) -> Result<RepositoryConfig, AppError> {
    let config_path = repo_root.join(".codemie").join("config.yaml");
    if !config_path.exists() {
        return Ok(RepositoryConfig::default());
    }
    let raw = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::Configuration(format!("cannot read config: {e}")))?;
    let cfg: RepositoryConfig = serde_yaml::from_str(&raw)
        .map_err(|e| AppError::Configuration(format!("invalid config: {e}")))?;
    Ok(cfg)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- ValidatedUrl acceptance ---

    #[test]
    fn validated_url_accepts_https() {
        assert!(
            ValidatedUrl::try_from("https://api.example.com/path").is_ok(),
            "https:// must be accepted"
        );
    }

    #[test]
    fn validated_url_accepts_http_loopback_ipv4() {
        assert!(
            ValidatedUrl::try_from("http://127.0.0.1/path").is_ok(),
            "http://127.0.0.1/ (loopback) must be accepted"
        );
    }

    #[test]
    fn validated_url_accepts_http_loopback_ipv4_range() {
        // 127.0.0.2 is still in 127.0.0.0/8
        assert!(
            ValidatedUrl::try_from("http://127.0.0.2/path").is_ok(),
            "http://127.0.0.2/ (loopback range) must be accepted"
        );
    }

    #[test]
    fn validated_url_accepts_http_localhost() {
        assert!(
            ValidatedUrl::try_from("http://localhost/path").is_ok(),
            "http://localhost/ must be accepted (syntactic loopback)"
        );
    }

    #[test]
    fn validated_url_accepts_https_with_port() {
        assert!(
            ValidatedUrl::try_from("https://api.example.com:8443/path").is_ok(),
            "https:// with port must be accepted"
        );
    }

    // --- ValidatedUrl rejection ---

    #[test]
    fn validated_url_rejects_http_non_loopback() {
        let err = ValidatedUrl::try_from("http://example.com");
        assert!(err.is_err(), "http:// for non-loopback must be rejected");
        assert_eq!(err.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn validated_url_rejects_userinfo() {
        let err = ValidatedUrl::try_from("https://user:pass@host/");
        assert!(err.is_err(), "URL with userinfo must be rejected");
        assert_eq!(err.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn validated_url_rejects_userinfo_username_only() {
        let err = ValidatedUrl::try_from("https://user@host/path");
        assert!(
            err.is_err(),
            "URL with username-only userinfo must be rejected"
        );
    }

    #[test]
    fn validated_url_rejects_fragment() {
        let err = ValidatedUrl::try_from("https://host/path#section");
        assert!(err.is_err(), "URL with fragment must be rejected");
        assert_eq!(err.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn validated_url_rejects_c0_control_char() {
        let raw = "https://host/path\x00";
        let err = ValidatedUrl::try_from(raw);
        assert!(
            err.is_err(),
            "URL with NUL control character must be rejected"
        );
        assert_eq!(err.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn validated_url_rejects_c0_tab() {
        let raw = "https://host/path\x09/more";
        let err = ValidatedUrl::try_from(raw);
        assert!(err.is_err(), "URL with TAB (C0) must be rejected");
    }

    #[test]
    fn validated_url_rejects_c1_control_char() {
        // U+0080 is the first C1 control character
        let raw = "https://host/path\u{0080}";
        let err = ValidatedUrl::try_from(raw);
        assert!(
            err.is_err(),
            "URL with C1 control character must be rejected"
        );
        assert_eq!(err.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn validated_url_rejects_ftp_scheme() {
        let err = ValidatedUrl::try_from("ftp://host/path");
        assert!(err.is_err(), "ftp:// scheme must be rejected");
    }

    // --- ValidatedAuthUrl ---

    #[test]
    fn validated_auth_url_accepts_https() {
        assert!(
            ValidatedAuthUrl::try_from("https://auth.example.com/token").is_ok(),
            "https:// auth URL must be accepted"
        );
    }

    #[test]
    fn validated_auth_url_rejects_http_non_loopback() {
        let err = ValidatedAuthUrl::try_from("http://auth.example.com/token");
        assert!(err.is_err(), "http:// auth URL must be rejected");
        assert_eq!(err.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn validated_auth_url_rejects_http_loopback() {
        // auth_url has no loopback exception (ADR-011 §3)
        let err = ValidatedAuthUrl::try_from("http://127.0.0.1/token");
        assert!(
            err.is_err(),
            "http://127.0.0.1/ must be rejected for auth_url (no loopback exception)"
        );
        assert_eq!(err.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn validated_auth_url_rejects_http_localhost() {
        let err = ValidatedAuthUrl::try_from("http://localhost/token");
        assert!(
            err.is_err(),
            "http://localhost/ must be rejected for auth_url (no loopback exception)"
        );
    }

    #[test]
    fn validated_auth_url_rejects_fragment() {
        let err = ValidatedAuthUrl::try_from("https://auth.example.com/token#frag");
        assert!(err.is_err(), "auth URL with fragment must be rejected");
    }

    #[test]
    fn validated_auth_url_rejects_userinfo() {
        let err = ValidatedAuthUrl::try_from("https://user:pass@auth.example.com/");
        assert!(err.is_err(), "auth URL with userinfo must be rejected");
    }

    #[test]
    fn validated_auth_url_deref_to_validated_url() {
        let auth = ValidatedAuthUrl::try_from("https://auth.example.com/token").unwrap();
        // Deref to &ValidatedUrl
        let _url: &ValidatedUrl = &*auth;
        assert_eq!(auth.as_str(), "https://auth.example.com/token");
    }

    // --- is_loopback_http ---

    #[test]
    fn is_loopback_http_true_for_127_0_0_1() {
        assert!(is_loopback_http("http://127.0.0.1/"));
    }

    #[test]
    fn is_loopback_http_true_for_127_range() {
        assert!(is_loopback_http("http://127.0.0.2/path"));
    }

    #[test]
    fn is_loopback_http_true_for_localhost() {
        assert!(is_loopback_http("http://localhost/path"));
    }

    #[test]
    fn is_loopback_http_true_for_ipv6_loopback() {
        assert!(is_loopback_http("http://[::1]/path"));
    }

    #[test]
    fn is_loopback_http_false_for_https() {
        assert!(!is_loopback_http("https://127.0.0.1/"));
    }

    #[test]
    fn is_loopback_http_false_for_non_loopback() {
        assert!(!is_loopback_http("http://example.com/"));
    }

    // --- resolve_validated_url precedence ---

    #[test]
    fn resolve_url_both_none_gives_none() {
        let result = resolve_validated_url(None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn resolve_url_flag_used_when_present() {
        let result = resolve_validated_url(
            Some("https://flag.example.com"),
            Some("https://config.example.com"),
        );
        let url = result.unwrap().unwrap();
        assert_eq!(url.as_str(), "https://flag.example.com");
    }

    #[test]
    fn resolve_url_config_used_when_flag_absent() {
        let result = resolve_validated_url(None, Some("https://config.example.com"));
        let url = result.unwrap().unwrap();
        assert_eq!(url.as_str(), "https://config.example.com");
    }

    #[test]
    fn resolve_url_invalid_flag_does_not_fallback_to_config() {
        // An invalid higher-precedence value must not fall through to the
        // lower-precedence config value (CLI contract §2, F-002 acceptance).
        let result = resolve_validated_url(
            Some("http://bad.example.com"),   // invalid: non-loopback http
            Some("https://good.example.com"), // valid: would succeed alone
        );
        assert!(
            result.is_err(),
            "invalid flag URL must error even when config URL is valid"
        );
        assert_eq!(result.unwrap_err().exit_code(), 2);
    }

    // --- resolve_validated_auth_url precedence ---

    #[test]
    fn resolve_auth_url_invalid_flag_does_not_fallback_to_config() {
        let result = resolve_validated_auth_url(
            Some("http://bad.example.com/token"),   // invalid: http for auth
            Some("https://good.example.com/token"), // valid
        );
        assert!(
            result.is_err(),
            "invalid flag auth URL must error even when config auth URL is valid"
        );
        assert_eq!(result.unwrap_err().exit_code(), 2);
    }

    #[test]
    fn resolve_auth_url_both_none_gives_none() {
        let result = resolve_validated_auth_url(None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // --- Existing: ResolvedConfig default ---

    #[test]
    fn default_resolved_config_is_all_none() {
        let cfg = ResolvedConfig::default();
        assert!(cfg.url.is_none());
        assert!(cfg.auth_url.is_none());
        assert!(cfg.project.is_none());
    }

    // --- Existing: RepositoryConfig serde rejection ---

    #[test]
    fn repository_config_rejects_credential_keys() {
        let yaml_with_token = r#"url: "https://example.com"
token: "secret"
"#;
        let result: Result<RepositoryConfig, _> = serde_yaml::from_str(yaml_with_token);
        assert!(result.is_err(), "credential key 'token' must be rejected");
    }

    #[test]
    fn repository_config_rejects_client_secret_key() {
        let yaml = "client_secret: \"s3cr3t\"\n";
        let result: Result<RepositoryConfig, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "credential key 'client_secret' must be rejected"
        );
    }

    #[test]
    fn repository_config_rejects_password_key() {
        let yaml = "password: \"s3cr3t\"\n";
        let result: Result<RepositoryConfig, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "credential key 'password' must be rejected"
        );
    }

    #[test]
    fn repository_config_accepts_valid_keys() {
        let yaml = r#"url: "https://api.example.com"
auth_url: "https://auth.example.com"
project: "my-project"
"#;
        let cfg: RepositoryConfig =
            serde_yaml::from_str(yaml).expect("valid config with all known keys should parse");
        assert_eq!(cfg.url.as_deref(), Some("https://api.example.com"));
        assert_eq!(cfg.auth_url.as_deref(), Some("https://auth.example.com"));
        assert_eq!(cfg.project.as_deref(), Some("my-project"));
    }

    #[test]
    fn repository_config_accepts_partial_keys() {
        let yaml = "project: \"default-project\"\n";
        let cfg: RepositoryConfig = serde_yaml::from_str(yaml).expect("single key should parse");
        assert!(cfg.url.is_none());
        assert!(cfg.auth_url.is_none());
        assert_eq!(cfg.project.as_deref(), Some("default-project"));
    }
}
