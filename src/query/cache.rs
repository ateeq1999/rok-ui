//! The query cache and the [`use_query`] hook.

use std::{
    any::Any,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use futures::channel::oneshot;
use gpui::{App, Global, Task};

use super::{options::Fetcher, services::task_cx, QueryError, QueryKey, QueryOptions};
use crate::Cx;

/// Data shared between the readers of one key, type-erased.
pub(crate) type AnyData = Rc<dyn Any>;

struct Fetch {
    /// Delivers the result to the UI thread. Dropping it ignores the result.
    _task: Task<()>,
    abort: tokio::task::AbortHandle,
}

pub(crate) struct Entry {
    pub(crate) data: Option<AnyData>,
    error: Option<QueryError>,
    updated_at: Option<Instant>,
    /// Bumped by `invalidate`; a fetch that started before the bump does not make the data
    /// fresh.
    version: u64,
    fresh_version: Option<u64>,
    fetch: Option<Fetch>,
    /// Identifies the running fetch, so a cancelled one cannot write its result.
    generation: u64,
    last_used: Instant,
    gc_time: Duration,
    waiters: Vec<oneshot::Sender<()>>,
    interval: Option<Task<()>>,
}

impl Entry {
    fn new(gc_time: Duration) -> Self {
        Self {
            data: None,
            error: None,
            updated_at: None,
            version: 0,
            fresh_version: None,
            fetch: None,
            generation: 0,
            last_used: Instant::now(),
            gc_time,
            waiters: Vec::new(),
            interval: None,
        }
    }

    fn is_invalidated(&self) -> bool {
        self.fresh_version != Some(self.version)
    }

    fn is_stale(&self, stale_time: Duration) -> bool {
        self.is_invalidated()
            || self
                .updated_at
                .is_none_or(|updated_at| updated_at.elapsed() >= stale_time)
    }

    pub(crate) fn set_data(&mut self, data: AnyData) {
        self.data = Some(data);
        self.error = None;
        self.updated_at = Some(Instant::now());
        self.fresh_version = Some(self.version);
    }

    fn cancel(&mut self) {
        if let Some(fetch) = self.fetch.take() {
            fetch.abort.abort();
        }
        self.generation += 1;
        // Waiters see their channel close and report a cancellation.
        self.waiters.clear();
    }
}

/// Every query's data, error and fetch state. One per app.
#[derive(Default)]
pub(crate) struct QueryCache {
    pub(crate) entries: HashMap<QueryKey, Entry>,
}

impl Global for QueryCache {}

pub(crate) fn cache(cx: &mut App) -> &mut QueryCache {
    cx.default_global::<QueryCache>()
}

/// Drop entries nobody has read for longer than their `gc_time`.
fn collect_garbage(cx: &mut App) {
    cache(cx).entries.retain(|_, entry| {
        entry.fetch.is_some()
            || !entry.waiters.is_empty()
            || entry.last_used.elapsed() < entry.gc_time
    });
}

/// How a fetch is run: the fetcher and its retry policy.
pub(crate) struct FetchPlan<T> {
    pub(crate) fetch: Fetcher<T>,
    pub(crate) retry: u32,
    pub(crate) retry_delay: Duration,
    pub(crate) refetch_interval: Option<Duration>,
    pub(crate) gc_time: Duration,
}

impl<T> From<&QueryOptions<T>> for FetchPlan<T> {
    fn from(options: &QueryOptions<T>) -> Self {
        Self {
            fetch: options.fetch.clone(),
            retry: options.retry,
            retry_delay: options.retry_delay,
            refetch_interval: options.refetch_interval,
            gc_time: options.gc_time,
        }
    }
}

/// Start fetching `key` unless a fetch is already running (concurrent readers share it).
pub(crate) fn start_fetch<T: Send + 'static>(cx: &mut App, key: &QueryKey, plan: &FetchPlan<T>) {
    let services = task_cx(cx);
    let entry = cache(cx)
        .entries
        .entry(key.clone())
        .or_insert_with(|| Entry::new(plan.gc_time));
    if entry.fetch.is_some() {
        return;
    }
    entry.generation += 1;
    let generation = entry.generation;
    let version = entry.version;

    let fetch = plan.fetch.clone();
    let (retries, mut delay) = (plan.retry, plan.retry_delay);
    let running = crate::runtime::spawn(async move {
        let mut attempt = 0;
        loop {
            match fetch(services.clone()).await {
                Err(_) if attempt < retries => {
                    attempt += 1;
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(30));
                }
                result => return result,
            }
        }
    });
    let abort = running.abort_handle();
    let finished_key = key.clone();
    let refetch_interval = plan.refetch_interval;
    let task = cx.spawn(async move |cx| {
        let result = running
            .await
            .unwrap_or_else(|_| Err(QueryError::cancelled()));
        cx.update(|cx| {
            finish(
                cx,
                &finished_key,
                generation,
                version,
                result,
                refetch_interval,
            );
        })
        .ok();
    });
    if let Some(entry) = cache(cx).entries.get_mut(key) {
        entry.fetch = Some(Fetch { _task: task, abort });
    }
}

/// Store a fetch result, wake waiters and re-render.
fn finish<T: 'static>(
    cx: &mut App,
    key: &QueryKey,
    generation: u64,
    version: u64,
    result: Result<T, QueryError>,
    refetch_interval: Option<Duration>,
) {
    let Some(entry) = cache(cx).entries.get_mut(key) else {
        return;
    };
    if entry.generation != generation {
        return;
    }
    entry.fetch = None;
    match result {
        Ok(data) => {
            entry.data = Some(Rc::new(data));
            entry.error = None;
            entry.updated_at = Some(Instant::now());
            // An invalidation while the fetch ran leaves the data stale.
            entry.fresh_version = Some(version);
        }
        Err(error) => {
            entry.error = Some(error);
            entry.updated_at = Some(Instant::now());
        }
    }
    for waiter in entry.waiters.drain(..) {
        waiter.send(()).ok();
    }
    let interval = refetch_interval.map(|interval| schedule_refetch(cx, key.clone(), interval));
    if let Some(entry) = cache(cx).entries.get_mut(key) {
        entry.interval = interval;
    }
    collect_garbage(cx);
    cx.refresh_windows();
}

/// After `interval`, mark `key` stale so readers refetch it, if anyone still reads it.
fn schedule_refetch(cx: &mut App, key: QueryKey, interval: Duration) -> Task<()> {
    cx.spawn(async move |cx| {
        cx.background_executor().timer(interval).await;
        cx.update(|cx| {
            let Some(entry) = cache(cx).entries.get_mut(&key) else {
                return;
            };
            if entry.last_used.elapsed() <= interval * 2 {
                entry.version += 1;
                cx.refresh_windows();
            }
        })
        .ok();
    })
}

/// The state of a query for one reader, returned by [`use_query`].
pub struct QueryResult<T> {
    data: Option<Rc<T>>,
    error: Option<QueryError>,
    is_fetching: bool,
    is_placeholder: bool,
    is_stale: bool,
    updated_at: Option<Instant>,
}

/// A query's status, for `match`ing in a component.
#[derive(Debug)]
pub enum QueryState<'a, T> {
    /// No data yet; the first fetch is running (or the query is disabled).
    Pending,
    /// No data, and the last fetch failed.
    Error(&'a QueryError),
    /// Data, possibly stale or from a placeholder while a fetch runs.
    Success(&'a T),
}

impl<T> QueryResult<T> {
    /// The status: pending, an error, or data.
    #[must_use]
    pub fn state(&self) -> QueryState<'_, T> {
        match (&self.data, &self.error) {
            (Some(data), _) => QueryState::Success(data),
            (None, Some(error)) => QueryState::Error(error),
            (None, None) => QueryState::Pending,
        }
    }

    /// The data, if there is any. Kept while a refetch runs, so lists do not flash empty.
    #[must_use]
    pub fn data(&self) -> Option<&T> {
        self.data.as_deref()
    }

    /// The data as a shared handle.
    #[must_use]
    pub fn data_rc(&self) -> Option<Rc<T>> {
        self.data.clone()
    }

    /// The error of the last fetch, if it failed.
    #[must_use]
    pub fn error(&self) -> Option<&QueryError> {
        self.error.as_ref()
    }

    /// No data and no error yet.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.data.is_none() && self.error.is_none()
    }

    /// Whether data is available.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.data.is_some()
    }

    /// Whether the last fetch failed.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Whether a fetch is running (the first one or a background refetch).
    #[must_use]
    pub fn is_fetching(&self) -> bool {
        self.is_fetching
    }

    /// Pending with a fetch running: show a spinner.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.is_pending() && self.is_fetching
    }

    /// Whether the data is placeholder or previous-key data rather than this key's.
    #[must_use]
    pub fn is_placeholder_data(&self) -> bool {
        self.is_placeholder
    }

    /// Whether the data is older than the query's `stale_time` or was invalidated.
    #[must_use]
    pub fn is_stale(&self) -> bool {
        self.is_stale
    }

    /// When the data or error was last stored.
    #[must_use]
    pub fn updated_at(&self) -> Option<Instant> {
        self.updated_at
    }

    /// The data, or why there is none, for `?` inside [`Suspense`](super::Suspense).
    ///
    /// # Errors
    ///
    /// [`Suspend::Pending`](super::Suspend::Pending) without data and error, and
    /// [`Suspend::Failed`](super::Suspend::Failed) when the fetch failed and there is no data.
    pub fn suspend(self) -> Result<Rc<T>, super::Suspend> {
        match (self.data, self.error) {
            (Some(data), _) => Ok(data),
            (None, Some(error)) => Err(super::Suspend::Failed(error)),
            (None, None) => Err(super::Suspend::Pending),
        }
    }
}

impl<T> Clone for QueryResult<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            error: self.error.clone(),
            is_fetching: self.is_fetching,
            is_placeholder: self.is_placeholder,
            is_stale: self.is_stale,
            updated_at: self.updated_at,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for QueryResult<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QueryResult")
            .field("data", &self.data)
            .field("error", &self.error)
            .field("is_fetching", &self.is_fetching)
            .finish_non_exhaustive()
    }
}

/// What one call site remembers between renders.
#[derive(Default)]
struct Observer {
    key: Option<QueryKey>,
    previous: Option<AnyData>,
    /// Watches window activation for `refetch_on_window_focus`.
    focus: Option<gpui::Subscription>,
}

fn downcast<T: 'static>(data: &AnyData) -> Option<Rc<T>> {
    Rc::downcast::<T>(data.clone()).ok()
}

/// Read a query, fetching it when needed, like TanStack Query's `useQuery`.
///
/// - The first reader of a key starts a fetch on the shared runtime; readers of the same key
///   share it.
/// - Data is cached per key. A reader that starts reading (first render, or a new key) refetches
///   in the background when the data is stale; the stale data shows meanwhile.
/// - [`invalidate`] makes matching queries stale and current readers refetch.
///
/// ```no_run
/// # use rok_ui::{prelude::*, query::{self, QueryOptions, QueryState}, query_key};
/// fn greeting_query() -> QueryOptions<String> {
///     QueryOptions::new(query_key!["greeting"], |_| async { Ok::<_, std::io::Error>("Hello".to_string()) })
/// }
///
/// #[component]
/// fn Greeting(cx: &mut Cx) -> impl IntoElement {
///     let greeting = query::use_query(cx, greeting_query());
///     match greeting.state() {
///         QueryState::Pending => div().child("Loading"),
///         QueryState::Error(error) => div().child(error.to_string()),
///         QueryState::Success(text) => div().child(text.clone()),
///     }
/// }
/// ```
#[track_caller]
pub fn use_query<T: Send + 'static>(cx: &mut Cx, options: QueryOptions<T>) -> QueryResult<T> {
    let observer = cx.window.use_state(cx.app, |_, _| Observer::default());
    let mounted = observer.read(cx.app).key.as_ref() == Some(&options.key);
    if options.refetch_on_window_focus && observer.read(cx.app).focus.is_none() {
        let Cx { window, app } = cx;
        observer.update(*app, |observer, observer_cx| {
            observer.focus = Some(observer_cx.observe_window_activation(
                window,
                |observer, window, cx| {
                    let Some(key) = observer.key.clone().filter(|_| window.is_window_active())
                    else {
                        return;
                    };
                    if let Some(entry) = cache(cx).entries.get_mut(&key) {
                        entry.version += 1;
                    }
                    cx.refresh_windows();
                },
            ));
        });
    }
    let plan = FetchPlan::from(&options);
    let QueryOptions {
        key,
        stale_time,
        gc_time,
        enabled,
        initial_data,
        placeholder_data,
        keep_previous_data,
        ..
    } = options;

    let entry = cache(cx).entries.entry(key.clone()).or_insert_with(|| {
        let mut entry = Entry::new(gc_time);
        if let Some(data) = initial_data {
            entry.set_data(Rc::new(data));
        }
        entry
    });
    entry.last_used = Instant::now();
    entry.gc_time = gc_time;
    let never_fetched = entry.data.is_none() && entry.error.is_none();
    let needs_fetch = enabled
        && entry.fetch.is_none()
        && (never_fetched
            || entry.is_invalidated()
            || (!mounted && (entry.error.is_some() || entry.is_stale(stale_time))));
    if needs_fetch {
        start_fetch(cx, &key, &plan);
    }

    let entry = &cache(cx).entries[&key];
    let own_data = entry.data.as_ref().and_then(downcast::<T>);
    let mut result = QueryResult {
        is_placeholder: false,
        is_stale: entry.is_stale(stale_time),
        is_fetching: entry.fetch.is_some(),
        error: entry.error.clone(),
        updated_at: entry.updated_at,
        data: own_data,
    };
    if entry.data.is_some() && result.data.is_none() {
        result.error = Some(QueryError::msg(format!(
            "query {key} is cached with a different data type"
        )));
    }
    if result.data.is_none() {
        let previous = observer.read(cx.app).previous.clone();
        if let Some(previous) = previous
            .filter(|_| keep_previous_data)
            .as_ref()
            .and_then(downcast::<T>)
        {
            result.data = Some(previous);
            result.is_placeholder = true;
        } else if let Some(placeholder) = placeholder_data {
            result.data = Some(Rc::new(placeholder));
            result.is_placeholder = true;
        }
    }
    let current = cache(cx).entries[&key].data.clone();
    observer.update(cx.app, |observer, _| {
        observer.key = Some(key);
        if current.is_some() {
            observer.previous = current;
        }
    });
    result
}

/// [`use_query`] for [`Suspense`](super::Suspense) content: the data, or a
/// [`Suspend`](super::Suspend) to return with `?`.
///
/// # Errors
///
/// As [`QueryResult::suspend`].
#[track_caller]
pub fn use_suspense_query<T: Send + 'static>(
    cx: &mut Cx,
    options: QueryOptions<T>,
) -> Result<Rc<T>, super::Suspend> {
    use_query(cx, options).suspend()
}

/// The derived value one call site keeps, and the data it was derived from.
struct Selection<T, U> {
    source: Rc<T>,
    selected: Rc<U>,
}

/// [`use_query`] that reads part of the data, like TanStack Query's `select` option: `select`
/// derives a value from the cached data, and the cache keeps the data as fetched, so other
/// readers of the key see all of it.
///
/// The derived value is remembered per call site and derived again only when the cached data
/// changes (a fetch, `set_query_data`), not on every render, so `select` can sort, filter or
/// count large lists.
///
/// ```no_run
/// # use rok_ui::{prelude::*, query::{self, QueryOptions}, query_key};
/// # #[derive(Clone)] struct Todo { done: bool }
/// fn todos_query() -> QueryOptions<Vec<Todo>> {
///     QueryOptions::new(query_key!["todos"], |_| async { Ok::<_, std::io::Error>(Vec::new()) })
/// }
///
/// #[component]
/// fn Remaining(cx: &mut Cx) -> impl IntoElement {
///     let remaining = query::use_query_select(cx, todos_query(), |todos| {
///         todos.iter().filter(|todo| !todo.done).count()
///     });
///     div().child(format!("{} left", remaining.data().copied().unwrap_or(0)))
/// }
/// ```
#[track_caller]
pub fn use_query_select<T, U, Select>(
    cx: &mut Cx,
    options: QueryOptions<T>,
    select: Select,
) -> QueryResult<U>
where
    T: Send + 'static,
    U: 'static,
    Select: FnOnce(&T) -> U,
{
    let key = gpui::ElementId::NamedChild(
        Box::new(gpui::ElementId::CodeLocation(
            *std::panic::Location::caller(),
        )),
        "rok-ui-query-select".into(),
    );
    let result = use_query(cx, options);
    let memo = cx
        .window
        .use_keyed_state(key, cx.app, |_, _| None::<Selection<T, U>>);
    let data = result.data.as_ref().map(|source| {
        let remembered = memo
            .read(cx.app)
            .as_ref()
            .filter(|memo| Rc::ptr_eq(&memo.source, source))
            .map(|memo| memo.selected.clone());
        remembered.unwrap_or_else(|| {
            let selected = Rc::new(select(source));
            memo.update(cx.app, |memo, _| {
                *memo = Some(Selection {
                    source: source.clone(),
                    selected: selected.clone(),
                });
            });
            selected
        })
    });
    QueryResult {
        data,
        error: result.error,
        is_fetching: result.is_fetching,
        is_placeholder: result.is_placeholder,
        is_stale: result.is_stale,
        updated_at: result.updated_at,
    }
}

/// Mark every query whose key starts with `prefix` stale. Readers on screen refetch; others
/// refetch when next read. A fetch already running for a matching key does not count as fresh.
pub fn invalidate(cx: &mut App, prefix: &QueryKey) {
    for (key, entry) in &mut cache(cx).entries {
        if key.starts_with(prefix) {
            entry.version += 1;
        }
    }
    cx.refresh_windows();
}

/// Store `data` for `key` as if it was just fetched (after a write that returns the new
/// value, for example).
pub fn set_query_data<T: 'static>(cx: &mut App, key: &QueryKey, data: T) {
    cache(cx)
        .entries
        .entry(key.clone())
        .or_insert_with(|| Entry::new(Duration::from_secs(5 * 60)))
        .set_data(Rc::new(data));
    cx.refresh_windows();
}

/// The cached data for `key`, if there is data of type `T`.
#[must_use]
pub fn get_query_data<T: 'static>(cx: &App, key: &QueryKey) -> Option<Rc<T>> {
    cx.try_global::<QueryCache>()?
        .entries
        .get(key)?
        .data
        .as_ref()
        .and_then(downcast::<T>)
}

/// Change the cached data for `key` in place. Returns whether there was data of type `T`.
pub fn update_query_data<T: Clone + 'static>(
    cx: &mut App,
    key: &QueryKey,
    update: impl FnOnce(&mut T),
) -> bool {
    let Some(mut data) = get_query_data::<T>(cx, key) else {
        return false;
    };
    update(Rc::make_mut(&mut data));
    set_query_data(cx, key, Rc::unwrap_or_clone(data));
    true
}

/// Fetch a query unless its cached data is fresh, and resolve with the data. Concurrent calls
/// and readers share one fetch. For loaders and code outside components.
pub fn fetch_query<T: Send + 'static>(
    cx: &mut App,
    options: &QueryOptions<T>,
) -> Task<Result<Rc<T>, QueryError>> {
    let fresh = cache(cx)
        .entries
        .get(&options.key)
        .filter(|entry| entry.fetch.is_none() && !entry.is_stale(options.stale_time))
        .and_then(|entry| entry.data.as_ref())
        .and_then(downcast::<T>);
    if let Some(data) = fresh {
        return Task::ready(Ok(data));
    }
    wait_for_fetch(cx, options)
}

/// The cached data for a query whatever its age, or [`fetch_query`] when there is none. The
/// usual call in a route loader.
pub fn ensure_query_data<T: Send + 'static>(
    cx: &mut App,
    options: &QueryOptions<T>,
) -> Task<Result<Rc<T>, QueryError>> {
    match get_query_data::<T>(cx, &options.key) {
        Some(data) => Task::ready(Ok(data)),
        None => wait_for_fetch(cx, options),
    }
}

/// Start fetching a query in the background so it is cached before it is shown.
pub fn prefetch_query<T: Send + 'static>(cx: &mut App, options: &QueryOptions<T>) {
    fetch_query(cx, options).detach();
}

fn wait_for_fetch<T: Send + 'static>(
    cx: &mut App,
    options: &QueryOptions<T>,
) -> Task<Result<Rc<T>, QueryError>> {
    start_fetch(cx, &options.key, &FetchPlan::from(options));
    let (sender, receiver) = oneshot::channel();
    let key = options.key.clone();
    if let Some(entry) = cache(cx).entries.get_mut(&key) {
        entry.last_used = Instant::now();
        entry.waiters.push(sender);
    }
    cx.spawn(async move |cx| {
        receiver.await.map_err(|_| QueryError::cancelled())?;
        cx.update(|cx| {
            let entry = cache(cx).entries.get(&key);
            match entry
                .and_then(|entry| entry.data.as_ref())
                .and_then(downcast::<T>)
            {
                Some(data) if entry.is_some_and(|entry| entry.error.is_none()) => Ok(data),
                _ => Err(entry
                    .and_then(|entry| entry.error.clone())
                    .unwrap_or_else(QueryError::cancelled)),
            }
        })
        .map_err(|_| QueryError::cancelled())?
    })
}

/// Stop the fetches of every query whose key starts with `prefix`. Their results are dropped
/// and [`fetch_query`] callers get an error.
pub fn cancel_queries(cx: &mut App, prefix: &QueryKey) {
    for (key, entry) in &mut cache(cx).entries {
        if key.starts_with(prefix) {
            entry.cancel();
        }
    }
}

/// Remove every query whose key starts with `prefix` from the cache, cancelling their fetches.
/// Readers on screen fetch again from scratch.
pub fn reset_queries(cx: &mut App, prefix: &QueryKey) {
    cache(cx).entries.retain(|key, entry| {
        let matches = key.starts_with(prefix);
        if matches {
            entry.cancel();
        }
        !matches
    });
    cx.refresh_windows();
}

/// How many queries whose key starts with `prefix` are fetching.
#[must_use]
pub fn fetching_count(cx: &App, prefix: &QueryKey) -> usize {
    cx.try_global::<QueryCache>().map_or(0, |cache| {
        cache
            .entries
            .iter()
            .filter(|(key, entry)| key.starts_with(prefix) && entry.fetch.is_some())
            .count()
    })
}

/// One cached query, for devtools and debugging.
#[derive(Clone, Debug)]
pub struct QueryInfo {
    /// The key.
    pub key: QueryKey,
    /// Whether there is data.
    pub has_data: bool,
    /// The last error, if the last fetch failed.
    pub error: Option<QueryError>,
    /// Whether a fetch is running.
    pub is_fetching: bool,
    /// Whether it was invalidated since the data was fetched.
    pub is_invalidated: bool,
    /// Time since the data or error was stored.
    pub age: Option<Duration>,
}

/// Every cached query, sorted by key.
#[must_use]
pub fn queries(cx: &App) -> Vec<QueryInfo> {
    let Some(cache) = cx.try_global::<QueryCache>() else {
        return Vec::new();
    };
    let mut infos: Vec<QueryInfo> = cache
        .entries
        .iter()
        .map(|(key, entry)| QueryInfo {
            key: key.clone(),
            has_data: entry.data.is_some(),
            error: entry.error.clone(),
            is_fetching: entry.fetch.is_some(),
            is_invalidated: entry.data.is_some() && entry.is_invalidated(),
            age: entry.updated_at.map(|updated_at| updated_at.elapsed()),
        })
        .collect();
    infos.sort_by_key(|info| info.key.to_string());
    infos
}
