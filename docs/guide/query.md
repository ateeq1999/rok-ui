# Data: queries, mutations and procedures

`rok_ui::query` (feature `query`) is an async data layer in the style of TanStack Query and
TanStack Start, for GPUI desktop apps. Components stay synchronous: each query is a cached
task on the shared background runtime ([`crate::runtime`]) that re-renders the window when it
finishes.

| TanStack | rok-ui |
|---|---|
| `queryOptions({ queryKey, queryFn, staleTime })` | `QueryOptions::new(query_key![..], fetch).stale_time(..)` |
| `useQuery(options)` | `query::use_query(cx, options)` |
| `useQuery({ ...options, select })` | `query::use_query_select(cx, options, \|data\| ..)` |
| `useSuspenseQuery` + `<Suspense>` | `use_suspense_query(cx, options)?` inside `Suspense::new(..)` |
| `queryClient.invalidateQueries({ queryKey })` | `query::invalidate(cx, &query_key![..])` (prefix match) |
| `setQueryData`, `ensureQueryData`, `prefetchQuery` | `set_query_data`, `ensure_query_data`, `prefetch_query` |
| `useMutation({ mutationFn, onSuccess, onMutate })` | `use_mutation(cx, MutationOptions::new(..).on_success(..).optimistic(..))` |
| `createServerFn().handler(..)` | `#[procedure] async fn ..` + `use_procedure(cx, ..)` |

## Queries

Define options with a function, so components, loaders and tests share them:

```
use std::time::Duration;
use rok_ui::{query::{QueryOptions, TaskCx}, query_key};

#[derive(Clone, Debug)]
pub struct Note {
    pub id: u64,
    pub title: String,
}

pub fn note_query(id: u64) -> QueryOptions<Note> {
    QueryOptions::new(query_key!["notes", id], move |_cx: TaskCx| async move {
        // Any async work: an HTTP call, a rok-db query, a file read.
        Ok::<_, std::io::Error>(Note { id, title: format!("Note {id}") })
    })
    .stale_time(Duration::from_secs(30))
}
# let _ = note_query(1);
```

Read it in a component with [`use_query`]. Components that take `cx: &mut Cx` get the window
and app as one handle:

```no_run
# use rok_ui::{prelude::*, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Clone)] struct Note { title: String }
# fn note_query(id: u64) -> QueryOptions<Note> {
#     QueryOptions::new(query_key!["notes", id], move |_| async move {
#         Ok::<_, std::io::Error>(Note { title: String::new() })
#     })
# }
#[component]
fn NotePage(id: u64, cx: &mut Cx) -> impl IntoElement {
    let note = query::use_query(cx, note_query(id));
    match note.state() {
        QueryState::Pending => Spinner::new().into_any_element(),
        QueryState::Error(error) => Alert::new("Could not load the note")
            .destructive()
            .description(error.to_string())
            .into_any_element(),
        QueryState::Success(note) => div().child(note.title.clone()).into_any_element(),
    }
}
```

How caching works:

- **One cache per app, keyed by [`QueryKey`].** Readers of the same key share the data and one
  in-flight fetch.
- **Fresh or stale.** Data is fresh for `stale_time` (default zero). A component that starts
  reading stale data shows it and refetches in the background.
- **Garbage collection.** Data nobody reads for `gc_time` (default five minutes) is dropped.
- **Retries.** `.retry(n)` retries with exponential backoff starting at `.retry_delay(..)`.
- **Other options.** `enabled`, `initial_data`, `placeholder_data`, `keep_previous_data`
  (pagination without flashing), `refetch_interval` and `refetch_on_window_focus`.

### Reading part of the data

[`use_query_select`] derives a value from the cached data, like TanStack's `select` option. The
cache keeps the data as fetched, so other readers of the key see all of it, and each call site
derives again only when the data changes, not on every render:

```no_run
# use rok_ui::{prelude::*, query::{self, QueryOptions}, query_key};
# #[derive(Clone)] struct Note { pinned: bool }
# fn notes_query() -> QueryOptions<Vec<Note>> { QueryOptions::new(query_key!["notes"], |_| async { Ok::<_, std::io::Error>(Vec::new()) }) }
#[component]
fn PinnedCount(cx: &mut Cx) -> impl IntoElement {
    let pinned = query::use_query_select(cx, notes_query(), |notes| {
        notes.iter().filter(|note| note.pinned).count()
    });
    div().child(format!("{} pinned", pinned.data().copied().unwrap_or(0)))
}
```

### Keys and invalidation

Keys are hierarchical. Invalidating a prefix marks every key below it stale; readers on screen
refetch, and others refetch when next read:

```no_run
# use rok_ui::{prelude::*, query, query_key};
# fn after_save(cx: &mut App) {
query::invalidate(cx, &query_key!["notes"]); // ["notes", 3], ["notes", "list", ..], ...
# }
```

### Outside components

For loaders, commands and startup code: [`fetch_query`] (fetch unless fresh),
[`ensure_query_data`] (any cached data, else fetch), [`prefetch_query`], [`set_query_data`],
[`update_query_data`], [`get_query_data`], [`cancel_queries`] and [`reset_queries`]. All return
GPUI `Task`s or act immediately, and share in-flight fetches with components.

## Context for background work

Fetchers, mutations and procedures run off the UI thread, so they cannot read GPUI state.
Register what they need once, and read it from the [`TaskCx`] they receive:

```no_run
# use rok_ui::{prelude::*, query::{self, TaskCx}};
#[derive(Clone)]
struct ApiClient { base: String }

fn init(cx: &mut App) {
    query::provide(cx, ApiClient { base: "https://api.example.com".into() });
}

async fn load(cx: TaskCx) -> Result<String, rok_ui::query::QueryError> {
    let client = cx.require::<ApiClient>()?;
    Ok(client.base.clone())
}
# let _ = (init, load);
```

With the `db` feature, the database connection is provided for you and [`crate::db::db_query`]
and [`crate::db::db_mutation`] read it.

## Suspense and error boundaries

[`Suspense`] renders content that reads queries without a loading branch. Inside it,
[`use_suspense_query`] returns the data or a [`Suspend`] that `?` hands to the boundary:

```no_run
# use rok_ui::{prelude::*, query::{self, QueryOptions, Suspense}, query_key};
# fn note_query(id: u64) -> QueryOptions<String> {
#     QueryOptions::new(query_key!["notes", id], |_| async { Ok::<_, std::io::Error>(String::new()) })
# }
#[component]
fn NotePanel(id: u64, cx: &mut Cx) -> impl IntoElement {
    Suspense::new(move |cx| {
        let note = query::use_suspense_query(cx, note_query(id))?;
        Ok(div().child(note.to_string()))
    })
    .fallback(Skeleton::new("note").h(px(120.)))
    .error(|error, _| Alert::new("Could not load the note").description(error.to_string()))
}
```

The fallback only covers the first load; refetches keep the current content on screen.
[`ErrorBoundary`] does the same for any `Result`. A failure stays inside its boundary, so
siblings keep rendering.

## Mutations

[`use_mutation`] runs writes in the background and tracks their state at the call site:

```no_run
# use rok_ui::{prelude::*, query::{self, MutationOptions, QueryError}, query_key};
#[component]
fn AddTodo(cx: &mut Cx) -> impl IntoElement {
    let add = query::use_mutation(
        cx,
        MutationOptions::new(|_cx, title: String| async move {
            // Save `title` somewhere.
            Ok::<_, QueryError>(title)
        })
        .invalidates(query_key!["todos"])
        // Instant UI: changed before the write runs, rolled back if it fails.
        .optimistic(|title: &String, cache| {
            cache.update(&query_key!["todos"], |todos: &mut Vec<String>| todos.push(title.clone()));
        }),
    )
    .on_success(|title, _, cx| {
        toast(cx, Toast::success(format!("Added {title}")));
    });

    Button::new("add")
        .label("Add")
        .loading(add.is_pending())
        .on_click(add.mutate_handler("Write docs".to_string()))
}
```

A run started while another is pending supersedes it. `status()`, `data()`, `error()` and
`reset(cx)` cover the rest of the lifecycle.

## Procedures

A procedure is a typed command declared once with [`procedure`](crate::procedure): validation
and authorization live inside it, and it names the queries it invalidates.

```no_run
use rok_ui::{prelude::*, procedure, query::{self, TaskCx}, query_key};

#[derive(Clone, Debug)]
pub struct NewNote {
    pub title: String,
}

#[derive(Debug)]
pub enum NoteError {
    EmptyTitle,
}

#[procedure(invalidates = [query_key!["notes"]])]
pub async fn create_note(_cx: TaskCx, input: NewNote) -> Result<u64, NoteError> {
    if input.title.trim().is_empty() {
        return Err(NoteError::EmptyTitle);
    }
    Ok(42) // the new note's id
}

#[component]
fn NewNoteButton(cx: &mut Cx) -> impl IntoElement {
    let create = query::use_procedure(cx, create_note);
    Button::new("create")
        .label("New note")
        .loading(create.is_pending())
        .on_click(create.mutate_handler(NewNote { title: "Untitled".into() }))
}
```

Outside components, `create_note.call(cx, input)` returns a `Task` with the typed result.

## Memoized helpers

[`memoize`](crate::memoize) caches an async helper by its arguments; concurrent callers share
one in-flight future (topcoat's `#[memoize]`):

```no_run
use rok_ui::{memoize, query::TaskCx};

#[memoize]
async fn exchange_rate(currency: String) -> f64 {
    // An expensive lookup, done once per currency.
    if currency == "EUR" { 1.08 } else { 1.0 }
}

# async fn use_it() {
let rate = exchange_rate("EUR".into()).await;
# let _ = rate;
# }
```

`query::memo::invalidate(prefix)` forgets results by module path or function path.
