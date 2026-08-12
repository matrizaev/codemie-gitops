/// Safe authenticated HTTP transport — T-002.
///
/// Implements `ApiClient`, the authenticated API client for CodeMie REST
/// operations (apply/lint network calls, `GET /v1/user` preflight, etc.).
/// This is distinct from the auth client in `auth/mod.rs`, which is for the
/// Keycloak token endpoint only.
///
/// Security invariants (ADR-011 §4, SEC-001/002/003/005):
/// - TLS via rustls: no OpenSSL runtime dependency; musl-compatible.
/// - All redirects disabled (`redirect::Policy::none()`; preferred per ADR-011 §4).
/// - Bearer token sent only in the `Authorization` header; never in URL or logs.
/// - Per-request timeout: 60 s (SEC-003, http-adapter.md §2.4).
/// - Response body bounded to 8 MiB; excess is classified as `E_API_INCOMPATIBLE`.
/// - Response JSON nesting depth bounded to 64 levels.
/// - GET retries up to 3 times on transient failures (5xx / 429 / connect error).
/// - POST / PUT / DELETE are never blindly retried.
/// - No credential value appears in tracing events or error messages (SEC-001/005).
use std::{sync::OnceLock, time::Duration};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::adapters::{ModificationMethod, PreparedRequest, PreparedWrite, PreparedWriteResponse};
use crate::config::ValidatedUrl;
use crate::error::AppError;

static RUSTLS_PROVIDER: OnceLock<()> = OnceLock::new();

/// Install the process-wide rustls ring crypto provider once.
///
/// Reqwest 0.13's `rustls-no-provider` feature deliberately leaves provider
/// selection to the application. If another component installed a provider
/// first, rustls keeps that process-wide selection.
pub(crate) fn ensure_rustls_provider() {
    RUSTLS_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

// ---------------------------------------------------------------------------
// Resource budget constants (SEC-003, http-adapter.md §2.4)
// ---------------------------------------------------------------------------

/// Maximum response body size: 8 MiB.
const RESPONSE_BODY_LIMIT: usize = 8 * 1024 * 1024;

/// Per-request timeout in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 60;

/// Maximum GET retry attempts (first attempt + up to 2 retries).
const GET_MAX_RETRIES: u32 = 3;

/// Maximum JSON nesting depth for response decoding.
const JSON_MAX_DEPTH: usize = 64;

/// Base jitter between GET retry attempts in milliseconds (attempt × base).
///
/// Reduced in tests to keep the suite fast while preserving the retry logic path.
#[cfg(not(test))]
const RETRY_BASE_JITTER_MS: u64 = 200;
#[cfg(test)]
const RETRY_BASE_JITTER_MS: u64 = 5;

// ---------------------------------------------------------------------------
// User response for capability preflight (GET /v1/user)
// ---------------------------------------------------------------------------

/// Fields consumed from `GET /v1/user` per `adapter-manifest-v2.42.0.json`.
///
/// Consumed: `user_id` and `projects[].name`. Role fields are not v32
/// authorization evidence.
/// Role values are never forwarded to logs or output (SEC-005).
#[derive(serde::Deserialize)]
struct UserResponse {
    /// Stable authenticated principal identifier.
    user_id: String,
    /// Per-project membership list.
    projects: Vec<UserProject>,
}

/// Per-project membership entry.
#[derive(serde::Deserialize)]
struct UserProject {
    /// Exact project identifier attached to this membership entry.
    name: String,
}

/// Sealed proof that the principal can exhaustively resolve one exact project.
#[derive(Debug, Clone)]
pub(crate) struct ExactProjectVisibility {
    effective_project: String,
    authenticated_user_id: String,
}

impl ExactProjectVisibility {
    pub(crate) fn matches(&self, effective_project: &str) -> bool {
        self.effective_project == effective_project
    }

    pub(crate) fn authenticated_user_id(&self) -> &str {
        &self.authenticated_user_id
    }
}

// ---------------------------------------------------------------------------
// ApiClient
// ---------------------------------------------------------------------------

/// Authenticated API client for CodeMie REST operations (T-002).
///
/// Holds a `reqwest::Client` pre-configured per ADR-011 §4 and SEC-002/003:
/// - rustls-tls only (no OpenSSL)
/// - 60-second per-request timeout
/// - `User-Agent: codemie-gitops/{version}`
/// - All redirects disabled (`redirect::Policy::none()`)
///
/// The bearer token is stored in the struct. It is **never** written to
/// tracing events or error messages (SEC-001/005).
pub struct ApiClient {
    /// Reqwest client shared across all requests on this instance.
    client: reqwest::Client,
    /// Bearer token. Never logged.
    token: String,
    /// Base URL bound at construction; methods accept per-call URL overrides.
    #[allow(dead_code)]
    base_url: ValidatedUrl,
}

impl ApiClient {
    /// Construct a new `ApiClient`.
    ///
    /// Builds the reqwest client once; all requests reuse it. `use_rustls_tls()`
    /// ensures no OpenSSL runtime dependency. `redirect::Policy::none()` disables
    /// all redirects (preferred per ADR-011 §4).
    pub fn new(base_url: ValidatedUrl, token: String) -> Result<Self, AppError> {
        ensure_rustls_provider();
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("codemie-gitops/", env!("CARGO_PKG_VERSION")))
            .build()
            // reqwest error intentionally discarded: it contains no actionable info
            // and we do not leak internal TLS or networking details (SEC-005).
            .map_err(|_e| AppError::Internal("failed to build API client".into()))?;
        Ok(ApiClient {
            client,
            token,
            base_url,
        })
    }

    /// Build the `Authorization: Bearer <token>` header value.
    ///
    /// The token string is never logged (SEC-001/005). If the token contains
    /// non-ASCII characters that make the header value invalid, a fixed
    /// placeholder is used; the server will return 401 → `Authentication`.
    fn auth_header_value(&self) -> reqwest::header::HeaderValue {
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.token)).unwrap_or_else(
            |_e| {
                // Non-ASCII token — fall back to a syntactically safe placeholder.
                reqwest::header::HeaderValue::from_static("Bearer invalid")
            },
        )
    }

    /// Construct the full request URL from a base `ValidatedUrl` and a path.
    ///
    /// Strips a trailing `/` from the base and a leading `/` from the path to
    /// prevent double-slash collisions. Callers supply `/v1/…`-style paths.
    fn join_url(url: &ValidatedUrl, path: &str) -> String {
        let base = url.as_str().trim_end_matches('/');
        let rest = path.trim_start_matches('/');
        format!("{base}/{rest}")
    }

    /// Map a reqwest send error to `AppError::Connectivity`.
    ///
    /// The reqwest error is discarded to prevent leaking internal URL, IP
    /// address, or TLS handshake details into the error chain (SEC-005).
    fn map_send_error(_e: reqwest::Error) -> AppError {
        AppError::Connectivity("API endpoint unreachable".into())
    }

    /// Map a modifying-request send failure. Once dispatch has started the
    /// client cannot prove that the server did not commit the request.
    fn map_modifying_send_error(_e: reqwest::Error) -> AppError {
        AppError::WriteUncertain("modifying request may have reached the server".into())
    }

    /// Classify a non-2xx status into an `AppError`.
    ///
    /// - 401 → `Authentication`
    /// - 403 → `Authorization`
    /// - 3xx when `is_modifying` is true → `Authentication` (ADR-011 §4:
    ///   redirects disabled on modifying requests; credentials must not be
    ///   replayed to a different origin)
    /// - Other non-2xx → `ApiIncompatible` (status code is safe metadata)
    fn classify_error_status(status: reqwest::StatusCode, is_modifying: bool) -> AppError {
        let code = status.as_u16();
        if code == 401 {
            return AppError::Authentication("API request unauthorized".into());
        }
        if code == 403 {
            return AppError::Authorization("API request forbidden".into());
        }
        if status.is_redirection() && is_modifying {
            // 3xx on POST/PUT/DELETE: the client will not follow the redirect.
            // Treat as an authentication/configuration error so the operator
            // investigates the endpoint configuration (ADR-011 §4).
            return AppError::Authentication(
                "API endpoint returned redirect on modifying request".into(),
            );
        }
        if is_modifying && matches!(code, 400 | 404 | 409 | 422) {
            return AppError::ServerRejected(format!("API rejected write with status {code}"));
        }
        AppError::ApiIncompatible(format!("API returned status {code}"))
    }

    /// Read the response body up to `RESPONSE_BODY_LIMIT` (8 MiB).
    ///
    /// Rejects early via `Content-Length` where possible; always re-checks
    /// the actual byte count after the read (SEC-003).
    async fn bounded_body(resp: reqwest::Response) -> Result<Vec<u8>, AppError> {
        // Early rejection if the server announces a too-large body.
        if let Some(len) = resp.content_length()
            && len as usize > RESPONSE_BODY_LIMIT
        {
            return Err(AppError::ApiIncompatible(
                "response body exceeds 8 MiB limit".into(),
            ));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|_e| AppError::Connectivity("failed to read response body".into()))?;
        if bytes.len() > RESPONSE_BODY_LIMIT {
            return Err(AppError::ApiIncompatible(
                "response body exceeds 8 MiB limit".into(),
            ));
        }
        Ok(bytes.to_vec())
    }

    /// Parse `body` as JSON into `T`, also enforcing the 64-level depth limit.
    ///
    /// Parse failure → `ApiIncompatible`. Depth > 64 → `ApiIncompatible`.
    /// Internal serde_json error messages are discarded (SEC-005).
    fn deserialize_json<T: DeserializeOwned>(body: &[u8]) -> Result<T, AppError> {
        let value: DuplicateCheckedValue = serde_json::from_slice(body)
            .map_err(|_e| AppError::ApiIncompatible("response is not valid JSON".into()))?;
        let value = value.0;
        if json_max_depth(&value, 0) > JSON_MAX_DEPTH {
            return Err(AppError::ApiIncompatible(
                "response JSON nesting exceeds 64 levels".into(),
            ));
        }
        serde_json::from_value(value).map_err(|_e| {
            AppError::ApiIncompatible("response JSON does not match expected shape".into())
        })
    }

    /// Send an authenticated GET request with up to `GET_MAX_RETRIES` attempts.
    ///
    /// Retries on: send failure (connection error), 429, and 5xx. Each retry
    /// is preceded by a `attempt × RETRY_BASE_JITTER_MS` sleep. 401 and 403
    /// are not retried. POST / PUT / DELETE use separate non-retrying methods.
    pub async fn get<T: DeserializeOwned>(
        &self,
        url: &ValidatedUrl,
        path: &str,
    ) -> Result<T, AppError> {
        let full_url = Self::join_url(url, path);
        let mut last_err =
            AppError::Connectivity("GET request failed after all retry attempts".into());

        for attempt in 0..GET_MAX_RETRIES {
            if attempt > 0 {
                let jitter = Duration::from_millis(RETRY_BASE_JITTER_MS * u64::from(attempt));
                tokio::time::sleep(jitter).await;
            }

            match self
                .client
                .get(&full_url)
                .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
                .send()
                .await
            {
                Err(e) => {
                    // Connection-level failure; retry on next attempt.
                    last_err = Self::map_send_error(e);
                }
                Ok(resp) => {
                    let status = resp.status();
                    // Retry on transient server errors and rate-limit responses.
                    if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    {
                        last_err = Self::classify_error_status(status, false);
                        continue;
                    }
                    // Non-retriable non-2xx (401, 403, 4xx, 3xx).
                    if !status.is_success() {
                        return Err(Self::classify_error_status(status, false));
                    }
                    let body = Self::bounded_body(resp).await?;
                    return Self::deserialize_json(&body);
                }
            }
        }

        Err(last_err)
    }

    /// Send an authenticated POST request. Not retried.
    ///
    /// POST may commit server-side state; a blind retry could cause duplicate
    /// writes (ADR-011). The request body is serialized as JSON.
    async fn post<B, T>(&self, url: &ValidatedUrl, path: &str, body: &B) -> Result<T, AppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let full_url = Self::join_url(url, path);
        let resp = self
            .client
            .post(&full_url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
            .json(body)
            .send()
            .await
            .map_err(Self::map_modifying_send_error)?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Self::classify_error_status(status, true));
        }
        let body_bytes = Self::bounded_body(resp).await?;
        Self::deserialize_json(&body_bytes)
    }

    /// Send an authenticated PUT request. Not retried.
    ///
    /// PUT may commit server-side state; a blind retry could cause duplicate
    /// writes. The request body is serialized as JSON.
    async fn put<B, T>(&self, url: &ValidatedUrl, path: &str, body: &B) -> Result<T, AppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let full_url = Self::join_url(url, path);
        let resp = self
            .client
            .put(&full_url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
            .json(body)
            .send()
            .await
            .map_err(Self::map_modifying_send_error)?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Self::classify_error_status(status, true));
        }
        let body_bytes = Self::bounded_body(resp).await?;
        Self::deserialize_json(&body_bytes)
    }

    /// Send an authenticated DELETE request. Not retried.
    ///
    /// The response body is drained (bounded) to release the underlying
    /// TCP connection cleanly, even when the body is not otherwise consumed.
    #[cfg(test)]
    async fn delete(&self, url: &ValidatedUrl, path: &str) -> Result<(), AppError> {
        let full_url = Self::join_url(url, path);
        let resp = self
            .client
            .delete(&full_url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
            .send()
            .await
            .map_err(Self::map_modifying_send_error)?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Self::classify_error_status(status, true));
        }
        // Drain the body to release the connection even when we don't use it.
        let _ = Self::bounded_body(resp).await;
        Ok(())
    }

    /// GET that returns `Ok(None)` on 404 and `Ok(Some(T))` on 200.
    ///
    /// All other non-2xx statuses and transport errors propagate normally.
    /// Used by adapters to distinguish "not found" (Create path) from errors.
    pub async fn get_optional<T: serde::de::DeserializeOwned>(
        &self,
        url: &ValidatedUrl,
        path: &str,
    ) -> Result<Option<T>, AppError> {
        let full_url = Self::join_url(url, path);
        let mut last_err =
            AppError::Connectivity("GET request failed after all retry attempts".into());

        for attempt in 0..GET_MAX_RETRIES {
            if attempt > 0 {
                let jitter = Duration::from_millis(RETRY_BASE_JITTER_MS * u64::from(attempt));
                tokio::time::sleep(jitter).await;
            }

            match self
                .client
                .get(&full_url)
                .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
                .send()
                .await
            {
                Err(e) => {
                    last_err = Self::map_send_error(e);
                }
                Ok(resp) => {
                    let status = resp.status();
                    // 404 → entity not found; return None for the Create path.
                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Ok(None);
                    }
                    if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    {
                        last_err = Self::classify_error_status(status, false);
                        continue;
                    }
                    if !status.is_success() {
                        return Err(Self::classify_error_status(status, false));
                    }
                    let body = Self::bounded_body(resp).await?;
                    return Self::deserialize_json::<T>(&body).map(Some);
                }
            }
        }

        Err(last_err)
    }

    /// POST that returns `Ok(None)` on authoritative Datasource 409 Conflict
    /// and `Ok(Some(T))` on success. No collision follow-up is performed.
    async fn post_or_conflict<B, T>(
        &self,
        url: &ValidatedUrl,
        path: &str,
        body: &B,
    ) -> Result<Option<T>, AppError>
    where
        B: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let full_url = Self::join_url(url, path);
        let resp = self
            .client
            .post(&full_url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
            .json(body)
            .send()
            .await
            .map_err(Self::map_modifying_send_error)?;

        let status = resp.status();
        if status == reqwest::StatusCode::CONFLICT {
            // Drain body to release the connection.
            let _ = Self::bounded_body(resp).await;
            return Ok(None);
        }
        if !status.is_success() {
            return Err(Self::classify_error_status(status, true));
        }
        let body_bytes = Self::bounded_body(resp).await?;
        Self::deserialize_json(&body_bytes).map(Some)
    }

    /// POST `multipart/form-data` with scalar query parameters.
    ///
    /// `query_params` are appended to the URL as percent-encoded `key=value` pairs.
    /// `file_parts` become repeated `files` multipart parts. Not retried.
    async fn post_multipart(
        &self,
        url: &ValidatedUrl,
        path: &str,
        query_params: &[(String, String)],
        file_parts: Vec<(String, Vec<u8>)>,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let full_url = Self::join_url_with_query(url, path, query_params);
        let form = Self::build_multipart_form(file_parts)?;
        let resp = self
            .client
            .post(&full_url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
            .multipart(form)
            .send()
            .await
            .map_err(Self::map_modifying_send_error)?;

        let status = resp.status();
        if status == reqwest::StatusCode::CONFLICT {
            let _ = Self::bounded_body(resp).await;
            return Ok(None);
        }
        if !status.is_success() {
            return Err(Self::classify_error_status(status, true));
        }
        let body_bytes = Self::bounded_body(resp).await?;
        Self::deserialize_json(&body_bytes).map(Some)
    }

    /// PUT `multipart/form-data` with scalar query parameters. Not retried.
    async fn put_multipart(
        &self,
        url: &ValidatedUrl,
        path: &str,
        query_params: &[(String, String)],
        file_parts: Vec<(String, Vec<u8>)>,
    ) -> Result<serde_json::Value, AppError> {
        let full_url = Self::join_url_with_query(url, path, query_params);
        let form = Self::build_multipart_form(file_parts)?;
        let resp = self
            .client
            .put(&full_url)
            .header(reqwest::header::AUTHORIZATION, self.auth_header_value())
            .multipart(form)
            .send()
            .await
            .map_err(Self::map_modifying_send_error)?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Self::classify_error_status(status, true));
        }
        let body_bytes = Self::bounded_body(resp).await?;
        Self::deserialize_json(&body_bytes)
    }

    /// The sole production modifying boundary.
    ///
    /// A caller cannot supply a raw method/path/body. Those values are released
    /// only by consuming an adapter-owned `PreparedWrite` whose private seal
    /// contains completed, kind-specific read evidence and the linked projected
    /// request (R-001 / SEC-Q007-002).
    pub(crate) async fn dispatch_prepared(
        prepared: PreparedWrite<'_>,
    ) -> Result<PreparedWriteResponse, AppError> {
        let (client, request) = prepared.into_request()?;
        let url = &client.base_url;
        match request {
            PreparedRequest::Json {
                method: ModificationMethod::Post,
                path,
                body,
                conflict_is_resolution_signal: true,
            } => client
                .post_or_conflict::<_, serde_json::Value>(url, &path, &body)
                .await
                .map(|response| match response {
                    Some(value) => PreparedWriteResponse::Success(value),
                    None => PreparedWriteResponse::Conflict,
                }),
            PreparedRequest::Json {
                method: ModificationMethod::Post,
                path,
                body,
                conflict_is_resolution_signal: false,
            } => client
                .post::<_, serde_json::Value>(url, &path, &body)
                .await
                .map(PreparedWriteResponse::Success),
            PreparedRequest::Json {
                method: ModificationMethod::Put,
                path,
                body,
                conflict_is_resolution_signal: _,
            } => client
                .put::<_, serde_json::Value>(url, &path, &body)
                .await
                .map(PreparedWriteResponse::Success),
            PreparedRequest::Multipart {
                method: ModificationMethod::Post,
                path,
                query_params,
                file_parts,
            } => client
                .post_multipart(url, &path, &query_params, file_parts)
                .await
                .map(|response| match response {
                    Some(value) => PreparedWriteResponse::Success(value),
                    None => PreparedWriteResponse::Conflict,
                }),
            PreparedRequest::Multipart {
                method: ModificationMethod::Put,
                path,
                query_params,
                file_parts,
            } => client
                .put_multipart(url, &path, &query_params, file_parts)
                .await
                .map(PreparedWriteResponse::Success),
        }
    }

    /// Append percent-encoded query parameters to a base path.
    fn join_url_with_query(
        url: &ValidatedUrl,
        path: &str,
        query_params: &[(String, String)],
    ) -> String {
        let mut full = Self::join_url(url, path);
        if !query_params.is_empty() {
            // Check if there's already a `?` in the path (e.g. from pre-built query strings).
            let sep = if full.contains('?') { '&' } else { '?' };
            let mut first = true;
            for (k, v) in query_params {
                if first {
                    full.push(sep);
                    first = false;
                } else {
                    full.push('&');
                }
                full.push_str(&encode_query_value(k));
                full.push('=');
                full.push_str(&encode_query_value(v));
            }
        }
        full
    }

    /// Build a `reqwest::multipart::Form` from `(filename, bytes)` pairs.
    ///
    /// All files are added as `files` parts (the File Datasource transport name).
    fn build_multipart_form(
        file_parts: Vec<(String, Vec<u8>)>,
    ) -> Result<reqwest::multipart::Form, AppError> {
        let mut form = reqwest::multipart::Form::new();
        for (filename, bytes) in file_parts {
            let part = reqwest::multipart::Part::bytes(bytes).file_name(filename);
            form = form.part("files", part);
        }
        Ok(form)
    }
}

struct DuplicateCheckedValue(serde_json::Value);

impl<'de> serde::Deserialize<'de> for DuplicateCheckedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = DuplicateCheckedValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value without duplicate object keys")
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue(value.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue(value.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue(value.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(serde_json::Value::Number)
                    .map(DuplicateCheckedValue)
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue(value.into()))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue(value.into()))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(DuplicateCheckedValue(serde_json::Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                self.visit_none()
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<DuplicateCheckedValue>()? {
                    values.push(value.0);
                }
                Ok(DuplicateCheckedValue(serde_json::Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, DuplicateCheckedValue>()? {
                    if values.insert(key, value.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON object key"));
                    }
                }
                Ok(DuplicateCheckedValue(serde_json::Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

// ---------------------------------------------------------------------------
// JSON depth utility
// ---------------------------------------------------------------------------

/// Measure the maximum nesting depth of a JSON value.
///
/// Scalars (null, bool, number, string) contribute zero depth at their
/// current position. Arrays and objects add one level per descent into them.
///
/// `current` is the depth of the container that holds `value`. Pass `0` on
/// the root call. An empty root object returns `1`.
fn json_max_depth(value: &serde_json::Value, current: usize) -> usize {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|v| json_max_depth(v, current + 1))
            .max()
            .unwrap_or(current + 1),
        serde_json::Value::Object(map) => map
            .values()
            .map(|v| json_max_depth(v, current + 1))
            .max()
            .unwrap_or(current + 1),
        _ => current,
    }
}

// ---------------------------------------------------------------------------
// Query-value percent-encoding (T-002)
// ---------------------------------------------------------------------------

/// Percent-encode a query parameter value per RFC 3986.
///
/// Uses `url::form_urlencoded` which encodes all characters except
/// unreserved ones (A–Z, a–z, 0–9, `-`, `_`, `.`, `~`). Spaces become `+`;
/// all other characters are `%XX`-encoded. This satisfies the T-002 mandate
/// that route parameters and query parameter values are encoded by a URL encoder.
pub fn encode_query_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

// ---------------------------------------------------------------------------
// Capability preflight (ADR-012 Option A)
// ---------------------------------------------------------------------------

/// Verify that the authenticated principal has sufficient privilege for
/// Workflow, Skill, or Datasource exhaustive resolution (ADR-012 Option A).
///
/// Calls `GET {url}/v1/user` (route from `adapter-manifest-v2.42.0.json`
/// §capabilityPreflight) and checks:
/// - `is_admin == true` (global admin), OR
/// - `is_maintainer == true` (global maintainer), OR
/// - One `projects[]` entry whose `name` equals `effective_project` and whose
///   `is_project_admin` member is true
///
/// Returns a project-bound proof when any condition holds; otherwise returns
/// `AppError::VisibilityUnproven`. Response body and role values are discarded
/// after the check and are never forwarded to logs or output (SEC-005).
pub async fn preflight_visibility(
    client: &ApiClient,
    url: &ValidatedUrl,
    effective_project: &str,
) -> Result<ExactProjectVisibility, AppError> {
    let user: UserResponse = client.get(url, "/v1/user").await?;

    if user.user_id.is_empty() || user.projects.iter().any(|project| project.name.is_empty()) {
        return Err(AppError::ApiIncompatible(
            "user response contains an empty consumed string".into(),
        ));
    }

    if user
        .projects
        .iter()
        .any(|project| project.name == effective_project)
    {
        return Ok(ExactProjectVisibility {
            effective_project: effective_project.to_owned(),
            authenticated_user_id: user.user_id,
        });
    }

    Err(AppError::VisibilityUnproven(
        "principal is not a member of the effective project".into(),
    ))
}

// ---------------------------------------------------------------------------
// Tests (T-002)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Build a `ValidatedUrl` from a string (panics if invalid).
    fn test_url(base: &str) -> ValidatedUrl {
        ValidatedUrl::try_from(base).expect("test URL must be valid")
    }

    /// Build an `ApiClient` bound to `base` with a fixed test token.
    fn test_client(base: &str) -> ApiClient {
        ApiClient::new(test_url(base), "test-token".into())
            .expect("ApiClient must construct in tests")
    }

    // -----------------------------------------------------------------------
    // ApiClient construction
    // -----------------------------------------------------------------------

    #[test]
    fn api_client_constructs_with_loopback_url() {
        let url = test_url("http://127.0.0.1/api");
        ApiClient::new(url, "tok".into()).expect("ApiClient must construct with loopback URL");
    }

    // -----------------------------------------------------------------------
    // join_url
    // -----------------------------------------------------------------------

    #[test]
    fn join_url_appends_path() {
        let url = test_url("https://api.example.com");
        assert_eq!(
            ApiClient::join_url(&url, "/v1/user"),
            "https://api.example.com/v1/user"
        );
    }

    #[test]
    fn join_url_strips_trailing_slash_from_base() {
        let url = test_url("https://api.example.com/");
        assert_eq!(
            ApiClient::join_url(&url, "/v1/user"),
            "https://api.example.com/v1/user"
        );
    }

    #[test]
    fn join_url_accepts_path_without_leading_slash() {
        let url = test_url("https://api.example.com");
        assert_eq!(
            ApiClient::join_url(&url, "v1/user"),
            "https://api.example.com/v1/user"
        );
    }

    // -----------------------------------------------------------------------
    // json_max_depth utility
    // -----------------------------------------------------------------------

    #[test]
    fn json_depth_scalar_at_root_is_zero() {
        assert_eq!(json_max_depth(&serde_json::json!(42), 0), 0);
    }

    #[test]
    fn json_depth_empty_object_is_one() {
        assert_eq!(json_max_depth(&serde_json::json!({}), 0), 1);
    }

    #[test]
    fn json_depth_flat_object_is_one() {
        assert_eq!(json_max_depth(&serde_json::json!({"a": 1, "b": "x"}), 0), 1);
    }

    #[test]
    fn json_depth_nested_object_three_levels() {
        let v = serde_json::json!({"a": {"b": {"c": 1}}});
        assert_eq!(json_max_depth(&v, 0), 3);
    }

    #[test]
    fn json_depth_64_levels_is_at_limit() {
        let mut v = serde_json::json!(1);
        for _ in 0..64 {
            v = serde_json::json!({ "x": v });
        }
        assert_eq!(json_max_depth(&v, 0), 64);
    }

    #[test]
    fn json_depth_65_levels_exceeds_limit() {
        let mut v = serde_json::json!(1);
        for _ in 0..65 {
            v = serde_json::json!({ "x": v });
        }
        assert!(
            json_max_depth(&v, 0) > JSON_MAX_DEPTH,
            "65-level nesting must exceed JSON_MAX_DEPTH ({JSON_MAX_DEPTH})"
        );
    }

    // -----------------------------------------------------------------------
    // T-002 AC: GET returns deserialized response
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_returns_deserialized_json() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/items")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":42,"name":"widget"}"#)
            .create_async()
            .await;

        #[derive(Deserialize)]
        struct Item {
            id: u32,
            name: String,
        }

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let item: Item = client.get(&url, "/v1/items").await.unwrap();
        assert_eq!(item.id, 42);
        assert_eq!(item.name, "widget");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: POST sends correct body and returns deserialized response
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn post_sends_json_body_and_returns_deserialized_response() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/v1/items")
            .match_header(
                "content-type",
                mockito::Matcher::Regex("application/json".into()),
            )
            .match_body(mockito::Matcher::Json(
                serde_json::json!({"name": "new-widget"}),
            ))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":99,"name":"new-widget"}"#)
            .create_async()
            .await;

        #[derive(Serialize)]
        struct CreateItem {
            name: String,
        }
        #[derive(Deserialize)]
        struct Item {
            id: u32,
            name: String,
        }

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let req = CreateItem {
            name: "new-widget".into(),
        };
        let item: Item = client.post(&url, "/v1/items", &req).await.unwrap();
        assert_eq!(item.id, 99);
        assert_eq!(item.name, "new-widget");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: 401 → AppError::Authentication
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_401_returns_authentication_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/resource")
            .with_status(401)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = client
            .get::<serde_json::Value>(&url, "/v1/resource")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Authentication(_)),
            "401 must produce AppError::Authentication, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: 403 → AppError::Authorization
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_403_returns_authorization_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/resource")
            .with_status(403)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = client
            .get::<serde_json::Value>(&url, "/v1/resource")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Authorization(_)),
            "403 must produce AppError::Authorization, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: 3xx on POST → AppError::Authentication (redirect blocked)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn post_3xx_returns_authentication_error() {
        // Redirects are disabled for all methods (ADR-011 §4 preferred form).
        // A 3xx on a modifying request is classified as Authentication so the
        // operator investigates the endpoint configuration.
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/v1/resource")
            .with_status(302)
            .with_header("location", "https://other.example.com/resource")
            .create_async()
            .await;

        #[derive(Serialize)]
        struct Empty {}

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = client
            .post::<_, serde_json::Value>(&url, "/v1/resource", &Empty {})
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Authentication(_)),
            "3xx on POST must produce AppError::Authentication, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: 5xx → AppError::ApiIncompatible
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn post_500_returns_api_incompatible() {
        // POST does not retry; a single 500 response produces the error immediately.
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/v1/resource")
            .with_status(500)
            .create_async()
            .await;

        #[derive(Serialize)]
        struct Empty {}

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = client
            .post::<_, serde_json::Value>(&url, "/v1/resource", &Empty {})
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "500 on POST must produce AppError::ApiIncompatible, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    #[tokio::test]
    async fn get_500_retries_and_returns_api_incompatible() {
        // GET retries up to GET_MAX_RETRIES times on 5xx; all attempts must fail
        // before returning ApiIncompatible.
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/resource")
            .with_status(500)
            .expect_at_least(1)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = client
            .get::<serde_json::Value>(&url, "/v1/resource")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "500 on GET (after retries) must produce AppError::ApiIncompatible, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
    }

    // -----------------------------------------------------------------------
    // T-002 AC: network error → AppError::Connectivity
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_connection_refused_returns_connectivity_error() {
        // Bind a port, drop the listener immediately, then attempt to connect.
        // All GET_MAX_RETRIES attempts will fail with connection refused.
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
            // listener dropped here; port is freed
        };
        let base = format!("http://127.0.0.1:{port}");
        let url = test_url(&base);
        let client = test_client(&base);

        let err = client
            .get::<serde_json::Value>(&url, "/v1/resource")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::Connectivity(_)),
            "connection refused must produce AppError::Connectivity, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
    }

    // -----------------------------------------------------------------------
    // T-002 AC: preflight_visibility — global admin → Ok
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn preflight_visibility_global_admin_returns_ok() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user_id":"user-1","is_admin":true,"projects":[{"name":"my-proj"}]}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        preflight_visibility(&client, &url, "my-proj")
            .await
            .expect("membership must pass regardless of admin role");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: preflight_visibility — global maintainer → Ok
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn preflight_visibility_global_maintainer_returns_ok() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user_id":"user-1","is_maintainer":true,"projects":[{"name":"my-proj"}]}"#,
            )
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        preflight_visibility(&client, &url, "my-proj")
            .await
            .expect("membership must pass regardless of maintainer role");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: preflight_visibility — project-admin → Ok
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn preflight_visibility_project_admin_returns_ok() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"my-proj","is_project_admin":true}]}"#,
            )
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        preflight_visibility(&client, &url, "my-proj")
            .await
            .expect("project-admin must pass preflight");
        _mock.assert_async().await;
    }

    #[tokio::test]
    async fn preflight_visibility_other_project_admin_is_visibility_unproven() {
        let mut server = mockito::Server::new_async().await;
        let visibility = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"other-project","is_project_admin":true}]}"#,
            )
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = preflight_visibility(&client, &url, "my-project")
            .await
            .expect_err("another project's admin role is not sufficient evidence");
        assert!(matches!(error, AppError::VisibilityUnproven(_)));
        visibility.assert_async().await;
    }

    #[tokio::test]
    async fn preflight_visibility_strictly_decodes_every_consumed_field() {
        let invalid_responses = [
            r#"{"is_maintainer":false,"projects":[]}"#,
            r#"{"is_admin":false,"projects":[]}"#,
            r#"{"is_admin":false,"is_maintainer":false}"#,
            r#"{"is_admin":false,"is_maintainer":false,"projects":[{"is_project_admin":true}]}"#,
            r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"my-project"}]}"#,
            r#"{"is_admin":"false","is_maintainer":false,"projects":[]}"#,
            r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"my-project","is_project_admin":"true"}]}"#,
        ];

        for response in invalid_responses {
            let mut server = mockito::Server::new_async().await;
            let visibility = server
                .mock("GET", "/v1/user")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(response)
                .create_async()
                .await;
            let url = test_url(&server.url());
            let client = test_client(&server.url());
            let error = preflight_visibility(&client, &url, "my-project")
                .await
                .expect_err("invalid consumed field must fail compatibility");
            assert!(matches!(error, AppError::ApiIncompatible(_)));
            visibility.assert_async().await;
        }
    }

    #[tokio::test]
    async fn preflight_visibility_ignores_additive_unconsumed_fields() {
        let mut server = mockito::Server::new_async().await;
        let visibility = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[{"name":"my-project","is_project_admin":true,"future_project_field":42}],"future_top_level":{"nested":true}}"#,
            )
            .create_async()
            .await;
        let url = test_url(&server.url());
        let client = test_client(&server.url());
        preflight_visibility(&client, &url, "my-project")
            .await
            .expect("additive unconsumed fields must remain compatible");
        visibility.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: preflight_visibility — no qualifying role → VisibilityUnproven
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn preflight_visibility_no_qualifying_role_returns_visibility_unproven() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user_id":"user-1","projects":[{"name":"other"}]}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = preflight_visibility(&client, &url, "proj")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::VisibilityUnproven(_)),
            "no qualifying role must produce AppError::VisibilityUnproven, got {err:?}"
        );
        assert_eq!(err.exit_code(), 2);
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: preflight_visibility — empty projects, no global role
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn preflight_visibility_empty_projects_no_global_role_returns_visibility_unproven() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user_id":"user-1","is_admin":false,"is_maintainer":false,"projects":[]}"#,
            )
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = preflight_visibility(&client, &url, "proj")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::VisibilityUnproven(_)),
            "empty projects + no global roles must produce VisibilityUnproven, got {err:?}"
        );
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-003/IR-012: missing consumed role fields are incompatible
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn preflight_visibility_missing_role_fields_are_api_incompatible() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"login":"alice"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = preflight_visibility(&client, &url, "proj")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ApiIncompatible(_)),
            "missing role fields must fail compatibility, got {err:?}"
        );
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: DELETE success
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn delete_success_returns_ok() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("DELETE", "/v1/resource/1")
            .with_status(204)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        client
            .delete(&url, "/v1/resource/1")
            .await
            .expect("204 DELETE must return Ok");
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: 3xx on DELETE → AppError::Authentication
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn delete_3xx_returns_authentication_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("DELETE", "/v1/resource/1")
            .with_status(301)
            .with_header("location", "https://other.example.com/resource/1")
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let err = client.delete(&url, "/v1/resource/1").await.unwrap_err();
        assert!(
            matches!(err, AppError::Authentication(_)),
            "3xx on DELETE must produce AppError::Authentication, got {err:?}"
        );
        _mock.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // T-002 AC: PUT success
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn put_success_returns_deserialized_response() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("PUT", "/v1/resource/1")
            .match_header(
                "content-type",
                mockito::Matcher::Regex("application/json".into()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":1,"status":"updated"}"#)
            .create_async()
            .await;

        #[derive(Serialize)]
        struct Update {
            status: String,
        }
        #[derive(Deserialize)]
        struct Resource {
            id: u32,
            status: String,
        }

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let req = Update {
            status: "updated".into(),
        };
        let res: Resource = client.put(&url, "/v1/resource/1", &req).await.unwrap();
        assert_eq!(res.id, 1);
        assert_eq!(res.status, "updated");
        _mock.assert_async().await;
    }

    #[tokio::test]
    async fn preflight_visibility_rejects_duplicate_security_keys() {
        let mut server = mockito::Server::new_async().await;
        let duplicate = server.mock("GET", "/v1/user").with_status(200).with_body(
            r#"{"user_id":"user-1","user_id":"user-2","is_admin":true,"is_maintainer":false,"projects":[]}"#,
        ).create_async().await;
        let url = test_url(&server.url());
        let error = preflight_visibility(&test_client(&server.url()), &url, "project")
            .await
            .unwrap_err();
        assert!(matches!(error, AppError::ApiIncompatible(_)));
        duplicate.assert_async().await;
    }
}
