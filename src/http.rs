#![doc = include_str!("../docs/guide/http.md")]

mod cancel;
mod client;
mod error;
mod middleware;
mod options;
mod session;

pub use bytes::Bytes;
pub use cancel::CancelToken;
pub use client::{path_segment, HttpClient, HttpClientBuilder, DEFAULT_BASE_URL};
pub use error::{ApiError, FieldDetails, FieldIssue};
pub use middleware::{ResponseInfo, Retry};
pub use options::{Method, Options, QueryValue};
pub use session::{Session, SessionState};
