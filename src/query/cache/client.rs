//! Reading and changing the cache from outside a component: invalidating, setting and
//! fetching data, cancelling and listing queries.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use futures::channel::oneshot;
use gpui::{App, Task};

use super::{cache, downcast, start_fetch, Entry, FetchPlan, QueryCache};
use crate::query::{QueryError, QueryKey, QueryOptions};

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
