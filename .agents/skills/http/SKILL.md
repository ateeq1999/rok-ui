---
name: http
description: Call JSON APIs with `rok_ui::http` (HttpClient, ApiError, Session), show API errors on forms, generate HTTP data layers with `cargo rok-ui g api`, or change the `http` feature.
---

# HTTP (`http` feature)

The guide is `docs/guide/http.md` (also the module docs); decisions are in
`docs/design/http.md`; the BLoC rules 10 to 15 are in `docs/guide/architecture.md`.

## App code

- One `HttpClient` per app, built in `main` with the `Session`; only providers call it. The
  `bloc-http` template (`cargo rok-ui new app --template bloc --http`) is the reference.
- Providers hold paths, verbs and DTOs (`serde` derives and renames). Repositories convert
  DTOs to models and return `Result<_, ApiError>`. Generate them: `cargo rok-ui g api ...`.
- Handlers ignore cancellations: `Err(error) if error.is_cancelled() => {}`.
- Sign-in and other calls where a 401 is an answer use `Options::skip_expire(true)`.
  Repositories own the token (`session.set_token`, `session.clear`); views watch the
  session only to navigate.
- Forms: `form::to_server_errors` + `Form::apply_server_errors` when a bloc submits;
  `form::use_api_form` when the form calls the API itself.
- The token is in memory unless `Session::persisted(path)`; say so when it matters
  (plain-text file). Keychain storage does not exist yet.

## Working on rok-ui itself

- `src/http/`: `client.rs` (requests, headers, the 401 rule), `error.rs` (envelope parsing
  and fallbacks), `options.rs`, `session.rs`, `cancel.rs`. `src/form/server.rs` is the forms
  bridge (features `form` + `http`).
- Tests use the mock server in `tests/http.rs` (tokio `TcpListener`, never the network) and
  `tests/http_form.rs` for forms. Add a case there for any change to request or error
  handling.
- reqwest stays at 0.12 with `rustls-tls-native-roots` (ring, which GPUI already builds);
  run `cargo deny check` after touching dependencies.
