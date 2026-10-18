# Architecture: BLoC

Every rok-ui app, small or large, is built the same way: three layers, one folder structure,
and a generator that writes the boilerplate. The pattern is BLoC (Business Logic Component),
as described at [bloclibrary.dev](https://bloclibrary.dev/architecture/).

| Layer | What it holds | Where |
|---|---|---|
| Data | Models; providers that talk to a database, an API or a file; repositories that turn that into models | `src/data/` |
| Business logic | Blocs and cubits: events (or method calls) in, immutable states out | `src/features/<feature>/bloc/` |
| Presentation | Pages and widgets that show states and add events; thin route files | `src/features/<feature>/view/`, `src/routes/` |

This module (feature `bloc`) has the pieces: [`Bloc`], [`Cubit`], [`Emitter`],
[`BlocProvider`], [`RepositoryProvider`], [`BlocBuilder`], [`BlocListener`], [`BlocConsumer`],
[`BlocSelector`], and the [`test`](mod@test) helpers. The bloc layer itself only needs the
GPUI-free `rok-ui-bloc` crate, which this module re-exports.

Start a new app in this layout with `cargo rok-ui new my-app --template bloc` (add `--http`
for an API client and sign-in, or `--layout workspace` for a crate per layer), and add
features with `cargo rok-ui g feature <name>` (see the CLI guide, `docs/guide/cli.md`).

## Folder structure

```text
my-app/
  Cargo.toml
  clippy.toml                 GPUI types are denied in the bloc layer (rule 3)
  build.rs                    turns src/routes into the route tree
  src/
    main.rs                   opens the window (and builds the HttpClient, with --http)
    lib.rs                    app, data, features, shared; the route tree
    app.rs                    composition root: repositories, then blocs, then routes
    data.rs                   barrel: models, providers, repositories (plus error.rs)
    data/
      error.rs                DataError, for non-HTTP providers
      models.rs
      models/note.rs          Note, NoteId: plain values the whole app shares
      providers.rs
      providers/notes_api.rs  where data comes from: paths, SQL, files, DTOs
      repositories.rs
      repositories/notes_repository.rs   trait NotesRepository + NotesRepositoryImpl
    features.rs
    features/
      notes.rs                barrel: bloc, view
      notes/
        bloc.rs               barrel; #![deny(clippy::disallowed_types)]
        bloc/notes_bloc.rs    NotesBloc
        bloc/notes_event.rs   NotesEvent (past tense: NoteAdded)
        bloc/notes_state.rs   NotesState (+ NotesStatus)
        view.rs
        view/notes_page.rs    NotesPage
        view/widgets.rs       widgets only this feature uses (optional)
    shared.rs                 code every feature may use
    shared/widgets.rs
    routes/
      __root.rs               layout: navigation (and the session guard, with --http)
      index.rs
      notes.rs                thin: renders features::notes::view::notes_page::NotesPage
  tests/
    features/main.rs          the `features` test crate
    features/notes.rs
    features/notes/notes_bloc_test.rs   events in, states out, with a fake repository
```

Modules with children are a file next to a directory (`notes.rs` and `notes/`), never
`mod.rs`. Barrel files only declare modules.

## The rules

Rules 1 to 9 hold for every app; rules 10 to 15 for apps that talk to an HTTP API.

1. **Dependencies point one way**: presentation -> business logic -> data. `data` never
   imports `features`; a bloc never imports a view; a feature never imports another feature
   (what two features share moves to `shared` or `data`).
2. **A bloc never holds another bloc.** Blocs that need the same data share a repository;
   a reaction across blocs ("when the user signs out, clear the notes") is a
   [`BlocListener`] in a view. Enforced: [`BlocHandle`] is not `Send`, so a bloc holding one
   does not compile, and the template's `clippy.toml` denies the type in the bloc layer.
3. **The bloc layer has no GPUI types.** No `App`, `Window`, `Entity`, elements or `Cx` in
   `features/<f>/bloc/`: blocs are plain Rust, testable without a window. Enforced by
   `clippy.toml` (below) and by `Bloc: Send + Sync`.
4. **States are immutable values**: `Clone + PartialEq`. A handler emits a new state; an
   equal state is not emitted, so views do not re-render for nothing.
5. **Names say what things are.** Events are past tense (`NoteAdded`, `NotesRequested`);
   states are `<Name>State` with a `<Name>Status`; files are `<name>_bloc.rs`,
   `<name>_cubit.rs`, `<name>_event.rs`, `<name>_state.rs`; repositories are a
   `<Name>Repository` trait with a `<Name>RepositoryImpl`; providers are `<Name>Api`,
   `<Name>MemoryProvider` and so on.
6. **Views only read state and add events** (or call cubit methods). A view never calls a
   repository or a provider.
7. **Repositories own data concerns**: caching and query keys, mapping DTOs and rows to
   models, and errors. Blocs see models and one error type.
8. **Route files are thin**: a route renders a feature's page. Loading, state and logic live
   in the feature.
9. **Barrel files, no `mod.rs`**, and one item per file in the bloc layer.
10. **One `HttpClient` per app**, built in `main` with the `Session`, and handed to the
    providers in `app.rs`. Only providers call it.
11. **Paths, verbs and wire names live in the provider file**: endpoints, DTOs and their
    `serde` renames. Models do not derive `serde` for the wire; repositories convert.
12. **Every HTTP failure is an `ApiError`.** Repositories return `Result<_, ApiError>`, the
    state carries it, and forms show it with `form::to_server_errors`.
13. **The session belongs to the data layer**: repositories sign in (store the token) and sign
    out. Views watch the session only to navigate (the template's `SessionGuard`).
14. **A cancelled request is not a failure.** Handlers match
    `Err(error) if error.is_cancelled() => {}` and emit nothing; searches use `Restartable`,
    submits `Droppable`.
15. **Calls where a 401 is an answer** (signing in, checking a password) use
    `Options::skip_expire(true)`, so a wrong password does not end the session.

### Enforcing rule 3

clippy's `disallowed_types` lint applies to a whole crate, so the template turns it off for
the crate and back on for each bloc layer:

```toml
# clippy.toml
disallowed-types = [
    { path = "gpui::App", reason = "the bloc layer has no GPUI: emit state, let views render it" },
    { path = "gpui::Window", reason = "the bloc layer has no GPUI: emit state, let views render it" },
    { path = "gpui::AnyElement", reason = "the bloc layer has no GPUI: emit state, let views render it" },
    { path = "gpui::Entity", reason = "the bloc layer has no GPUI: emit state, let views render it" },
    { path = "rok_ui::Cx", reason = "the bloc layer has no GPUI: emit state, let views render it" },
    { path = "rok_ui::bloc::BlocHandle", reason = "a bloc never holds another bloc" },
]
```

```toml
# Cargo.toml
[lints.clippy]
disallowed_types = "allow"
```

```text
// src/features/notes/bloc.rs
#![deny(clippy::disallowed_types)]
```

## A bloc

Events go in; the handler emits states through the [`Emitter`]. Handlers are `async`: they
await the repository, and they run on the shared runtime, never on the UI thread.

```rust,no_run
use std::sync::Arc;

use rok_ui::bloc::{Bloc, BoxFuture, Emitter};

/// A note.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub title: String,
}

/// Where notes come from: a trait, so tests can pass a fake.
pub trait NotesRepository: Send + Sync {
    fn list(&self) -> BoxFuture<'_, Result<Vec<Note>, String>>;
}

/// What happened (past tense).
#[derive(Debug)]
pub enum NotesEvent {
    NotesRequested,
}

/// What the view shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum NotesState {
    #[default]
    Initial,
    Loading,
    Loaded(Vec<Note>),
    Failed(String),
}

pub struct NotesBloc {
    repository: Arc<dyn NotesRepository>,
}

impl Bloc for NotesBloc {
    type Event = NotesEvent;
    type State = NotesState;

    fn initial_state(&self) -> NotesState {
        NotesState::Initial
    }

    async fn on(&self, event: NotesEvent, emit: &Emitter<NotesState>) {
        match event {
            NotesEvent::NotesRequested => {
                emit.emit(NotesState::Loading);
                match self.repository.list().await {
                    Ok(notes) => emit.emit(NotesState::Loaded(notes)),
                    Err(error) => emit.emit(NotesState::Failed(error)),
                };
            }
        }
    }
}
```

A [`Cubit`] is the same without events: its methods change the state through its emitter.
Use one when the events would only name the methods.

### Concurrency

[`Bloc::concurrency`] says what happens when an event arrives while others run:

| Mode | Behavior | Use for |
|---|---|---|
| `Sequential` (default) | One at a time, in order | Writes |
| `Droppable` | Ignored while one of the same variant runs | Submits, refresh buttons |
| `Restartable` | Cancels the running one of the same variant | Search as you type |
| `Concurrent` | All at once | Independent reads |
| `Debounce(delay)` | Waits `delay` after the last one of the same variant, then handles it; a newer one cancels it | Search as you type, without a call per key |
| `Throttle(window)` | Handles one, then drops the same variant until `window` has passed | Save buttons, scroll handlers |

`Concurrency::debounce(Duration::from_millis(300))` and `Concurrency::throttle(..)` build
the timed modes. Closing a bloc (its provider leaves the tree) cancels its running handlers.

## Providing and reading

`app.rs` builds the repositories once and provides blocs above the routes, so every page reads
the same instances. Views read a bloc with `cx.bloc::<B>()` and re-render through
[`BlocBuilder`] (or `cx.watch_bloc`):

```rust,no_run
# use std::sync::Arc;
# use rok_ui::{prelude::*, bloc::{Bloc, BlocBuilder, BlocProvider, BoxFuture, Emitter, RepositoryProvider}};
# #[derive(Clone, Debug, PartialEq)] pub struct Note { pub title: String }
# pub trait NotesRepository: Send + Sync { fn list(&self) -> BoxFuture<'_, Vec<Note>>; }
# struct Fixed;
# impl NotesRepository for Fixed { fn list(&self) -> BoxFuture<'_, Vec<Note>> { Box::pin(async { Vec::new() }) } }
# #[derive(Debug)] pub enum NotesEvent { NotesRequested }
# #[derive(Clone, Debug, PartialEq)] pub enum NotesState { Initial, Loaded(Vec<Note>) }
# pub struct NotesBloc { repository: Arc<dyn NotesRepository> }
# impl Bloc for NotesBloc {
#     type Event = NotesEvent; type State = NotesState;
#     fn initial_state(&self) -> NotesState { NotesState::Initial }
#     async fn on(&self, _: NotesEvent, _: &Emitter<NotesState>) {}
# }
#[component]
fn NotesPage(cx: &mut Cx) -> impl IntoElement {
    let notes = cx.bloc::<NotesBloc>();
    let refresh = notes.clone();
    div()
        .child(Button::new("refresh").label("Refresh").on_click(move |_, _, _| {
            refresh.add(NotesEvent::NotesRequested);
        }))
        .child(BlocBuilder::new(&notes, |state, _, _| match state {
            NotesState::Initial => div().child("Nothing yet"),
            NotesState::Loaded(notes) => div().child(format!("{} notes", notes.len())),
        }))
}

fn app() -> impl IntoElement {
    RepositoryProvider::new()
        .provide::<dyn NotesRepository>(Arc::new(Fixed))
        .child(
            BlocProvider::bloc(|scope| NotesBloc { repository: scope.repository::<dyn NotesRepository>() })
                .child(NotesPage::new()),
        )
}
```

- [`BlocBuilder::build_when`] skips rebuilds for states that do not matter to it;
  [`BlocSelector`] rebuilds only when a selected value changes.
- [`BlocListener`] runs side effects (navigate, toast) once per state change, with
  [`listen_when`](BlocListener::listen_when); [`BlocConsumer`] is a builder and a listener.
- `BlocProvider::new().with_bloc(..).with_cubit(..)` provides several at once
  ([`MultiBlocProvider`]), and [`Repositories`] builds a set of repositories once.

### State that survives a restart

Settings, drafts and the last opened tab should come back after a restart. With the
`persist` feature, implement `Hydrated` (it returns the `PersistOptions` the state is saved
with) and provide the bloc with `with_hydrated_bloc` (or a cubit with `with_hydrated_cubit`):

```text
impl Hydrated for SettingsBloc {
    fn persist(&self) -> PersistOptions {
        PersistOptions::new("settings").version(2).migrate(|from, old| upgrade(from, old))
    }
}

BlocProvider::new().with_hydrated_bloc(|_| SettingsBloc)
```

The bloc starts from the state its last run saved, and each change is written (debounced, as
versioned JSON in the app's config directory) once the state is still. The state must be
`Serialize` and `Deserialize`. When its shape changes, raise the version and migrate older
files: a saved state that cannot be read is skipped with a warning, and the bloc starts from
its initial state. Hydrate what the user chose (settings, filters, drafts), not what the API
returned: that belongs to the repository.

## Observing every bloc

A [`BlocObserver`] set once with [`set_observer`] sees every bloc and cubit in the process:
creation, each event added, each state change, handler panics and closing. Use it for
logging, analytics and error reporting; [`LogObserver`] prints everything to standard
error. Events and states are `Debug` for this reason.

```rust
use std::fmt::Debug;

use rok_ui::bloc::{set_observer, BlocObserver};

struct Analytics;

impl BlocObserver for Analytics {
    fn on_event(&self, bloc: &'static str, event: &dyn Debug) {
        // Send `bloc` and `event` to your analytics service.
        let _ = (bloc, event);
    }

    fn on_error(&self, bloc: &'static str, error: &str) {
        eprintln!("{bloc} failed: {error}");
    }
}

set_observer(Analytics);
# rok_ui::bloc::clear_observer();
```

A handler that panics is reported to `on_error`; the bloc keeps handling later events.

## Testing

Blocs are plain Rust, so their tests need no window: events in, states out, with a fake
repository.

```rust
use rok_ui::bloc::{test, Bloc, Emitter};

struct Counter;

impl Bloc for Counter {
    type Event = i32;
    type State = i32;

    fn initial_state(&self) -> i32 {
        0
    }

    async fn on(&self, by: i32, emit: &Emitter<i32>) {
        emit.update(|count| *count += by);
    }
}

assert_eq!(test::run(Counter, [1, 2, 0]), [1, 3], "an equal state is not emitted again");
```

[`test::run_settled`] waits for each event before adding the next, and
[`test::run_cubit`] does the same for cubits. [`bloc_test!`] writes the common shape as one
test function:

```rust
# use rok_ui::bloc::{bloc_test, Bloc, Emitter};
# struct Counter;
# impl Bloc for Counter {
#     type Event = i32;
#     type State = i32;
#     fn initial_state(&self) -> i32 { 0 }
#     async fn on(&self, by: i32, emit: &Emitter<i32>) { emit.update(|count| *count += by); }
# }
bloc_test! {
    adding_counts_up,
    build: Counter,
    act: [1, 2],
    expect: [1, 3],
}
```

The generator writes one test per event in `tests/features/<feature>/`; repositories and
providers that call an API are tested against `rok_ui::http::testing::MockServer` (feature
`http-testing`).

## Workspace layout

A large app can split the layers into crates, so the compiler enforces rules 1 and 3: a
feature's business logic cannot use GPUI when GPUI is not one of its dependencies.

```text
cargo rok-ui new my-app --template bloc --layout workspace          # add --http for an API
```

The mapping is fixed, so code moves without changes:

| Folder | Crate | Depends on |
|---|---|---|
| `src/data/` | `crates/<app>-data` | `rok-ui-bloc` (repositories return its `BoxFuture`); over HTTP, `rok-ui-http` and `serde` |
| `src/features/<f>/bloc/` | `crates/<app>-<f>` | `<app>-data`, `rok-ui-bloc`, over HTTP `rok-ui-http` (not `rok-ui`: no GPUI by construction) |
| `tests/features/<f>/` | `crates/<app>-<f>/tests/` | the same, so bloc tests build without GPUI |
| `src/features/<f>/view/`, `src/routes/`, `src/app.rs` | `crates/<app>` | everything above, `rok-ui` |

The app crate keeps the single-crate paths: its `lib.rs` has `pub use <app>_data as data;`
and each `features/<f>.rs` has `pub use <app>_<f> as bloc;`, so views, routes and `app.rs`
still say `crate::data::..` and `crate::features::<f>::bloc::..`. Inside the moved crates,
`crate::data::` becomes `crate::` (data) or `<app>_data::` (features), and `rok_ui::bloc::`
and `rok_ui::http::` become `rok_ui_bloc::` and `rok_ui_http::`: the GPUI-free crates
rok-ui re-exports, so the types are the same on both sides.

`rok-ui.toml` at the workspace root says `layout = "workspace"` and names the app crate, and
`cargo rok-ui generate` (run at the root) writes each file into its crate: a new feature gets
its crate (`Cargo.toml`, and a `clippy.toml` that keeps blocs from holding blocs) and a line
at the `# rok-ui:features` marker in the app crate's `Cargo.toml`. The first HTTP provider
(`g api`) adds `rok-ui-http` and `serde` to the data crate at its `# rok-ui:dependencies`
marker.
