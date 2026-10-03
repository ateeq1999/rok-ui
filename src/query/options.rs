//! [`QueryOptions`]: what to fetch and how to cache it.

use std::{fmt, future::Future, pin::Pin, sync::Arc, time::Duration};

use super::{QueryError, QueryKey, TaskCx};

/// A boxed `Send` future.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub(crate) type Fetcher<T> = Arc<dyn Fn(TaskCx) -> BoxFuture<Result<T, QueryError>> + Send + Sync>;

/// Everything about one query: its key, how to fetch it, and how long to cache it.
///
/// Options are plain values, built by functions, so loaders, components and tests share them
/// (TanStack's `queryOptions`):
///
/// ```
/// use std::time::Duration;
/// use rok_ui::{query::QueryOptions, query_key};
///
/// #[derive(Clone)]
/// struct Note { title: String }
///
/// fn note_query(id: u64) -> QueryOptions<Note> {
///     QueryOptions::new(query_key!["notes", id], move |_cx| async move {
///         Ok::<_, std::io::Error>(Note { title: format!("Note {id}") })
///     })
///     .stale_time(Duration::from_secs(30))
/// }
/// # let _ = note_query(1);
/// ```
pub struct QueryOptions<T> {
    pub(crate) key: QueryKey,
    pub(crate) fetch: Fetcher<T>,
    pub(crate) stale_time: Duration,
    pub(crate) gc_time: Duration,
    pub(crate) retry: u32,
    pub(crate) retry_delay: Duration,
    pub(crate) enabled: bool,
    pub(crate) initial_data: Option<T>,
    pub(crate) placeholder_data: Option<T>,
    pub(crate) keep_previous_data: bool,
    pub(crate) refetch_interval: Option<Duration>,
    pub(crate) refetch_on_window_focus: bool,
}

impl<T: Send + 'static> QueryOptions<T> {
    /// A query for `key` that runs `fetch` on the shared background runtime.
    ///
    /// `fetch` gets a [`TaskCx`] with the values registered with
    /// [`provide`](super::provide). Its error can be any `std::error::Error` (or a
    /// [`QueryError`]).
    pub fn new<F, Fut, E>(key: QueryKey, fetch: F) -> Self
    where
        F: Fn(TaskCx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        E: Into<QueryError>,
    {
        Self {
            key,
            fetch: Arc::new(move |cx| {
                let future = fetch(cx);
                Box::pin(async move { future.await.map_err(Into::into) })
            }),
            stale_time: Duration::ZERO,
            gc_time: Duration::from_secs(5 * 60),
            retry: 0,
            retry_delay: Duration::from_millis(500),
            enabled: true,
            initial_data: None,
            placeholder_data: None,
            keep_previous_data: false,
            refetch_interval: None,
            refetch_on_window_focus: false,
        }
    }
}

impl<T> QueryOptions<T> {
    /// The key.
    #[must_use]
    pub fn key(&self) -> &QueryKey {
        &self.key
    }

    /// How long fetched data counts as fresh. A component that starts reading stale data
    /// refetches it in the background; fresh data is served from the cache. Default: zero
    /// (always stale, so every new reader refetches).
    #[must_use]
    pub fn stale_time(mut self, stale_time: Duration) -> Self {
        self.stale_time = stale_time;
        self
    }

    /// How long data nobody reads stays cached. Default: five minutes.
    #[must_use]
    pub fn gc_time(mut self, gc_time: Duration) -> Self {
        self.gc_time = gc_time;
        self
    }

    /// Retry a failed fetch up to `retries` times, waiting `retry_delay` and doubling it each
    /// time (at most 30 seconds). Default: no retries.
    #[must_use]
    pub fn retry(mut self, retries: u32) -> Self {
        self.retry = retries;
        self
    }

    /// The wait before the first retry. Default: 500 ms.
    #[must_use]
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Whether the query may fetch. A disabled query only reads the cache. Default: enabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Data to seed the cache with when the key has none. It counts as fetched now, so it is
    /// served until it goes stale.
    #[must_use]
    pub fn initial_data(mut self, data: T) -> Self {
        self.initial_data = Some(data);
        self
    }

    /// Data to show while the first fetch runs. It is not cached.
    #[must_use]
    pub fn placeholder_data(mut self, data: T) -> Self {
        self.placeholder_data = Some(data);
        self
    }

    /// When the key changes (a new page, a new search), keep showing the previous key's data
    /// until the new data arrives. Default: off.
    #[must_use]
    pub fn keep_previous_data(mut self, keep: bool) -> Self {
        self.keep_previous_data = keep;
        self
    }

    /// Mark the data stale when the reading window becomes active again, so it refetches (the
    /// user may have changed it elsewhere). Default: off.
    #[must_use]
    pub fn refetch_on_window_focus(mut self, refetch: bool) -> Self {
        self.refetch_on_window_focus = refetch;
        self
    }

    /// Refetch this often while a component reads the query.
    #[must_use]
    pub fn refetch_interval(mut self, interval: Duration) -> Self {
        self.refetch_interval = Some(interval);
        self
    }
}

impl<T: Clone> Clone for QueryOptions<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            fetch: self.fetch.clone(),
            stale_time: self.stale_time,
            gc_time: self.gc_time,
            retry: self.retry,
            retry_delay: self.retry_delay,
            enabled: self.enabled,
            initial_data: self.initial_data.clone(),
            placeholder_data: self.placeholder_data.clone(),
            keep_previous_data: self.keep_previous_data,
            refetch_interval: self.refetch_interval,
            refetch_on_window_focus: self.refetch_on_window_focus,
        }
    }
}

impl<T> fmt::Debug for QueryOptions<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryOptions")
            .field("key", &self.key)
            .field("stale_time", &self.stale_time)
            .field("gc_time", &self.gc_time)
            .field("retry", &self.retry)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}
