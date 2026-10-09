# HTTP

`rok_ui::http` (feature `http`) is the client for JSON APIs in the data layer: an
[`HttpClient`], one error type, [`ApiError`], and the signed-in [`Session`]. In the BLoC
architecture only providers call it (rules 10 to 15 of the architecture guide).

## The client

Build one client in `main`, with the session, and hand it to the providers:

```rust,no_run
use std::time::Duration;

use rok_ui::http::{HttpClient, Session};

let session = Session::new();
let client = HttpClient::builder()
    .base_url("https://api.example.com/v1") // default: ROK_API_URL, else http://localhost:8080
    .header("X-App-Version", "1.0")
    .timeout(Duration::from_secs(15)) // default: 30 seconds
    .session(session.clone())
    .build();
assert_eq!(client.api_url("/notes"), "https://api.example.com/v1/notes");
```

Every request sends `Accept: application/json`, the client's headers, then
`Authorization: Bearer <token>` while the session has a token, then the request's own headers,
which win.

## Requests

```rust,no_run
use rok_ui::http::{path_segment, ApiError, Bytes, HttpClient, Options};

#[derive(serde::Deserialize)]
struct NoteDto {
    id: u64,
    title: String,
}

#[derive(serde::Serialize)]
struct NewNoteDto<'a> {
    title: &'a str,
}

async fn calls(client: &HttpClient) -> Result<(), ApiError> {
    // GET /notes?q=milk (a `None` or empty value is left out)
    let notes: Vec<NoteDto> = client
        .request("/notes", Options::get().query("q", "milk").query("tag", None::<String>))
        .await?;
    // POST a JSON body
    let note: NoteDto = client
        .request("/notes", Options::post().json(&NewNoteDto { title: "Milk" }))
        .await?;
    // DELETE; the body is ignored
    client
        .request_empty(&format!("/notes/{}", path_segment(&note.id)), Options::delete())
        .await?;
    // Raw bytes up and down
    let _: Bytes = client
        .request_bytes("/files/1", Options::get())
        .await?;
    client
        .request_empty("/files", Options::put().raw_body(vec![1, 2, 3]).content_type("image/png"))
        .await?;
    let _ = (notes, note.title);
    Ok(())
}
```

- An empty body (or a 204) decodes as `null`: fine for `()` and `Option<T>` (`None`); for any
  other `T` it is an `ApiError` with code `empty_response`.
- A raw body wins over a JSON one; its content type defaults to `application/octet-stream`.
- Requests are `async` and run on the shared runtime. Called from outside a tokio runtime (a
  GPUI task), they move onto it, so any executor can await them.

## Errors

Every failure is an [`ApiError`]: `status`, a machine-readable `code`, a `message` for people,
and per-field `details`. It is parsed from the server's envelope:

```text
{"error": {"code": "validation_failed", "message": "Check the form",
           "details": {"email": [{"code": "taken", "message": "Already registered"}]}}}
```

| Situation | `status` | `code` | `message` |
|---|---|---|---|
| Envelope | the status | its code, else `unknown` | its message, else below |
| 413 without a message | 413 | `unknown` | The file is too large. |
| 415 without a message | 415 | `unknown` | That file type is not supported. |
| Other, without a message | the status | `unknown` | the reason phrase, else "Request failed" |
| No response (network, timeout) | 0 | `network_error` | Cannot reach the server |
| Cancelled | 0 | `cancelled` | (never shown) |

```rust
use rok_ui::http::ApiError;

let error = ApiError::from_response(404, Some("Not Found"), b"<html>");
assert_eq!((error.code.as_str(), error.message.as_str()), ("unknown", "Not Found"));
assert!(ApiError::cancelled().is_cancelled());
```

## Cancellation

Dropping a request's future cancels it: a `Restartable` bloc event or a closed bloc does this
for you. To cancel explicitly, pass a [`CancelToken`]; the call returns
`ApiError::cancelled()` at once. A cancelled request is never a failure: match
`Err(error) if error.is_cancelled() => {}` and emit nothing.

```rust,no_run
use rok_ui::http::{CancelToken, HttpClient, Options};

async fn search(client: &HttpClient, token: &CancelToken) {
    match client.request::<Vec<String>>("/search", Options::get().cancel(token)).await {
        Ok(results) => println!("{results:?}"),
        Err(error) if error.is_cancelled() => {}
        Err(error) => eprintln!("{error}"),
    }
}
```

## The session

[`Session`] holds the bearer token and an `expired` flag. Clones share it; it is `Send + Sync`
and a bloc `Observable`, so a `BlocListener` on it reacts when it changes (the `bloc-http`
template's `SessionGuard` sends the user to sign in).

- `set_token(token)` signs in (and clears `expired`); `clear()` signs out.
- A **401 expires the session** only when the request did not set
  [`skip_expire`](Options::skip_expire), sent a token, and the session still holds that token
  (a slow request from before a new sign-in does not sign the user out).
- Use `skip_expire(true)` where a 401 is an answer: signing in, checking a password.

```rust
use rok_ui::http::Session;

let session = Session::with_token("abc");
assert!(session.is_signed_in());
session.expire();
assert!(session.is_expired() && !session.is_signed_in());
```

The token lives in memory. `Session::persisted(path)` keeps it in a JSON file across launches;
the file is plain text, readable by anyone who can read the user's files, so prefer
short-lived tokens. OS keychain storage is not available yet.

## Retries, logging and token refresh

The client builder adds behavior around every request:

```rust,no_run
use std::time::Duration;

use rok_ui::http::{HttpClient, Options, Retry, Session};

#[derive(serde::Deserialize)]
struct TokenDto {
    token: String,
}

let session = Session::new();
let client = HttpClient::builder()
    .session(session)
    // GET, HEAD, PUT and DELETE get up to 3 more tries when the server cannot be reached or
    // answers 502, 503 or 504. A POST is never repeated.
    .retry(Retry::idempotent(3).backoff(Duration::from_millis(200)))
    // Every attempt: method, URL, status, time taken, attempt number.
    .on_response(|info| println!("{info}"))
    // A 401 on the session's token: get a new one and retry once, instead of signing out.
    .refresh_token(|client| async move {
        let fresh: TokenDto = client
            .request("/sessions/refresh", Options::post().skip_expire(true))
            .await?;
        Ok(fresh.token)
    })
    .build();
# let _ = client;
```

`log_requests()` prints each attempt to standard error. With `refresh_token`, several
requests failing at once share one refresh; the session expires only when the refresh
itself fails.

## Testing against a mock server

With the `http-testing` feature (in `[dev-dependencies]`), `http::testing::MockServer`
serves canned replies on localhost: `server.on(Method::Get, "/notes").reply_json(200, &notes)`,
one-shot replies with `.once(503, "")`, delays with `.delay(..)`, and the requests it got
from `server.requests()`. Point an `HttpClient` at `server.url()` and test providers and
repositories without the network.

## Errors on forms

With the `form` feature, `form::to_server_errors` sorts an `ApiError` for a form: 422 details
go to their fields (through `ServerErrorOptions::map` when the API's names differ), a 409, 400
or 401 to the field you choose, a 429 to "Too many attempts. Try again in a minute.", and
anything else to the form. `Form::apply_server_errors` shows them (for forms that submit
through a bloc), and `form::use_api_form` wires a form that calls the API directly, with
`form_error()` and an `on_success` that runs only after a success.

## Generating it

`cargo rok-ui g api notes --dto "NoteDto:id:NoteId,title:String" --endpoint
"list_notes:GET:/notes:Vec<NoteDto>"` writes the provider, DTOs, models and repository; the
JSON form also covers query parameters, bodies, sign-in and form errors (see the CLI guide).
