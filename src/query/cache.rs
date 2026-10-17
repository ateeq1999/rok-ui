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

mod client;
mod hooks;
mod result;

pub use client::{
    cancel_queries, ensure_query_data, fetch_query, fetching_count, get_query_data, invalidate,
    prefetch_query, queries, reset_queries, set_query_data, update_query_data, QueryInfo,
};
use hooks::downcast;
pub use hooks::{use_query, use_query_select, use_suspense_query};
pub use result::{QueryResult, QueryState};

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
