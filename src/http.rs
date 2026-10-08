//! A typed HTTP client for JSON APIs (feature `http`): [`HttpClient`], [`ApiError`] and
//! [`Session`].
//!
//! Requests are `async` and run on the shared [`runtime`](crate::runtime): call them from a
//! bloc handler, a repository, a query or a procedure. Errors are always an [`ApiError`] with
//! the HTTP status, a machine-readable `code` and a message for people, parsed from the
//! server's `{"error": {"code", "message", "details"}}` envelope.
//!
//! ```no_run
//! use rok_ui::http::{ApiError, HttpClient, Options, Session};
//!
//! #[derive(serde::Deserialize)]
//! struct Note {
//!     id: u64,
//!     title: String,
//! }
//!
//! # async fn example() -> Result<(), ApiError> {
//! let session = Session::new();
//! let client = HttpClient::builder()
//!     .base_url("https://api.example.com")
//!     .session(session.clone())
//!     .build();
//! session.set_token("secret");
//! let notes: Vec<Note> = client
//!     .request("/notes", Options::get().query("search", "milk"))
//!     .await?;
//! # let _ = notes;
//! # Ok(())
//! # }
//! ```
//!
//! See the HTTP guide (`docs/guide/http.md`) for the rules: where the client lives, how
//! errors reach forms ([`crate::form::ServerErrors`]), and what a 401 does to the session.

mod cancel;
mod client;
mod error;
mod options;
mod session;

pub use bytes::Bytes;
pub use cancel::CancelToken;
pub use client::{path_segment, HttpClient, HttpClientBuilder, DEFAULT_BASE_URL};
pub use error::{ApiError, FieldDetails, FieldIssue};
pub use options::{Method, Options, QueryValue};
pub use session::{Session, SessionState};
