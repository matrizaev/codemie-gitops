use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize structured tracing early so any diagnostic output produced
    // during startup is captured. The RUST_LOG env var controls the filter;
    // JSON format is available via RUST_LOG_FORMAT=json (configured in T-002).
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    codemie_gitops::run(std::env::args_os()).await
}
