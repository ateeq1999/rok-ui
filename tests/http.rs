//! `rok_ui::http` against a mock server on localhost: headers, bodies, query strings, empty
//! responses, the error envelope, session expiry, cancellation and network failures.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use rok_ui::http::{ApiError, CancelToken, HttpClient, Options, Session};
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

/// A request the server received.
#[derive(Clone, Debug, Default)]
struct Received {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Received {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// What the server answers.
struct Reply {
    status: u16,
    reason: &'static str,
    body: Vec<u8>,
    delay: Duration,
}

impl Reply {
    fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            reason: reason(status),
            body: body.as_bytes().to_vec(),
            delay: Duration::ZERO,
        }
    }

    fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        500 => "Internal Server Error",
        _ => "",
    }
}

type Handler = Arc<dyn Fn(&Received) -> Reply + Send + Sync>;

/// Serve `handler` on a free port; returns the base URL and what the server received.
fn serve(
    handler: impl Fn(&Received) -> Reply + Send + Sync + 'static,
) -> (String, Arc<Mutex<Vec<Received>>>) {
    let handler: Handler = Arc::new(handler);
    let received = Arc::new(Mutex::new(Vec::new()));
    let listener = rok_ui::runtime::block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    let log = received.clone();
    rok_ui::runtime::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let handler = handler.clone();
            let log = log.clone();
            rok_ui::runtime::spawn(async move {
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
                let mut request = Received {
                    method: request_line.next().unwrap_or_default().to_string(),
                    target: request_line.next().unwrap_or_default().to_string(),
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
                let reply = handler(&request);
                log.lock().unwrap().push(request);
                tokio::time::sleep(reply.delay).await;
                let head = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                    reply.status,
                    reply.reason,
                    reply.body.len()
                );
                stream.write_all(head.as_bytes()).await.ok();
                stream.write_all(&reply.body).await.ok();
                stream.shutdown().await.ok();
            });
        }
    });
    (format!("http://{address}"), received)
}

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    rok_ui::runtime::block_on(future)
}

#[derive(Debug, Deserialize, PartialEq)]
struct Note {
    id: u64,
    title: String,
}

#[test]
fn requests_send_headers_query_and_token() {
    let (url, received) = serve(|_| Reply::json(200, r#"[{"id": 1, "title": "Milk"}]"#));
    let session = Session::with_token("secret");
    let client = HttpClient::builder()
        .base_url(format!("{url}/v1/"))
        .header("X-App", "rok")
        .header("X-Override", "client")
        .session(session)
        .build();
    let notes: Vec<Note> = block_on(
        client.request(
            "/notes",
            Options::get()
                .query("search", "milk")
                .query("tag", None::<String>)
                .query("empty", "")
                .query("page", 2)
                .header("x-override", "request"),
        ),
    )
    .unwrap();
    assert_eq!(
        notes,
        [Note {
            id: 1,
            title: "Milk".into()
        }]
    );
    let request = received.lock().unwrap()[0].clone();
    assert_eq!(request.method, "GET");
    assert_eq!(request.target, "/v1/notes?search=milk&page=2");
    assert_eq!(request.header("accept"), Some("application/json"));
    assert_eq!(request.header("authorization"), Some("Bearer secret"));
    assert_eq!(request.header("x-app"), Some("rok"));
    assert_eq!(request.header("x-override"), Some("request"));
}

#[test]
fn bodies_are_json_or_raw() {
    let (url, received) = serve(|_| Reply::json(204, ""));
    let client = HttpClient::new(url);
    block_on(client.request_empty(
        "/a",
        Options::post().json(&serde_json::json!({"title": "x"})),
    ))
    .unwrap();
    block_on(client.request_empty("/b", Options::put().raw_body(b"\x00\x01".to_vec()))).unwrap();
    block_on(
        client.request_empty(
            "/c",
            Options::patch()
                .json(&1)
                .raw_body(b"png".to_vec())
                .content_type("image/png"),
        ),
    )
    .unwrap();
    let received = received.lock().unwrap();
    assert_eq!(received[0].header("content-type"), Some("application/json"));
    assert_eq!(received[0].body, br#"{"title":"x"}"#);
    assert_eq!(received[1].method, "PUT");
    assert_eq!(
        received[1].header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(received[1].body, b"\x00\x01");
    assert_eq!(received[2].header("content-type"), Some("image/png"));
    assert_eq!(received[2].body, b"png");
}

#[test]
fn empty_responses_decode_as_null() {
    let (url, _) = serve(|request| match request.target.as_str() {
        "/no-content" => Reply::json(204, ""),
        _ => Reply::json(200, ""),
    });
    let client = HttpClient::new(url);
    let unit: () = block_on(client.request("/no-content", Options::delete())).unwrap();
    assert_eq!(unit, ());
    let missing: Option<Note> = block_on(client.request("/empty", Options::get())).unwrap();
    assert_eq!(missing, None);
    let error = block_on(client.request::<Note>("/empty", Options::get())).unwrap_err();
    assert_eq!((error.status, error.code.as_str()), (200, "empty_response"));
    assert!(error.message.contains("Note"), "{}", error.message);
    let bytes = block_on(client.request_bytes("/no-content", Options::get())).unwrap();
    assert!(bytes.is_empty());
}

#[test]
fn errors_come_from_the_envelope_or_the_status() {
    let (url, _) = serve(|request| match request.target.as_str() {
        "/invalid" => Reply::json(
            422,
            r#"{"error": {"code": "validation_failed", "message": "Check the form",
                "details": {"email": [{"code": "taken", "message": "Already registered"}]}}}"#,
        ),
        "/large" => Reply::json(413, ""),
        "/html" => Reply::json(500, "<html>oops</html>"),
        _ => Reply::json(200, "{not json"),
    });
    let client = HttpClient::new(url);
    let error = block_on(client.request::<Note>("/invalid", Options::post())).unwrap_err();
    assert_eq!(
        (error.status, error.code.as_str()),
        (422, "validation_failed")
    );
    assert_eq!(error.field_message("email"), Some("Already registered"));
    let error = block_on(client.request_empty("/large", Options::post())).unwrap_err();
    assert_eq!(
        (error.code.as_str(), error.message.as_str()),
        ("unknown", "The file is too large.")
    );
    let error = block_on(client.request_empty("/html", Options::get())).unwrap_err();
    assert_eq!(error.message, "Internal Server Error");
    let error = block_on(client.request::<Note>("/garbage", Options::get())).unwrap_err();
    assert_eq!(
        (error.status, error.code.as_str()),
        (200, "invalid_response")
    );
}

#[test]
fn a_401_expires_the_session_it_was_sent_with() {
    let session = Session::new();
    let changer = session.clone();
    let (url, _) = serve(move |request| {
        if request.target == "/race" {
            // The user signed in again while this request was in flight.
            changer.set_token("newer");
        }
        Reply::json(
            401,
            r#"{"error": {"code": "unauthorized", "message": "Sign in again"}}"#,
        )
    });
    let client = HttpClient::builder()
        .base_url(url)
        .session(session.clone())
        .build();

    let error = block_on(client.request_empty("/anonymous", Options::get())).unwrap_err();
    assert_eq!(error.status, 401);
    assert!(!session.is_expired(), "no token was sent");

    session.set_token("a");
    block_on(client.request_empty("/login", Options::post().skip_expire(true))).unwrap_err();
    assert!(
        session.is_signed_in() && !session.is_expired(),
        "skip_expire"
    );

    block_on(client.request_empty("/race", Options::get())).unwrap_err();
    assert_eq!(
        session.token().as_deref(),
        Some("newer"),
        "a newer token is kept"
    );

    block_on(client.request_empty("/me", Options::get())).unwrap_err();
    assert!(session.is_expired() && !session.is_signed_in());

    session.set_token("b");
    block_on(client.request_empty(
        "/custom",
        Options::get().header("Authorization", "Bearer other"),
    ))
    .unwrap_err();
    assert!(
        !session.is_expired(),
        "the session's token was not the one sent"
    );
}

#[test]
fn cancelled_requests_end_at_once_and_expire_nothing() {
    let (url, _) = serve(|_| Reply::json(401, "").delayed(Duration::from_secs(5)));
    let session = Session::with_token("t");
    let client = HttpClient::builder()
        .base_url(url)
        .session(session.clone())
        .build();
    let token = CancelToken::new();
    let canceller = token.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        canceller.cancel();
    });
    let started = std::time::Instant::now();
    let error = block_on(client.request_empty("/slow", Options::get().cancel(&token))).unwrap_err();
    assert!(error.is_cancelled(), "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!session.is_expired());
    let error = block_on(client.request_empty("/slow", Options::get().cancel(&token))).unwrap_err();
    assert!(
        error.is_cancelled(),
        "an already cancelled token ends the request before it starts"
    );
}

#[test]
fn unreachable_servers_are_network_errors() {
    // Bind and drop to find a port nobody listens on.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let client = HttpClient::builder()
        .base_url(format!("http://127.0.0.1:{port}"))
        .timeout(Duration::from_secs(5))
        .build();
    let error = block_on(client.request_empty("/", Options::get())).unwrap_err();
    assert_eq!(error, ApiError::network());
    assert_eq!(
        (error.status, error.message.as_str()),
        (0, "Cannot reach the server")
    );
}

#[test]
fn requests_work_outside_a_tokio_runtime() {
    let (url, _) = serve(|_| Reply::json(200, r#"{"id": 7, "title": "Off runtime"}"#));
    let client = HttpClient::new(url);
    // Like a GPUI task: an executor without a tokio reactor.
    let note: Note = futures::executor::block_on(client.request("/note", Options::get())).unwrap();
    assert_eq!(note.id, 7);
}
