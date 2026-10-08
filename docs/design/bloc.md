# Design: BLoC in rok-ui

Status: implemented (Part L of `roadmap.md`). This note records the decisions and where they
differ from the request in `bloc-implementation-prompt.md`.

## Layers

| Layer | Lives in | May use |
|---|---|---|
| Data: providers, repositories | `src/data/` | rok-db, `rok_ui::http`, `rok_ui::query` keys |
| Business logic: bloc / cubit, events, states | `src/features/<f>/bloc/` | `rok_ui::bloc` core types and repositories only |
| Presentation: views, routes | `src/features/<f>/view/`, `src/routes/` | everything in rok-ui |

## Crates

- **`rok-ui-bloc`** (new, GPUI-free): `Bloc`, `Cubit`, `Emitter`, `BlocHandle`, `CubitHandle`,
  `Concurrency`, `Observable`, `Subscription` and the `test` helpers. It depends on `tokio`
  (rt, sync) and nothing from GPUI, so a bloc crate in a workspace app can depend on it alone.
- **`rok_ui::bloc`** (feature `bloc`): re-exports the core and adds the GPUI side:
  `BlocProvider`, `MultiBlocProvider`, `RepositoryProvider`, `MultiRepositoryProvider`,
  `BlocBuilder`, `BlocListener`, `BlocConsumer`, `BlocSelector`, and `cx.bloc::<B>()`,
  `cx.cubit::<C>()`, `cx.watch_bloc::<B>()`, `cx.repository::<R>()`.

## The primitives (final API)

```rust
pub trait Bloc: Send + Sync + 'static {
    type Event: Send + 'static;
    type State: Clone + PartialEq + Send + Sync + 'static;
    fn initial_state(&self) -> Self::State;
    fn on(&self, event: Self::Event, emit: &Emitter<Self::State>) -> impl Future<Output = ()> + Send;
    fn concurrency(&self, _event: &Self::Event) -> Concurrency { Concurrency::Sequential }
}

pub trait Cubit: Send + Sync + 'static {
    type State: Clone + PartialEq + Send + Sync + 'static;
    fn emitter(&self) -> &Emitter<Self::State>;
}
```

- **Handlers are async and get no `Cx`.** The sketch passed `cx: &mut Cx` to `on`; that would
  put a GPUI type in the bloc layer and break rule 3, and a handler that awaits a repository
  cannot hold `&mut App` across the await. Handlers run on `rok_ui::runtime` (tokio) and learn
  everything from the event and the repositories the bloc was built with.
- **`Emitter<S>`** holds the current state. `emit.emit(state)` stores it and notifies
  subscribers only when it differs (`PartialEq`), so an equal state never re-renders.
  `emit.update(|s| ..)`, `emit.state()`, `emit.is_closed()`. After the bloc closes, emits are
  ignored.
- **`BlocHandle<B>`** (cheap clone) is what views use: `add(event)`, `state()`, `close()`.
  It is deliberately `!Send`: `Bloc` requires `Send + Sync`, so a bloc that stores another
  bloc's handle does not compile (rule 2; a trybuild test proves it). Cross-bloc reactions go
  through a `BlocListener` in the view, or a stream a shared repository exposes.
- **Cubits** own an `Emitter` and expose methods. `CubitHandle<C>` derefs to `C`;
  `emitter().spawn(future)` runs async work on the runtime, cancelled when the cubit closes.

## Concurrency

`Bloc::concurrency(&event)` picks a mode per event (the default is `Sequential`):

| Mode | Behavior |
|---|---|
| `Sequential` | One queue per bloc: sequential events run one at a time, in arrival order |
| `Droppable` | Per event variant: an event arriving while one of its variant runs is ignored |
| `Restartable` | Per event variant: a new event aborts the running one of its variant |
| `Concurrent` | Runs at once |

"Variant" means `std::mem::discriminant(&event)`. flutter_bloc applies transformers per
handler; one sequential queue per bloc (rather than per variant) is the safer default for
"add then delete" sequences, as requested.

Each handler is a tokio task. Closing a bloc (explicitly, or when its provider leaves the
tree) aborts every running task and ignores later events and emits. A cancelled handler emits
nothing.

## Providers and scope

GPUI has no build context. A provider is a component that creates its bloc once (keyed
element state, so it lives as long as the provider renders) and renders its children inside a
scope element, the same technique the router uses for per-window history: the scope is pushed
on a thread-local stack while its subtree lays out, prepaints and paints. Lookups
(`cx.bloc::<B>()`) read the innermost scope, so they work in components and views rendered
below the provider, not in the function that builds the provider itself. Event handlers run
outside layout, so read the handle while rendering and move it into the closure.

When the provider stops rendering, GPUI drops its element state; the owner it held closes the
bloc and aborts its handlers.

## Re-rendering

A bloc's state change calls `cx.refresh_windows()` on the UI thread (the query cache does the
same). GPUI re-renders whole windows, so `build_when` / `BlocSelector` decide *which state a
builder shows*, with flutter's semantics: when `build_when(previous, current)` is false the
builder keeps showing the last state it built with.

`BlocListener` subscribes once (element state) and runs its callback on the UI thread exactly
once per state change that passes `listen_when`, with a `Window` for navigation and toasts.

## Testing

`rok_ui::bloc::test::run(bloc, events)` adds the events, waits until every handler finishes
and returns the states emitted, on a private runtime: the `bloc_test` equivalent. Repositories
are traits, so tests pass a fake.

## Enforcing rule 3 (no GPUI in the bloc layer)

clippy's `disallowed-types` applies to a whole crate, so in a single-crate app it cannot allow
GPUI in views and forbid it in blocs by itself. The template does both:

1. `clippy.toml` lists GPUI and HTTP types as disallowed, the crate root allows the lint, and
   each `features/<f>/bloc.rs` re-denies it with `#![deny(clippy::disallowed_types)]`.
2. `Bloc: Send + Sync` already rejects `Entity`, `Window`, `App` and element types, which are
   not `Send`.

In a workspace app the bloc crates simply do not depend on `rok-ui`, only on `rok-ui-bloc`.

## Differences from the request

- `on` takes no `Cx` and is async (see above).
- Builders and listeners take the handle (`BlocBuilder::new(&notes, ..)`) rather than looking
  the bloc up by type: one widget type then serves blocs and cubits.
- `BlocProvider::bloc(..)` and `BlocProvider::cubit(..)` instead of one overloaded `new`: Rust
  cannot tell the two traits apart in one blanket implementation.
- Snapshot tests use a small in-repo helper instead of `insta`, and the JSON Schema is written
  by the CLI's own types instead of `schemars`, to keep the dependency tree as it is (see
  `docs/guide/cli.md`).

## Open questions

- A `--layout workspace` option for `cargo rok-ui new` is documented (crate mapping in the
  architecture guide) but not implemented.
- Hydrated (persisted) blocs, like `hydrated_bloc`, could reuse `persist`; not started.
