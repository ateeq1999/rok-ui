#![doc = include_str!("../docs/guide/database.md")]

use std::{collections::HashMap, fmt, future::Future};

use gpui::{App, ElementId, Global, SharedString, Task, Window};

pub use rok_db;
pub use rok_db::Db;

use crate::hooks::use_keyed_state;

/// The background runtime rok-db's futures run on: the shared
/// [`crate::runtime`].
#[must_use]
pub fn runtime() -> &'static tokio::runtime::Runtime {
    crate::runtime::get()
}

/// Why a database call produced no result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbError {
    /// No connection yet: call [`connect`] or [`set_connection`] first.
    NotConnected,
    /// The database or rok-db reported an error.
    Failed(SharedString),
    /// The task stopped before finishing (the runtime shut down or it panicked).
    Cancelled,
}

impl fmt::Display for DbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str("not connected to a database"),
            Self::Failed(message) => formatter.write_str(message),
            Self::Cancelled => formatter.write_str("the database task was cancelled"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rok_db::Error> for DbError {
    fn from(error: rok_db::Error) -> Self {
        Self::Failed(error.to_string().into())
    }
}

/// The app's database connection, and a counter that makes every query re-run
/// when it changes.
#[derive(Default)]
struct Connection {
    db: Option<Db>,
    epoch: u64,
    generations: HashMap<SharedString, u64>,
}

impl Global for Connection {}

/// Connect to `url` in the background and make it the app's connection.
pub fn connect(url: impl Into<String>, cx: &mut App) -> Task<Result<(), DbError>> {
    let url = url.into();
    let connecting = runtime().spawn(async move { Db::connect(&url).await });
    cx.spawn(async move |cx| {
        let db = connecting.await.map_err(|_| DbError::Cancelled)??;
        cx.update(|cx| set_connection(db, cx))
            .map_err(|_| DbError::Cancelled)
    })
}

/// Use `db` as the app's connection (built with `Db::builder()` for pool and
/// cache settings, or shared with other code). Queries re-run with it.
pub fn set_connection(db: Db, cx: &mut App) {
    crate::query::provide(cx, db.clone());
    let connection = cx.default_global::<Connection>();
    connection.db = Some(db);
    connection.epoch += 1;
    crate::query::invalidate(cx, &crate::query::QueryKey::root());
    cx.refresh_windows();
}

/// Query options that read the database with the app's connection, for
/// [`query::use_query`](crate::query::use_query), loaders and `fetch_query`:
///
/// ```no_run
/// # use rok_ui::{db, query::QueryOptions, query_key};
/// fn database_is_up() -> QueryOptions<bool> {
///     db::db_query(query_key!["db", "up"], |db| async move { db.ping().await.map(|()| true) })
/// }
/// ```
///
/// Without a connection the query fails with [`DbError::NotConnected`].
pub fn db_query<T, Build, Fetch>(
    key: crate::query::QueryKey,
    build: Build,
) -> crate::query::QueryOptions<T>
where
    T: Send + 'static,
    Build: Fn(Db) -> Fetch + Send + Sync + 'static,
    Fetch: Future<Output = rok_db::Result<T>> + Send + 'static,
{
    let build = std::sync::Arc::new(build);
    crate::query::QueryOptions::new(key, move |cx| {
        let db = cx.get::<Db>().cloned();
        let build = build.clone();
        async move {
            let db = db.ok_or(DbError::NotConnected)?;
            build(db).await.map_err(DbError::from)
        }
    })
}

/// Mutation options that write with the app's connection, for
/// [`query::use_mutation`](crate::query::use_mutation). Add `.invalidates(..)` for the queries
/// that read the tables it changes.
pub fn db_mutation<I, O, Build, Write>(build: Build) -> crate::query::MutationOptions<I, O, DbError>
where
    I: Send + 'static,
    O: Send + 'static,
    Build: Fn(Db, I) -> Write + Send + Sync + 'static,
    Write: Future<Output = rok_db::Result<O>> + Send + 'static,
{
    let build = std::sync::Arc::new(build);
    crate::query::MutationOptions::new(move |cx, input| {
        let db = cx.get::<Db>().cloned();
        let build = build.clone();
        async move {
            let db = db.ok_or(DbError::NotConnected)?;
            build(db, input).await.map_err(DbError::from)
        }
    })
}

/// The app's connection, if there is one. `Db` is a cheap handle to the pool.
pub fn connection(cx: &App) -> Option<Db> {
    cx.try_global::<Connection>()
        .and_then(|connection| connection.db.clone())
}

/// Start `query` on the database runtime.
fn start<T, Query>(query: Query) -> tokio::task::JoinHandle<rok_db::Result<T>>
where
    T: Send + 'static,
    Query: Future<Output = rok_db::Result<T>> + Send + 'static,
{
    runtime().spawn(query)
}

/// Run database work with the app's connection: queries, inserts, transactions.
/// The returned task resolves on the UI thread's executor; `.await` it from
/// `cx.spawn`, or `.detach()` it for fire-and-forget writes.
pub fn run<T, Build, Query>(cx: &App, build: Build) -> Task<Result<T, DbError>>
where
    T: Send + 'static,
    Build: FnOnce(Db) -> Query,
    Query: Future<Output = rok_db::Result<T>> + Send + 'static,
{
    let Some(db) = connection(cx) else {
        return Task::ready(Err(DbError::NotConnected));
    };
    let running = start(build(db));
    cx.background_executor().spawn(async move {
        match running.await {
            Ok(result) => result.map_err(DbError::from),
            Err(_) => Err(DbError::Cancelled),
        }
    })
}

/// Make every [`use_query`] with this key fetch again, and every
/// [`query`](crate::query) whose key starts with `query_key![key]`.
pub fn invalidate(key: impl Into<SharedString>, cx: &mut App) {
    let key = key.into();
    crate::query::invalidate(cx, &crate::query_key![key.as_ref()]);
    *cx.default_global::<Connection>()
        .generations
        .entry(key)
        .or_default() += 1;
    cx.refresh_windows();
}

/// Make every [`use_query`] and every [`query`](crate::query) fetch again.
pub fn invalidate_all(cx: &mut App) {
    crate::query::invalidate(cx, &crate::query::QueryKey::root());
    cx.default_global::<Connection>().epoch += 1;
    cx.refresh_windows();
}

/// The state of a [`use_query`]: data from the last successful fetch, the error
/// of the last failed one, and whether a fetch is running.
#[derive(Clone, Debug)]
pub struct Query<T> {
    data: Option<T>,
    error: Option<DbError>,
    loading: bool,
}

impl<T> Query<T> {
    /// The latest data. Kept while a refetch runs, so lists do not flash empty.
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// The error of the last fetch, if it failed.
    pub fn error(&self) -> Option<&DbError> {
        self.error.as_ref()
    }

    /// Whether a fetch is running.
    pub fn is_loading(&self) -> bool {
        self.loading
    }
}

/// A query's cached state for one element.
struct QueryCell<T> {
    query: Query<T>,
    /// The (epoch, key generation) fetched last; `None` before the first fetch.
    fetched: Option<(u64, u64)>,
    fetch_task: Option<Task<()>>,
}

/// Fetch data for the calling element, like React Query's `useQuery`: the first
/// render starts `query` and returns a loading state, the result re-renders the
/// window, and later renders return the cached result until the query is
/// [`invalidate`]d (or the connection changes).
///
/// `key` names the data for invalidation; elements using the same key keep
/// their own copies but refetch together.
#[deprecated(
    since = "0.6.0",
    note = "use `query::use_query(cx, db::db_query(query_key![..], |db| ..))`; removed in 0.8"
)]
pub fn use_query<T, Build, Fetch>(
    key: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
    build: Build,
) -> Query<T>
where
    T: Clone + Send + 'static,
    Build: FnOnce(Db) -> Fetch,
    Fetch: Future<Output = rok_db::Result<T>> + Send + 'static,
{
    let key = key.into();
    let current = {
        let connection = cx.default_global::<Connection>();
        (
            connection.epoch,
            connection.generations.get(&key).copied().unwrap_or(0),
        )
    };
    let state = use_keyed_state(
        ElementId::Name(format!("rok-ui-db-query:{key}").into()),
        window,
        cx,
        || QueryCell::<T> {
            query: Query {
                data: None,
                error: None,
                loading: true,
            },
            fetched: None,
            fetch_task: None,
        },
    );
    if state.read(cx).fetched != Some(current) {
        let fetching = run(cx, build);
        let target = state.clone();
        let fetch = window.spawn(cx, async move |cx| {
            let result = fetching.await;
            cx.update(|window, cx| {
                target.update(cx, |cell| {
                    cell.query.loading = false;
                    match result {
                        Ok(data) => {
                            cell.query.data = Some(data);
                            cell.query.error = None;
                        }
                        Err(error) => cell.query.error = Some(error),
                    }
                });
                window.refresh();
            })
            .ok();
        });
        state.update(cx, |cell| {
            cell.fetched = Some(current);
            cell.query.loading = true;
            cell.fetch_task = Some(fetch);
        });
    }
    state.read(cx).query.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn calls_without_a_connection_fail_fast(cx: &mut gpui::TestAppContext) {
        let task = cx.update(|cx| run(cx, |db| async move { db.ping().await }));
        let result = cx.executor().block_test(task);
        assert_eq!(result, Err(DbError::NotConnected));
    }

    #[test]
    fn database_errors_come_back_as_failures() {
        // Nothing listens on port 1, so the lazy pool fails on first use.
        let db = runtime()
            .block_on(async {
                Db::builder()
                    .acquire_timeout(std::time::Duration::from_secs(2))
                    .connect_lazy("postgres://rok:rok@127.0.0.1:1/rok")
            })
            .expect("a lazy pool is created without connecting");
        let result = runtime()
            .block_on(start(async move { db.ping().await }))
            .expect("the task finishes");
        let error = DbError::from(result.expect_err("there is no database"));
        assert!(matches!(error, DbError::Failed(_)), "{error:?}");
    }
}
