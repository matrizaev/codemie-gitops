/// T-003: Pinned compatibility preflight.
///
/// Validates consumed target capabilities against the checked-in manifest
/// before write operations. Architecture: ADR-004.
///
/// ## Policy (from `specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json`
/// `compatibilityPolicy`)
///
/// - `infoEndpointIsIdentity: false` — `/v1/info` alone CANNOT gate writes.
///   A successful `check_compatibility` result does NOT prove full server
///   compatibility. The coordinator (R-001) MUST ALSO invoke
///   `http::preflight_visibility` before any modifying operation. This function
///   is one additive step in the write gate, not the complete gate.
///
/// - `missingOrInvalidConsumedResponseField: "fail-before-write"` — if any
///   consumed field is absent or invalid in the `GET /v1/info` response, fail
///   before any write with `AppError::ApiIncompatible`.
///
/// ## Source traceability
///
/// - Requirements: IR-002/005/008–010, QR-009
/// - Architecture: ADR-004
/// - Task: T-003
/// - Manifest baseline: `specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json`
///   §source.backend.commit = `2a481c290c99bf30ef80aadafa03d876a7f5f732`
use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::ApiClient;

// ---------------------------------------------------------------------------
// Pinned manifest constant
// ---------------------------------------------------------------------------

/// Expected backend commit SHA from `adapter-manifest-v2.42.0.json`
/// §source.backend.commit.
///
/// Hard-coded as a compile-time constant so that any inadvertent edit to the
/// pinned SHA is caught at test-compile time without requiring a file-system
/// read at runtime.
///
/// Source baseline: tag `2.42.0`, commit
/// `2a481c290c99bf30ef80aadafa03d876a7f5f732`
/// (see `specs/codemie-cicd-tool/contracts/source-baseline.md`).
pub const EXPECTED_BACKEND_COMMIT: &str = "2a481c290c99bf30ef80aadafa03d876a7f5f732";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Diagnostic record produced when `check_compatibility` succeeds.
///
/// Consumers may inspect this for tracing or audit purposes. The
/// `server_commit` field holds the `version` string returned by `GET /v1/info`;
/// in a correctly configured deployment the server reports its source commit
/// SHA as `APP_VERSION`.
///
/// `compatible` is `true` only when `server_commit == expected_commit`.
/// Per `infoEndpointIsIdentity: false`, a `compatible: true` result is
/// necessary but NOT sufficient for write authorization — the coordinator
/// must also verify visibility via `preflight_visibility`.
#[derive(Debug)]
pub struct CompatibilityResult {
    /// Version string returned by `GET /v1/info` (the `version` field).
    ///
    /// Never forwarded to logs or user-visible output (SEC-005).
    pub server_commit: String,

    /// Expected commit SHA from the pinned manifest.
    pub expected_commit: String,

    /// `true` when `server_commit == expected_commit`.
    pub compatible: bool,
}

/// Fields consumed from the `GET /v1/info` response.
///
/// Only `version` is consumed; `message` and `description` are present in the
/// wire response but are intentionally not decoded here — server-supplied text
/// strings must not enter logs or tracing events (SEC-005).
///
/// The non-optional `version: String` field enforces the
/// `missingOrInvalidConsumedResponseField: "fail-before-write"` policy: if
/// the field is absent, `serde_json` returns an error which `ApiClient`
/// maps to `AppError::ApiIncompatible` before any write proceeds.
#[derive(serde::Deserialize)]
struct InfoResponse {
    /// Server-reported version, used for the pinned-manifest comparison.
    version: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check runtime compatibility against the pinned manifest.
///
/// This function is **additive** and must be called **before every write
/// operation**. It does NOT block reads or lint operations.
///
/// ## Steps (all must pass)
///
/// 1. Issue `GET {base_url}/v1/info`. A network failure or non-2xx status
///    propagates as the appropriate `AppError` variant (e.g.
///    `AppError::ApiIncompatible` for 5xx, `AppError::Authentication` for 401).
///
/// 2. Decode the response into `InfoResponse`. If the consumed `version` field
///    is absent or the body is not valid JSON, the `ApiClient` deserialization
///    path returns `AppError::ApiIncompatible` — satisfying the
///    `missingOrInvalidConsumedResponseField: "fail-before-write"` policy.
///
/// 3. Compare `version` against `EXPECTED_BACKEND_COMMIT`. A mismatch →
///    `AppError::ApiIncompatible`.
///
/// ## `infoEndpointIsIdentity: false`
///
/// Even when this function returns `Ok(())`, the caller MUST NOT treat it as
/// the sole compatibility gate. Callers (R-001) must additionally invoke
/// `http::preflight_visibility` before any modifying operation.
///
/// ## Security
///
/// - The `version` string and server response body are never written to
///   tracing events or diagnostic output (SEC-005).
/// - Error messages contain only safe static strings; no server-supplied
///   values are forwarded (SEC-005).
pub async fn check_compatibility(
    client: &ApiClient,
    base_url: &ValidatedUrl,
) -> Result<(), AppError> {
    let span = tracing::info_span!("preflight.check_compatibility");
    let _enter = span.enter();

    tracing::debug!("preflight: issuing GET /v1/info");

    // Step 1 + 2: call the endpoint and decode consumed fields.
    // Any transport or decoding error propagates directly — the response body
    // and any internal error description are discarded by ApiClient (SEC-005).
    let info: InfoResponse = client.get(base_url, "/v1/info").await?;

    // Step 3: compare version against the pinned commit SHA.
    let result = CompatibilityResult {
        server_commit: info.version.clone(),
        expected_commit: EXPECTED_BACKEND_COMMIT.to_owned(),
        compatible: info.version == EXPECTED_BACKEND_COMMIT,
    };

    if result.compatible {
        // Version string intentionally omitted from the log event (SEC-005).
        tracing::info!("preflight: server version matches pinned manifest commit");
        Ok(())
    } else {
        // Version string intentionally omitted from the log event (SEC-005).
        tracing::warn!("preflight: server version does not match pinned manifest commit");
        Err(AppError::ApiIncompatible(
            "server version does not match pinned manifest commit".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests (T-003)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn test_url(base: &str) -> ValidatedUrl {
        ValidatedUrl::try_from(base).expect("test URL must be valid")
    }

    fn test_client(base: &str) -> ApiClient {
        ApiClient::new(test_url(base), "test-token".into())
            .expect("ApiClient must construct in tests")
    }

    // -----------------------------------------------------------------------
    // T-003 AC: matching commit → Ok
    //
    // Mock server returns the pinned commit SHA in the `version` field.
    // In a correctly configured deployment `APP_VERSION` is set to the source
    // commit SHA, so the equality check passes.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_compatibility_matching_commit_returns_ok() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"version":"{commit}","message":"Codemie","description":"CI/CD"}}"#,
                commit = EXPECTED_BACKEND_COMMIT
            ))
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        check_compatibility(&client, &url)
            .await
            .expect("matching commit must return Ok");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-003 AC: non-matching version → AppError::ApiIncompatible
    //
    // When the server reports a version that differs from the pinned commit,
    // writes must be blocked.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_compatibility_mismatched_version_returns_api_incompatible() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            // Returns a version string that does NOT match the pinned commit.
            .with_body(r#"{"version":"0.16.0","message":"Codemie","description":"CI/CD"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = check_compatibility(&client, &url)
            .await
            .expect_err("version mismatch must produce an error");
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "version mismatch must produce AppError::ApiIncompatible, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-003 AC: missing consumed `version` field → AppError::ApiIncompatible
    //
    // The manifest policy `missingOrInvalidConsumedResponseField:
    // "fail-before-write"` requires that a response missing the consumed
    // `version` field be rejected before any write.
    //
    // Mechanism: `version: String` (non-optional) in `InfoResponse`; serde
    // deserialization fails → `ApiClient` maps it to `AppError::ApiIncompatible`.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_compatibility_missing_version_field_returns_api_incompatible() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            // Response body has no `version` field.
            .with_body(r#"{"message":"Codemie","description":"CI/CD"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = check_compatibility(&client, &url)
            .await
            .expect_err("missing version field must produce an error");
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "missing version field must produce AppError::ApiIncompatible, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-003 AC: server 5xx → AppError::ApiIncompatible (via GET retry path)
    //
    // A server error on `GET /v1/info` blocks compatibility acceptance and
    // therefore blocks any subsequent write.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_compatibility_5xx_returns_api_incompatible() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/info")
            .with_status(500)
            .expect_at_least(1)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = check_compatibility(&client, &url)
            .await
            .expect_err("5xx on GET /v1/info must produce an error");
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "5xx must produce AppError::ApiIncompatible, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
    }

    // -----------------------------------------------------------------------
    // T-003 AC: 404 → AppError::ApiIncompatible
    //
    // A 4xx (other than 401/403) on `GET /v1/info` fails before write.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_compatibility_404_returns_api_incompatible() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/info")
            .with_status(404)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = check_compatibility(&client, &url)
            .await
            .expect_err("404 on GET /v1/info must produce an error");
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "404 must produce AppError::ApiIncompatible, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-003 AC: `infoEndpointIsIdentity: false` — version match is additive
    //
    // Even when `check_compatibility` returns `Ok(())`, the caller MUST run
    // additional preflight steps (e.g. `preflight_visibility`) before any
    // write. `check_compatibility` is not the sole gate.
    //
    // This test verifies the behavioral boundary: the function succeeds when
    // the version matches but makes no claim about full compatibility.
    // R-001 enforces the "both checks required" invariant; this test confirms
    // that `check_compatibility` in isolation does NOT block a valid version.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_compatibility_version_match_is_one_of_multiple_required_checks() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"version":"{commit}","message":"Codemie","description":"CI/CD"}}"#,
                commit = EXPECTED_BACKEND_COMMIT
            ))
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());

        // check_compatibility returns Ok — the version check passes.
        // NOTE: The coordinator (R-001) is responsible for also calling
        // `http::preflight_visibility` before any write. `check_compatibility`
        // alone does not prove write authorization (infoEndpointIsIdentity: false).
        check_compatibility(&client, &url)
            .await
            .expect("matching version must return Ok for its additive check");
        _mock.assert_async().await;

        // No write is issued here; the test confirms only that the additive
        // check itself passes. The GET-only nature of this function means it
        // cannot perform writes regardless of its return value.
    }

    // -----------------------------------------------------------------------
    // T-003 AC: empty JSON body → AppError::ApiIncompatible
    //
    // An empty or non-JSON body fails strict decoding before write.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn check_compatibility_empty_body_returns_api_incompatible() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{}")
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = check_compatibility(&client, &url)
            .await
            .expect_err("empty JSON object must fail because 'version' field is absent");
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "empty body must produce AppError::ApiIncompatible, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // Compile-time invariant: EXPECTED_BACKEND_COMMIT matches the manifest
    //
    // This test will fail to compile if the constant is accidentally changed
    // from the reviewed manifest value, providing a lightweight audit guard.
    // -----------------------------------------------------------------------

    #[test]
    fn expected_backend_commit_matches_manifest_value() {
        // Value taken directly from
        // `specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json`
        // §source.backend.commit.
        assert_eq!(
            EXPECTED_BACKEND_COMMIT,
            "2a481c290c99bf30ef80aadafa03d876a7f5f732",
            "EXPECTED_BACKEND_COMMIT must match the pinned manifest value"
        );
    }
}
