#![doc = "Library facade for the CodeMie declarative CI/CD CLI."]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod adapters;
mod auth;
mod cli;
mod config;
mod coordinator;
mod declaration_schema;
mod domain;
mod error;
mod http;
mod input;
mod lint;
mod output;
mod pagination;
mod parse;
mod projection;
mod render;
mod save;
mod schema;
mod strict_json;
mod validate;

use std::ffi::OsString;
use std::process::ExitCode;

/// Runs one CLI invocation and returns its process exit status.
///
/// Arguments must include the binary name as their first item, matching
/// [`std::env::args_os`]. All diagnostics and outcomes are written through the
/// stable CLI presentation boundary.
pub async fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let code = cli::run_from(args).await;
    match u8::try_from(code) {
        Ok(code) => ExitCode::from(code),
        Err(_) => ExitCode::FAILURE,
    }
}
