/// CLI command surface.
///
/// Implements the exact command surface from contracts/cli.md §1:
///
/// ```text
/// codemie-gitops lint  --file <path> [--repo-root <path>]
///                      [--follow-symlinks] [--output text|json]
/// codemie-gitops apply --file <path> [--repo-root <path>] [--url <url>]
///                      [--follow-symlinks]
///                      [--adopt-workflow-id <uuid>] [--output text|json]
/// codemie-gitops login [--url <url>] [--auth-url <url>]
///                      [--client-id <id>] [--email <email>]
/// ```
///
/// Security invariants (SEC-001, ADR-011):
/// - `--token`, `--client-secret`, `--password` flags are NOT defined and
///   MUST NOT be accepted. Any unknown flag causes clap to exit with code 2.
/// - Secret credentials (`CODEMIE_TOKEN`, `CODEMIE_CLIENT_SECRET`,
///   `CODEMIE_PASSWORD`) are resolved from environment only at runtime.
/// - Non-secret selectors (`--client-id`, `--email`) may be supplied as flags.
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::output::OutputMode;

/// The top-level CLI structure.
#[derive(Parser, Debug)]
#[command(
    name = "codemie-gitops",
    version,
    about = "CodeMie declarative CI/CD CLI",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Validate declarations offline (no network access).
    Lint {
        /// Path to the declaration YAML file.
        #[arg(long, short = 'f')]
        file: PathBuf,

        /// Repository root directory. Defaults to the nearest ancestor with
        /// a `.git` directory.
        #[arg(long)]
        repo_root: Option<PathBuf>,

        /// Follow symbolic links during discovery and sidecar reads.
        #[arg(long)]
        follow_symlinks: bool,

        /// Output format: `text` (default) or `json`.
        #[arg(long, default_value = "text")]
        output: OutputMode,
    },

    /// Apply declarations to CodeMie (requires network access).
    Apply {
        /// Path to the declaration YAML file.
        #[arg(long, short = 'f')]
        file: PathBuf,

        /// Repository root directory. Defaults to the nearest ancestor with
        /// a `.git` directory.
        #[arg(long)]
        repo_root: Option<PathBuf>,

        /// Target API URL. Resolved as flag > CODEMIE_URL > config url.
        ///
        /// Accepts `https://` unconditionally and `http://` only for loopback
        /// addresses (runtime-enforced per SEC-002).
        #[arg(long, env = "CODEMIE_URL")]
        url: Option<String>,

        /// Follow symbolic links during discovery and sidecar reads.
        #[arg(long)]
        follow_symlinks: bool,

        /// Adopt an existing Workflow entity by its canonical UUID.
        ///
        /// Invocation selector only; never YAML, local state, or output.
        /// Valid only for Workflow declarations.
        #[arg(long)]
        adopt_workflow_id: Option<String>,

        /// Output format: `text` (default) or `json`.
        #[arg(long, default_value = "text")]
        output: OutputMode,
    },

    /// Obtain and print a bearer token to stdout (no other output on success).
    Login {
        /// Target API URL, used for Mode (b) local-auth
        /// (`POST {url}/v1/local-auth/login`).
        /// Resolved as flag > CODEMIE_URL > config url.
        #[arg(long, env = "CODEMIE_URL")]
        url: Option<String>,

        /// Keycloak authentication endpoint, used for Mode (a) and Mode (c).
        /// Resolved as flag > CODEMIE_AUTH_URL > config auth_url.
        /// HTTPS required; `http://` is E_CONFIGURATION, exit 2.
        #[arg(long, env = "CODEMIE_AUTH_URL")]
        auth_url: Option<String>,

        /// Client ID (non-secret selector).
        /// Resolved as flag > CODEMIE_CLIENT_ID.
        /// Defaults to `codemie-sdk` for Mode (c) when unset.
        #[arg(long, env = "CODEMIE_CLIENT_ID")]
        client_id: Option<String>,

        /// Email address (non-secret selector).
        /// Resolved as flag > CODEMIE_EMAIL.
        #[arg(long, env = "CODEMIE_EMAIL")]
        email: Option<String>,
    },
}

/// Entry point for the command dispatcher.
///
/// Returns the process exit code. This is a stub; full command dispatch
/// including config resolution, validation, and network calls is implemented
/// across F-002 through R-001.
pub async fn run() -> i32 {
    let cli = Cli::parse();
    match cli.command {
        Command::Lint { .. } => {
            todo!("lint implemented in F-002 through F-005")
        }
        Command::Apply { .. } => {
            todo!("apply implemented in F-002 through R-001")
        }
        Command::Login { .. } => {
            todo!("login implemented in F-002 and T-001")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_binary_name_is_codemie_gitops() {
        let cmd = Cli::command();
        assert_eq!(cmd.get_name(), "codemie-gitops");
    }

    #[test]
    fn cli_has_lint_subcommand() {
        let cmd = Cli::command();
        assert!(
            cmd.get_subcommands().any(|s| s.get_name() == "lint"),
            "lint subcommand must be defined"
        );
    }

    #[test]
    fn cli_has_apply_subcommand() {
        let cmd = Cli::command();
        assert!(
            cmd.get_subcommands().any(|s| s.get_name() == "apply"),
            "apply subcommand must be defined"
        );
    }

    #[test]
    fn cli_has_login_subcommand() {
        let cmd = Cli::command();
        assert!(
            cmd.get_subcommands().any(|s| s.get_name() == "login"),
            "login subcommand must be defined"
        );
    }

    #[test]
    fn cli_does_not_have_token_flag_on_login() {
        let cmd = Cli::command();
        let login = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "login")
            .expect("login subcommand must exist");
        // SEC-001: --token must not be an accepted flag.
        assert!(
            login.get_arguments().all(|a| a.get_long() != Some("token")),
            "--token flag must not exist on login (SEC-001)"
        );
    }

    #[test]
    fn cli_does_not_have_client_secret_flag_on_login() {
        let cmd = Cli::command();
        let login = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "login")
            .expect("login subcommand must exist");
        assert!(
            login.get_arguments().all(|a| a.get_long() != Some("client-secret")),
            "--client-secret flag must not exist on login (SEC-001)"
        );
    }

    #[test]
    fn cli_does_not_have_password_flag_on_login() {
        let cmd = Cli::command();
        let login = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "login")
            .expect("login subcommand must exist");
        assert!(
            login.get_arguments().all(|a| a.get_long() != Some("password")),
            "--password flag must not exist on login (SEC-001)"
        );
    }

    #[test]
    fn apply_has_url_flag() {
        let cmd = Cli::command();
        let apply = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "apply")
            .expect("apply subcommand must exist");
        assert!(
            apply.get_arguments().any(|a| a.get_long() == Some("url")),
            "apply must have --url flag"
        );
    }

    #[test]
    fn apply_has_adopt_workflow_id_flag() {
        let cmd = Cli::command();
        let apply = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "apply")
            .expect("apply subcommand must exist");
        assert!(
            apply.get_arguments().any(|a| a.get_long() == Some("adopt-workflow-id")),
            "apply must have --adopt-workflow-id flag"
        );
    }

    #[test]
    fn output_mode_default_is_text() {
        assert_eq!("text".parse::<OutputMode>().unwrap(), OutputMode::Text);
    }
}
