mod adapters;
mod auth;
mod cli;
mod config;
mod discovery;
mod error;
mod http;
mod index;
mod output;
mod projection;
mod reference;
mod render;
mod schema;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize structured tracing early so any diagnostic output produced
    // during startup is captured. The RUST_LOG env var controls the filter;
    // JSON format is available via RUST_LOG_FORMAT=json (configured in T-002).
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let exit_code = cli::run().await;
    std::process::exit(exit_code);
}
