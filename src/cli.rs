/// CLI command surface.
///
/// Implements the exact command surface from contracts/cli.md §1:
///
/// ```text
/// codemie-gitops lint  --file <path> [--output text|json]
/// codemie-gitops apply --file <path> [--url <url>]
///                      [--adopt-workflow-id <uuid>] [--output text|json]
/// codemie-gitops save  --kind <kind> --project <project> --file <path>
///                      [--slug <slug> | --name <name> | --repo-name <name>]
///                      [--id <workflow-uuid>] [--url <url>]
///                      [--output text|json]
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
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::auth::{AuthStrategy, Credentials, RawAuthSelection, login_with_strategy};
use crate::config::{ResolveConfigArgs, resolve_config};
use crate::coordinator::{self, ApplyCommand, RawApplyCommand};
use crate::lint::{self, LintCommand, RawLintCommand};
use crate::output::OutputMode;
use crate::save::SaveKind;

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

        /// Output format: `text` (default) or `json`.
        #[arg(long, default_value = "text")]
        output: OutputMode,
    },

    /// Apply declarations to CodeMie (requires network access).
    Apply {
        /// Path to the declaration YAML file.
        #[arg(long, short = 'f')]
        file: PathBuf,

        /// Target API URL. Resolved as flag > CODEMIE_URL.
        ///
        /// Accepts `https://` unconditionally and `http://` only for loopback
        /// addresses (runtime-enforced per SEC-002).
        #[arg(long, env = "CODEMIE_URL")]
        url: Option<String>,

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

    /// Save one server entity as a new local declaration.
    Save {
        #[arg(long)]
        kind: SaveKind,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        repo_name: Option<String>,
        /// Select an unmarked Workflow by its canonical hyphenated UUID.
        /// Valid only with `--kind Workflow`, together with `--slug`.
        #[arg(long = "id")]
        workflow_id: Option<String>,
        #[arg(long, short = 'f')]
        file: PathBuf,
        #[arg(long)]
        project: String,
        #[arg(long, env = "CODEMIE_URL")]
        url: Option<String>,
        #[arg(long, default_value = "text")]
        output: OutputMode,
    },

    /// Obtain and print a bearer token to stdout (no other output on success).
    Login {
        /// Target API URL, used for Mode (b) local-auth
        /// (`POST {url}/v1/local-auth/login`).
        /// Resolved as flag > CODEMIE_URL.
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

/// Read `CODEMIE_TOKEN`, rejecting a non-UTF-8 value explicitly rather
/// than silently treating it as unset (SEC-001).
fn read_token_env() -> Result<String, crate::error::AppError> {
    match std::env::var("CODEMIE_TOKEN") {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotUnicode(_)) => Err(crate::error::AppError::Configuration(
            "CODEMIE_TOKEN must be valid UTF-8".into(),
        )),
        Err(std::env::VarError::NotPresent) => Ok(String::new()),
    }
}

/// Entry point for the command dispatcher.
///
/// Returns the process exit code. Config resolution and auth-mode selection
/// are performed before any network access; any `AppError` at these stages
/// writes a safe message to stderr and returns the error's exit code.
///
/// Stdout is empty on every failure path in this function.
pub async fn run_from(args: impl IntoIterator<Item = OsString>) -> i32 {
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            // Help/version output is printed to stdout and exits 0; every
            // other clap failure (unknown flag, bad value, missing argument)
            // is reported as the single closed usage diagnostic instead of
            // clap's raw text, which could echo the offending argument
            // (SEC-005, contracts/cli.md §7).
            if error.use_stderr() {
                crate::output::write_failure(
                    &crate::error::AppError::Usage("invalid command line".into()),
                    OutputMode::Text,
                );
                return 2;
            } else {
                let _ = error.print();
                return 0;
            }
        }
    };
    match cli.command {
        Command::Lint { file, output } => {
            let command = match LintCommand::try_from(RawLintCommand { file }) {
                Ok(command) => command,
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    return error.exit_code();
                }
            };
            match lint::lint(command).await {
                Ok(result) => {
                    if result.write(output).is_ok() {
                        0
                    } else {
                        crate::output::write_failure(
                            &crate::error::AppError::Internal(
                                "failed to write command output".into(),
                            ),
                            output,
                        );
                        2
                    }
                }
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    error.exit_code()
                }
            }
        }

        Command::Apply {
            file,
            url,
            adopt_workflow_id,
            output,
        } => {
            let args = ResolveConfigArgs {
                flag_url: url,
                flag_auth_url: None,
            };
            let config = match resolve_config(&args) {
                Ok(config) => config,
                Err(e) => {
                    crate::output::write_failure(&e, output);
                    return e.exit_code();
                }
            };
            let base_url = match config.url {
                Some(url) => url,
                None => {
                    let e = crate::error::AppError::Configuration(
                        "target URL is required for apply".into(),
                    );
                    crate::output::write_failure(&e, output);
                    return e.exit_code();
                }
            };
            let token = match read_token_env() {
                Ok(token) => token,
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    return error.exit_code();
                }
            };
            let command = match ApplyCommand::try_from(RawApplyCommand {
                file,
                base_url,
                token,
                adopt_workflow_id,
            }) {
                Ok(command) => command,
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    return error.exit_code();
                }
            };
            match coordinator::apply(command).await {
                Ok(outcome) => {
                    if outcome.write(output).is_ok() {
                        0
                    } else {
                        crate::output::write_failure(
                            &crate::error::AppError::Internal(
                                "failed to write command output".into(),
                            ),
                            output,
                        );
                        2
                    }
                }
                Err(e) => {
                    crate::output::write_failure(&e, output);
                    e.exit_code()
                }
            }
        }

        Command::Save {
            kind,
            slug,
            name,
            repo_name,
            workflow_id,
            file,
            project,
            url,
            output,
        } => {
            let command = crate::save::RawSaveCommand {
                kind,
                project,
                slug,
                name,
                repo_name,
                workflow_id,
                file,
                url,
            };
            let command = match crate::save::SaveCommand::try_from(command) {
                Ok(command) => command,
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    return error.exit_code();
                }
            };
            let config = match resolve_config(&ResolveConfigArgs {
                flag_url: command.url().map(str::to_owned),
                flag_auth_url: None,
            }) {
                Ok(config) => config,
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    return error.exit_code();
                }
            };
            let base_url = match config.url {
                Some(url) => url,
                None => {
                    let error = crate::error::AppError::Configuration(
                        "target URL is required for save".into(),
                    );
                    crate::output::write_failure(&error, output);
                    return error.exit_code();
                }
            };
            let command = match command.with_resolved_url(base_url.to_string()) {
                Ok(command) => command,
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    return error.exit_code();
                }
            };
            match crate::save::save(command).await {
                Ok(outcome) => {
                    if outcome.write(output).is_ok() {
                        0
                    } else {
                        crate::output::write_failure(
                            &crate::error::AppError::Internal(
                                "failed to write command output".into(),
                            ),
                            output,
                        );
                        2
                    }
                }
                Err(error) => {
                    crate::output::write_failure(&error, output);
                    error.exit_code()
                }
            }
        }

        Command::Login {
            url,
            auth_url,
            client_id,
            email,
        } => {
            let args = ResolveConfigArgs {
                flag_url: url,
                flag_auth_url: auth_url,
            };
            let config = match resolve_config(&args) {
                Ok(c) => c,
                Err(e) => {
                    crate::output::write_failure(&e, OutputMode::default());
                    return e.exit_code();
                }
            };
            let credentials = match Credentials::from_env(client_id, email) {
                Ok(credentials) => credentials,
                Err(e) => {
                    crate::output::write_failure(&e, OutputMode::default());
                    return e.exit_code();
                }
            };
            let strategy = match AuthStrategy::try_from(RawAuthSelection {
                credentials,
                auth_url_configured: config.auth_url.is_some(),
            }) {
                Ok(strategy) => strategy,
                Err(e) => {
                    crate::output::write_failure(&e, OutputMode::default());
                    return e.exit_code();
                }
            };
            match login_with_strategy(
                strategy,
                config.url.as_ref().map(|u| u.as_str()),
                config.auth_url.as_ref().map(|u| u.as_str()),
            )
            .await
            {
                Ok(token) => {
                    // Print the bearer token to stdout, newline-terminated.
                    // No other output on the success path (contracts/cli.md §7).
                    // A broken stdout (e.g. piped to a closed consumer) is an
                    // error, not a panic: write failures are reported as a
                    // closed internal diagnostic with exit 2.
                    let mut stdout = std::io::stdout().lock();
                    if writeln!(stdout, "{token}").is_err() {
                        crate::output::write_failure(
                            &crate::error::AppError::Internal("failed to write login token".into()),
                            OutputMode::default(),
                        );
                        return 2;
                    }
                    0
                }
                Err(e) => {
                    crate::output::write_failure(&e, OutputMode::default());
                    e.exit_code()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // --- Existing: binary name and subcommand presence ---

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

    // --- Existing: SEC-001 flag rejection ---

    #[test]
    fn cli_does_not_have_token_flag_on_login() {
        let cmd = Cli::command();
        let login = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "login")
            .expect("login subcommand must exist");
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
            login
                .get_arguments()
                .all(|a| a.get_long() != Some("client-secret")),
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
            login
                .get_arguments()
                .all(|a| a.get_long() != Some("password")),
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
            apply
                .get_arguments()
                .any(|a| a.get_long() == Some("adopt-workflow-id")),
            "apply must have --adopt-workflow-id flag"
        );
    }

    #[test]
    fn output_mode_default_is_text() {
        assert_eq!("text".parse::<OutputMode>().unwrap(), OutputMode::Text);
    }

    // --- F-002: negative surface (required --file flag) ---

    #[test]
    fn lint_requires_file_flag() {
        let result = Cli::try_parse_from(["codemie-gitops", "lint"]);
        assert!(
            result.is_err(),
            "lint without --file must be rejected by clap (exit 2)"
        );
    }

    #[test]
    fn apply_requires_file_flag() {
        let result = Cli::try_parse_from(["codemie-gitops", "apply"]);
        assert!(
            result.is_err(),
            "apply without --file must be rejected by clap (exit 2)"
        );
    }

    #[test]
    fn lint_with_file_flag_parses_successfully() {
        let result = Cli::try_parse_from(["codemie-gitops", "lint", "--file", "decl.yaml"]);
        assert!(result.is_ok(), "lint with --file must parse successfully");
    }

    #[test]
    fn apply_with_file_flag_parses_successfully() {
        let result = Cli::try_parse_from(["codemie-gitops", "apply", "--file", "decl.yaml"]);
        assert!(result.is_ok(), "apply with --file must parse successfully");
    }

    #[test]
    fn repository_flags_are_absent_from_file_commands() {
        for command in ["lint", "apply", "save"] {
            let cli = Cli::command();
            let subcommand = cli
                .get_subcommands()
                .find(|candidate| candidate.get_name() == command)
                .expect("file command must exist");
            assert!(subcommand.get_arguments().all(|argument| {
                !matches!(argument.get_long(), Some("repo-root" | "follow-symlinks"))
            }));
        }
    }

    #[test]
    fn save_requires_explicit_project() {
        let result = Cli::try_parse_from([
            "codemie-gitops",
            "save",
            "--kind",
            "Skill",
            "--name",
            "skill-a",
            "--file",
            "skill.yaml",
        ]);
        assert!(result.is_err());
    }

    // --- F-002: --adopt-workflow-id only on apply ---

    #[test]
    fn lint_does_not_have_adopt_workflow_id_flag() {
        let cmd = Cli::command();
        let lint = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "lint")
            .expect("lint subcommand must exist");
        assert!(
            lint.get_arguments()
                .all(|a| a.get_long() != Some("adopt-workflow-id")),
            "--adopt-workflow-id must not exist on lint"
        );
    }

    #[test]
    fn login_does_not_have_adopt_workflow_id_flag() {
        let cmd = Cli::command();
        let login = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "login")
            .expect("login subcommand must exist");
        assert!(
            login
                .get_arguments()
                .all(|a| a.get_long() != Some("adopt-workflow-id")),
            "--adopt-workflow-id must not exist on login"
        );
    }

    #[test]
    fn lint_rejects_adopt_workflow_id_flag_at_parse() {
        let result = Cli::try_parse_from([
            "codemie-gitops",
            "lint",
            "--file",
            "decl.yaml",
            "--adopt-workflow-id",
            "00000000-0000-0000-0000-000000000001",
        ]);
        assert!(
            result.is_err(),
            "--adopt-workflow-id on lint must be rejected by clap"
        );
    }

    // --- F-002: secret flags must be rejected at parse time ---

    #[test]
    fn login_rejects_unknown_token_flag() {
        let result = Cli::try_parse_from(["codemie-gitops", "login", "--token", "secret-value"]);
        assert!(
            result.is_err(),
            "--token flag must be rejected by clap (SEC-001, E_USAGE)"
        );
    }

    #[test]
    fn login_rejects_unknown_client_secret_flag() {
        let result =
            Cli::try_parse_from(["codemie-gitops", "login", "--client-secret", "secret-value"]);
        assert!(
            result.is_err(),
            "--client-secret flag must be rejected by clap (SEC-001, E_USAGE)"
        );
    }

    #[test]
    fn login_rejects_unknown_password_flag() {
        let result = Cli::try_parse_from(["codemie-gitops", "login", "--password", "secret-value"]);
        assert!(
            result.is_err(),
            "--password flag must be rejected by clap (SEC-001, E_USAGE)"
        );
    }

    // --- F-002: login parses successfully with non-secret flags ---

    #[test]
    fn login_parses_with_client_id_and_email() {
        let result = Cli::try_parse_from([
            "codemie-gitops",
            "login",
            "--client-id",
            "my-client",
            "--email",
            "user@example.com",
        ]);
        assert!(
            result.is_ok(),
            "login with --client-id and --email must parse"
        );
    }

    #[test]
    fn login_parses_without_any_flags() {
        let result = Cli::try_parse_from(["codemie-gitops", "login"]);
        assert!(
            result.is_ok(),
            "login with no flags must parse (all optional)"
        );
    }
}
