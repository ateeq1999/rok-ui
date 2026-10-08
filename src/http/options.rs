//! What one request sends: method, body, headers, query, and how it may end.

use serde::Serialize;

use super::{ApiError, CancelToken};

/// The HTTP method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Method {
    /// `GET`.
    #[default]
    Get,
    /// `POST`.
    Post,
    /// `PUT`.
    Put,
    /// `PATCH`.
    Patch,
    /// `DELETE`.
    Delete,
    /// `HEAD`.
    Head,
}

impl Method {
    /// The method's name: `"GET"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
        }
    }
}

/// The request body.
#[derive(Clone, Debug)]
pub(super) enum Body {
    /// Serialized JSON (or why serializing failed).
    Json(Result<Vec<u8>, String>),
    /// Bytes as they are, with their content type.
    Raw(Vec<u8>),
}

/// A value for [`Options::query`]: `None` and `""` leave the parameter out.
pub trait QueryValue {
    /// The text to send, or `None` to leave the parameter out.
    fn query_value(&self) -> Option<String>;
}

impl QueryValue for str {
    fn query_value(&self) -> Option<String> {
        (!self.is_empty()).then(|| self.to_string())
    }
}

impl QueryValue for String {
    fn query_value(&self) -> Option<String> {
        self.as_str().query_value()
    }
}

impl<T: QueryValue + ?Sized> QueryValue for &T {
    fn query_value(&self) -> Option<String> {
        (**self).query_value()
    }
}

impl<T: QueryValue> QueryValue for Option<T> {
    fn query_value(&self) -> Option<String> {
        self.as_ref().and_then(QueryValue::query_value)
    }
}

macro_rules! display_query_values {
    ($($ty:ty),*) => {
        $(impl QueryValue for $ty {
            fn query_value(&self) -> Option<String> {
                Some(self.to_string())
            }
        })*
    };
}

display_query_values!(
    bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64
);

/// What one request sends. Start from a method ([`Options::get`], [`Options::post`], ..) and
/// add to it:
///
/// ```
/// use rok_ui::http::Options;
///
/// let options = Options::post()
///     .json(&serde_json::json!({ "title": "Milk" }))
///     .query("notify", true)
///     .query("tag", None::<String>) // left out
///     .header("X-Request-Id", "42");
/// # let _ = options;
/// ```
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub(super) method: Method,
    pub(super) body: Option<Body>,
    pub(super) content_type: Option<String>,
    pub(super) headers: Vec<(String, String)>,
    pub(super) query: Vec<(String, String)>,
    pub(super) skip_expire: bool,
    pub(super) cancel: Option<CancelToken>,
}

impl Options {
    /// A request with `method`.
    #[must_use]
    pub fn new(method: Method) -> Self {
        Self {
            method,
            ..Self::default()
        }
    }

    /// `GET`.
    #[must_use]
    pub fn get() -> Self {
        Self::new(Method::Get)
    }

    /// `POST`.
    #[must_use]
    pub fn post() -> Self {
        Self::new(Method::Post)
    }

    /// `PUT`.
    #[must_use]
    pub fn put() -> Self {
        Self::new(Method::Put)
    }

    /// `PATCH`.
    #[must_use]
    pub fn patch() -> Self {
        Self::new(Method::Patch)
    }

    /// `DELETE`.
    #[must_use]
    pub fn delete() -> Self {
        Self::new(Method::Delete)
    }

    /// The method.
    #[must_use]
    pub fn method(&self) -> Method {
        self.method
    }

    /// Send `body` as JSON (`Content-Type: application/json`). A raw body set with
    /// [`raw_body`](Self::raw_body) wins over this.
    #[must_use]
    pub fn json<T: Serialize + ?Sized>(mut self, body: &T) -> Self {
        if !matches!(self.body, Some(Body::Raw(_))) {
            let bytes = serde_json::to_vec(body).map_err(|error| error.to_string());
            self.body = Some(Body::Json(bytes));
        }
        self
    }

    /// Send `bytes` as they are, as `application/octet-stream` unless
    /// [`content_type`](Self::content_type) says otherwise. Wins over [`json`](Self::json).
    #[must_use]
    pub fn raw_body(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.body = Some(Body::Raw(bytes.into()));
        self
    }

    /// The `Content-Type` of the body.
    #[must_use]
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Send a header. A later header with the same name (in any case) replaces this one and
    /// the client's defaults, including `Accept`, `Content-Type` and `Authorization`.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        self.headers
            .retain(|(existing, _)| !existing.eq_ignore_ascii_case(&name));
        self.headers.push((name, value.into()));
        self
    }

    /// Add a query parameter. `None` and empty strings are left out, so optional filters can
    /// be passed as they are.
    #[must_use]
    pub fn query(mut self, name: impl Into<String>, value: impl QueryValue) -> Self {
        if let Some(value) = value.query_value() {
            self.query.push((name.into(), value));
        }
        self
    }

    /// Do not expire the session when this request gets a 401. For calls where a 401 is an
    /// answer, not a lost session: signing in with a wrong password, checking a password.
    #[must_use]
    pub fn skip_expire(mut self, skip: bool) -> Self {
        self.skip_expire = skip;
        self
    }

    /// End the request with [`ApiError::cancelled()`] when `token` is cancelled.
    #[must_use]
    pub fn cancel(mut self, token: &CancelToken) -> Self {
        self.cancel = Some(token.clone());
        self
    }

    /// The body as bytes and its content type, or why it could not be serialized.
    pub(super) fn body_bytes(&self) -> Result<Option<(Vec<u8>, String)>, ApiError> {
        let content_type = |default: &str| {
            self.content_type
                .clone()
                .unwrap_or_else(|| default.to_string())
        };
        match &self.body {
            None => Ok(None),
            Some(Body::Raw(bytes)) => Ok(Some((
                bytes.clone(),
                content_type("application/octet-stream"),
            ))),
            Some(Body::Json(Ok(bytes))) => {
                Ok(Some((bytes.clone(), content_type("application/json"))))
            }
            Some(Body::Json(Err(message))) => Err(ApiError::new(
                0,
                "invalid_body",
                format!("The request body could not be written: {message}"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_skips_none_and_empty() {
        let options = Options::get()
            .query("a", "x")
            .query("b", "")
            .query("c", None::<u32>)
            .query("d", Some(3))
            .query("e", String::new())
            .query("f", false);
        assert_eq!(
            options.query,
            [("a", "x"), ("d", "3"), ("f", "false")]
                .map(|(name, value)| (name.to_string(), value.to_string()))
        );
    }

    #[test]
    fn later_headers_replace_earlier_ones() {
        let options = Options::get()
            .header("Accept", "text/plain")
            .header("accept", "image/png");
        assert_eq!(
            options.headers,
            [("accept".to_string(), "image/png".to_string())]
        );
    }

    #[test]
    fn raw_bodies_win_over_json() {
        let options = Options::post().raw_body(b"abc".to_vec()).json(&1);
        let (bytes, content_type) = options.body_bytes().unwrap().unwrap();
        assert_eq!(
            (bytes.as_slice(), content_type.as_str()),
            (&b"abc"[..], "application/octet-stream")
        );
        let options = Options::post()
            .json(&1)
            .raw_body(b"x".to_vec())
            .content_type("image/png");
        assert_eq!(options.body_bytes().unwrap().unwrap().1, "image/png");
        let options = Options::post().json(&[1, 2]);
        assert_eq!(
            options.body_bytes().unwrap().unwrap(),
            (b"[1,2]".to_vec(), "application/json".to_string())
        );
    }
}
