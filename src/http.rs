#![doc = include_str!("../docs/guide/http.md")]

mod cancel;
mod client;
mod error;
mod middleware;
mod multipart;
mod options;
mod session;
#[cfg(feature = "http-testing")]
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
