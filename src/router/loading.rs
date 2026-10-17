//! Route loaders: starting a route's data before it renders, on navigation and on preload,
//! and the pending state of what they started.

#[cfg(feature = "query")]
use std::cell::Cell;
use std::{collections::HashMap, rc::Rc};

#[cfg(feature = "query")]
use gpui::AnyWindowHandle;
use gpui::{App, Global, SharedString, Window};

#[cfg(feature = "query")]
use super::history::CURRENT_WINDOW;
use super::{location, with_window, Location, Pattern, RouteMatch, Router};

/// Run `loader` unless it already ran for this exact location.
pub(super) fn run_loader_once(route: &RouteMatch, loader: &Loader, cx: &mut App) {
    let href = route.location.href();
    let last = cx
        .default_global::<Loaders>()
        .last_loaded
        .insert(route.pattern.clone(), href.clone());
    if last.as_ref() == Some(&href) {
        return;
    }
    #[cfg(feature = "query")]
    let fetching_before = fetching_keys(cx);
    loader(route, cx);
    #[cfg(feature = "query")]
    {
        let started: Vec<_> = fetching_keys(cx)
            .into_iter()
            .filter(|key| !fetching_before.contains(key))
            .collect();
        let window = CURRENT_WINDOW.with(Cell::get);
        let load = cx
            .default_global::<Loaders>()
            .started
            .entry(window)
            .or_insert_with(|| WindowLoad::new(href.clone()));
        if !started.is_empty() && load.since.is_none() {
            load.since = Some(std::time::Instant::now());
        }
        load.keys.extend(started);
    }
}

#[cfg(feature = "query")]
pub(super) fn fetching_keys(cx: &App) -> Vec<crate::query::QueryKey> {
    crate::query::queries(cx)
        .into_iter()
        .filter(|query| query.is_fetching)
        .map(|query| query.key)
        .collect()
}

/// When this window shows a new location, cancel the fetches the previous location's loader
/// started and that are still running (the user navigated away before they finished).
#[cfg(feature = "query")]
pub(super) fn cancel_stale_loads(location: &Location, cx: &mut App) {
    let href = location.href();
    let window = CURRENT_WINDOW.with(Cell::get);
    let previous = {
        let loaders = cx.default_global::<Loaders>();
        match loaders.started.get_mut(&window) {
            Some(load) if load.shown == href => return,
            Some(load) => std::mem::replace(load, WindowLoad::new(href)).keys,
            None => {
                loaders.started.insert(window, WindowLoad::new(href));
                return;
            }
        }
    };
    let still_fetching = fetching_keys(cx);
    for key in previous {
        if still_fetching.contains(&key) {
            crate::query::cancel_queries(cx, &key);
        }
    }
}

/// Starts loading a route's data before it renders (prefetching queries).
pub type Loader = Rc<dyn Fn(&RouteMatch, &mut App)>;

/// Loaders seen by routers, for link preloading, and the location each one last loaded.
#[derive(Default)]
pub(super) struct Loaders {
    registered: Vec<(Pattern, Loader)>,
    last_loaded: HashMap<SharedString, SharedString>,
    /// Per window (or for the app): the location shown last, and the query fetches its loader
    /// started, cancelled when the location changes.
    #[cfg(feature = "query")]
    started: HashMap<Option<AnyWindowHandle>, WindowLoad>,
}

impl Global for Loaders {}

/// What one window's current location loaded.
#[cfg(feature = "query")]
struct WindowLoad {
    /// The location shown.
    shown: SharedString,
    /// The query fetches its loaders started.
    keys: Vec<crate::query::QueryKey>,
    /// When its loaders started their first fetch.
    since: Option<std::time::Instant>,
}

#[cfg(feature = "query")]
impl WindowLoad {
    fn new(shown: SharedString) -> Self {
        Self {
            shown,
            keys: Vec::new(),
            since: None,
        }
    }
}

/// Whether the current location's loaders are still fetching, from [`load_state`].
#[cfg(feature = "query")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoadState {
    /// A query fetch the current location's loaders started is still running.
    pub is_loading: bool,
    /// When those loaders started fetching; `None` when they started nothing (their data
    /// was fresh in the cache).
    pub since: Option<std::time::Instant>,
}

/// Whether the loaders of the current location are still fetching, for a progress bar or a
/// pending screen: in the window whose router is rendering (or [`with_window`]'s), else the
/// active window. Only fetches the loaders started through the query cache count. For a
/// progress bar that does not flash on fast loads, use [`use_pending`].
#[cfg(feature = "query")]
#[must_use]
pub fn load_state(cx: &App) -> LoadState {
    let window = CURRENT_WINDOW
        .with(Cell::get)
        .or_else(|| cx.active_window());
    load_state_in(window, cx)
}

#[cfg(feature = "query")]
fn load_state_in(window: Option<AnyWindowHandle>, cx: &App) -> LoadState {
    let Some(load) = cx
        .try_global::<Loaders>()
        .and_then(|loaders| loaders.started.get(&window))
    else {
        return LoadState::default();
    };
    let fetching = fetching_keys(cx);
    LoadState {
        is_loading: load.keys.iter().any(|key| fetching.contains(key)),
        since: load.since,
    }
}

/// When one [`use_pending`] call site started showing, and the timer that re-renders it.
#[cfg(feature = "query")]
#[derive(Default)]
struct Pending {
    shown_at: Option<std::time::Instant>,
    timer: Option<gpui::Task<()>>,
}

/// Whether to show a pending indicator for the current location's loaders, like TanStack
/// Router's `pendingMs` and `pendingMinMs`: it turns on once loading has taken `delay`, so fast
/// loads never flash it, and once on it stays on for at least `min`, so it never blinks.
///
/// ```no_run
/// # use std::time::Duration;
/// # use rok_ui::{prelude::*, components::Progress, router};
/// #[component]
/// fn LoadingBar(cx: &mut Cx) -> impl IntoElement {
///     let pending = router::use_pending(cx, Duration::from_millis(300), Duration::from_millis(500));
///     div().when(pending, |bar| bar.child(Progress::new(60.)))
/// }
/// ```
#[cfg(feature = "query")]
#[track_caller]
pub fn use_pending(
    cx: &mut crate::Cx,
    delay: std::time::Duration,
    min: std::time::Duration,
) -> bool {
    use std::time::Instant;

    let state = cx.window.use_state(cx.app, |_, _| Pending::default());
    let load = load_state_in(Some(cx.window.window_handle()), cx.app);
    let now = Instant::now();
    let shown_at = state.read(cx.app).shown_at;
    // (whether to show, when to look again)
    let (show, wake) = match (load.is_loading, load.since, shown_at) {
        (true, _, Some(_)) => (true, None),
        (true, Some(since), None) if now >= since + delay => (true, None),
        (true, Some(since), None) => (false, Some(since + delay)),
        (false, _, Some(shown)) if now < shown + min => (true, Some(shown + min)),
        _ => (false, None),
    };
    let window = cx.window.window_handle();
    state.update(cx.app, |state, cx| {
        state.shown_at = match (show, state.shown_at) {
            (true, None) => Some(now),
            (true, shown) => shown,
            (false, _) => None,
        };
        state.timer = wake.map(|at| {
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(at.saturating_duration_since(Instant::now()))
                    .await;
                window.update(cx, |_, window, _| window.refresh()).ok();
            })
        });
    });
    show
}

/// Links preloaded because they were on screen, and the location they were seen on.
#[derive(Default)]
struct VisiblePreloads {
    location: SharedString,
    done: std::collections::HashSet<SharedString>,
}

impl Global for VisiblePreloads {}

/// Preload `target` once per location the window shows, after this frame is drawn.
pub(super) fn preload_once_visible(target: SharedString, window: &Window, cx: &mut App) {
    let shown = with_window(window.window_handle(), || location(cx).href());
    let seen = cx.default_global::<VisiblePreloads>();
    if seen.location != shown {
        seen.location = shown;
        seen.done.clear();
    }
    if seen.done.insert(target.clone()) {
        cx.defer(move |cx| preload(&target, cx));
    }
}

/// Run the loaders of routes matching `path` (a link the user is about to follow), so its data
/// starts loading early. `Link::preload(true)` calls this on hover.
pub fn preload(path: &str, cx: &mut App) {
    let location = Location::parse(path);
    let matching: Vec<(RouteMatch, Loader)> = cx
        .try_global::<Loaders>()
        .map(|loaders| {
            loaders
                .registered
                .iter()
                .filter_map(|(pattern, loader)| {
                    let (params, _) = pattern.matches(location.path())?;
                    Some((
                        RouteMatch {
                            pattern: pattern.source.clone(),
                            params,
                            location: location.clone(),
                        },
                        loader.clone(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    for (route, loader) in matching {
        loader(&route, cx);
    }
}

impl Router {
    /// Make this router's loaders available to [`preload`].
    pub(super) fn register_loaders(&self, cx: &mut App) {
        let loaders = cx.default_global::<Loaders>();
        for route in &self.routes {
            let Some(loader) = &route.loader else {
                continue;
            };
            match loaders
                .registered
                .iter_mut()
                .find(|(pattern, _)| pattern.source == route.pattern.source)
            {
                Some(registered) => registered.1 = loader.clone(),
                None => loaders
                    .registered
                    .push((route.pattern.clone(), loader.clone())),
            }
        }
    }
}
