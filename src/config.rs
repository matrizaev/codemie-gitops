//! Validated runtime configuration sourced only from flags and environment.

use crate::error::AppError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("URL contains control characters")]
    ControlCharacters,
    #[error("URL is not syntactically valid")]
    Syntax(#[source] url::ParseError),
    #[error("URL violates the configured origin policy: {0}")]
    Policy(&'static str),
}

/// A validated CodeMie API origin.
#[derive(Debug, Clone)]
pub(crate) struct ValidatedUrl(url::Url);

impl ValidatedUrl {
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn join_api_path(&self, path: &str) -> url::Url {
        let (path, query) = path
            .split_once('?')
            .map_or((path, None), |(path, query)| (path, Some(query)));
        let base_path = self.0.path().trim_end_matches('/');
        let request_path = path.trim_start_matches('/');
        let joined_path = if base_path.is_empty() {
            format!("/{request_path}")
        } else {
            format!("{base_path}/{request_path}")
        };
        let mut joined = self.0.clone();
        joined.set_path(&joined_path);
        joined.set_query(query);
        joined
    }
}

impl std::fmt::Display for ValidatedUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for ValidatedUrl {
    type Error = ConfigError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        reject_controls(raw)?;
        let parsed = url::Url::parse(raw).map_err(ConfigError::Syntax)?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(ConfigError::Policy("scheme must be http or https"));
        }
        if parsed.fragment().is_some() {
            return Err(ConfigError::Policy("fragment is not permitted"));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(ConfigError::Policy("userinfo is not permitted"));
        }
        if parsed.host_str().is_none_or(str::is_empty) {
            return Err(ConfigError::Policy("host must be non-empty"));
        }
        if parsed.scheme() == "http" && !is_loopback_http(&parsed) {
            return Err(ConfigError::Policy(
                "http is permitted only for loopback destinations",
            ));
        }
        Ok(Self(parsed))
    }
}

impl TryFrom<String> for ValidatedUrl {
    type Error = ConfigError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::try_from(raw.as_str())
    }
}

/// A validated HTTPS identity-provider endpoint.
#[derive(Debug, Clone)]
pub(crate) struct ValidatedAuthUrl(url::Url);

impl ValidatedAuthUrl {
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for ValidatedAuthUrl {
    type Error = ConfigError;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        let validated = ValidatedUrl::try_from(raw)?;
        if validated.0.scheme() != "https" {
            return Err(ConfigError::Policy(
                "authentication endpoint must use https",
            ));
        }
        Ok(Self(validated.0))
    }
}

impl TryFrom<String> for ValidatedAuthUrl {
    type Error = ConfigError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::try_from(raw.as_str())
    }
}

fn reject_controls(raw: &str) -> Result<(), ConfigError> {
    if raw.chars().any(|character| {
        let codepoint = character as u32;
        codepoint <= 0x1f || (0x7f..=0x9f).contains(&codepoint)
    }) {
        Err(ConfigError::ControlCharacters)
    } else {
        Ok(())
    }
}

fn is_loopback_http(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => domain == "localhost",
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

/// Raw non-secret configuration already resolved by clap from flag or env.
pub(crate) struct ResolveConfigArgs {
    pub(crate) flag_url: Option<String>,
    pub(crate) flag_auth_url: Option<String>,
}

/// Validated runtime endpoints.
pub(crate) struct ResolvedConfig {
    pub(crate) url: Option<ValidatedUrl>,
    pub(crate) auth_url: Option<ValidatedAuthUrl>,
}

pub(crate) fn resolve_config(args: &ResolveConfigArgs) -> Result<ResolvedConfig, AppError> {
    Ok(ResolvedConfig {
        url: args
            .flag_url
            .as_deref()
            .map(ValidatedUrl::try_from)
            .transpose()?,
        auth_url: args
            .flag_auth_url
            .as_deref()
            .map(ValidatedAuthUrl::try_from)
            .transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_url_accepts_https_and_loopback_http() {
        assert!(ValidatedUrl::try_from("https://api.example.com/base").is_ok());
        assert!(ValidatedUrl::try_from("http://127.0.0.1:8000").is_ok());
        assert!(ValidatedUrl::try_from("http://localhost:8000").is_ok());
    }

    #[test]
    fn target_url_rejects_remote_http_userinfo_and_fragments() {
        assert!(ValidatedUrl::try_from("http://example.com").is_err());
        assert!(ValidatedUrl::try_from("https://user@example.com").is_err());
        assert!(ValidatedUrl::try_from("https://example.com/#fragment").is_err());
    }

    #[test]
    fn auth_url_requires_https() {
        assert!(ValidatedAuthUrl::try_from("https://id.example.com/token").is_ok());
        assert!(ValidatedAuthUrl::try_from("http://localhost/token").is_err());
    }
}
