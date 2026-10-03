# Database

The `db` feature gives rok-ui apps `PostgreSQL` out of the box through
[rok-db](https://crates.io/crates/rok-db), a type-safe async ORM built on sqlx. rok-ui runs
rok-db on a small background tokio runtime and brings results back to the UI thread, so your
components can show live data with a single hook.

```toml
rok-ui = { version = "0.6", features = ["db"] }

# Column types and migrations, forwarded to rok-db:
rok-ui = { version = "0.6", features = ["db", "db-chrono", "db-uuid", "db-json", "db-migrate"] }
```

| Feature | Adds |
|---|---|
| `db` | `rok_ui::db`, rok-db, a tokio runtime (2 worker threads) |
| `db-chrono` | `chrono::DateTime<Utc>`, `NaiveDateTime`, `NaiveDate`, `NaiveTime` columns |
| `db-uuid` | `uuid::Uuid` columns |
| `db-json` | `serde_json::Value` and `Json<T>` columns |
| `db-migrate` | `db.migrate("./migrations")` |

`db` is not part of `full`, because sqlx and tokio add noticeably to build times.

## How it fits together

```text
 UI thread (GPUI)                         background (tokio, 2 threads)
 ─────────────────                        ─────────────────────────────
 query::use_query(cx, users) ── query ──▶ User::query().all(&db).await
        ▲                                         │
        └──── rows, window re-renders ◀───────────┘
```

- `rok_ui::db::Db` is rok-db's connection handle: a cheap, cloneable pool.
- Futures you pass to `db::run`, `db::db_query` and `db::db_mutation` run on the shared tokio
  runtime ([`crate::runtime`]), so they must be
  `Send + 'static`. Clone what they need into them, and don't capture UI handles.
- Results come back as GPUI tasks, so the UI thread never blocks on the database.

## Defining models

Models are plain structs with rok-db's `Model` derive. If rok-db is not a direct dependency of
your app, tell the derive where to find it with `crate = "rok_ui::db::rok_db"`:

```rust,no_run
use rok_ui::db::rok_db::prelude::*;

#[derive(Debug, Clone, Model)]
#[rok(crate = "rok_ui::db::rok_db", table = "users")]
struct User {
    #[rok(primary_key, generated)]   // BIGSERIAL, filled in by Postgres
    id: i64,
    email: String,
    name: Option<String>,
}
```

Every column gets a typed constant (`User::EMAIL`, `User::NAME`) for filters and ordering. See
rok-db's README for relations (`has_many`, `belongs_to`), joins, timestamps, soft deletes,
optimistic locking, keyset pagination, custom column names and the full query API. rok-ui
depends on rok-db 0.3.

## Connecting

Connect once at startup. Queries that render before the connection is ready show
`DbError::NotConnected`, then run again automatically once it connects:

```rust,no_run
# use rok_ui::prelude::*;
use rok_ui::db;

Application::new().with_assets(rok_ui::Assets).run(|cx: &mut App| {
    rok_ui::init(cx);
    let url = std::env::var("DATABASE_URL").expect("set DATABASE_URL");
    let connecting = db::connect(url, cx);
    cx.spawn(async move |_| {
        if let Err(error) = connecting.await {
            eprintln!("could not connect: {error}");
        }
    })
    .detach();
    // open windows...
});
```

To configure the pool, query cache or slow-query logging, build the `Db` yourself on the
database runtime and hand it over:

```rust,no_run
# use rok_ui::prelude::*;
use std::time::Duration;
use rok_ui::db::{self, Db};

# fn example(cx: &mut App, url: &str) -> Result<(), rok_ui::db::rok_db::Error> {
let db = db::runtime().block_on(
    Db::builder()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .query_cache(500)
        .slow_query_threshold(Duration::from_millis(200))
        .connect(url),
)?;
db::set_connection(db, cx);
# Ok(())
# }
```

`set_connection` can be called again at any time, for example to switch accounts. Every query
re-runs with the new connection.

## Reading data: `db_query`

[`db::db_query`](crate::db::db_query) builds [query options](crate::query) that read the
database with the app's connection. Read them with `query::use_query`, which works like
TanStack Query's `useQuery`:

1. The first render starts the query and returns a pending state.
2. When the result arrives, the window re-renders and the hook returns the data (or the error).
3. Readers of the same key share the cached data and one fetch.
4. `query::invalidate(cx, &query_key!["users"])` (or a new connection) marks every key starting
   with `["users"]` stale. Readers refetch and keep showing the old data until the new data
   arrives, so lists don't flash empty.

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::prelude::*}, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "users")]
# struct User { #[rok(primary_key, generated)] id: i64, email: String, name: Option<String> }
fn users_query() -> QueryOptions<Vec<User>> {
    db::db_query(query_key!["users"], |db| async move {
        User::query().order_by(User::EMAIL.asc()).all(&db).await
    })
}

#[component]
fn UserList(cx: &mut Cx) -> impl IntoElement {
    let users = query::use_query(cx, users_query());
    match users.state() {
        QueryState::Error(error) => Alert::new("Could not load users")
            .destructive()
            .description(error.to_string())
            .into_any_element(),
        QueryState::Success(users) if users.is_empty() => Empty::new()
            .icon(IconName::User)
            .title("No users yet")
            .into_any_element(),
        QueryState::Success(users) => div()
            .flex()
            .flex_col()
            .children(users.iter().map(|user| {
                Item::new(("user", user.id as usize))
                    .title(user.email.clone())
                    .description(user.name.clone().unwrap_or_default())
            }))
            .into_any_element(),
        QueryState::Pending => Spinner::new().into_any_element(),
    }
}
```

The 0.5 hook, `db::use_query(key, window, cx, ..)`, still works but is deprecated and goes away
in 0.8. `db::invalidate("users", cx)` invalidates both kinds of queries.

### Keys

Include every input the query depends on in the key, so different inputs are cached
separately. Keys are hierarchical, so one invalidation covers every page:

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::prelude::*}, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "users")]
# struct User { #[rok(primary_key, generated)] id: i64, email: String, name: Option<String> }
# fn example(cx: &mut Cx) {
let page = cx.use_state(|| 1u64);
let current = page.get(cx);
let users = query::use_query(
    cx,
    db::db_query(query_key!["users", "page", current], move |db| async move {
        User::query().order_by(User::ID.asc()).paginate(&db, current, 20).await
    })
    .keep_previous_data(true),
);
// After an insert: query::invalidate(cx, &query_key!["users"]) refetches every page.
# }
```

### Filtering from input

Put the filter in the key and the closure. Each new value gets its own cache entry:

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::prelude::*}, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "users")]
# struct User { #[rok(primary_key, generated)] id: i64, email: String, name: Option<String> }
# fn example(cx: &mut Cx) {
let search = use_input_state("search", cx.window, cx.app, |state| state.with_placeholder("Search"));
let term = search.read(cx).text().to_string();
let pattern = format!("%{term}%");
let matches = query::use_query(
    cx,
    db::db_query(query_key!["users", "search", term], move |db| {
        let pattern = pattern.clone();
        async move { User::filter(User::EMAIL.ilike(pattern)).limit(50).all(&db).await }
    }),
);
# }
```

Every keystroke starts a query for the new term. To query less often, debounce the term with
the `state` feature's `use_debounced` and key on the debounced value.

## Writing data with mutations

`db::db_mutation(|db, input| async move { .. })` gives mutation options for
`query::use_mutation`, with pending and error state, invalidation and optimistic updates:

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::prelude::*}, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "users")]
# struct User { #[rok(primary_key, generated)] id: i64, email: String, name: Option<String> }
# fn example(cx: &mut Cx, new_user: User) {
let add = query::use_mutation(
    cx,
    db::db_mutation(|db, user: User| async move { user.insert(&db).await })
        .invalidates(query_key!["users"]),
);
let button = Button::new("add").loading(add.is_pending()).on_click(add.mutate_handler(new_user));
# }
```

## Writing data: `run`

`db::run(cx, |db| async move { … })` runs any database work (inserts, updates, deletes,
transactions, raw SQL) and returns a GPUI `Task<Result<T, DbError>>`. Await it in `cx.spawn` to
react to the result, then invalidate the queries that read the changed tables:

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::prelude::*}, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "users")]
# struct User { #[rok(primary_key, generated)] id: i64, email: String, name: Option<String> }
fn add_user(email: String, cx: &mut App) {
    let insert = db::run(cx, move |db| async move {
        User { id: 0, email, name: None }.insert(&db).await
    });
    cx.spawn(async move |cx| {
        let result = insert.await;
        cx.update(|cx| match result {
            Ok(user) => {
                toast(cx, Toast::success(format!("Added {}", user.email)));
                db::invalidate("users", cx);
            }
            Err(error) => {
                toast(cx, Toast::error("Could not add the user").description(error.to_string()));
            }
        })
        .ok();
    })
    .detach();
}
```

For writes nobody needs to wait for, `db::run(..).detach()` is enough.

### Transactions

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::{self, prelude::*}}};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "accounts")]
# struct Account { #[rok(primary_key, generated)] id: i64, balance: i64 }
# fn example(cx: &mut App, from: i64, to: i64, amount: i64) {
let transfer = db::run(cx, move |db| async move {
    db.transaction(|tx| Box::pin(async move {
        Account::filter(Account::ID.eq(from)).update().increment(Account::BALANCE, -amount).exec(&mut *tx).await?;
        Account::filter(Account::ID.eq(to)).update().increment(Account::BALANCE, amount).exec(&mut *tx).await?;
        Ok::<_, rok_db::Error>(())
    }))
    .await
});
# }
```

The transaction commits when the closure returns `Ok` and rolls back on `Err`.

### Schema setup and migrations

```rust,no_run
# use rok_ui::{prelude::*, db};
# fn example(cx: &mut App) {
// One statement or a whole script, without parameters:
db::run(cx, |db| async move { db.execute("CREATE TABLE IF NOT EXISTS notes (id BIGSERIAL PRIMARY KEY)").await }).detach();

// With `db-migrate`, sqlx migrations from a folder:
db::run(cx, |db| async move { db.migrate("./migrations").await }).detach();
# }
```

## Errors

Every call returns `DbError`:

| Variant | When |
|---|---|
| `NotConnected` | No connection yet (`connect` is still running, failed, or was never called) |
| `Failed(message)` | rok-db or `PostgreSQL` reported an error; `message` is its text |
| `Cancelled` | The task stopped before finishing (the runtime shut down or the future panicked) |

`DbError` implements `Display` and `std::error::Error`. To act on a specific database error,
such as a unique violation, inspect `rok_db::Error` inside the closure before it is converted:

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::prelude::*}, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "users")]
# struct User { #[rok(primary_key, generated)] id: i64, email: String, name: Option<String> }
# fn example(cx: &mut App, user: User) {
let insert = db::run(cx, move |db| async move {
    match user.insert(&db).await {
        Err(error) if error.is_unique_violation() => Ok(None),   // already registered
        other => other.map(Some),
    }
});
# }
```

## Escape hatches

- `db::connection(cx)` returns the current `Db`. Clone it into your own tokio tasks for
  streaming (`query.stream(&db)`), long-running jobs, or code shared with a server.
- `db::runtime()` is the tokio runtime itself: `db::runtime().spawn(..)` or `block_on(..)` at
  startup.
- `rok_ui::db::rok_db` is the whole of rok-db, sqlx access included.

## Testing

`tests/db.rs` in the rok-ui repository runs against a real server when
`ROK_UI_TEST_DATABASE_URL` is set, and skips otherwise. The same pattern works for your app:

```sh
docker run -d --rm --name app-test-db -e POSTGRES_PASSWORD=test -p 55432:5432 postgres:17-alpine
ROK_UI_TEST_DATABASE_URL=postgres://postgres:test@127.0.0.1:55432/postgres cargo test
```

Database replies arrive on real threads, outside GPUI's test scheduler, so call
`cx.executor().allow_parking()` and wait with `cx.executor().block_test(task)`:

```rust,no_run
# use rok_ui::{prelude::*, db::{self, rok_db::prelude::*}, query::{self, QueryOptions, QueryState}, query_key};
# #[derive(Debug, Clone, Model)]
# #[rok(crate = "rok_ui::db::rok_db", table = "users")]
# struct User { #[rok(primary_key, generated)] id: i64, email: String, name: Option<String> }
#[gpui::test]
fn loads_users(cx: &mut gpui::TestAppContext) {
    let Ok(url) = std::env::var("ROK_UI_TEST_DATABASE_URL") else { return };
    cx.executor().allow_parking();
    let connecting = cx.update(|cx| db::connect(url, cx));
    cx.executor().block_test(connecting).unwrap();
    let count = cx.update(|cx| db::run(cx, |db| async move { User::query().count(&db).await }));
    assert!(cx.executor().block_test(count).is_ok());
}
```

## Full example

`examples/db_users.rs` is a complete user list: it connects, creates its table, lists rows
with `query::use_query` and `db_query`, and inserts and deletes with `run` and `invalidate`.

```sh
DATABASE_URL=postgres://user:password@localhost/app cargo run --example db_users --features db
```

## Reference

| Item | What it does |
|---|---|
| `connect(url, cx)` | Connect in the background and make it the app's connection |
| `set_connection(db, cx)`, `connection(cx)` | Set or get the `Db` |
| `run(cx, \|db\| async { … })` | Any database work, as a `Task<Result<T, DbError>>` |
| `db_query(query_key![..], \|db\| async { … })` | Query options for `query::use_query`, loaders and `fetch_query` |
| `db_mutation(\|db, input\| async { … })` | Mutation options for `query::use_mutation` |
| `use_query(key, window, cx, \|db\| async { … })` | Deprecated 0.5 hook; use `db_query` |
| `invalidate(key, cx)`, `invalidate_all(cx)` | Make queries fetch again |
| `runtime()` | The shared tokio runtime rok-db runs on |
| `DbError` | `NotConnected`, `Failed(message)`, `Cancelled` |
| `rok_db` | The rok-db crate: models, queries, transactions, raw SQL |
