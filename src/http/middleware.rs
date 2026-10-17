//! What the client does around each request: retries, a response hook, and refreshing the
//! session's token on a 401.

use std::{fmt, future::Future, pin::Pin, sync::Arc, time::Duration};

use super::{ApiError, HttpClient, Method};

/// Retries for requests that are safe to repeat. Set it with
/// [`HttpClientBuilder::retry`](super::HttpClientBuilder::retry).
///
/// Only idempotent methods (`GET`, `HEAD`, `PUT`, `DELETE`) are retried, and only when the
/// server could not be reached or answered 502, 503 or 504. A `POST` is never repeated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retry {
    retries: u32,
    backoff: Duration,
}

impl Retry {
    /// Up to `retries` more attempts for idempotent requests. Default backoff: 200 ms,
    /// doubling each time, at most 10 seconds.
    #[must_use]
    pub fn idempotent(retries: u32) -> Self {
        Self {
            retries,
            backoff: Duration::from_millis(200),
        }
    }

    /// The wait before the first retry; each later one waits twice as long.
    #[must_use]
    pub fn backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    /// Whether attempt number `attempt` (1 for the first) failing with `error` gets another.
    pub(super) fn allows(&self, method: Method, error: &ApiError, attempt: u32) -> bool {
        let idempotent = matches!(
            method,
            Method::Get | Method::Head | Method::Put | Method::Delete
        );
        let transient = error.is_network() || matches!(error.status, 502..=504);
        idempotent && transient && attempt <= self.retries
    }

    /// How long to wait after attempt number `attempt` failed.
    pub(super) fn delay(&self, attempt: u32) -> Duration {
        let factor = 2_u32.saturating_pow(attempt.saturating_sub(1));
        self.backoff
            .saturating_mul(factor)
            .min(Duration::from_secs(10))
    }
}

/// One finished attempt, for [`HttpClientBuilder::on_response`](super::HttpClientBuilder::on_response).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseInfo {
    /// The method.
    pub method: Method,
    /// The full URL, without the query string.
    pub url: String,
    /// The HTTP status, or `0` when no response arrived.
    pub status: u16,
    /// How long the attempt took.
    pub elapsed: Duration,
    /// 1 for the first attempt, 2 for the first retry, and so on.
    pub attempt: u32,
}

impl fmt::Display for ResponseInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.status == 0 {
            "no response".to_string()
        } else {
            self.status.to_string()
        };
        write!(
            formatter,
            "{} {} -> {status} ({} ms)",
            self.method.as_str(),
            self.url,
            self.elapsed.as_millis()
        )?;
        if self.attempt > 1 {
            write!(formatter, ", attempt {}", self.attempt)?;
        }
        Ok(())
    }
}

pub(super) type ResponseHook = Arc<dyn Fn(&ResponseInfo) + Send + Sync>;

pub(super) type RefreshFuture = Pin<Box<dyn Future<Output = Result<String, ApiError>> + Send>>;
pub(super) type Refresh = Arc<dyn Fn(HttpClient) -> RefreshFuture + Send + Sync>;

tokio::task_local! {
    /// Set while a refresh runs, so the refresh's own request never refreshes again.
    pub(super) static REFRESHING: ();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_idempotent_transient_failures_are_retried() {
        let retry = Retry::idempotent(2);
        let unavailable = ApiError::new(503, "unknown", "Service Unavailable");
        assert!(retry.allows(Method::Get, &ApiError::network(), 1));
        assert!(retry.allows(Method::Delete, &unavailable, 2));
        assert!(
            !retry.allows(Method::Get, &unavailable, 3),
            "out of retries"
        );
        assert!(
            !retry.allows(Method::Post, &ApiError::network(), 1),
            "POST is never repeated"
        );
        assert!(!retry.allows(Method::Get, &ApiError::new(500, "x", "y"), 1));
        assert!(!retry.allows(Method::Get, &ApiError::cancelled(), 1));
    }

    #[test]
    fn backoff_doubles_up_to_a_cap() {
        let retry = Retry::idempotent(9).backoff(Duration::from_millis(100));
        assert_eq!(retry.delay(1), Duration::from_millis(100));
        assert_eq!(retry.delay(3), Duration::from_millis(400));
        assert_eq!(retry.delay(30), Duration::from_secs(10));
    }

    #[test]
    fn response_info_reads_well() {
        let info = ResponseInfo {
            method: Method::Get,
            url: "https://api.example.com/notes".into(),
            status: 0,
            elapsed: Duration::from_millis(12),
            attempt: 2,
        };
        assert_eq!(
            info.to_string(),
            "GET https://api.example.com/notes -> no response (12 ms), attempt 2"
        );
    }
}
