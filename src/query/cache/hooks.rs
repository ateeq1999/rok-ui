//! The query hooks: [`use_query`], [`use_suspense_query`] and [`use_query_select`].

use std::{rc::Rc, time::Instant};

#[cfg(doc)]
use super::invalidate;
use super::{cache, start_fetch, AnyData, Entry, FetchPlan, QueryResult};
use crate::query::{QueryError, QueryKey, QueryOptions};
use crate::Cx;

/// What one call site remembers between renders.
#[derive(Default)]
struct Observer {
    key: Option<QueryKey>,
    previous: Option<AnyData>,
    /// Watches window activation for `refetch_on_window_focus`.
    focus: Option<gpui::Subscription>,
}

pub(super) fn downcast<T: 'static>(data: &AnyData) -> Option<Rc<T>> {
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

/// [`use_query`] for [`Suspense`](crate::query::Suspense) content: the data, or a
/// [`Suspend`](crate::query::Suspend) to return with `?`.
///
/// # Errors
///
/// As [`QueryResult::suspend`].
#[track_caller]
pub fn use_suspense_query<T: Send + 'static>(
    cx: &mut Cx,
    options: QueryOptions<T>,
) -> Result<Rc<T>, crate::query::Suspend> {
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
