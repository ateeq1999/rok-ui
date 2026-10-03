---
name: db
description: Work on rok-ui's `db` feature or write app code that reads and writes PostgreSQL through rok-db (models, db_query, db_mutation, db::run, invalidation, tests).
---

# Database (`db` feature, rok-db)

`src/db.rs` is glue: it runs rok-db on the shared tokio runtime (`crate::runtime`) and hands
results back to GPUI. It never builds SQL itself. The guide is `docs/guide/database.md` (also
the module docs).

## Which rok-db you have

rok-ui depends on `rok-db = "0.1"` from crates.io (check `Cargo.lock`). Version 0.1.0 has
models, relations, `filter` / `order_by` / `select` / `group_by`, `paginate`, `stream`,
`memoize`, transactions and migrations. Features on rok-db's `main` branch (joins,
`cursor_paginate`, composite keys, tenancy, audit log, change feeds, `all_with` tuples) are
**not** available until rok-db releases them and rok-ui bumps the requirement. Do not use them
in rok-ui code, docs or examples before then. When unsure, look at the source in
`~/.cargo/registry/src/*/rok-db-core-<version>/`.

## App code patterns

- Models: `#[derive(Model)]` from `rok_ui::db::rok_db::prelude::*`. Without a direct rok-db
  dependency add `#[rok(crate = "rok_ui::db::rok_db")]`.
- Reads: define query options in a function and reuse them, so every reader shares one key:

  ```rust
  fn users_query() -> QueryOptions<Vec<User>> {
      db::db_query(query_key!["users"], |db| async move {
          User::query().order_by(User::EMAIL.asc()).all(&db).await
      })
  }
  ```

  Read with `query::use_query(cx, users_query())` and match on `.state()`
  (`Pending`, `Error`, `Success`). Never call the deprecated `db::use_query` in new code.
- Writes: `db::db_mutation(|db, input| async move { .. }).invalidates(query_key!["users"])`
  with `query::use_mutation`, or `db::run(cx, |db| async move { .. })` plus
  `db::invalidate("users", cx)` for one-off work.
- Keys: the first key part is the table or resource (`["users", id]`), so invalidating
  `["users"]` refreshes every query that reads it.
- Futures run on another thread: they must be `Send + 'static`. Clone the `Db` and the inputs
  into them. Never capture `Entity`, `Window`, `App` or other UI handles.
- Errors surface as `DbError::{NotConnected, Failed, Cancelled}`. Show `NotConnected` as a
  loading or setup state, not as a failure: queries rerun when the connection arrives.
- Pool, cache and slow-query settings: build the `Db` with `Db::builder()` on
  `db::runtime()` and pass it to `db::set_connection`.
- Never format user input into SQL. Use typed filters (`User::EMAIL.eq(input)`); raw SQL goes
  through rok-db's bound-parameter APIs only.

## Changing `src/db.rs`

- Keep it a thin adapter over `crate::query`: new read helpers return `QueryOptions`, new
  write helpers return `MutationOptions`.
- Everything stays behind `#[cfg(feature = "db")]`. Column-type features forward to rok-db
  (`db-chrono = ["db", "rok-db/chrono"]`). Run `scripts/check-features.sh db db-chrono db-uuid
  db-json db-migrate`.
- Behavior tests go in `tests/db.rs`. They skip unless `ROK_UI_TEST_DATABASE_URL` is set; run
  them against a real server:

  ```sh
  docker run -d --rm -e POSTGRES_PASSWORD=rok -p 55432:5432 postgres:17-alpine
  ROK_UI_TEST_DATABASE_URL=postgres://postgres:rok@127.0.0.1:55432/postgres \
      cargo test --features db --test db
  ```

  Use a table name unique to the test and drop it at the end, so parallel runs don't clash.
- Update `docs/guide/database.md`, the `db_users` example and `llms.txt` when the API changes.
- A bug in query building belongs in rok-db, not in a workaround here: reproduce it with
  rok-db alone and report it there.
