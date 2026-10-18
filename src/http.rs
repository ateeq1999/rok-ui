#![doc = include_str!("../docs/guide/http.md")]

pub use rok_ui_http::{
    content_type_for, describe_user_agent, path_segment, ApiError, Bytes, CancelToken,
    FieldDetails, FieldIssue, HttpClient, HttpClientBuilder, Method, Multipart, Options,
    QueryValue, ResponseInfo, Retry, Session, SessionState, DEFAULT_BASE_URL,
};

/// A mock API server for app tests (feature `http-testing`).
#[cfg(feature = "http-testing")]
pub mod testing {
    pub use rok_ui_http::testing::*;
}
