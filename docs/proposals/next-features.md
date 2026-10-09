# Proposal: what to build after BLoC and HTTP

Status: proposal, for discussion. Nothing here is implemented.

rok-ui now has a standard app architecture (BLoC), a generator that writes features in it,
and an HTTP client whose errors reach forms. This proposal lists what would make that stack
complete for real apps, ordered by how much each item removes from the code people write by
hand. Each item says what problem it solves, a sketch of the API, its size, and its risks.

Sizes: **S** is a few days, **M** about a week, **L** two weeks or more.

## Summary

| # | Feature | Area | Size | Priority |
|---|---|---|---|---|
| 1 | Event transformers: debounce and throttle | bloc | S | High |
| 2 | `BlocObserver`: one place for logging, analytics and errors | bloc | S | High |
| 3 | Hydrated blocs: state that survives a restart | bloc | M | High |
| 4 | Bloc devtools: event and state timeline | bloc, devtools | M | Medium |
| 5 | Streams into blocs (`emit.for_each`), live data over SSE and WebSockets | bloc, http | M | Medium |
| 6 | HTTP middleware: retries, logging, token refresh | http | M | High |
| 7 | Multipart uploads and downloads with progress | http | M | Medium |
| 8 | Keychain token storage | http | S | Medium |
| 9 | Cached repositories on the query cache | data | M | High |
| 10 | `g api --from openapi.yaml` | cli | L | High |
| 11 | Incremental generators: `g event`, `g field`, `g remove` | cli | M | Medium |
| 12 | `cargo rok-ui doctor`: checks the architecture rules | cli | M | Medium |
| 13 | Workspace layout: `new --layout workspace` | cli | M | Low |
| 14 | Testing kit: `MockServer`, `bloc_test!`, view snapshots | testing | M | High |
| 15 | Small helpers: clipboard, save file, user agent | http, platform | S | Low |

A suggested order is at the end.

## Business logic

### 1. Event transformers: debounce and throttle

**Problem.** Search-as-you-type wants `Restartable` *and* a 300 ms pause before the call;
"save" buttons want at most one call per second. Today each app writes its own timer.

**Proposal.** Extend `Concurrency` with timed modes:

```text
fn concurrency(&self, event: &SearchEvent) -> Concurrency {
    match event {
        SearchEvent::QueryChanged { .. } => Concurrency::debounce(Duration::from_millis(300)),
        SearchEvent::SaveRequested => Concurrency::throttle(Duration::from_secs(1)),
        _ => Concurrency::Sequential,
    }
}
```

`debounce` restarts on each event and runs the last one after the pause; `throttle` runs the
first and drops the rest for the window. The generator gets
`"concurrency": "debounce:300ms"`.

**Size** S. **Risks:** none beyond test timing; the existing idle tracking already covers
pending timers.

### 2. `BlocObserver`

**Problem.** Apps want one place to log every event and transition, report handler panics,
and send analytics, without touching each bloc.

**Proposal.** A process-wide observer, set once in `main`:

```text
rok_ui::bloc::set_observer(MyObserver);

impl BlocObserver for MyObserver {
    fn on_event(&self, bloc: &'static str, event: &dyn Debug) {}
    fn on_transition(&self, bloc: &'static str, from: &dyn Debug, to: &dyn Debug) {}
    fn on_error(&self, bloc: &'static str, error: &dyn Error) {}
}
```

Requires `Event: Debug` and `State: Debug` only when an observer is set (checked at
registration through a `Debug` bound on a separate `ObservedBloc` impl, so blocs that do not
derive `Debug` still compile). Handler panics are caught and reported instead of killing the
worker.

**Size** S. **Risks:** a slow observer slows every emit; document that it must not block.

### 3. Hydrated blocs

**Problem.** Settings, drafts and the last opened tab should survive a restart. `persist`
does this for stores, not for blocs.

**Proposal.** An opt-in trait, saved through the existing `persist` machinery (debounced,
versioned, with migrations):

```text
impl HydratedBloc for SettingsBloc {
    fn key(&self) -> &'static str { "settings" }
    fn version(&self) -> u32 { 1 }
}

BlocProvider::new().with_hydrated_bloc(|scope| SettingsBloc::new(..))
```

`State: Serialize + DeserializeOwned`. The generator gets `"hydrated": true`.

**Size** M. **Risks:** state shapes change; versioning and `migrate` are required, and a state
that fails to load falls back to `initial_state` with a logged warning.

### 4. Bloc devtools

**Problem.** "Why is the page in this state?" is the most common BLoC question, and the
devtools overlay shows routes and queries but not blocs.

**Proposal.** A "Blocs" tab in the devtools overlay (Ctrl-Shift-D): every live bloc, its
current state, and a timeline of events and transitions (built on the observer from item 2),
with filtering by bloc and a "copy as test" button that writes a `test::run` call reproducing
the sequence.

**Size** M. **Risks:** memory: keep a bounded ring buffer per bloc.

### 5. Streams into blocs; live data

**Problem.** Live data (a chat, prices, `db::watch_changes`) arrives as a stream. A handler
can loop over it, but cancellation and "one subscription per bloc" are easy to get wrong.

**Proposal.**

```text
// In a handler: emits for each item until the bloc closes or the handler is restarted.
emit.for_each(repository.messages(room), |state, message| {
    let mut state = state.clone();
    state.messages.push(message);
    state
}).await;
```

and, in `rok_ui::http`, `client.events(path)` (server-sent events) returning a `Stream` of
typed items, with reconnects and the same `ApiError` and session rules. WebSockets follow
the same shape behind a separate feature.

**Size** M (SSE), +M (WebSockets). **Risks:** reconnect policy; start with SSE, which is plain
HTTP and needs no new dependency.

## HTTP and data

### 6. HTTP middleware

**Problem.** Real APIs need retries with backoff for idempotent calls, request logging, and
refreshing an access token on 401 instead of signing the user out.

**Proposal.** Layers on the client builder, run in order around each request:

```text
HttpClient::builder()
    .layer(Retry::idempotent(3).backoff(Duration::from_millis(200)))
    .layer(Log::requests())
    .layer(RefreshToken::new(|client| async move {
        client.request::<TokenDto>("/sessions/refresh", Options::post().skip_expire(true)).await
    }))
```

`RefreshToken` runs once per 401 (concurrent 401s wait for the same refresh), retries the
request with the new token, and expires the session only if the refresh fails. That keeps
rule 13 (the data layer owns the session) intact.

**Size** M. **Risks:** retrying non-idempotent requests; `Retry::idempotent` only retries
`GET`, `HEAD`, `PUT` and `DELETE`, and never after a response arrived.

### 7. Multipart uploads; progress

**Problem.** File uploads are out of scope today (raw bodies only), and large downloads have
no progress.

**Proposal.** `Options::multipart(Form::new().text("title", t).file("file", path))`, and
`client.download(path, destination, |progress| ..)` returning the final path. The generator
gets `"multipart": [...]` on endpoints.

**Size** M. **Risks:** one more reqwest feature (`multipart`); check `cargo deny`.

### 8. Keychain token storage

**Problem.** `Session::persisted` stores the token in a plain file.

**Proposal.** `Session::in_keychain(service)` on macOS Keychain, Windows Credential Manager and
the Secret Service on Linux, falling back to memory (never to a file) when unavailable.

**Size** S. **Risks:** a new dependency (`keyring`); Linux needs a running secret service, so
the fallback must be explicit and documented.

### 9. Cached repositories on the query cache

**Problem.** Repositories call the network every time. The query cache already has keys,
stale times, deduplication and invalidation, but repositories do not use it.

**Proposal.** A small wrapper that makes a repository method cached:

```text
fn list(&self) -> BoxFuture<'_, Result<Vec<Note>, ApiError>> {
    self.cache.get(query_key!["notes"], StaleTime::seconds(30), || self.provider.list_notes(None))
}

fn add(&self, ..) -> .. {
    self.cache.invalidating(query_key!["notes"], self.provider.create_note(..))
}
```

This keeps rule 7 (repositories own caching and keys). The generator gets `"cache": {"key":
"notes", "stale": "30s"}` on read methods and invalidates the feature's key on writes.

**Size** M. **Risks:** the cache lives on the UI side today; the wrapper must run on the
runtime without touching GPUI (rule 3), which means a small `Send` cache handle.

## Generator and tooling

### 10. `g api --from openapi.yaml`

**Problem.** Teams with an OpenAPI document retype every endpoint and DTO.

**Proposal.** `cargo rok-ui g api notes --from openapi.yaml --tag notes` converts the
document's operations and schemas into the existing JSON spec (`endpoints`, `dtos` with
renames, `oneOf` as untagged `variants`), prints it with `--print-spec` for review, then runs
the normal pipeline. Unsupported constructs are errors that name the OpenAPI path.

**Size** L. **Risks:** OpenAPI is large; scope to 3.0/3.1 JSON and YAML, `$ref` within one
file, and the schema shapes the spec can already express.

### 11. Incremental generators

**Problem.** After the first `g feature`, adding an event means editing four files by hand,
and regenerating conflicts with edits.

**Proposal.** Edit existing files through `syn` instead of rewriting them:

```text
cargo rok-ui g event notes NoteArchived:id:NoteId --calls archive
cargo rok-ui g field notes NotesState.filter:String
cargo rok-ui g remove feature notes
```

`g event` adds the enum variant, a handler arm calling the repository method (adding the
method to the trait, the implementation and the fake in the test), and a test. Files are
re-formatted with rustfmt; anything the tool cannot place is reported with the snippet.

**Size** M. **Risks:** hand-edited files may not have the expected shape; the tool must fail
safely (exit 2) rather than guess.

### 12. `cargo rok-ui doctor`

**Problem.** Rules 1, 6, 8 and 11 are documented but only rule 2 and 3 are enforced.

**Proposal.** A checker that parses the crate and reports violations with file and line:
a feature importing another feature, a view calling a repository, a route file with more than
rendering, a model deriving `Serialize`, an `HttpClient` outside providers, `mod.rs` files,
non-past-tense events. `--fix` applies the safe ones (renames). CI runs it in the templates
job.

**Size** M. **Risks:** false positives; every check needs an allow comment
(`// rok-ui: allow(feature-import)`).

### 13. Workspace layout

**Problem.** Large apps want the layers as crates, so the compiler enforces rules 1 and 3.

**Proposal.** `cargo rok-ui new app --template bloc --layout workspace` creates
`crates/app-data`, `crates/app-<feature>` (depending on `rok-ui-bloc` only) and `crates/app`,
following the mapping already in the architecture guide; `generate` detects the layout and
writes into the right crate.

**Size** M. **Risks:** two layouts to keep working in the generator; snapshot tests for both.

## Testing

### 14. Testing kit

**Problem.** Every app re-implements the mock server from rok-ui's own tests, and bloc tests
repeat the same set-up.

**Proposal.**

- `rok_ui::http::testing::MockServer` (feature `http-testing`): routes, recorded requests,
  delays, on localhost.
- `bloc_test!` for the common shape:

  ```text
  bloc_test! {
      adding_a_note,
      build: NotesBloc::new(Arc::new(FakeNotes::default())),
      act: [NotesEvent::NoteAdded { title: "Milk".into() }],
      expect: [loading(), success_with(1)],
  }
  ```

- View snapshots: render a page in a test window with a given state and compare its element
  tree (text, roles) to a checked-in snapshot.

The generator writes tests with these instead of hand-rolled fakes.

**Size** M. **Risks:** the macro's error messages; keep it a thin layer over `test::run`.

### 15. Small helpers

`copy_to_clipboard(text, cx)`, `save_bytes(bytes, suggested_name, cx)` (a save dialog), and
`describe_user_agent()` (app name, version, OS) for a default `User-Agent` header. They were
deferred from the HTTP work. **Size** S.

## Suggested order

1. **0.8:** items 1, 2, 6, 14. They make what exists safe to ship: timed events, one place for
   errors, token refresh and retries, and test tools apps can reuse.
2. **0.9:** items 3, 9, 11, 4. State that persists, caching through repositories, editing
   generated features without conflicts, and seeing what blocs do.
3. **0.10:** items 10, 12, 5, 7, 8. Bigger tooling and transport work.
4. **Later:** items 13 and 15, and the open items already in the roadmap (scoped `Cx` values,
   `#[shard]`, a public API check in CI, the devtools forms panel).

## Open questions

- Should `BlocObserver` and devtools share one event bus with the query devtools, or stay
  separate?
- Is `debounce` better as a concurrency mode (item 1) or as a separate transformer that
  composes with the four modes?
- For OpenAPI import, is a separate crate (`rok-ui-openapi`) preferable, to keep the CLI's
  dependency tree small?
