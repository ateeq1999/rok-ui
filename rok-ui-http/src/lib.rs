//! The HTTP client behind `rok_ui::http`, without GPUI: an [`HttpClient`] for JSON APIs,
//! [`ApiError`] (parsed from the API's error envelope, with field errors that rok-ui forms
//! show), a [`Session`] for the signed-in token, retries, token refresh, cancellation and
//! multipart uploads.
//!
//! Apps use it through `rok_ui::http` (feature `http`); the HTTP guide in rok-ui's
//! `docs/guide/http.md` covers it. Depend on this crate directly where GPUI should not reach,
//! such as the data crate of an app split into crates.
//!
//! ```no_run
//! use rok_ui_http::{HttpClient, Options, Session};
//!
//! # async fn load() -> Result<(), rok_ui_http::ApiError> {
//! let session = Session::new();
//! let client = HttpClient::builder()
//!     .base_url("https://api.example.com")
//!     .session(session)
//!     .build();
//! let notes: serde_json::Value = client.request("/notes", Options::get()).await?;
//! # Ok(())
//! # }
//! ```

mod cancel;
mod client;
mod error;
mod middleware;
mod multipart;
mod options;
pub mod runtime;
mod session;
#[cfg(feature = "testing")]
pub mod testing;
mod user_agent;

pub use bytes::Bytes;
pub use cancel::CancelToken;
pub use client::{path_segment, HttpClient, HttpClientBuilder, DEFAULT_BASE_URL};
pub use error::{ApiError, FieldDetails, FieldIssue};
pub use middleware::{ResponseInfo, Retry};
pub use multipart::{content_type_for, Multipart};
pub use options::{Method, Options, QueryValue};
pub use session::{Session, SessionState};
pub use user_agent::describe_user_agent;
