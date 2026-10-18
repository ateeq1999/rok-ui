# rok-ui-http

The HTTP client behind `rok_ui::http`, without GPUI: an `HttpClient` for JSON APIs, `ApiError`
(parsed from the API's error envelope, with field errors that rok-ui forms show), a `Session`
for the signed-in token, retries, token refresh, cancellation and multipart uploads.

Apps use it through `rok_ui::http` (feature `http`). Depend on this crate directly where GPUI
should not reach: the data crate of an app split into crates
(`cargo rok-ui new --template bloc --http --layout workspace`).

See the [HTTP guide](https://github.com/ateeq1999/rok-ui/blob/main/docs/guide/http.md).
