#![doc = include_str!("../docs/guide/router.md")]

use std::{cell::Cell, collections::HashMap, rc::Rc};

use gpui::{
    actions, div, prelude::*, AnyElement, AnyWindowHandle, App, Bounds, ElementId, Global,
    GlobalElementId, InspectorElementId, KeyBinding, LayoutId, Pixels, SharedString,
    StyleRefinement, Window,
};

use crate::{
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// Include the route tree that `rok_ui_build::routes(..).generate()` wrote in `build.rs`: a
/// `routes` module with one typed route per page file and `routes::tree()`.
///
/// `routes!("src/route_tree.rs")` includes a checked-in tree written with
/// `rok_ui_build::routes(..).write_to(..)` instead.
#[macro_export]
macro_rules! routes {
    () => {
        include!(concat!(env!("OUT_DIR"), "/rok_ui_routes.rs"));
    };
    ($path:literal) => {
        include!($path);
    };
}

mod blocker;
mod search;
#[doc(hidden)]
pub mod typed;

pub use blocker::{use_blocker, Blocker, PendingNavigation};
/// Declare a route file's route; see [`routes!`](crate::routes).
pub use rok_ui_macros::file_route;
/// Derive [`Search`] for a struct of query parameters.
pub use rok_ui_macros::Search;
#[doc(hidden)]
pub use search::parse_value;
pub use search::{replace_search, update_search, use_search, Search};
pub use typed::{navigate_to, replace_to, use_params, Route};

actions!(
    rok_router,
    [
        /// Go back one history entry (Alt-Left).
        GoBack,
        /// Go forward one history entry (Alt-Right).
        GoForward,
    ]
);

/// Called after every navigation with the new location.
type NavigateListener = Rc<dyn Fn(&Location, &mut App)>;

/// One navigation history: the app's, or one window's.
struct History {
    entries: Vec<SharedString>,
    index: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: vec!["/".into()],
            index: 0,
        }
    }
}

/// Every history, and the navigation listeners.
#[derive(Default)]
struct Histories {
    per_window: bool,
    app: History,
    windows: HashMap<AnyWindowHandle, History>,
    listeners: Vec<NavigateListener>,
}

impl Global for Histories {}

thread_local! {
    /// The window whose router is rendering (or that code runs for), so `&mut App`
    /// functions find its history in per-window mode.
    static CURRENT_WINDOW: Cell<Option<AnyWindowHandle>> = const { Cell::new(None) };
}

/// Give each window its own history (enhance.md E.7), instead of one history for the app.
/// Off by default. Turn it on at startup, before opening windows.
///
/// Inside a window's `Router` (its routes and their components), the router functions use
/// that window's history; elsewhere they use the active window's. Code that runs for a
/// specific window can say so with [`with_window`].
pub fn set_per_window_history(per_window: bool, cx: &mut App) {
    cx.default_global::<Histories>().per_window = per_window;
}

/// Run `body` with the router functions targeting `window`'s history (in per-window mode).
pub fn with_window<R>(window: AnyWindowHandle, body: impl FnOnce() -> R) -> R {
    let previous = CURRENT_WINDOW.with(|current| current.replace(Some(window)));
    let result = body();
    CURRENT_WINDOW.with(|current| current.set(previous));
    result
}

/// Register the back / forward key bindings. Called by [`crate::init`].
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("alt-left", GoBack, None),
        KeyBinding::new("alt-right", GoForward, None),
    ]);
    cx.on_action(|_: &GoBack, cx| back(cx));
    cx.on_action(|_: &GoForward, cx| forward(cx));
}

fn history(cx: &mut App) -> &mut History {
    let window = CURRENT_WINDOW
        .with(Cell::get)
        .or_else(|| cx.active_window());
    let histories = cx.default_global::<Histories>();
    match window {
        Some(window) if histories.per_window => histories.windows.entry(window).or_default(),
        _ => &mut histories.app,
    }
}

/// Re-render every window and tell listeners where the app is now.
fn changed(cx: &mut App) {
    let location = location(cx);
    let listeners = cx.default_global::<Histories>().listeners.clone();
    for listener in listeners {
        listener(&location, cx);
    }
    cx.refresh_windows();
}

/// Go to `path`, adding it to the history. Forward entries are dropped, as in a
/// browser. Navigating to the current location does nothing.
pub fn navigate(path: impl Into<SharedString>, cx: &mut App) {
    let path = normalize(path.into());
    if blocker::intercept(blocker::PendingNavigation::Push(path.clone()), cx) {
        return;
    }
    let history = history(cx);
    if history.entries[history.index] == path {
        return;
    }
    history.entries.truncate(history.index + 1);
    history.entries.push(path);
    history.index += 1;
    changed(cx);
}

/// Go to `path`, replacing the current history entry.
pub fn replace(path: impl Into<SharedString>, cx: &mut App) {
    let path = normalize(path.into());
    if blocker::intercept(blocker::PendingNavigation::Replace(path.clone()), cx) {
        return;
    }
    let history = history(cx);
    let index = history.index;
    history.entries[index] = path;
    changed(cx);
}

/// Run `loader` unless it already ran for this exact location.
fn run_loader_once(route: &RouteMatch, loader: &Loader, cx: &mut App) {
    let href = route.location.href();
    let last = cx
        .default_global::<Loaders>()
        .last_loaded
        .insert(route.pattern.clone(), href.clone());
    if last.as_ref() != Some(&href) {
        loader(route, cx);
    }
}

/// Renders its child with [`CURRENT_WINDOW`] set, so components inside a router read their
/// window's history while they lay out and paint.
struct WindowScope {
    window: AnyWindowHandle,
    child: AnyElement,
}

impl IntoElement for WindowScope {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WindowScope {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let child = &mut self.child;
        (
            with_window(self.window, || child.request_layout(window, cx)),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let child = &mut self.child;
        with_window(self.window, || child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = &mut self.child;
        with_window(self.window, || child.paint(window, cx));
    }
}

/// `replace` for redirects and guards: blockers do not apply.
fn replace_unblocked(path: SharedString, cx: &mut App) {
    let path = normalize(path);
    let history = history(cx);
    let index = history.index;
    history.entries[index] = path;
    changed(cx);
}

/// Go back one entry, if there is one.
pub fn back(cx: &mut App) {
    if can_go_back(cx) && !blocker::intercept(blocker::PendingNavigation::Back, cx) {
        history(cx).index -= 1;
        changed(cx);
    }
}

/// Go forward one entry, if there is one.
pub fn forward(cx: &mut App) {
    if can_go_forward(cx) && !blocker::intercept(blocker::PendingNavigation::Forward, cx) {
        history(cx).index += 1;
        changed(cx);
    }
}

/// Whether there is an entry to go back to.
pub fn can_go_back(cx: &mut App) -> bool {
    history(cx).index > 0
}

/// Whether there is an entry to go forward to.
pub fn can_go_forward(cx: &mut App) -> bool {
    let history = history(cx);
    history.index + 1 < history.entries.len()
}

/// Every history entry, oldest first, and the index of the current one.
pub fn history_entries(cx: &mut App) -> (Vec<SharedString>, usize) {
    let history = history(cx);
    (history.entries.clone(), history.index)
}

/// The current location.
pub fn location(cx: &mut App) -> Location {
    let history = history(cx);
    Location::parse(&history.entries[history.index])
}

/// Call `listener` after every navigation (for analytics, saving the last page,
/// or syncing other state).
pub fn on_navigate(listener: impl Fn(&Location, &mut App) + 'static, cx: &mut App) {
    cx.default_global::<Histories>()
        .listeners
        .push(Rc::new(listener));
}

/// Save the location under `key` (with the `persist` feature) and restore it now, so the app
/// reopens where the user left it. Call it once at startup, after `rok_ui::init`.
///
/// ```no_run
/// # fn startup(cx: &mut rok_ui::gpui::App) {
/// rok_ui::router::persist_location("main-window", cx);
/// # }
/// ```
#[cfg(feature = "persist")]
pub fn persist_location(key: &str, cx: &mut App) {
    let store = crate::persist::persisted_store(cx, &format!("location-{key}"), || "/".to_string());
    let saved = store.peek();
    if saved != "/" {
        replace_unblocked(saved.into(), cx);
    }
    on_navigate(
        move |location, _| store.set(location.href().to_string()),
        cx,
    );
}

/// Whether the current path is `path`, or below it unless `exact`. For marking
/// the active item in navigation.
pub fn is_active(path: &str, exact: bool, cx: &mut App) -> bool {
    let current = location(cx);
    let path = normalize(SharedString::from(path.to_string()));
    let target = Location::parse(&path);
    if exact || target.path() == "/" {
        return current.path() == target.path();
    }
    current.path() == target.path() || current.path().starts_with(&format!("{}/", target.path()))
}

/// Ensure a leading slash and drop a trailing one (except for the root).
fn normalize(path: SharedString) -> SharedString {
    let trimmed = path.trim();
    let mut normalized = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    let query_start = normalized.find(['?', '#']).unwrap_or(normalized.len());
    if query_start > 1 && normalized[..query_start].ends_with('/') {
        normalized.remove(query_start - 1);
    }
    normalized.into()
}

/// A path and its query string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    path: SharedString,
    query: Vec<(SharedString, SharedString)>,
}

impl Location {
    /// A path with query pairs, encoded: `build("/search", &[("q", "a b")])` is
    /// `/search?q=a%20b`.
    #[must_use]
    pub fn build(path: &str, query: &[(impl AsRef<str>, impl AsRef<str>)]) -> String {
        let mut built = path.to_string();
        for (index, (key, value)) in query.iter().enumerate() {
            built.push(if index == 0 { '?' } else { '&' });
            built.push_str(&typed::encode(key.as_ref()));
            built.push('=');
            built.push_str(&typed::encode(value.as_ref()));
        }
        built
    }

    /// The path and query string, encoded.
    #[must_use]
    pub fn href(&self) -> SharedString {
        Self::build(&self.path, &self.query).into()
    }

    /// Split `"/users/42?tab=posts"` into its path and decoded query pairs.
    #[must_use]
    pub fn parse(full_path: &str) -> Self {
        let without_fragment = full_path.split('#').next().unwrap_or_default();
        let (path, query) = without_fragment
            .split_once('?')
            .unwrap_or((without_fragment, ""));
        let query = query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                (decode(key).into(), decode(value).into())
            })
            .collect();
        Self {
            path: SharedString::from(path.to_string()),
            query,
        }
    }

    /// The path without the query string: `/users/42`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The first value of query parameter `key`.
    #[must_use]
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(name, _)| name.as_ref() == key)
            .map(|(_, value)| value.as_ref())
    }

    /// Every query parameter, decoded, in order.
    #[must_use]
    pub fn query_pairs(&self) -> &[(SharedString, SharedString)] {
        &self.query
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.href())
    }
}

/// Decode `%XX` escapes and `+` (as a space) in a query component.
fn decode(component: &str) -> String {
    let bytes = component.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            b'%' if index + 2 < bytes.len() => {
                match u8::from_str_radix(&component[index + 1..index + 3], 16) {
                    Ok(byte) => {
                        decoded.push(byte);
                        index += 2;
                    }
                    Err(_) => decoded.push(b'%'),
                }
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// One segment of a route pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Segment {
    Static(SharedString),
    Param(SharedString),
    Wildcard(SharedString),
}

/// A compiled route pattern such as `/users/:id`.
#[derive(Clone, Debug)]
struct Pattern {
    source: SharedString,
    segments: Vec<Segment>,
}

impl Pattern {
    fn parse(source: &str) -> Self {
        let segments = source
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                if let Some(name) = segment.strip_prefix(':') {
                    Segment::Param(name.to_string().into())
                } else if let Some(name) = segment.strip_prefix('*') {
                    Segment::Wildcard(name.to_string().into())
                } else {
                    Segment::Static(segment.to_string().into())
                }
            })
            .collect();
        Self {
            source: normalize(SharedString::from(source.to_string())),
            segments,
        }
    }

    /// The captured parameters and a specificity score, if `path` matches.
    fn matches(&self, path: &str) -> Option<(HashMap<SharedString, SharedString>, usize)> {
        let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
        let mut params = HashMap::new();
        let mut score = 0;
        for (index, segment) in self.segments.iter().enumerate() {
            match segment {
                Segment::Wildcard(name) => {
                    let rest = parts.get(index..).unwrap_or_default().join("/");
                    params.insert(name.clone(), decode(&rest).into());
                    return Some((params, score * 3 + 1));
                }
                Segment::Static(text) => {
                    if parts.get(index) != Some(&text.as_ref()) {
                        return None;
                    }
                    score = score * 3 + 3;
                }
                Segment::Param(name) => {
                    let part = parts.get(index)?;
                    params.insert(name.clone(), decode(part).into());
                    score = score * 3 + 2;
                }
            }
        }
        (parts.len() == self.segments.len()).then_some((params, score))
    }
}

/// The route that matched the current location, passed to route builders.
#[derive(Clone, Debug)]
pub struct RouteMatch {
    pattern: SharedString,
    params: HashMap<SharedString, SharedString>,
    location: Location,
}

impl RouteMatch {
    /// The pattern that matched, such as `/users/:id` (empty for "not found").
    #[must_use]
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// A `:name` or `*name` parameter.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<SharedString> {
        self.params.get(name).cloned()
    }

    /// A parameter parsed as `T` (numbers, ids).
    #[must_use]
    pub fn param_as<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.params.get(name)?.parse().ok()
    }

    /// The matched path.
    #[must_use]
    pub fn path(&self) -> SharedString {
        self.location.path.clone()
    }

    /// The first value of query parameter `key`.
    #[must_use]
    pub fn query(&self, key: &str) -> Option<&str> {
        self.location.query(key)
    }

    /// The full location, with its query.
    #[must_use]
    pub fn location(&self) -> &Location {
        &self.location
    }
}

/// Builds a route's element; `None` falls through to the "not found" route (a typed route whose
/// parameters do not parse).
type RouteBuilder = Rc<dyn Fn(&RouteMatch, &mut Window, &mut App) -> Option<AnyElement>>;
type NotFoundBuilder = Rc<dyn Fn(&RouteMatch, &mut Window, &mut App) -> AnyElement>;
/// A guard: `Ok(())` lets the route render.
pub type Guard = Rc<dyn Fn(&Location, &mut App) -> Result<(), RouteControl>>;

/// Starts loading a route's data before it renders (prefetching queries).
pub type Loader = Rc<dyn Fn(&RouteMatch, &mut App)>;

/// One registered route.
struct RouteEntry {
    pattern: Pattern,
    guards: Vec<Guard>,
    loader: Option<Loader>,
    build: RouteBuilder,
}

impl RouteEntry {
    fn new(pattern: Pattern, build: RouteBuilder) -> Self {
        Self {
            pattern,
            guards: Vec::new(),
            loader: None,
            build,
        }
    }
}

/// Loaders seen by routers, for link preloading, and the location each one last loaded.
#[derive(Default)]
struct Loaders {
    registered: Vec<(Pattern, Loader)>,
    last_loaded: HashMap<SharedString, SharedString>,
}

impl Global for Loaders {}

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

/// What a guard does instead of rendering the route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteControl {
    /// Go to this path instead, replacing the current history entry.
    Redirect(SharedString),
    /// Render the "not found" route.
    NotFound,
}

impl RouteControl {
    /// Redirect to `path`.
    #[must_use]
    pub fn redirect(path: impl Into<SharedString>) -> Self {
        Self::Redirect(path.into())
    }

    /// Redirect to a typed route.
    #[must_use]
    pub fn redirect_to<R: Route>(route: &R) -> Self {
        Self::Redirect(route.href())
    }
}

/// Renders the element of the route that matches the current location.
#[derive(IntoElement)]
pub struct Router {
    routes: Vec<RouteEntry>,
    redirects: Vec<(Pattern, SharedString)>,
    guards: Vec<(Pattern, Guard)>,
    not_found: Option<NotFoundBuilder>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Router);

impl Router {
    /// A router with no routes.
    #[must_use]
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            redirects: Vec::new(),
            guards: Vec::new(),
            not_found: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Show `build`'s element when the location matches `pattern`.
    #[must_use]
    pub fn route<E: IntoElement>(
        mut self,
        pattern: &str,
        build: impl Fn(&RouteMatch, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.routes.push(RouteEntry::new(
            Pattern::parse(pattern),
            Rc::new(move |route, window, cx| Some(build(route, window, cx).into_any_element())),
        ));
        self
    }

    /// Show `build`'s element for typed route `R`. If the location matches `R`'s pattern but a
    /// parameter does not parse (`/notes/abc` for a numeric id), the "not found" route shows.
    ///
    /// ```
    /// # use rok_ui::{prelude::*, router::Router, typed_route};
    /// typed_route! { pub struct NoteRoute = "/notes/:id" { pub id: u64 } }
    ///
    /// let router = Router::new().route_to(|note: NoteRoute, _, _| div().child(format!("Note {}", note.id)));
    /// # let _ = router;
    /// ```
    #[must_use]
    pub fn route_to<R: Route, E: IntoElement>(
        mut self,
        build: impl Fn(R, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.routes.push(RouteEntry::new(
            Pattern::parse(R::PATTERN),
            Rc::new(move |route, window, cx| {
                R::from_match(route).map(|typed| build(typed, window, cx).into_any_element())
            }),
        ));
        self
    }

    /// Run `guard` before rendering any route at or below `prefix` (a pattern such as
    /// `/settings` or `/teams/:team`). A guard that returns
    /// [`RouteControl::Redirect`] sends the user elsewhere; the guarded route never renders.
    ///
    /// ```
    /// # use rok_ui::{prelude::*, router::{Router, RouteControl}};
    /// # fn signed_in(_: &App) -> bool { false }
    /// let router = Router::new()
    ///     .route("/login", |_, _, _| div().child("Sign in"))
    ///     .route("/account", |_, _, _| div().child("Your account"))
    ///     .guard("/account", |location, cx| {
    ///         if signed_in(cx) {
    ///             Ok(())
    ///         } else {
    ///             Err(RouteControl::redirect(format!("/login?next={}", location.path())))
    ///         }
    ///     });
    /// # let _ = router;
    /// ```
    #[must_use]
    pub fn guard(
        mut self,
        prefix: &str,
        guard: impl Fn(&Location, &mut App) -> Result<(), RouteControl> + 'static,
    ) -> Self {
        let pattern = format!("{}/*__rok_rest", prefix.trim_end_matches('/'));
        self.guards.push((Pattern::parse(&pattern), Rc::new(guard)));
        self
    }

    /// A route generated from a route file: its own guards run before it renders, and `build`
    /// may return `None` to fall through to "not found". Used by `rok_ui::routes!()`.
    #[doc(hidden)]
    #[must_use]
    pub fn __file_route(
        mut self,
        pattern: &str,
        guards: Vec<Guard>,
        loader: Option<Loader>,
        build: impl Fn(&RouteMatch, &mut Window, &mut App) -> Option<AnyElement> + 'static,
    ) -> Self {
        self.routes.push(RouteEntry {
            pattern: Pattern::parse(pattern),
            guards,
            loader,
            build: Rc::new(build),
        });
        self
    }

    /// Start loading data for routes matching `pattern` before they render: on navigation
    /// (once per location) and when a `Link::preload(true)` link to them is hovered. Loaders
    /// usually prefetch queries that the page then reads:
    ///
    /// ```
    /// # use rok_ui::{prelude::*, query::{self, QueryOptions}, query_key, router::Router};
    /// # fn note_query(id: u64) -> QueryOptions<String> {
    /// #     QueryOptions::new(query_key!["notes", id], |_| async { Ok::<_, std::io::Error>(String::new()) })
    /// # }
    /// let router = Router::new()
    ///     .route("/notes/:id", |_, _, _| div())
    ///     .loader("/notes/:id", |route, cx| {
    ///         if let Some(id) = route.param_as::<u64>("id") {
    ///             query::prefetch_query(cx, &note_query(id));
    ///         }
    ///     });
    /// # let _ = router;
    /// ```
    #[must_use]
    pub fn loader(
        mut self,
        pattern: &str,
        loader: impl Fn(&RouteMatch, &mut App) + 'static,
    ) -> Self {
        let source = Pattern::parse(pattern).source;
        let loader: Loader = Rc::new(loader);
        for route in &mut self.routes {
            if route.pattern.source == source {
                route.loader = Some(loader.clone());
            }
        }
        self
    }

    /// [`Router::loader`] for a typed route.
    #[must_use]
    pub fn loader_to<R: Route>(self, loader: impl Fn(&R, &mut App) + 'static) -> Self {
        self.loader(R::PATTERN, move |route, cx| {
            if let Some(typed) = R::from_match(route) {
                loader(&typed, cx);
            }
        })
    }

    /// Replace the location with `to` when it matches `from`. Parameters in
    /// `from` can be used in `to`: `.redirect("/u/:id", "/users/:id")`.
    #[must_use]
    pub fn redirect(mut self, from: &str, to: impl Into<SharedString>) -> Self {
        self.redirects.push((Pattern::parse(from), to.into()));
        self
    }

    /// Shown when no route matches. Without one, nothing is shown.
    #[must_use]
    pub fn not_found<E: IntoElement>(
        mut self,
        build: impl Fn(&RouteMatch, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.not_found = Some(Rc::new(move |route, window, cx| {
            build(route, window, cx).into_any_element()
        }));
        self
    }

    /// The best route for `location`, with its parameters.
    fn resolve(&self, location: &Location) -> Option<(RouteMatch, &RouteEntry)> {
        self.routes
            .iter()
            .enumerate()
            .filter_map(|(index, route)| {
                let (params, score) = route.pattern.matches(location.path())?;
                Some((score, index, route, params))
            })
            // Highest score first; earlier declarations win ties.
            .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
            .map(|(_, _, route, params)| {
                (
                    RouteMatch {
                        pattern: route.pattern.source.clone(),
                        params,
                        location: location.clone(),
                    },
                    route,
                )
            })
    }

    /// Make this router's loaders available to [`preload`].
    fn register_loaders(&self, cx: &mut App) {
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

    /// The first guard over `location` that refuses it.
    fn check_guards(&self, location: &Location, cx: &mut App) -> Result<(), RouteControl> {
        for (pattern, guard) in &self.guards {
            if pattern.matches(location.path()).is_some() {
                guard(location, cx)?;
            }
        }
        if let Some((_, route)) = self.resolve(location) {
            for guard in &route.guards {
                guard(location, cx)?;
            }
        }
        Ok(())
    }

    /// The target of the first redirect matching `location`, parameters filled in.
    fn redirect_for(&self, location: &Location) -> Option<SharedString> {
        self.redirects.iter().find_map(|(pattern, to)| {
            let (params, _) = pattern.matches(location.path())?;
            let mut target = to.to_string();
            for (name, value) in &params {
                target = target
                    .replace(&format!(":{name}"), value)
                    .replace(&format!("*{name}"), value);
            }
            Some(target.into())
        })
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Router {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = window.window_handle();
        let content = with_window(handle, || self.render_content(window, cx));
        WindowScope {
            window: handle,
            child: content.into_any_element(),
        }
    }
}

impl Router {
    fn render_content(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut location = location(cx);
        let mut not_found = false;
        // Follow redirects and guard redirects (a few hops at most, so a cycle cannot hang
        // the app).
        for _ in 0..8 {
            let target = match self.redirect_for(&location) {
                Some(target) => Some(target),
                None => match self.check_guards(&location, cx) {
                    Ok(()) => None,
                    Err(RouteControl::Redirect(target)) => Some(target),
                    Err(RouteControl::NotFound) => {
                        not_found = true;
                        None
                    }
                },
            };
            let Some(target) = target else {
                break;
            };
            replace_unblocked(target, cx);
            location = self::location(cx);
        }
        self.register_loaders(cx);
        let content =
            match self
                .resolve(&location)
                .filter(|_| !not_found)
                .and_then(|(route, entry)| {
                    if let Some(loader) = &entry.loader {
                        run_loader_once(&route, loader, cx);
                    }
                    (entry.build)(&route, window, cx)
                }) {
                Some(content) => content,
                None => match &self.not_found {
                    Some(build) => build(
                        &RouteMatch {
                            pattern: SharedString::default(),
                            params: HashMap::new(),
                            location,
                        },
                        window,
                        cx,
                    ),
                    None => div().into_any_element(),
                },
            };
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(content)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

styles! {
    LINK = {
        root: {
            cursor: pointer,
            color: primary,
            border: 1,
            border_color: transparent,
            radius: sm,
            hover: { underline: true },
            focus: { border_color: ring },
        },
        active: { font: semibold },
    }
}

/// A clickable element that navigates to a path. Styled as a text link by
/// default; restyle with `.sx(..)` or wrap any element (a card, a row).
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    to: SharedString,
    replace: bool,
    preload: bool,
    exact: bool,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Link);

impl Link {
    /// A link to `to`. `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, to: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            to: to.into(),
            replace: false,
            preload: false,
            exact: false,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A link to a typed route. Its element id is the route's path; use
    /// [`Link::new`] when two links to the same path are siblings.
    ///
    /// ```
    /// # use rok_ui::{prelude::*, typed_route};
    /// typed_route! { pub struct NoteRoute = "/notes/:id" { pub id: u64 } }
    /// let link = Link::to(&NoteRoute { id: 3 }).child("Open note 3");
    /// # let _ = link;
    /// ```
    pub fn to<R: Route>(route: &R) -> Self {
        let href = route.href();
        Self::new(ElementId::Name(href.clone()), href)
    }

    /// Add typed search params to the link's path.
    #[must_use]
    pub fn search<S: Search>(mut self, search: &S) -> Self {
        let location = Location::parse(&self.to);
        self.to = Location::build(location.path(), &search.to_query()).into();
        self
    }

    /// Run the target route's loaders when the pointer enters the link, so its data is
    /// loading before the click (TanStack Router's `preload: "intent"`). Default: off.
    #[must_use]
    pub fn preload(mut self, preload: bool) -> Self {
        self.preload = preload;
        self
    }

    /// Replace the current history entry instead of adding one.
    #[must_use]
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// Only count as active on exactly this path, not on paths below it.
    #[must_use]
    pub fn exact(mut self, exact: bool) -> Self {
        self.exact = exact;
        self
    }
}

impl ParentElement for Link {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Link {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let active = is_active(&self.to, self.exact, cx);
        let (to, replace_entry) = (self.to, self.replace);
        let preload_target = self.preload.then(|| to.clone());
        let element = div()
            .id(self.id)
            .tab_index(0)
            .when_some(preload_target, |element, target| {
                element.on_hover(move |hovered, _, cx| {
                    if *hovered {
                        preload(&target, cx);
                    }
                })
            })
            .sx((&LINK.root, active.then_some(&LINK.active), &self.sx))
            .children(self.children)
            .apply_style_overrides(&self.style_overrides);
        crate::components::interaction::on_activate(
            element,
            Rc::new(move |window: &mut Window, cx: &mut App| {
                with_window(window.window_handle(), || {
                    if replace_entry {
                        replace(to.clone(), cx);
                    } else {
                        navigate(to.clone(), cx);
                    }
                });
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(router: &Router, path: &str) -> Option<(SharedString, Vec<(String, String)>)> {
        router.resolve(&Location::parse(path)).map(|(route, _)| {
            let mut params: Vec<(String, String)> = route
                .params
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect();
            params.sort();
            (route.pattern.clone(), params)
        })
    }

    fn router() -> Router {
        Router::new()
            .route("/", |_, _, _| div())
            .route("/users/:id", |_, _, _| div())
            .route("/users/new", |_, _, _| div())
            .route("/files/*path", |_, _, _| div())
            .route("/users/:id/posts/:post", |_, _, _| div())
    }

    #[test]
    fn the_most_specific_pattern_wins() {
        let router = router();
        assert_eq!(resolve(&router, "/").unwrap().0.as_ref(), "/");
        assert_eq!(
            resolve(&router, "/users/42").unwrap(),
            ("/users/:id".into(), vec![("id".into(), "42".into())])
        );
        // Declared after `:id`, but a static segment beats a parameter.
        assert_eq!(
            resolve(&router, "/users/new").unwrap().0.as_ref(),
            "/users/new"
        );
        assert_eq!(
            resolve(&router, "/users/7/posts/9").unwrap().1,
            [("id".into(), "7".into()), ("post".into(), "9".into())]
        );
        assert_eq!(
            resolve(&router, "/files/docs/2026/report.pdf").unwrap().1,
            [("path".into(), "docs/2026/report.pdf".into())]
        );
        assert!(resolve(&router, "/missing").is_none());
        assert!(resolve(&router, "/users").is_none());
    }

    #[test]
    fn locations_parse_queries_and_redirects_fill_parameters() {
        let location = Location::parse("/search?q=hello+world&tag=%D8%B9%D8%B1%D8%A8%D9%8A&empty");
        assert_eq!(location.path(), "/search");
        assert_eq!(location.query("q"), Some("hello world"));
        assert_eq!(location.query("tag"), Some("عربي"));
        assert_eq!(location.query("empty"), Some(""));
        assert_eq!(location.query("missing"), None);

        let router = Router::new().redirect("/u/:id", "/users/:id");
        assert_eq!(
            router
                .redirect_for(&Location::parse("/u/5"))
                .as_ref()
                .map(std::convert::AsRef::as_ref),
            Some("/users/5")
        );
        assert_eq!(normalize("users/".into()).as_ref(), "/users");
        assert_eq!(normalize("/".into()).as_ref(), "/");
    }

    #[cfg(feature = "persist")]
    #[gpui::test]
    fn the_location_is_saved_and_restored(cx: &mut gpui::TestAppContext) {
        let key = format!("test-{}", std::process::id());
        cx.update(|cx| {
            crate::persist::set_app_name(cx, "rok-ui-tests");
            persist_location(&key, cx);
            navigate("/inbox/7?tab=info", cx);
        });
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        // A new app (fresh history) restores it.
        let restored = cx.update(|cx| {
            cx.set_global(Histories::default());
            persist_location(&key, cx);
            location(cx)
        });
        assert_eq!(restored.path(), "/inbox/7");
        assert_eq!(restored.query("tab"), Some("info"));
        let file = crate::persist::config_dir()
            .join("rok-ui-tests")
            .join(format!("location-{key}.json"));
        std::fs::remove_file(file).ok();
    }

    #[gpui::test]
    fn history_goes_back_and_forward(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            assert_eq!(location(cx).path(), "/");
            navigate("/a", cx);
            navigate("/b", cx);
            navigate("/b", cx); // same location: no new entry
            back(cx);
            assert_eq!(location(cx).path(), "/a");
            assert!(can_go_forward(cx));
            navigate("/c", cx); // drops "/b"
            assert!(!can_go_forward(cx));
            back(cx);
            back(cx);
            assert_eq!(location(cx).path(), "/");
            assert!(!can_go_back(cx));
            forward(cx);
            replace("/z?x=1", cx);
            assert_eq!(location(cx).path(), "/z");
            assert_eq!(location(cx).query("x"), Some("1"));
            assert!(is_active("/z", true, cx));
            navigate("/z/inner", cx);
            assert!(is_active("/z", false, cx));
            assert!(!is_active("/z", true, cx));
        });
    }
}
