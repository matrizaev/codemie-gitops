/// Safe HTTP transport stub.
///
/// Provides a `rustls`-backed reqwest client with the security settings
/// required by contracts/http-adapter.md and ADR-011:
/// - TLS via rustls (no OpenSSL runtime dependency; musl-compatible).
/// - Per-request timeout: 60 seconds (SEC-003).
/// - Invocation deadline: 300 seconds propagated to all operations.
/// - Redirects disabled for authentication POST requests.
/// - Response body bounded to 8 MiB; header block bounded to 16 KiB.
/// - Only GET requests may be retried (max 3 attempts with jitter).
///
/// Full transport implementation is in T-002.
use crate::error::AppError;

/// A wrapper around a configured reqwest HTTP client.
#[derive(Debug)]
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    /// Construct a new HTTP client with the required security settings.
    ///
    /// This is a stub; complete timeout, redirect, and TLS configuration
    /// is implemented in T-002.
    pub fn new() -> Result<Self, AppError> {
        let inner = reqwest::Client::builder()
            .use_rustls_tls()
            .build()
            .map_err(|e| AppError::Internal(format!("failed to build HTTP client: {e}")))?;
        Ok(HttpClient { inner })
    }

    /// Return a reference to the inner reqwest client.
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_client_constructs() {
        let client = HttpClient::new().expect("HTTP client must construct without errors");
        // Verify the inner client is accessible.
        let _inner: &reqwest::Client = client.inner();
    }
}
