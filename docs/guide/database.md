# Database

The `db` feature gives rok-ui apps `PostgreSQL` out of the box through
[rok-db](https://crates.io/crates/rok-db), a type-safe async ORM built on sqlx. rok-ui runs
rok-db on a small background tokio runtime and brings results back to the UI thread, so your
components can show live data with a single hook.

```toml
rok-ui = { version = "0.5", features = ["db"] }

# Column types and migrations, forwarded to rok-db:
rok-ui = { version = "0.5", features = ["db", "db-chrono", "db-uuid", "db-json", "db-migrate"] }
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
 db::use_query("users", …) ──── query ──▶ User::query().all(&db).await
        ▲                                         │
        └──── rows, window re-renders ◀───────────┘
```

- `rok_ui::db::Db` is rok-db's connection handle: a cheap, cloneable pool.
- Futures you pass to `db::run` and `db::use_query` run on the tokio runtime, so they must be
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
rok-db's README for relations (`has_many`, `belongs_to`), timestamps, custom column names and
the full query API.

## Connecting

Connect once at startup. Queries that render before the connection is ready show
`DbError::NotConnected`, then run again automatically once it connects:

```rust,ignore
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
    // open windows…
});
```

To configure the pool, query cache or slow-query logging, build the `Db` yourself on the
database runtime and hand it over:

```rust,ignore
use std::time::Duration;
use rok_ui::db::{self, Db};

let db = db::runtime().block_on(
    Db::builder()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .query_cache(500)
        .slow_query_threshold(Duration::from_millis(200))
        .connect(&url),
)?;
db::set_connection(db, cx);
```

`set_connection` can be called again at any time, for example to switch accounts. Every query
re-runs with the new connection.

## Reading data: `use_query`

`db::use_query(key, window, cx, |db| async move { … })` works like React Query's `useQuery`:

1. The first render starts the query and returns a loading state.
2. When the result arrives, the window re-renders and the hook returns the data (or the error).
3. Later renders return the cached result without touching the database.
4. `db::invalidate(key, cx)` (or a new connection) marks it stale. The next render fetches
   again and keeps showing the old data until the new data arrives, so lists don't flash
   empty.

```rust,ignore
#[component]
fn UserList(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let users = db::use_query("users", window, cx, |db| async move {
        User::query().order_by(User::EMAIL.asc()).all(&db).await
    });

    match (users.data(), users.error()) {
        (_, Some(error)) => Alert::new("Could not load users")
            .destructive()
            .description(error.to_string())
            .into_any_element(),
        (Some(users), None) if users.is_empty() => Empty::new()
            .icon(IconName::User)
            .title("No users yet")
            .into_any_element(),
        (Some(users), None) => div()
            .flex()
            .flex_col()
            .children(users.iter().map(|user| {
                Item::new(("user", user.id as usize))
                    .title(user.email.clone())
                    .description(user.name.clone().unwrap_or_default())
            }))
            .into_any_element(),
        (None, None) => Spinner::new().into_any_element(),
    }
}
```

`Query<T>` has `data()`, `error()` and `is_loading()`. `T` must be `Clone + Send`, and rok-db
rows usually are.

### Keys

The key names the data for invalidation. Include every input the query depends on, so
different inputs are cached separately:

```rust,ignore
let page = use_state(window, cx, || 1u64);
let current = page.get(cx);
let users = db::use_query(format!("users:page:{current}"), window, cx, move |db| async move {
    User::query().order_by(User::ID.asc()).paginate(&db, current, 20).await
});
// users.data() is a rok-db `Page<User>`: `items`, `total`, `total_pages()`, `has_next()`.
```

Each element caches its own result. Two components using the same key fetch separately but
are invalidated together. Invalidation matches the whole key, so after inserting a user, call
`db::invalidate` for the keys you know are affected, or `db::invalidate_all(cx)`.

### Filtering from input

Pass the filter into the key and the closure. Each new value gets its own cache entry:

```rust,ignore
let search = use_input_state("search", window, cx, |state| state.with_placeholder("Search"));
let term = search.read(cx).text().to_string();
let pattern = format!("%{term}%");
let matches = db::use_query(format!("users:search:{term}"), window, cx, move |db| async move {
    User::filter(User::EMAIL.ilike(pattern)).limit(50).all(&db).await
});
```

Every keystroke starts a query for the new term. To query less often, debounce the term with
the `state` feature's `use_debounced` and key on the debounced value.

## Writing data: `run`

`db::run(cx, |db| async move { … })` runs any database work (inserts, updates, deletes,
transactions, raw SQL) and returns a GPUI `Task<Result<T, DbError>>`. Await it in `cx.spawn` to
react to the result, then invalidate the queries that read the changed tables:

```rust,ignore
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

```rust,ignore
let transfer = db::run(cx, move |db| async move {
    db.transaction(|tx| Box::pin(async move {
        Account::filter(Account::ID.eq(from)).update().increment(Account::BALANCE, -amount).exec(&mut *tx).await?;
        Account::filter(Account::ID.eq(to)).update().increment(Account::BALANCE, amount).exec(&mut *tx).await?;
        Ok::<_, rok_db::Error>(())
    }))
    .await
});
```

The transaction commits when the closure returns `Ok` and rolls back on `Err`.

### Schema setup and migrations

```rust,ignore
// One statement or a whole script, without parameters:
db::run(cx, |db| async move { db.execute(include_str!("schema.sql")).await }).detach();

// With `db-migrate`, sqlx migrations from a folder:
db::run(cx, |db| async move { db.migrate("./migrations").await }).detach();
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

```rust,ignore
let insert = db::run(cx, move |db| async move {
    match user.insert(&db).await {
        Err(error) if error.is_unique_violation() => Ok(None),   // already registered
        other => other.map(Some),
    }
});
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

```rust,ignore
#[gpui::test]
fn loads_users(cx: &mut gpui::TestAppContext) {
    let Ok(url) = std::env::var("ROK_UI_TEST_DATABASE_URL") else { return };
    cx.executor().allow_parking();
    let connecting = cx.update(|cx| db::connect(url, cx));
    cx.executor().block_test(connecting).unwrap();
    let count = cx.update(|cx| db::run(cx, |db| async move { User::count(&db).await }));
    assert!(cx.executor().block_test(count).is_ok());
}
```

## Full example

`examples/db_users.rs` is a complete user list: it connects, creates its table, lists rows
with `use_query`, and inserts and deletes with `run` and `invalidate`.

```sh
DATABASE_URL=postgres://user:password@localhost/app cargo run --example db_users --features db
```

## Reference

| Item | What it does |
|---|---|
| `connect(url, cx)` | Connect in the background and make it the app's connection |
| `set_connection(db, cx)`, `connection(cx)` | Set or get the `Db` |
| `run(cx, \|db\| async { … })` | Any database work, as a `Task<Result<T, DbError>>` |
| `use_query(key, window, cx, \|db\| async { … })` | Cached `Query<T>` for a component |
| `invalidate(key, cx)`, `invalidate_all(cx)` | Make queries fetch again |
| `runtime()` | The tokio runtime rok-db runs on |
| `DbError` | `NotConnected`, `Failed(message)`, `Cancelled` |
| `rok_db` | The rok-db crate: models, queries, transactions, raw SQL |
