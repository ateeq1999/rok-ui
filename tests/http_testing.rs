//! `MockServer`: routes, one-shot replies, delays, recorded requests.

use std::time::Duration;

use rok_ui::http::{testing::MockServer, ApiError, CancelToken, HttpClient, Method, Options};

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    rok_ui::runtime::block_on(future)
}

#[test]
fn routes_answer_and_requests_are_recorded() {
    let server = MockServer::start();
    server
        .on(Method::Get, "/notes")
        .reply_json(200, &serde_json::json!([{"id": 1}]));
    server.on(Method::Post, "/notes").reply(201, r#"{"id": 2}"#);
    let client = HttpClient::new(server.url());

    let notes: serde_json::Value =
        block_on(client.request("/notes", Options::get().query("q", "milk"))).unwrap();
    assert_eq!(notes[0]["id"], 1);
    let created: serde_json::Value = block_on(client.request(
        "/notes",
        Options::post().json(&serde_json::json!({"title": "Milk"})),
    ))
    .unwrap();
    assert_eq!(created["id"], 2);

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        (requests[0].method.as_str(), requests[0].query.as_str()),
        ("GET", "q=milk")
    );
    assert_eq!(requests[1].json()["title"], "Milk");
    assert_eq!(requests[1].header("content-type"), Some("application/json"));
    assert_eq!(server.requests_to("/notes").len(), 2);

    let missing = block_on(client.request_empty("/nothing", Options::get())).unwrap_err();
    assert_eq!((missing.status, missing.code.as_str()), (404, "not_found"));
    server.clear_requests();
    assert_eq!(server.requests(), []);
}

#[test]
fn once_replies_come_first_then_the_default() {
    let server = MockServer::start();
    server
        .on(Method::Get, "/flaky")
        .once(503, "")
        .once(502, "")
        .reply(200, "{}");
    let client = HttpClient::new(server.url());
    let statuses: Vec<u16> = (0..3)
        .map(
            |_| match block_on(client.request_empty("/flaky", Options::get())) {
                Ok(()) => 200,
                Err(error) => error.status,
            },
        )
        .collect();
    assert_eq!(statuses, [503, 502, 200]);
}

#[test]
fn delays_let_tests_cancel_requests() {
    let server = MockServer::start();
    server
        .on(Method::Get, "/slow")
        .delay(Duration::from_secs(5))
        .reply(200, "{}");
    let client = HttpClient::new(server.url());
    let token = CancelToken::new();
    let canceller = token.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        canceller.cancel();
    });
    let error: ApiError =
        block_on(client.request_empty("/slow", Options::get().cancel(&token))).unwrap_err();
    assert!(error.is_cancelled());
}
