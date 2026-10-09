# Reactive state

The `state` feature (part of `full`) brings in
[rok-ui-hooks](https://crates.io/crates/rok-ui-hooks): fine-grained signals, memos, effects
and stores with a React-flavoured API. It is re-exported as `rok_ui::state::signals`, and
`rok_ui::state` adds the glue that makes GPUI re-render when the values your UI reads change.

```toml
rok-ui = "0.8"                                                    # `full` includes `state`
rok-ui = { version = "0.8", default-features = false, features = ["state", "button"] }
```

## Which state tool to use

rok-ui has three ways to hold state. They work together.

| You want | Use | Re-renders |
|---|---|---|
| A value local to one component, like React's `useState` | `rok_ui::hooks::use_state` | The component's window |
| A value shared across views, windows or background tasks | A signal or `Store` from `rok_ui::state` | Views that `track` it, or windows that `use_tracked` it |
| A whole screen with methods and lifecycle | A GPUI view (`Entity<T>` with `cx.notify()`) | That view |
| A value every component in a subtree can read (the current account, a theme variant) | `rok_ui::context::Provide` and `cx.context::<T>()` | Whatever reads it, when the provider re-renders |
| Business logic with events and states | A bloc (`rok_ui::bloc`, see the architecture guide) | `BlocBuilder`s of that bloc |

For per-row state in a list, `cx.keyed(("row", id)).use_state(..)` keeps each row's state
with its id when rows move.

Signals win when the same data is read in many places: a cart, the signed-in user, settings,
a document being edited. Every reader updates when it changes, and nobody has to pass
`Entity` handles around or call `notify` by hand.

## The building blocks

These come straight from rok-ui-hooks and work outside GPUI too, so you can unit-test your
state without opening a window:

```rust
use rok_ui::state::{batch, create_memo, create_signal, create_store, untrack};

// A signal: a value, plus a write handle.
let (count, set_count) = create_signal(1);

// A memo: a derived value, recomputed lazily and only when its inputs change.
let doubled = create_memo({ let count = count.clone(); move || count.get() * 2 }, ());
set_count.set(5);
assert_eq!(doubled.get(), 10);

// `update` changes a value in place.
set_count.update(|count| *count += 1);
assert_eq!(count.get(), 6);

// A store: one shared value with selectors, like Zustand.
#[derive(Clone, Default, PartialEq)]
struct Settings { dark: bool, font_size: f32 }
let settings = create_store(Settings { dark: false, font_size: 14. });
let dark = settings.select(|settings| settings.dark);   // a memo of one field
settings.update(|settings| settings.font_size = 16.);  // `dark` did not change
assert!(!dark.get());

// Batch several writes so readers see them as one change.
batch(|| {
    set_count.set(0);
    settings.update(|settings| settings.dark = true);
});
assert!(dark.get());

// Read without subscribing.
let _peek = untrack(|| count.get());
```

- **`get()`** clones the value and subscribes the current computation (an effect, a memo or a
  `track` closure). **`with(|value| ..)`** borrows instead of cloning.
- **`get_untracked()` and `peek()`** read without subscribing.
- **Writes run nothing immediately.** Effects run once the outermost write or `batch` ends,
  and always see a consistent snapshot.

## Connecting signals to GPUI

GPUI re-renders a view when it is notified. The helpers below notify for you when a signal
read inside them changes. Each one runs its closure to learn which signals it depends on,
then subscribes to exactly those.

### `cx.track(..)`: a view that follows signals

```rust,no_run
use rok_ui::prelude::*;
use rok_ui::state::{Store, TrackSignals};

# #[derive(Clone)] struct Item;
struct CartBadge {
    cart: Store<Vec<Item>>,
}

impl CartBadge {
    fn new(cart: Store<Vec<Item>>, cx: &mut Context<Self>) -> Self {
        let watched = cart.clone();
        // Re-render whenever the cart changes, wherever the change comes from.
        cx.track(move || watched.with(|_| ()));
        Self { cart }
    }
}

impl Render for CartBadge {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Badge::new(self.cart.with(|items| items.len().to_string()))
    }
}
```

- The tracking lasts as long as the view: it is disposed when the entity is released.
- Reads are tracked again on every change, so conditional reads work:
  `cx.track(move || if mode.get() == Mode::Detailed { details.get(); })`.
- Track only what the view displays. For a large store, track a selector
  (`store.select(|state| state.count)`) so unrelated changes skip the render.

### `use_signal(..)`: a signal owned by a component

The signal is created on the first render and kept while the component stays at the same
place in the tree, like `use_state`. Changing it re-renders the window.

```rust,no_run
# use rok_ui::prelude::*;
#[component]
fn Counter(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let (count, set_count) = use_signal(window, cx, || 0);
    Button::new("increment")
        .label(format!("Clicked {} times", count.get()))
        .on_click(move |_, _, _| set_count.update(|count| *count += 1))
}
```

The write handle can be cloned into anything, including background tasks, timers and other
components through props, and the component still updates. That's the difference from
`hooks::use_state`, whose setter needs `&mut App`.

### `use_tracked(..)`: a component that reads shared signals

```rust,no_run
# use rok_ui::{prelude::*, state::Store};
# #[derive(Clone)] struct Item { price: f64 }
#[component]
fn CartTotal(cart: Store<Vec<Item>>, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let watched = cart.clone();
    use_tracked(window, cx, move || watched.with(|_| ()));
    let total: f64 = cart.with(|items| items.iter().map(|item| item.price).sum());
    Large::new(format!("{total:.2} SAR"))
}
```

`use_tracked` is set up on the first render at its call site and keeps the handles it
captured then. Pass long-lived handles such as stores and app-wide signals, not values that
change identity every render.

## Effects

An effect runs a side effect now, and again whenever a signal it read changes. Keep the
returned `Effect`; dropping it stops the effect.

```rust,no_run
# #[derive(Clone, Default, Debug)] struct Settings { theme: String }
use rok_ui::state::{create_effect, create_store};

let settings = create_store(Settings::default());
let saved = settings.clone();
let _autosave = create_effect(
    move || {
        let settings = saved.get();
        let _ = std::fs::write("settings.txt", format!("{settings:?}"));
    },
    (),
);
```

Pass `()` as the second argument for automatic tracking, or a tuple of handles,
`(count.clone(),)`, to re-run only when those change (React's dependency array). Store
effects next to what they serve, for example in a view's struct, so they live exactly as long.

> An effect that writes a signal it also reads would loop forever. rok-ui-hooks detects this
> and panics with a clear message instead of hanging. Derive the value with a memo instead.

## Async data, debouncing and throttling

`rok_ui::init` runs rok-ui-hooks' `tick` on GPUI's executor, so these work in a rok-ui app
without a loop of your own:

```rust,no_run
use std::time::Duration;
use rok_ui::state::{create_signal, signals::{use_debounced, use_resource, ResourceState}};

# async fn search(term: &str) -> Result<Vec<String>, String> { Ok(vec![term.to_string()]) }
// A search box: wait for 300 ms of quiet before searching.
let (query, set_query) = create_signal(String::new());
let settled = use_debounced(&query, Duration::from_millis(300));

// Async data with loading and error states. `refetch()` runs it again.
let results = use_resource({
    let settled = settled.read_signal();
    move || {
        let term = settled.get();
        async move { search(&term).await }
    }
});

match results.get() {
    ResourceState::Idle | ResourceState::Loading => { /* spinner */ }
    ResourceState::Ready(hits) => { /* list */ }
    ResourceState::Failed(message) => { /* error */ }
}
```

rok-ui-hooks polls futures on the UI thread, so they may hold `Rc` and borrow freely, but they
must not block. For blocking or CPU-heavy work, use `cx.background_executor().spawn(..)`.
For `PostgreSQL`, use the `db` feature, which has its own runtime.

## Fine-grained stores: `#[derive(Store)]`

A `Store<T>` notifies every reader when any part of `T` changes. `#[derive(Store)]` generates
a store with one signal per field instead, so a view re-renders only for the fields it reads:

```rust,no_run
use rok_ui::state::Store;

#[derive(Store, Clone)]
struct Todo {
    title: String,
    done: bool,
}

let todo = Todo { title: "Write docs".into(), done: false }.into_store(); // a `TodoStore`
todo.title().get();                    // reads (and tracks) only `title`
todo.set_done(true);                   // re-renders only readers of `done`
todo.update_title(|title| title.push('!'));
let snapshot: Todo = todo.get();       // every field
todo.set(snapshot);                    // replace every field in one batch
```

## Saving stores: `persist`

With the `persist` feature, `persist::persisted_store(cx, "settings", Settings::default)` is a
`Store` read from `<config dir>/<app name>/settings.json` at startup and written back 300 ms
after it stops changing. The value is JSON (`serde`), saved with a version number;
`PersistOptions::new("settings").version(2).migrate(|from, json| ..)` upgrades files written by
older versions, and `.directory(..)` saves elsewhere. Name the app's folder with
`persist::set_app_name(cx, "notes")` (the default is the executable's name).

## Patterns

**App-wide state.** Create stores once at startup and pass them to the views that need them.
Or keep them in a GPUI global, which you can reach from anywhere with `cx`:

```rust,no_run
# use rok_ui::{prelude::*, state::{create_store, Store}};
# #[derive(Clone)] struct User;
# #[derive(Clone)] struct Item;
#[derive(Clone)]
struct AppState { user: Store<Option<User>>, cart: Store<Vec<Item>> }
impl gpui::Global for AppState {}

# fn example(cx: &mut App) {
cx.set_global(AppState { user: create_store(None), cart: create_store(Vec::new()) });
let cart = cx.global::<AppState>().cart.clone();
# }
```

**Updating from a click.** Clone the write handle into the handler. No `cx` is needed to
write a signal:

```rust,no_run
# use rok_ui::{prelude::*, state::{create_store, Store}};
# #[derive(Clone)] struct Item { name: &'static str, price: f64 }
# impl Item { fn new(name: &'static str, price: f64) -> Self { Self { name, price } } }
# let cart: Store<Vec<Item>> = create_store(Vec::new());
let add = {
    let cart = cart.clone();
    move |_: &ClickEvent, _: &mut Window, _: &mut App| {
        cart.update(|items| items.push(Item::new("Coffee", 12.0)));
    }
};
```

**Testing.** Signals, memos, stores and effects run without GPUI, so plain `#[test]`s cover
your state logic. For views, `cx.track` works in `#[gpui::test]`. Call `cx.update(rok_ui::init)`
first so re-renders are delivered.

## Reference

| Item | What it does |
|---|---|
| `create_signal(value)` | `(ReadSignal<T>, WriteSignal<T>)` |
| `create_memo(f, deps)` | Lazily derived `Memo<T>` |
| `create_store(value)` | Shared `Store<T>` with `select`, `subscribe`, `update` |
| `create_effect(f, deps)` | Side effect; keep the returned `Effect` |
| `batch(f)`, `untrack(f)` | Group writes; read without subscribing |
| `TrackSignals::track(&mut cx, f)` | Re-render a view when signals read by `f` change |
| `use_signal(window, cx, init)` | Component-owned signal that re-renders the window |
| `use_tracked(window, cx, f)` | Re-render the window when signals read by `f` change |
| `signals::*` | All of rok-ui-hooks: contexts, keyed lists, resources, timers, `Root` scopes |
