//! A mock HTTP server for tests (feature `http-testing`): routes, canned replies and the
//! requests it received, on localhost. Never the network.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use super::Method;

/// A request the server received.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Received {
    /// `GET`, `POST`, ...
    pub method: String,
    /// The path, without the query string: `/notes/1`.
    pub path: String,
    /// The query string without `?` (empty when there was none).
    pub query: String,
    /// Headers in the order they arrived.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl Received {
    /// The first header named `name` (any case).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The body as text.
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The body parsed as JSON (`Value::Null` when it is not JSON).
    #[must_use]
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null)
    }
}

#[derive(Clone, Debug)]
struct Reply {
    status: u16,
    body: Vec<u8>,
    delay: Duration,
}

#[derive(Debug)]
struct Route {
    method: Method,
    path: String,
    once: VecDeque<Reply>,
    always: Option<Reply>,
    delay: Duration,
}

#[derive(Debug, Default)]
struct State {
    routes: Vec<Route>,
    received: Vec<Received>,
}

/// A mock HTTP server on a free localhost port, for testing providers, repositories and
/// anything else that calls an API.
///
/// Routes match on method and path (not the query). Unknown routes answer 404.
///
/// ```
/// use rok_ui::http::{testing::MockServer, HttpClient, Method, Options};
///
/// let server = MockServer::start();
/// server.on(Method::Get, "/notes").reply_json(200, &serde_json::json!([{"title": "Milk"}]));
/// server.on(Method::Post, "/notes").once(503, "").reply(201, "{}");
///
/// let client = HttpClient::new(server.url());
/// let notes: serde_json::Value =
///     rok_ui::runtime::block_on(client.request("/notes", Options::get())).unwrap();
/// assert_eq!(notes[0]["title"], "Milk");
/// assert_eq!(server.requests()[0].path, "/notes");
/// ```
#[derive(Clone, Debug)]
pub struct MockServer {
    url: String,
    state: Arc<Mutex<State>>,
}

fn lock(state: &Mutex<State>) -> std::sync::MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl MockServer {
    /// Start a server on the shared runtime. It stops when the process ends.
    ///
    /// # Panics
    ///
    /// Panics if no localhost port can be bound.
    #[must_use]
    pub fn start() -> Self {
        let listener = crate::runtime::block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("a free localhost port");
        let address = listener.local_addr().expect("a bound address");
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        crate::runtime::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                crate::runtime::spawn(serve(stream, shared.clone()));
            }
        });
        Self {
            url: format!("http://{address}"),
            state,
        }
    }

    /// The base URL: `http://127.0.0.1:<port>`.
    #[must_use]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// The route for `method` and `path`, to set its replies. Calling it again for the same
    /// route returns the same route.
    #[allow(clippy::must_use_candidate)] // See `RouteBuilder`.
    pub fn on(&self, method: Method, path: &str) -> RouteBuilder {
        let mut state = lock(&self.state);
        let index = state
            .routes
            .iter()
            .position(|route| route.method == method && route.path == path)
            .unwrap_or_else(|| {
                state.routes.push(Route {
                    method,
                    path: path.to_string(),
                    once: VecDeque::new(),
                    always: None,
                    delay: Duration::ZERO,
                });
                state.routes.len() - 1
            });
        RouteBuilder {
            state: self.state.clone(),
            index,
        }
    }

    /// Every request received so far, oldest first.
    #[must_use]
    pub fn requests(&self) -> Vec<Received> {
        lock(&self.state).received.clone()
    }

    /// The requests to `path`.
    #[must_use]
    pub fn requests_to(&self, path: &str) -> Vec<Received> {
        self.requests()
            .into_iter()
            .filter(|request| request.path == path)
            .collect()
    }

    /// Forget the requests received so far (routes stay).
    pub fn clear_requests(&self) {
        lock(&self.state).received.clear();
    }
}

/// Sets a route's replies; see [`MockServer::on`].
#[derive(Debug)]
pub struct RouteBuilder {
    state: Arc<Mutex<State>>,
    index: usize,
}

// Setting a reply is the point of each call; using the returned builder to chain more is
// optional (`server.on(..).reply(..);`).
#[allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]
impl RouteBuilder {
    fn with_route(self, change: impl FnOnce(&mut Route)) -> Self {
        change(&mut lock(&self.state).routes[self.index]);
        self
    }

    /// Answer every request with `status` and `body` (after the [`once`](Self::once)
    /// replies are used up). A route without replies answers 404.
    pub fn reply(self, status: u16, body: &str) -> Self {
        let body = body.as_bytes().to_vec();
        self.with_route(|route| {
            route.always = Some(Reply {
                status,
                body,
                delay: Duration::ZERO,
            });
        })
    }

    /// [`reply`](Self::reply) with `body` as JSON.
    ///
    /// # Panics
    ///
    /// Panics if `body` cannot be serialized.
    pub fn reply_json(self, status: u16, body: &impl Serialize) -> Self {
        let body = serde_json::to_string(body).expect("a serializable body");
        self.reply(status, &body)
    }

    /// Answer the next request with `status` and `body`, once. Several `once` replies are
    /// used in order (a 503, then a 503, then whatever [`reply`](Self::reply) says).
    pub fn once(self, status: u16, body: &str) -> Self {
        let body = body.as_bytes().to_vec();
        self.with_route(|route| {
            route.once.push_back(Reply {
                status,
                body,
                delay: Duration::ZERO,
            });
        })
    }

    /// Wait `delay` before every reply on this route (to test timeouts and cancellation).
    pub fn delay(self, delay: Duration) -> Self {
        self.with_route(|route| route.delay = delay)
    }
}

/// Answer one connection.
async fn serve(mut stream: tokio::net::TcpStream, state: Arc<Mutex<State>>) {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut chunk).await.unwrap_or(0);
        if read == 0 {
            return;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let head = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
    let mut lines = head.lines();
    let mut request_line = lines.next().unwrap_or_default().split(' ');
    let method = request_line.next().unwrap_or_default().to_string();
    let target = request_line.next().unwrap_or_default();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut request = Received {
        method,
        path: path.to_string(),
        query: query.to_string(),
        ..Received::default()
    };
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            request
                .headers
                .push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    let length: usize = request
        .header("content-length")
        .and_then(|length| length.parse().ok())
        .unwrap_or(0);
    let mut body = buffer[header_end..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut chunk).await.unwrap_or(0);
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    request.body = body;
    let reply = {
        let mut state = lock(&state);
        let route = state
            .routes
            .iter_mut()
            .find(|route| route.method.as_str() == request.method && route.path == request.path);
        let reply = route.and_then(|route| {
            let mut reply = route.once.pop_front().or_else(|| route.always.clone())?;
            reply.delay = route.delay;
            Some(reply)
        });
        state.received.push(request);
        reply.unwrap_or(Reply {
            status: 404,
            body: br#"{"error": {"code": "not_found", "message": "No mock route"}}"#.to_vec(),
            delay: Duration::ZERO,
        })
    };
    tokio::time::sleep(reply.delay).await;
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
        reply.status,
        reason(reply.status),
        reply.body.len()
    );
    stream.write_all(head.as_bytes()).await.ok();
    stream.write_all(&reply.body).await.ok();
    stream.shutdown().await.ok();
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}
