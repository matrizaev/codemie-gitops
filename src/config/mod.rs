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

/// Fully resolved, validated configuration for a single invocation.
///
/// All fields have been merged from flag > env > config precedence and
/// validated as `ValidatedUrl` where applicable. Credential fields are never
/// stored here — they are read from the environment at the auth boundary
/// (auth/mod.rs) and never persisted.
#[derive(Debug, Default)]
pub struct ResolvedConfig {
    /// Resolved target API URL. `None` means not configured.
    pub url: Option<String>,

    /// Resolved Keycloak authentication endpoint. `None` means not configured.
    pub auth_url: Option<String>,

    /// Resolved effective default project. `None` means not configured.
    pub project: Option<String>,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resolved_config_is_all_none() {
        let cfg = ResolvedConfig::default();
        assert!(cfg.url.is_none());
        assert!(cfg.auth_url.is_none());
        assert!(cfg.project.is_none());
    }

    #[test]
    fn repository_config_rejects_credential_keys() {
        // The schema forbids credential keys; serde deny_unknown_fields
        // enforces this at deserialisation.
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
        assert!(result.is_err(), "credential key 'client_secret' must be rejected");
    }

    #[test]
    fn repository_config_accepts_valid_keys() {
        let yaml = r#"url: "https://api.example.com"
auth_url: "https://auth.example.com"
project: "my-project"
"#;
        let cfg: RepositoryConfig = serde_yaml::from_str(yaml)
            .expect("valid config with all known keys should parse");
        assert_eq!(cfg.url.as_deref(), Some("https://api.example.com"));
        assert_eq!(cfg.auth_url.as_deref(), Some("https://auth.example.com"));
        assert_eq!(cfg.project.as_deref(), Some("my-project"));
    }

    #[test]
    fn repository_config_accepts_partial_keys() {
        let yaml = "project: \"default-project\"\n";
        let cfg: RepositoryConfig =
            serde_yaml::from_str(yaml).expect("single key should parse");
        assert!(cfg.url.is_none());
        assert!(cfg.auth_url.is_none());
        assert_eq!(cfg.project.as_deref(), Some("default-project"));
    }
}
