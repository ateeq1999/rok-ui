# Design: `rok_ui::http`

Status: implemented (Part M of `roadmap.md`). This note records the decisions and where they
differ from the request.

## Runtime

reqwest's async client needs a tokio reactor; GPUI's executor is not one.
`rok_ui::runtime` already is: one multi-threaded tokio runtime shared by `db`, `query` and
`bloc`. So `http` owns no runtime:

- `HttpClient::request` is an `async fn`. Called from a bloc handler, a query fetcher or a
  procedure (all on `rok_ui::runtime`) it runs in place.
- Called from anywhere else (a GPUI task, a test), it notices there is no tokio context
  (`Handle::try_current`) and moves the request onto `rok_ui::runtime`, awaiting the join
  handle. The result is a plain value, so GPUI code awaits it like any other future.

## Cancellation

Two ways, both ending in `ApiError::cancelled()` (or no result at all) and never in a failure
state (rule 14):

1. Dropping the future drops the reqwest request. A `Restartable` bloc event, a closed bloc
   and a query cancelled because the user navigated away all work this way; the task is gone,
   so nothing is emitted.
2. `Options::cancel(&token)` with a `CancelToken` (cheap clone): `token.cancel()` makes the
   pending call return `Err(ApiError::cancelled())` at once. Callers check
   `error.is_cancelled()` and do nothing.

A cancelled request never reaches the session-expiry check.

## Session

The request asked for a `#[derive(Store)]` store. Stores are rok-ui signals: single-threaded
(`Rc`) and meant for the UI thread. Requests run on tokio worker threads and must read the
token and call `expire()` there, so `Session` is `Send + Sync` (a mutex around the token and
an expiry flag) and notifies subscribers on change. `Session` implements the bloc
`Observable` trait, so views watch it with `BlocBuilder` / `BlocListener` like any bloc, and
guards read `session.is_expired()`.

Expiry rule, as in the reference client: a `401` expires the session only when the call did
not pass `skip_expire`, a token was sent, and the session still holds that same token.

## Token storage

In memory by default. `Session::persisted(path)` writes the token to a JSON file (put it in
the app's config directory, `rok_ui::persist::config_dir()`): anyone who can read the user's
files can read the token. There is no keychain dependency in the tree today; OS keychain storage is
listed in the roadmap as not started.

## Dependencies

| Crate | Why | Features |
|---|---|---|
| `reqwest` 0.12 | The transport the request names. 0.13 defaults to aws-lc (a C build); 0.12 uses ring, which GPUI's own reqwest fork already builds | `default-features = false`, `json`, `rustls-tls-native-roots` (rustls, the OS trust store; no OpenSSL) |
| `bytes` | `request_bytes` returns `Bytes` (already in the tree through reqwest and GPUI) | default |
| `serde`, `serde_json` | DTOs and the error envelope (already optional dependencies) | |

Tests use a small mock server written on `tokio::net::TcpListener` in the test file, not a
mock-server crate and never the network.

## Differences from the request

- `Session` is not a `Store` (see above).
- Multipart uploads, websockets and OAuth flows beyond `api_url` are out of scope.
- The `copy_to_clipboard` / `save_bytes` / `describe_user_agent` helpers are left for a
  follow-up (the request allows it); they are listed in the roadmap.
