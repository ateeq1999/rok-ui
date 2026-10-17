//! The client: base URL, default headers, the session, and the request methods.

use std::{fmt, fmt::Write as _, sync::Arc, time::Duration};

use bytes::Bytes;
use serde::de::DeserializeOwned;

use super::{
    middleware::{Refresh, RefreshFuture, ResponseHook, REFRESHING},
    ApiError, Options, ResponseInfo, Retry, Session,
};

/// The base URL when neither the builder nor `ROK_API_URL` sets one.
pub const DEFAULT_BASE_URL: &str = "http://localhost:8080";

/// Escape `value` for use as one path segment: `path_segment("a b/c")` is `"a%20b%2Fc"`.
#[must_use]
pub fn path_segment(value: &impl fmt::Display) -> String {
    let text = value.to_string();
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Builds an [`HttpClient`].
#[derive(Default)]
pub struct HttpClientBuilder {
    base_url: Option<String>,
    headers: Vec<(String, String)>,
    timeout: Option<Duration>,
    session: Option<Session>,
    retry: Option<Retry>,
    on_response: Vec<ResponseHook>,
    refresh: Option<Refresh>,
}

impl std::fmt::Debug for HttpClientBuilder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpClientBuilder")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("retry", &self.retry)
            .field("refresh", &self.refresh.is_some())
            .finish_non_exhaustive()
    }
}

impl HttpClientBuilder {
    /// The URL paths are relative to: `https://api.example.com/v1`. Default: the
    /// `ROK_API_URL` environment variable, else [`DEFAULT_BASE_URL`].
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// A header every request sends (a request's own header with the same name wins).
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// How long a request may take before it fails with a network error. Default: 30 seconds.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// The session whose token requests send, and which a 401 expires.
    #[must_use]
    pub fn session(mut self, session: Session) -> Self {
        self.session = Some(session);
        self
    }

    /// Retry idempotent requests that fail for a transient reason (see [`Retry`]).
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use rok_ui::http::{HttpClient, Retry};
    ///
    /// let client = HttpClient::builder()
    ///     .retry(Retry::idempotent(3).backoff(Duration::from_millis(250)))
    ///     .build();
    /// # let _ = client;
    /// ```
    #[must_use]
    pub fn retry(mut self, retry: Retry) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Call `hook` after every attempt, with its method, URL, status and timing (for logging
    /// and metrics). Hooks run in the order they were added.
    #[must_use]
    pub fn on_response(mut self, hook: impl Fn(&ResponseInfo) + Send + Sync + 'static) -> Self {
        self.on_response.push(Arc::new(hook));
        self
    }

    /// Print every attempt to standard error: `GET https://.../notes -> 200 (31 ms)`.
    #[must_use]
    pub fn log_requests(self) -> Self {
        self.on_response(|info| eprintln!("[http] {info}"))
    }

    /// When a request gets a 401 with the session's token, call `refresh` for a new token,
    /// store it in the session and send the request again; expire the session only if the
    /// refresh fails. Concurrent 401s share one refresh.
    ///
    /// `refresh` gets the client to make its call. That call should use
    /// [`Options::skip_expire`] (a 401 there means the refresh token is gone too); it never
    /// triggers another refresh.
    ///
    /// ```no_run
    /// use rok_ui::http::{HttpClient, Options, Session};
    ///
    /// #[derive(serde::Deserialize)]
    /// struct TokenDto {
    ///     token: String,
    /// }
    ///
    /// let session = Session::new();
    /// let client = HttpClient::builder()
    ///     .session(session)
    ///     .refresh_token(|client| async move {
    ///         let fresh: TokenDto = client
    ///             .request("/sessions/refresh", Options::post().skip_expire(true))
    ///             .await?;
    ///         Ok(fresh.token)
    ///     })
    ///     .build();
    /// # let _ = client;
    /// ```
    #[must_use]
    pub fn refresh_token<F, Fut>(mut self, refresh: F) -> Self
    where
        F: Fn(HttpClient) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String, ApiError>> + Send + 'static,
    {
        self.refresh = Some(Arc::new(move |client| -> RefreshFuture {
            Box::pin(refresh(client))
        }));
        self
    }

    /// The client.
    ///
    /// # Panics
    ///
    /// Panics if the TLS backend cannot start (the OS trust store cannot be read), like
    /// `reqwest::Client::new`.
    #[must_use]
    pub fn build(self) -> HttpClient {
        let base_url = self
            .base_url
            .or_else(|| {
                std::env::var("ROK_API_URL")
                    .ok()
                    .filter(|url| !url.is_empty())
            })
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let client = reqwest::Client::builder()
            .timeout(self.timeout.unwrap_or(Duration::from_secs(30)))
            .build()
            .expect("the HTTP client's TLS backend starts");
        HttpClient {
            inner: Arc::new(Inner {
                client,
                base_url: base_url.trim_end_matches('/').to_string(),
                headers: self.headers,
                session: self.session,
                retry: self.retry,
                on_response: self.on_response,
                refresh: self.refresh,
                refreshing: tokio::sync::Mutex::new(()),
            }),
        }
    }
}

struct Inner {
    client: reqwest::Client,
    base_url: String,
    headers: Vec<(String, String)>,
    session: Option<Session>,
    retry: Option<Retry>,
    on_response: Vec<ResponseHook>,
    refresh: Option<Refresh>,
    /// Held while a token refresh runs, so concurrent 401s share it.
    refreshing: tokio::sync::Mutex<()>,
}

/// Why one attempt failed, and the token it sent.
struct Failure {
    error: ApiError,
    sent_token: Option<String>,
}

/// A client for one JSON API. Cheap to clone; build it once (in `main`) and share it with the
/// providers that use it.
///
/// Every request:
///
/// - goes to [`api_url(path)`](Self::api_url),
/// - sends `Accept: application/json`, the client's headers, `Authorization: Bearer <token>`
///   while the session has one, then the request's own headers (which win),
/// - returns the decoded body, or an [`ApiError`]: the server's envelope, a network failure
///   (status `0`, `"network_error"`) or a cancellation ([`ApiError::is_cancelled`]),
/// - expires the session on a 401, unless the request [skips
///   that](Options::skip_expire), sent no token, or the session's token changed meanwhile.
///
/// Requests run on the shared [`runtime`](crate::runtime) when they are not already on a tokio
/// runtime, so they can be awaited from GPUI tasks too.
#[derive(Clone)]
pub struct HttpClient {
    inner: Arc<Inner>,
}

impl fmt::Debug for HttpClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpClient")
            .field("base_url", &self.inner.base_url)
            .field("session", &self.inner.session)
            .finish_non_exhaustive()
    }
}

/// A successful response, read.
struct Response {
    status: u16,
    body: Bytes,
}

impl HttpClient {
    /// A builder.
    #[must_use]
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::default()
    }

    /// A client for `base_url` with no session.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::builder().base_url(base_url).build()
    }

    /// The base URL, without a trailing slash.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// The session, if the client has one.
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        self.inner.session.as_ref()
    }

    /// The full URL for `path`: the base URL joined with `path`. Absolute URLs (`https://..`)
    /// are returned as they are.
    #[must_use]
    pub fn api_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            self.inner.base_url.clone()
        } else {
            format!("{}/{path}", self.inner.base_url)
        }
    }

    /// Send a request and decode its JSON body as `T`.
    ///
    /// An empty body (or a 204) decodes as `null`: fine for `()` and `Option<T>` (which is
    /// `None`), an error naming the problem for anything else.
    ///
    /// # Errors
    ///
    /// The server's error, a network failure, a cancellation, or a body that is not a `T`.
    pub async fn request<T: DeserializeOwned + Send + 'static>(
        &self,
        path: &str,
        options: Options,
    ) -> Result<T, ApiError> {
        let response = self.send(path, options).await?;
        let body: &[u8] = if response.body.iter().all(u8::is_ascii_whitespace) {
            b"null"
        } else {
            &response.body
        };
        serde_json::from_slice(body).map_err(|error| {
            if body == b"null" {
                ApiError::new(
                    response.status,
                    "empty_response",
                    format!(
                        "The server sent no data, but the app expected a `{}`",
                        std::any::type_name::<T>()
                    ),
                )
            } else {
                ApiError::new(
                    response.status,
                    "invalid_response",
                    format!("The server's answer could not be read: {error}"),
                )
            }
        })
    }

    /// Send a request and ignore its body.
    ///
    /// # Errors
    ///
    /// The server's error, a network failure or a cancellation.
    pub async fn request_empty(&self, path: &str, options: Options) -> Result<(), ApiError> {
        self.send(path, options).await.map(|_| ())
    }

    /// Send a request and return its body as bytes (downloads, images).
    ///
    /// # Errors
    ///
    /// The server's error, a network failure or a cancellation.
    pub async fn request_bytes(&self, path: &str, options: Options) -> Result<Bytes, ApiError> {
        self.send(path, options).await.map(|response| response.body)
    }

    /// Send the request on a tokio runtime, honoring its cancel token.
    async fn send(&self, path: &str, options: Options) -> Result<Response, ApiError> {
        let cancel = options.cancel.clone();
        if cancel
            .as_ref()
            .is_some_and(super::CancelToken::is_cancelled)
        {
            return Err(ApiError::cancelled());
        }
        let client = self.clone();
        let url = self.api_url(path);
        let work = async move { client.send_now(url, options).await };
        let request = async move {
            if tokio::runtime::Handle::try_current().is_ok() {
                work.await
            } else {
                // Not on a tokio runtime (a GPUI task, a plain test): reqwest needs one.
                crate::runtime::spawn(work)
                    .await
                    .unwrap_or_else(|_| Err(ApiError::cancelled()))
            }
        };
        match cancel {
            None => request.await,
            Some(token) => {
                let cancelled = std::pin::pin!(token.cancelled());
                let request = std::pin::pin!(request);
                match futures::future::select(request, cancelled).await {
                    futures::future::Either::Left((result, _)) => result,
                    futures::future::Either::Right(((), _)) => Err(ApiError::cancelled()),
                }
            }
        }
    }

    /// Send with retries and token refresh, reporting each attempt.
    async fn send_now(&self, url: String, options: Options) -> Result<Response, ApiError> {
        let inner = &self.inner;
        let mut attempt = 1;
        let mut refreshed = false;
        loop {
            let started = std::time::Instant::now();
            let outcome = self.attempt(&url, &options).await;
            if !inner.on_response.is_empty() {
                let info = ResponseInfo {
                    method: options.method,
                    url: url.clone(),
                    status: match &outcome {
                        Ok(response) => response.status,
                        Err(failure) => failure.error.status,
                    },
                    elapsed: started.elapsed(),
                    attempt,
                };
                for hook in &inner.on_response {
                    hook(&info);
                }
            }
            let Failure { error, sent_token } = match outcome {
                Ok(response) => return Ok(response),
                Err(failure) => failure,
            };
            if let Some(retry) = &inner.retry {
                if retry.allows(options.method, &error, attempt) {
                    tokio::time::sleep(retry.delay(attempt)).await;
                    attempt += 1;
                    continue;
                }
            }
            if error.status == 401 && !options.skip_expire {
                if let (Some(session), Some(token)) = (&inner.session, &sent_token) {
                    if !refreshed && self.refresh(token).await {
                        refreshed = true;
                        attempt += 1;
                        continue;
                    }
                    session.expire_if_current(token);
                }
            }
            return Err(error);
        }
    }

    /// Get a new token after `rejected` got a 401. Returns whether the session now holds a
    /// different token to retry with.
    async fn refresh(&self, rejected: &str) -> bool {
        let inner = &self.inner;
        let (Some(refresh), Some(session)) = (&inner.refresh, &inner.session) else {
            return false;
        };
        // The refresh's own request never refreshes again.
        if REFRESHING.try_with(|()| ()).is_ok() {
            return false;
        }
        let _guard = inner.refreshing.lock().await;
        match session.token() {
            // Another request refreshed while this one waited.
            Some(current) if current != rejected => return true,
            None => return false,
            Some(_) => {}
        }
        match REFRESHING.scope((), refresh(self.clone())).await {
            Ok(token) => {
                session.set_token(token);
                true
            }
            Err(_) => false,
        }
    }

    /// One attempt: build the request, send it, read the answer.
    async fn attempt(&self, url: &str, options: &Options) -> Result<Response, Failure> {
        let inner = &self.inner;
        let method = reqwest::Method::from_bytes(options.method.as_str().as_bytes())
            .unwrap_or(reqwest::Method::GET);
        let mut headers: Vec<(String, String)> = vec![("Accept".into(), "application/json".into())];
        let mut set = |name: &str, value: String| {
            headers.retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
            headers.push((name.to_string(), value));
        };
        for (name, value) in &inner.headers {
            set(name, value.clone());
        }
        let token = inner.session.as_ref().and_then(Session::token);
        if let Some(token) = &token {
            set("Authorization", format!("Bearer {token}"));
        }
        let body = options.body_bytes().map_err(|error| Failure {
            error,
            sent_token: None,
        })?;
        if let Some((_, content_type)) = &body {
            set("Content-Type", content_type.clone());
        }
        for (name, value) in &options.headers {
            set(name, value.clone());
        }
        // The token this request actually sends (a request may override Authorization).
        let sent_token = token.filter(|token| {
            headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("Authorization") && *value == format!("Bearer {token}")
            })
        });
        let failure = |error| Failure {
            error,
            sent_token: sent_token.clone(),
        };
        let mut request = inner.client.request(method, url);
        if !options.query.is_empty() {
            request = request.query(&options.query);
        }
        for (name, value) in headers {
            request = request.header(name, value);
        }
        if let Some((bytes, _)) = body {
            request = request.body(bytes);
        }
        let response = request
            .send()
            .await
            .map_err(|_| failure(ApiError::network()))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|_| failure(ApiError::network()))?;
        if status.is_success() {
            return Ok(Response {
                status: status.as_u16(),
                body,
            });
        }
        Err(failure(ApiError::from_response(
            status.as_u16(),
            status.canonical_reason(),
            &body,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_urls_join_paths() {
        let client = HttpClient::new("https://api.example.com/v1/");
        assert_eq!(client.base_url(), "https://api.example.com/v1");
        assert_eq!(client.api_url("/notes"), "https://api.example.com/v1/notes");
        assert_eq!(
            client.api_url("notes/1"),
            "https://api.example.com/v1/notes/1"
        );
        assert_eq!(client.api_url(""), "https://api.example.com/v1");
        assert_eq!(
            client.api_url("https://cdn.example.com/a.png"),
            "https://cdn.example.com/a.png"
        );
    }

    #[test]
    fn the_base_url_defaults_to_the_environment() {
        // The only test that reads or writes ROK_API_URL.
        std::env::set_var("ROK_API_URL", "https://from-env.example.com");
        assert_eq!(
            HttpClient::builder().build().base_url(),
            "https://from-env.example.com"
        );
        assert_eq!(
            HttpClient::new("https://explicit.example.com").base_url(),
            "https://explicit.example.com"
        );
        std::env::remove_var("ROK_API_URL");
        assert_eq!(HttpClient::builder().build().base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn path_segments_are_escaped() {
        assert_eq!(path_segment(&"a b/c?d"), "a%20b%2Fc%3Fd");
        assert_eq!(path_segment(&42), "42");
        assert_eq!(path_segment(&"caf\u{e9}"), "caf%C3%A9");
    }
}
