# rok-ui enhancement plan

Status: proposal, 2026-10-03. Baseline: rok-ui 0.5.0.

This plan raises rok-ui in two directions at once:

1. **Developer experience** for app code, modeled on the TanStack family (Start, Router, Query,
   Form) and on topcoat's shards, procedures and memoization, adapted to GPUI desktop apps:
   file-based typed routing, loaders, a query cache, typed commands, and forms.
2. **Engineering standards**, modeled on [tokio-rs/topcoat](https://github.com/tokio-rs/topcoat)
   (from the axum and tokio maintainers): lints, macro architecture, docs-as-tests, CI,
   releases, project structure and coding-agent skills.

Contents:

- Part A: What we can learn (topcoat, TanStack, constraints)
- Part B: TanStack-style DX conventions
- Part C: Reactive state and data (query cache, shards, procedures, memoize, stores)
- Part D: Forms (TanStack Form)
- Part E: Routing v2 (file-based and typed, TanStack Router)
- Part F: Folder structure (apps and the rok-ui repository)
- Part G: Component standards
- Part H: Engineering standards (topcoat)
- Part I: Roadmap and acceptance criteria
- Part J: Migration guide (0.5 -> 1.0)
- Part K: Decisions needed

---

## Part A: What we can learn

### A.1 From topcoat

| topcoat | What it is | rok-ui today | Plan |
|---|---|---|---|
| `#[shard]` | A component re-rendered on the server when its arguments change; latest request wins, previous HTML stays until the new one arrives | None; async data needs `db::use_query` plus hand-written loading UI | `#[shard]`: async UI re-run off the UI thread when its arguments change (C.3) |
| `#[procedure]` | Typed async server function callable from the browser; validate and authorize inside | `db::run` (database only, untyped errors) | `#[procedure]`: typed commands with mutation state, invalidation, optional remote transport (C.4) |
| `#[memoize]` | Per-request cache; concurrent callers share one in-flight future | None | `#[memoize]` on `&Cx` helpers (C.5) |
| `suspense`, `error_boundary` | Fallbacks for loading and errors | None | `Suspense`, `ErrorBoundary` (C.3) |
| `module_router!` + `discover` | Route tree inferred from the module tree, no build step | Manual `.route(..)` | File-based routing (E.1) with a code-based alternative (E.2) |
| `href!(handler, PostId(1))` | Typed links checked by the compiler | String links | Typed routes and `Link::to` (E.3) |
| `path_param!`, `#[query_params]` | Typed, validated, memoized params | `route.param_as::<u64>("id")` | Generated `Params`, `#[derive(Search)]` (E.4) |
| `#[layout]` + `Slot` | Layouts by path prefix | Nested `Router`s by hand | Layout routes with `Outlet` (E.1) |
| Functions, not middlewares | Guards are memoized `fn(cx: &Cx)` helpers | `if`s in builders | `before_load` + `Cx` helpers (E.5) |
| `#[derive(Props)]` | Builder whose `build()` exists only when required props are set | `#[prop(optional)]` | Typestate props (G.1) |
| `#[key(item.id)]` | Stable identity in loops | `use_keyed_state` by hand | `#[key(..)]` in `view!` (G.2) |
| `topcoat ui` CLI | Copy components into your project | Dependency only | `cargo rok-ui add` (G.4) |
| Grammar / macro crate split | Macros are testable libraries | One macro crate | Split (H.2) |
| `pedantic`, `deny(unsafe_code)`, `mod_module_files`, `broken_intra_doc_links` | Workspace lints | Defaults | Adopt (H.1) |
| `docs/` as source of truth, `include_str!`, no `ignore` snippets | Guides are compiled doctests | 4 guides; 118 `ignore` blocks | Finish (H.3) |
| `.agents/skills/`, `AGENTS.md`, `llms.txt` | Agent-readable conventions | None | Adopt (H.5) |
| release-plz, Conventional Commits, semantic PR check, `cargo hack`, `cargo udeps` | Release and CI hygiene | Manual releases, `check-features.sh` | Adopt (H.4) |

### A.2 From TanStack

| TanStack | Idea | Plan |
|---|---|---|
| Router: file-based routes, generated `routeTree.gen.ts`, `createFileRoute('/posts/$postId')` | Routes are files; the tree and its types are generated | E.1 |
| Router: typed `Link to` + `params`, `useParams({ from })` | Navigation cannot target a route that does not exist | E.3 |
| Router: `validateSearch` | Search params are typed state with defaults | E.4 |
| Router: `beforeLoad`, `redirect()`, `notFound()`, route `context` | Guards and control flow as return values; injected dependencies | E.5 |
| Router: `loader`, `staleTime`, `pendingComponent`, `pendingMs`, `errorComponent`, `preload: "intent"` | Data before render, cached; pending UI with delay; preloading | E.6 |
| Router: `useBlocker`, scroll restoration, devtools | Polish | E.7, C.8 |
| Query: `queryOptions`, keys, stale / gc time, retries, dedup, refetch on focus, `setQueryData`, prefix invalidation, `useSuspenseQuery`, `ensureQueryData` | A general async cache with reusable option objects | C.2 |
| Query: `useMutation` with optimistic updates and rollback | Writes with instant UI | C.4 |
| Start: `createServerFn().validator().handler()`, middleware | Typed RPC | C.4 |
| Form: `useForm`, `form.Field`, validators (`onChange`, `onBlur`, `onSubmit`, async with debounce), field meta, array fields, `form.Subscribe`, linked fields, schema adapters, `createFormHook` | Headless, type-safe forms | Part D |
| Store: derived stores, selectors | Covered by rok-ui-hooks; add field-level stores | C.7 |

### A.3 Constraints specific to rok-ui

- GPUI renders synchronously; components cannot be `async fn`. Every async feature here is a
  sync component backed by a cached task that re-renders the window when it completes (the
  pattern `db::use_query` already uses).
- `deferred` cannot nest (GPUI 0.2 panics), so overlays keep going through `layer.rs`.
- Desktop apps have no server by default: "server functions" become background commands that
  can optionally run remotely (C.4).
- `db` already starts a tokio runtime. All async features share one `rok_ui::runtime`.
- Rust has no runtime reflection or code generation in the editor. File-based routing uses a
  build script (like TanStack's generator) so types exist before the app compiles.

---

## Part B: TanStack-style DX conventions

These conventions apply to every new API in this plan, so the pieces feel like one framework.

1. **Hooks take `cx` first, then an options value.** `use_query(cx, note_query(id))`,
   `use_form(cx, sign_up_form())`, `use_procedure(cx, create_note)`. Options are plain values
   built by functions, so they can be shared between loaders, components and tests
   (TanStack's `queryOptions` pattern).
2. **Options are builders with defaults.** `QueryOptions::new(key, fetcher).stale_time(..)`.
   Every option has a documented default; nothing is required except what defines the thing.
3. **State is read through a typed state object**, never through loose tuples:
   `query.state()` -> `QueryState::{Pending, Error, Success}`; `field.state().meta.errors`.
4. **Render props for headless pieces.** Forms and fields hand you their state and handlers and
   you render whatever you like: `form.field(SignUp::EMAIL, |field| Input::bound(field))`.
5. **Types flow end to end.** Route params, search params, loader data, query data, form
   values and procedure inputs are Rust types. A typo is a compile error, not a runtime miss.
6. **Colocation.** A route file holds its params, search schema, loader, pending and error
   UI and component. A feature folder holds its queries, procedures and forms (Part F).
7. **Escape hatches everywhere.** Every declarative API has an imperative twin
   (`router::navigate`, `query::fetch_query`, `form.set_value`) for code outside components.
8. **Devtools see everything.** Routes, queries, mutations and forms report to one devtools
   panel (C.8).

---

## Part C: Reactive state and data

### C.1 `Cx`: one context handle for app code

Hooks today take `(window, cx)` and string keys. Introduce `Cx`, used by components, loaders,
shards, procedures, guards and memoized helpers (topcoat's `&Cx`):

```rust
pub struct Cx<'a> { pub window: &'a mut Window, pub app: &'a mut App, scope: ScopeId }

impl Cx<'_> {
    pub fn get<T: Global>(&self) -> &T;                                // app context by type
    pub fn try_get<T: Global>(&self) -> Option<&T>;
    pub fn with<T: 'static>(&mut self, value: T) -> ScopedCx<'_>;      // scoped values
    pub fn keyed(&mut self, key: impl Hash) -> ScopedCx<'_>;           // stable identity
}
```

`#[component]` accepts `cx: &mut Cx` in place of `window, cx`; the old form keeps working.

### C.2 `rok_ui::query`: a general async cache (TanStack Query)

```rust
// features/notes/queries.rs: options are values, reused by loaders and components.
pub fn note_query(id: u64) -> QueryOptions<Note> {
    QueryOptions::new(query_key!["notes", id], move |_cx| async move { api::note(id).await })
        .stale_time(Duration::from_secs(30))
}

pub fn notes_query(search: NotesSearch) -> QueryOptions<Vec<Note>> {
    QueryOptions::new(query_key!["notes", "list", search.clone()], move |_cx| async move {
        api::notes(&search).await
    })
}

// In a component:
let note = use_query(cx, note_query(id));
match note.state() {
    QueryState::Pending => Spinner::new().into_any_element(),
    QueryState::Error(error) => ErrorView::new(error).into_any_element(),
    QueryState::Success(note) => NoteCard::new(note).into_any_element(),
}

// Or, under a Suspense boundary, data without a pending branch:
let note = use_suspense_query(cx, note_query(id));     // Note
```

- **Hierarchical keys** (`query_key!["notes", id]`): `invalidate(cx, query_key!["notes"])`
  invalidates every key with that prefix (today's `db::invalidate` matches exact keys only).
- **One cache per app** with in-flight dedup: two components asking for the same key share one
  fetch (topcoat's memoize fan-out).
- Options: `stale_time`, `gc_time`, `retry` with backoff, `enabled`, `select`,
  `placeholder_data`, `initial_data`, `keep_previous_data`, `refetch_on_window_focus`,
  `refetch_interval`.
- Imperative: `ensure_query_data` (for loaders), `prefetch_query`, `fetch_query`,
  `set_query_data`, `cancel_queries`, `invalidate`, `reset`.
- Query state is a rok-ui-hooks signal, so `cx.track` works on it.
- `db::use_query` becomes `use_query(cx, db_query(key, |db| ..))`; a shim keeps the 0.5 form.

### C.3 `#[shard]`, `Suspense` and `ErrorBoundary`

A shard is an async function that produces UI and re-runs off the UI thread when its arguments
change. The latest run wins and the previous content stays until the new content arrives:

```rust
#[shard]
async fn search_results(cx: &Cx, query: String) -> Result<impl IntoElement> {
    let products = catalog::search(&query).await?;   // background, Send
    Ok(view! {
        #[key(product.id)]
        for product in products { Item(("product", product.id), title = product.name) }
    })
}

Suspense::new()
    .fallback(Skeleton::new().h(px(120.)))
    .child(
        ErrorBoundary::new(|error, _, _| Alert::new("Search failed").description(error.to_string()))
            .child(search_results(query.get())),
    )
```

The macro splits the function into a `Send` data phase on the shared runtime and a UI phase on
the UI thread; non-`Send` data is a compile error that names the offending binding. Shards
share the query cache, so `invalidate` re-runs them.

### C.4 `#[procedure]` and mutations (TanStack Start server functions)

```rust
// features/notes/procedures.rs
#[procedure(invalidates = [query_key!["notes"]])]
async fn create_note(cx: &Cx, input: NewNote) -> Result<Note, NoteError> {
    input.validate()?;                                  // validation lives with the command
    let db = cx.get::<Database>();
    Ok(Note::from(input).insert(db).await?)
}

// In a component: a mutation with pending / error / data state.
let create = use_procedure(cx, create_note)
    .on_success(|note, cx| toast(cx, Toast::success(format!("Created {}", note.title))))
    .optimistic(|input, cache| {
        cache.update(query_key!["notes", "list"], |notes: &mut Vec<Note>| notes.push(input.preview()))
    });

Button::new("save")
    .loading(create.is_pending())
    .on_click(move |_, _, cx| create.mutate(cx, draft.clone()));
```

- Typed input, output and errors (no stringly `DbError::Failed`).
- `invalidates` re-runs matching queries and shards on success; optimistic updates roll back
  on error.
- `use_mutation(cx, MutationOptions::new(|cx, input| async { .. }))` for ad-hoc writes.
- **Remote transport** (`procedure-remote`): the same procedure is served by an axum router
  (`rok_ui::procedure::axum_router()`) and called over HTTP + serde from the desktop app. One
  definition covers local-only and client / server apps, like TanStack Start.

### C.5 `#[memoize]`

```rust
#[memoize]                       // cached per scope; concurrent callers share one future
async fn current_user(cx: &Cx) -> Option<User> { session::load(cx).await }
```

Scopes: per frame (sync), per navigation (loaders and guards), or per app with explicit
invalidation.

### C.6 Signals (unchanged foundation)

`rok_ui::state` (0.5) stays the reactive core: signals, memos, effects, stores and the GPUI
glue (`cx.track`, `use_signal`, `use_tracked`). Queries, forms and router state are built on it.

### C.7 Fine-grained stores and persistence

- `#[derive(Store)]` generates per-field signals (`todo.title()`, `todo.done()`), so views
  re-render only for fields they read.
- `persisted_store(cx, "settings", Settings::default)`: serde-backed, saved to the platform
  config dir with debounced writes and versioned migrations.

### C.8 Devtools

`Devtools` overlay (feature `devtools`, toggled with a shortcut): route tree, history and
pending loads; query cache entries with state, age and observers; mutations; forms with field
state; active shards; signal graph size.

---

## Part D: Forms (TanStack Form)

A headless, type-safe form library (`forms` feature) with rok-ui field components on top. It
follows TanStack Form's model: a form owns typed values; fields subscribe to their slice;
validators run on events you choose; the form exposes fine-grained state for rendering.

### D.1 Defining a form

```rust
#[derive(FormValues, Clone, Default, Serialize, Deserialize)]
pub struct SignUp {
    pub email: String,
    pub password: String,
    pub confirm: String,
    pub plan: Plan,
    pub accept_terms: bool,
    pub team: Vec<Invite>,          // array field
}

pub fn sign_up_form() -> FormOptions<SignUp> {
    FormOptions::new(SignUp::default())
        .validators(Validators::new().on_submit(|values| {
            (values.password != values.confirm)
                .then(|| FormError::field(SignUp::CONFIRM, "Passwords do not match"))
        }))
        .on_submit(|values, cx| use_procedure_call(cx, create_account, values))
}
```

`#[derive(FormValues)]` generates typed field paths (`SignUp::EMAIL`, `SignUp::TEAM.at(i).EMAIL`)
used for subscriptions, errors and focus.

### D.2 Rendering fields (render props)

```rust
#[component]
fn SignUpForm(cx: &mut Cx) -> impl IntoElement {
    let form = use_form(cx, sign_up_form());

    form.render(|form| view! {
        FieldGroup {
            {form.field(SignUp::EMAIL)
                .validators(Validators::new()
                    .on_blur(|email: &String| (!email.contains('@')).then_some("Enter a valid email"))
                    .on_change_async_debounce(Duration::from_millis(400), |email, cx| async move {
                        api::email_taken(&email).await.then_some("This email is already registered")
                    }))
                .render(|field| view! {
                    Field {
                        FieldLabel("Email")
                        {Input::bound(&field).leading_icon(IconName::Mail)}
                        if field.state().meta.is_touched {
                            for error in field.state().meta.errors { FieldError(error) }
                        }
                        if field.state().meta.is_validating { Spinner }
                    }
                })}
            {form.field(SignUp::PASSWORD).render(|field| PasswordField::new(&field))}
            {form.subscribe(|state| (state.can_submit, state.is_submitting), |(can_submit, submitting)| {
                Button::new("submit").label("Create account").disabled(!can_submit).loading(submitting)
                    .on_click(form.submit_handler())
            })}
        }
    })
}
```

### D.3 Field and form state

| Field state | Meaning |
|---|---|
| `value` | The typed value at the field path |
| `meta.errors`, `meta.error_map` | Errors, grouped by the validator event that produced them |
| `meta.is_touched`, `is_blurred`, `is_dirty`, `is_pristine` | Interaction state |
| `meta.is_validating` | An async validator is running |
| `handle_change(value)`, `handle_blur()` | What bound inputs call |

| Form state | Meaning |
|---|---|
| `values`, `errors`, `field_meta` | Whole-form state |
| `can_submit`, `is_submitting`, `is_submitted`, `submission_attempts` | Submit lifecycle |
| `is_valid`, `is_dirty`, `is_touched`, `is_validating` | Aggregates |

`form.subscribe(selector, render)` re-renders only when the selected slice changes, like
TanStack's `form.Subscribe` / `useStore(form.store, selector)`.

### D.4 Validation

- **Events**: `on_mount`, `on_change`, `on_blur`, `on_submit`, plus async variants with
  debounce (`on_change_async_debounce`). Errors are kept per event in `error_map`, so a blur
  error does not hide a submit error.
- **Levels**: field validators, and form validators that can return errors for any field
  (`FormError::field(path, msg)`).
- **Linked fields**: `.listen_to([SignUp::PASSWORD])` re-validates `confirm` when `password`
  changes (TanStack's `onChangeListenTo`).
- **Schema adapters** (TanStack's Standard Schema support): a `Validator<T>` trait with
  adapters for [`garde`](https://crates.io/crates/garde) and
  [`validator`](https://crates.io/crates/validator) behind features, so one derive validates
  both the form and the procedure input:

```rust
#[derive(FormValues, garde::Validate, Clone, Default)]
pub struct NewNote {
    #[garde(length(min = 1, max = 120))] pub title: String,
    #[garde(skip)] pub body: String,
}

FormOptions::new(NewNote::default()).schema(GardeSchema)    // field errors from garde paths
```

- **Server / procedure errors**: a procedure returning `FieldErrors` maps them back onto fields
  after submit (`.on_submit_error(FormErrors::from_procedure)`).

### D.5 Array and nested fields

```rust
form.array_field(SignUp::TEAM).render(|team| view! {
    #[key(index)]
    for index in 0..team.len() {
        Row(spacing = px(8.)) {
            {form.field(SignUp::TEAM.at(index).EMAIL).render(|field| Input::bound(&field))}
            Button(("remove", index)).ghost().icon_only(IconName::Trash)
                .on_click(team.remove_handler(index))
        }
    }
    Button("add-invite").outline().label("Add teammate").on_click(team.push_handler(Invite::default()))
})
```

`push_value`, `insert_value`, `remove_value`, `swap_values`, `move_value`, `replace_value`.

### D.6 Bound inputs and the app form hook (`createFormHook`)

- Every rok-ui control gets a `bound(&field)` constructor: `Input`, `Textarea`, `Checkbox`,
  `Switch`, `RadioGroup`, `Select`, `Combobox`, `Slider`, `DatePicker`, `InputOtp`.
- `create_form_hook!` builds an app-specific hook with pre-registered field components, so
  forms read as one line per field (TanStack's `createFormHook` / `useAppForm`):

```rust
create_form_hook! {
    pub use_app_form,
    fields: { TextField, PasswordField, SelectField, CheckboxField },
    form: { SubmitButton },
}

let form = use_app_form(cx, sign_up_form());
form.app_field(SignUp::EMAIL, |f| f.text_field("Email"));
form.app_field(SignUp::PLAN, |f| f.select_field("Plan", Plan::ALL));
form.app_form(|f| f.submit_button("Create account"));
```

### D.7 Behavior details

- Enter submits single-line forms; Ctrl/Cmd-Enter submits from a textarea.
- On a failed submit, focus moves to the first invalid field and it scrolls into view.
- `form.reset()`, `form.reset_field(path)`, `form.set_value(path, value)`, `form.validate()`.
- Dirty forms integrate with the router blocker (E.7): `use_blocker(cx, form.is_dirty())`.
- Persisted drafts: `FormOptions::persist("sign-up-draft")` stores values with C.7.
- RTL and BidiText for labels and errors as everywhere else.

---

## Part E: Routing v2 (TanStack Router)

### E.1 File-based routing

Routes live in `src/routes/`. A build script generates the route tree, typed route structs and
`mod` declarations into `OUT_DIR` (TanStack's `routeTree.gen.ts`), so there is nothing to
register by hand and every link is type-checked.

```rust
// build.rs
fn main() {
    rok_ui_build::routes("src/routes").generate().unwrap();
}

// src/main.rs
rok_ui::routes!();             // include!(concat!(env!("OUT_DIR"), "/routes.rs"))

fn main() {
    Application::new().with_assets(rok_ui::Assets).run(|cx| {
        rok_ui::init(cx);
        open_main_window(cx, RouterProvider::new(routes::tree()));
    });
}
```

**File conventions** (TanStack's, mapped to Rust files):

| File | Route | Notes |
|---|---|---|
| `routes/__root.rs` | Root layout | Wraps every route; app shell, providers |
| `routes/index.rs` | `/` | Index route of its folder |
| `routes/about.rs` | `/about` | |
| `routes/notes.rs` | `/notes` layout | Renders `Outlet` for children |
| `routes/notes/index.rs` | `/notes` | |
| `routes/notes/$id.rs` | `/notes/:id` | `$name` is a path param |
| `routes/notes.$id.edit.rs` | `/notes/:id/edit` | Flat route: dots are segments |
| `routes/files/$.rs` | `/files/*` | Splat (catch-all) |
| `routes/_auth.rs` | none | Pathless layout: wraps children without a URL segment |
| `routes/_auth/settings.rs` | `/settings` | Rendered inside `_auth` |
| `routes/(marketing)/pricing.rs` | `/pricing` | Group folder: organizes files only |
| `routes/-components/note_card.rs` | none | `-` prefix: ignored by the generator, colocated helpers |
| `routes/[rok-ui].rs` | `/rok-ui` | Brackets escape special characters |

**A route file** declares one route with `file_route!`. The generator checks that `$` params in
the path match the declared params, and generates `routes::NotesId` for links:

```rust
// src/routes/notes/$id.rs
use rok_ui::prelude::*;
use crate::features::notes::{NoteCard, note_query};

file_route! {
    params: { id: u64 },
    search: NoteSearch,
    loader: |params, cx| async move { ensure_query_data(cx, note_query(params.id)).await },
    pending: || Skeleton::new().h(px(200.)),
    error: |error| Alert::new("Could not load the note").description(error.to_string()),
    component: NotePage,
}

#[derive(Search, Clone, Default, PartialEq)]
pub struct NoteSearch { #[search(default)] pub tab: NoteTab }

#[component]
fn NotePage(cx: &mut Cx) -> impl IntoElement {
    let note = use_loader_data::<Route>(cx);          // typed loader output
    let NoteSearch { tab } = use_search::<Route>(cx);
    NoteCard::new(note, tab)
}
```

`Route` in a file refers to that file's generated route type, like TanStack's `Route.useParams()`.

**Why a build script and not a proc macro:** proc macros cannot reliably track new files, while
`build.rs` reruns on changes to `src/routes` (`cargo:rerun-if-changed`). A
`cargo rok-ui routes` command writes the same output to a checked-in `src/route_tree.rs` for
teams that prefer reviewing generated code.

### E.2 Code-based routing (alternative)

The same tree without files, for small apps and libraries:

```rust
let tree = route_tree![
    root(AppShell) [
        index(Home),
        "notes" => layout(NotesLayout) [ index(NotesList), ":id" => NotePage ],
        "settings/*" => Settings,
    ],
    not_found(NotFound),
];
```

The 0.5 `Router::new().route(..)` API stays as `router::legacy` for one release (Part J).

### E.3 Typed links and navigation

```rust
Link::to(routes::NotesId { id: 3 }).child("Open")                      // compile-checked
Link::to(routes::NotesId { id: 3 }).search(NoteSearch { tab: NoteTab::History })
Link::to(routes::Notes).active_props(style! { font: semibold })        // active styling
router::navigate(cx, routes::NotesIdEdit { id: 3 });
router::navigate(cx, Navigate::to(routes::Notes).replace(true));
let params = use_params::<routes::NotesId>(cx);                        // NotesId { id }
```

Every generated route type implements `Display` (path building with percent-encoding) and
`FromStr` (matching), so typed and string navigation share one code path.

### E.4 Typed search params

```rust
#[derive(Search, Clone, Default, PartialEq)]
struct NotesSearch {
    #[search(default = 1)] page: u32,
    q: Option<String>,
    #[search(default)] sort: SortOrder,       // enums parse from strings
}

let search = use_search::<routes::Notes>(cx);              // validated; invalid -> defaults
router::update_search::<routes::Notes>(cx, |s| s.page += 1); // new history entry (or .replace())
```

Search params are the recommended home for shareable, restorable UI state (filters, tabs,
pagination). Loaders can depend on them with `loader_deps: |search| search.page`.

### E.5 Guards, context and control flow

```rust
// src/routes/_auth.rs: every child of _auth requires a signed-in user.
file_route! {
    before_load: |cx| async move {
        let user = require_user(cx).await?;                 // memoized helper (C.5)
        Ok(RouteContext::new().with(user))                  // available to child loaders
    },
    component: AuthedLayout,
}

fn require_user(cx: &Cx) -> impl Future<Output = Result<User, RouteControl>> + '_ {
    async move {
        current_user(cx).await.clone().ok_or_else(|| {
            RouteControl::redirect(routes::Login { next: Some(router::location(cx).href()) })
        })
    }
}
```

`RouteControl` is `Redirect`, `NotFound` or an error; loaders return the same. App-level
context (`RouterProvider::new(tree).context(AppContext { db, session })`) is available to every
guard and loader through `cx.get::<AppContext>()`.

### E.6 Loaders, pending UI and preloading

- Loaders run on the shared runtime, use the query cache (C.2) keyed by route, params and
  `loader_deps`, are deduped, and are cancelled when the user navigates away.
- `pending` shows after `pending_ms` (default 300 ms) and stays at least `pending_min_ms`
  (default 300 ms) to avoid flicker; the previous page stays visible until then.
- `Link::to(..).preload(Preload::Intent)` loads on hover or focus; `Preload::Viewport` when
  visible. App default: `RouterProvider::new(tree).default_preload(Preload::Intent)`.
- `router::state(cx)` exposes `is_loading`, `pending_location` and `status` for a progress bar.

### E.7 Per-window routers, blocking, scroll and persistence

- **Per-window history**: each `RouterProvider` owns its history (default). `.app_wide()` keeps
  the 0.5 behavior. Fixes the 0.5 limitation.
- `use_blocker(cx, condition)` with a confirm dialog when navigating away from dirty state.
- Scroll restoration per history entry for `Scaffold` bodies and `ScrollArea`.
- `RouterProvider::persist("main-window")` restores location and search on launch.
- Optional page transitions using the existing `motion` presence.
- Alt+Left / Alt+Right and mouse back / forward buttons.

---

## Part F: Folder structure

### F.1 Recommended app structure

`cargo rok-ui new my-app` scaffolds this layout (with `--template minimal | full | db`):

```text
my-app/
|-- Cargo.toml
|-- build.rs                    # rok_ui_build::routes("src/routes")
|-- rok-ui.toml                 # CLI config: component dir, routes dir, theme preset
|-- assets/                     # icons, images, fonts
|-- migrations/                 # rok-db migrations (template `db`)
|-- src/
|   |-- main.rs                 # Application, rok_ui::init, windows
|   |-- app.rs                  # app context: Database, Session, stores (cx.set_global)
|   |-- routes/                 # file-based routes (Part E)
|   |   |-- __root.rs
|   |   |-- index.rs
|   |   |-- _auth.rs
|   |   |-- _auth/
|   |   |   |-- notes.rs
|   |   |   `-- notes/
|   |   |       |-- index.rs
|   |   |       `-- $id.rs
|   |   `-- login.rs
|   |-- features/               # domain code, one folder per feature
|   |   |-- notes.rs            # barrel: mod + pub use
|   |   `-- notes/
|   |       |-- model.rs        # rok-db models, domain types
|   |       |-- queries.rs      # QueryOptions functions (C.2)
|   |       |-- procedures.rs   # #[procedure]s (C.4)
|   |       |-- forms.rs        # FormValues + FormOptions (Part D)
|   |       `-- components.rs   # feature UI (NoteCard, NoteEditor)
|   |-- components/             # shared app UI
|   |   `-- ui/                 # components vendored with `cargo rok-ui add` (G.4)
|   `-- state.rs                # app-wide stores (C.6, C.7)
`-- tests/
    |-- routes.rs               # navigation, guards, loaders with gpui::test
    `-- features/               # procedure and form tests
```

Rules: routes stay thin (params, search, loader, component); domain logic lives in
`features/`; anything shared by two features moves to `components/` or a new feature. Barrel
files (`notes.rs` next to `notes/`) follow topcoat's style rule; no `mod.rs`.

### F.2 rok-ui repository structure

Move to a workspace layout like topcoat's, so each concern is a crate with its own tests:

```text
rok-ui/
|-- Cargo.toml                  # [workspace], [workspace.dependencies], [workspace.lints]
|-- AGENTS.md  CLAUDE.md  llms.txt
|-- .agents/skills/             # check, commit, pr, prose, style, macro, component, release
|-- docs/                       # guide sources mirroring module paths (H.3)
|   |-- router.md  router/{file_routes,loaders,search,guards}.md
|   |-- query.md  forms.md  forms/{validation,arrays,app_form}.md  state.md  db.md
|-- crates/
|   |-- rok-ui/                 # facade: re-exports, features, prelude
|   |-- rok-ui-components/      # the component library (today's src/components)
|   |-- rok-ui-core/            # theme, sx, motion, bidi, fonts, hooks, layer
|   |-- rok-ui-router/          # router runtime
|   |-- rok-ui-query/           # query cache, mutations, procedures runtime
|   |-- rok-ui-forms/           # forms runtime
|   |-- rok-ui-db/              # rok-db integration
|   |-- rok-ui-grammar/         # macro ASTs and codegen (H.2)
|   |-- rok-ui-macros/          # proc-macro entry points
|   |-- rok-ui-build/           # file-route generator for build.rs
|   `-- rok-ui-cli/             # cargo rok-ui new | add | routes
|-- examples/                   # each a crate: gallery, app_shell, notes (file routes), db_users
|-- templates/                  # cargo rok-ui new templates
`-- tests/                      # cross-crate integration tests
```

Users keep depending on `rok-ui` only; features select sub-crates. Splitting also improves
compile times (sub-crates build in parallel) and lets `rok-ui-build` stay free of GPUI.

---

## Part G: Component standards

### G.1 Typestate props

```rust
#[component]
fn Avatar(
    #[into] src: SharedString,            // required; accepts &str
    #[default] size: AvatarSize,          // optional, Default
    #[default(px(32.))] radius: Pixels,   // optional, custom default
    cx: &mut Cx,
) -> impl IntoElement { .. }
```

The builder implements `IntoElement` only once every required prop is set; a missing prop is a
compile error naming it. `#[prop(optional)]` stays as a deprecated alias for one release.

### G.2 Markup identity: `#[key(..)]`

```rust
view! {
    #[key(todo.id)]
    for todo in todos { TodoRow(todo) }
}
```

Keyed scopes keep hook state per item across reorders.

### G.3 The component contract

`docs/contributing/components.md`, enforced by review and the `component` agent skill:

- Struct, then inherent impl, then trait impls; tests at the end of the file.
- Builders return `Self` and are `#[must_use]`; every visual component implements `Styled`
  and `SxStyled` and applies `.sx` last.
- Controlled by default (`value` / `on_change`); uncontrolled variants use `default_value` and
  keep state per id. One naming scheme everywhere (audit `selected_index`, `checked`, `open`).
- Form controls implement `bound(&field)` (D.6).
- Text slots go through `BidiText`; direction-sensitive layout has an RTL test.
- Keyboard: `tab_index`, activation through `on_activate`, a visible focus ring.
- Each component has a compiled doc example, a gallery entry, a render test in every theme and
  a behavior test per interactive path.

### G.4 `cargo rok-ui add`

Copies a component's source into `src/components/ui/` with imports rewritten (shadcn,
`topcoat ui`). Components stay available as a dependency; vendoring is for heavy changes.

---

## Part H: Engineering standards (topcoat)

### H.1 Lints and formatting

```toml
[workspace.lints.rust]
unsafe_code = "deny"            # one allowed site: the DirectWrite lookup in fonts, own module, SAFETY docs
missing_docs = "warn"

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
mod_module_files = "deny"
too_many_lines = "allow"
```

Current pedantic findings: about 1,050 (about 830 `must_use`). Fix mechanically, one PR per
area, then make CI `-D warnings`. `rustfmt.toml` (nightly in CI): `max_width = 100`,
`wrap_comments`, `format_code_in_doc_comments`, `imports_granularity = "Crate"`,
`group_imports = "StdExternalCrate"`. Style rules: barrel files, struct-then-impls, no
needless allocations, ASCII-only code and docs, `[workspace.dependencies]` with versions only.

### H.2 Macro architecture

- `rok-ui-grammar`: lossless AST nodes (`pub` token fields, spans kept), `Parse` that validates
  without interpreting, `ParseOption` for optional nodes, private `mod kw`, semantics as
  methods, `ToTokens` codegen.
- `rok-ui-macros`: entry points only (parse, match, `to_compile_error()`).
- Tests: deterministic-expansion tests, trybuild pass / fail cases, doc examples. Every new
  macro (`file_route!`, `#[shard]`, `#[procedure]`, `#[memoize]`, `#[derive(Search)]`,
  `#[derive(FormValues)]`, `create_form_hook!`, `#[derive(Store)]`) follows this from day one.

### H.3 Documentation

- `docs/` mirrors module paths; modules embed guides with `#![doc = include_str!(..)]`.
- No `ignore` snippets: convert the 118 existing ones to compiled doctests with hidden setup
  lines and `no_run`.
- Prose rules: plain English, current state only, one line per paragraph, ASCII only.
- `llms.txt`: a dense API and conventions digest for coding agents, linked from the README.
- docs.rs: `--cfg docsrs` with `doc_cfg` badges for feature-gated items.

### H.4 CI and releases

| Job | Change |
|---|---|
| fmt | Nightly rustfmt with the new config |
| clippy | `--all-targets --all-features --locked -D warnings`, pedantic |
| features | `cargo hack clippy --each-feature --no-dev-deps` replaces `check-features.sh` |
| udeps | `cargo +nightly udeps --all-targets --all-features` |
| docs | Nightly, `RUSTDOCFLAGS="--cfg docsrs -D warnings"` |
| test | 3-OS matrix; Postgres service on Linux so `tests/db.rs` runs |
| templates | `cargo rok-ui new` for each template, then `cargo check` |
| msrv | Keep |
| semantic-pr | Conventional Commits titles (`feat(router): ..`) |
| release | release-plz: version and changelog PRs from commits; publish on merge |

Commit scopes: `router`, `query`, `forms`, `state`, `db`, `procedure`, `shard`, `macros`,
`build`, `cli`, `theme`, `bidi`, `fonts`, and component names.

### H.5 Coding-agent setup

```text
AGENTS.md            -> points to llms.txt and the skills
CLAUDE.md            -> "@AGENTS.md"
llms.txt             -> API + conventions digest
.agents/skills/
  check/       -> local commands mirroring CI
  commit/      -> Conventional Commits with rok-ui scopes
  pr/          -> describe the full diff; summary, testing, AI disclaimer
  prose/       -> guide placement and writing rules
  style/       -> code style, docs, test philosophy (behavior, not hardcoded values)
  macro/       -> grammar / macro split rules
  component/   -> the component contract (G.3) and checklist
  route/       -> file-route conventions (E.1) and checklist
  release/     -> versioning and publish order (grammar, macros, sub-crates, facade)
.claude -> .agents   (symlink, as in topcoat)
```

---

## Part I: Roadmap

| Release | Theme | Contents | Breaking? |
|---|---|---|---|
| **0.6** | Foundation | H.1 lints and fmt, H.2 macro split, H.3 doc conversion and `llms.txt`, H.4 CI and release-plz, H.5 skills, barrel files, shared `rok_ui::runtime`, F.2 workspace split (facade keeps the same API) | No |
| **0.7** | Data | C.1 `Cx`, C.2 query cache and `QueryOptions`, C.5 `#[memoize]`, C.3 `Suspense` / `ErrorBoundary` / `#[shard]`, C.4 procedures and mutations (local) | `db::use_query` (shim kept) |
| **0.8** | Router v2 | E.1 file-based routing (`rok-ui-build`), E.2 code-based tree, E.3 typed links, E.4 search, E.5 guards, E.6 loaders and preload, E.7 per-window history and blocking | `Router` (0.5 API kept as `router::legacy`) |
| **0.9** | Forms | Part D: forms runtime, validators, arrays, schema adapters, bound inputs, `create_form_hook!` | No |
| **0.10** | Ecosystem | F.1 `cargo rok-ui new` templates, G.4 `cargo rok-ui add`, C.7 field stores and persistence, C.8 devtools, G.1 typestate props, G.2 keys | `#[prop(optional)]` deprecated |
| **0.11** | Reach | C.4 remote procedures (axum), `router::legacy` removed | Yes (legacy removal) |
| **1.0** | Stabilize | API review against the contract and Part B conventions; docs complete; semver from here | - |

The `notes` example grows with each release into the reference app: file routes, loaders,
search params, shards, procedures, forms and rok-db.

### Acceptance criteria

- **0.6**: pedantic clippy clean with `-D warnings`; zero `ignore` doc blocks; `cargo hack`
  green; release-plz opens the release PR; public API unchanged (checked with
  `cargo public-api` diff).
- **0.7**: two components reading one key trigger one fetch; prefix invalidation refetches all
  matching queries and shards; a failing shard renders its `ErrorBoundary` without affecting
  siblings; an optimistic update rolls back on error.
- **0.8**: adding `routes/notes/$id.rs` makes `routes::NotesId` available with no other edits;
  linking with a wrong param type fails to compile (trybuild); a loader is not re-run within
  `stale_time`; navigating away cancels its loader; two windows keep independent histories;
  a guard redirect never renders the guarded component.
- **0.9**: blur and submit errors are kept apart (`error_map`); a debounced async validator runs
  once per pause; array field operations keep each row's state (keys); garde errors land on
  the right fields; a failed submit focuses the first invalid field.
- **0.10**: each template builds in CI; `cargo rok-ui add button` produces a compiling copy;
  a missing required prop fails to compile with the prop name (trybuild).

---

## Part J: Migration guide (0.5 -> 1.0)

Each release ships `docs/migration/<version>.md`; this is the summary.

| From (0.5) | To | When | Compatibility |
|---|---|---|---|
| `src/components/mod.rs`, single crate | Workspace sub-crates behind the `rok-ui` facade | 0.6 | Same public paths; nothing to change |
| `db::use_query(key, window, cx, \|db\| ..)` | `use_query(cx, db_query(key, \|db\| ..))` | 0.7 | Old function kept as a shim with a deprecation warning until 0.8 |
| `db::invalidate("users", cx)` | `query::invalidate(cx, query_key!["users"])` (prefix match) | 0.7 | Old function forwards to the new one |
| `db::run(cx, \|db\| ..)` for writes | `#[procedure]` + `use_procedure`, or `use_mutation` | 0.7 | `db::run` stays (it is the low-level API) |
| `Router::new().route("/notes/:id", ..)` | `src/routes/notes/$id.rs` with `file_route!`, or `route_tree!` | 0.8 | 0.5 API moves to `router::legacy`; removed in 0.11 |
| `route.param_as::<u64>("id")` | `use_params::<routes::NotesId>(cx).id` | 0.8 | Legacy only |
| `Link::new(id, "/notes/3")` | `Link::to(routes::NotesId { id: 3 })` | 0.8 | String links keep working through `Link::href(..)` |
| App-wide history | Per-window `RouterProvider` | 0.8 | `.app_wide()` restores the old behavior |
| Hand-written form state with `use_input_state` | `use_form` + `bound(&field)` | 0.9 | `use_input_state` stays for standalone inputs |
| `#[prop(optional)]` | `#[default]` | 0.10 | Alias with deprecation warning until 1.0 |
| `use_keyed_state(format!(..))` in loops | `#[key(..)]` in `view!` | 0.10 | Both work |

**Migrating the `notes` example to file routes (0.8)** is the worked example in
`docs/migration/0.8.md`:

1. Add `build.rs` with `rok_ui_build::routes("src/routes")` and `rok_ui::routes!()` in `main.rs`.
2. Move each `.route(pattern, builder)` into `src/routes/<path>.rs` with `file_route!`, turning
   `:id` into a `$id.rs` file name and `route.param_as` into `params: { id: u64 }`.
3. Move the `AdaptiveScaffold` shell into `routes/__root.rs` and render `Outlet`.
4. Replace string `Link`s and `router::navigate("/..")` with generated route types; the
   compiler lists every call site to fix.
5. Move note loading into loaders with `QueryOptions` from `features/notes/queries.rs`.

---

## Part K: Decisions needed

1. **Version cadence**: one minor per phase (Part I), or larger batches?
2. **Workspace split (F.2) in 0.6**: do it early while the API is small (proposed), or after
   1.0?
3. **File routing naming**: TanStack conventions (`$id.rs`, `_layout`, `(group)`, `-ignored`)
   as proposed, or Rust-flavored names (`by_id.rs` + `#[param]`) that avoid `$` in file names?
4. **Router v2 compatibility**: keep the 0.5 router as `router::legacy` until 0.11 (proposed),
   or replace it outright in 0.8?
5. **Form validation adapters**: `garde` only (newer, derive-friendly), or `garde` and
   `validator`?
6. **Procedures remote mode**: axum via your `axum-rok-http`, or local-only for now?
7. **rok-ui-hooks**: build field stores and the query cache's signal primitives in rok-ui-hooks
   first (publish 0.4; the local checkout has uncommitted changes beyond 0.3.0), or in rok-ui?
8. **rok-db helpers**: first-class (`Model::query_options()`, relation preloading in loaders),
   or keep rok-db as one generic data source?
9. **Formatting**: nightly rustfmt in CI (needed for import grouping), or stable without the
   two import options?

## Sources

- topcoat at `765c87f`: `README.md`, `llms.txt`, `AGENTS.md`, `.agents/skills/*`,
  `Cargo.toml` (`[workspace.lints]`, `[workspace.dependencies]`), `rustfmt.toml`, `clippy.toml`,
  `release-plz.toml`, `.github/workflows/ci.yml`, `docs/runtime/{shard,procedure}.md`,
  `docs/context/{memoize,functions_not_middlewares}.md`, `docs/router/{href,layout,module}.md`,
  `docs/view/props.md`, `crates/topcoat-router/{grammar,macro}`.
- TanStack Router (file-based routing conventions, `createFileRoute`, loaders, search
  validation, `beforeLoad`), Query (`queryOptions`, mutations), Start (`createServerFn`) and
  Form (`useForm`, field API, validators, array fields, `createFormHook`), as documented on
  tanstack.com. Written from knowledge of those APIs; not fetched for this revision.
- rok-ui 0.5.0 measurements: about 1,050 pedantic clippy warnings, 118 `ignore` doc blocks,
  `src/components/mod.rs` and `src/theme/mod.rs`, one `unsafe` block in `src/fonts.rs`.
