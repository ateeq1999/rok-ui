//! Navigation histories (one for the app, or one per window), moving through them, and scroll
//! restoration.

use std::{cell::Cell, collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, AnyWindowHandle, App, Bounds, Element, ElementId, Global, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Point, SharedString, Window,
};

use super::{blocker, Location};

/// Called after every navigation with the new location.
type NavigateListener = Rc<dyn Fn(&Location, &mut App)>;

/// One navigation history: the app's, or one window's.
pub(super) struct History {
    entries: Vec<SharedString>,
    index: usize,
    /// Scroll offsets by (entry index, scroll area), for scroll restoration.
    scroll: HashMap<(usize, SharedString), Point<Pixels>>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: vec!["/".into()],
            index: 0,
            scroll: HashMap::new(),
        }
    }
}

/// Every history, and the navigation listeners.
#[derive(Default)]
pub(super) struct Histories {
    per_window: bool,
    app: History,
    windows: HashMap<AnyWindowHandle, History>,
    listeners: Vec<NavigateListener>,
}

impl Global for Histories {}

thread_local! {
    /// The window whose router is rendering (or that code runs for), so `&mut App`
    /// functions find its history in per-window mode.
    pub(super) static CURRENT_WINDOW: Cell<Option<AnyWindowHandle>> = const { Cell::new(None) };
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

pub(super) fn history(cx: &mut App) -> &mut History {
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
pub(super) fn changed(cx: &mut App) {
    #[cfg(feature = "query")]
    crate::query::memo::end_navigation();
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
    let index = history.index;
    history.scroll.retain(|(entry, _), _| *entry <= index);
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
    history.scroll.retain(|(entry, _), _| *entry != index);
    changed(cx);
}

/// Renders its child with [`CURRENT_WINDOW`] set, so components inside a router read their
/// window's history while they lay out and paint.
pub(super) struct WindowScope {
    pub(super) window: AnyWindowHandle,
    pub(super) child: AnyElement,
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
pub(super) fn replace_unblocked(path: SharedString, cx: &mut App) {
    let path = normalize(path);
    let history = history(cx);
    let index = history.index;
    history.entries[index] = path;
    history.scroll.retain(|(entry, _), _| *entry != index);
    changed(cx);
}

/// Scroll restoration for a scroll container `area` tracked by `handle`: save its offset under
/// the window's current history entry and, when the entry changed since `last` (the entry the
/// container showed at its previous render, `None` for a container just built), put back the
/// offset it had there, or the top. Returns the current entry for the caller to remember.
#[cfg_attr(
    not(any(feature = "scroll-area", feature = "scaffold")),
    allow(dead_code)
)]
pub(crate) fn restore_scroll(
    area: &SharedString,
    handle: &gpui::ScrollHandle,
    last: Option<usize>,
    window: &Window,
    cx: &mut App,
) -> usize {
    with_window(window.window_handle(), || {
        let history = history(cx);
        let entry = history.index;
        let saved = |history: &History| history.scroll.get(&(entry, area.clone())).copied();
        match last {
            Some(previous) if previous != entry => {
                history
                    .scroll
                    .insert((previous, area.clone()), handle.offset());
                handle.set_offset(saved(history).unwrap_or_default());
            }
            None => {
                if let Some(offset) = saved(history) {
                    handle.set_offset(offset);
                }
            }
            Some(_) => {
                history
                    .scroll
                    .insert((entry, area.clone()), handle.offset());
            }
        }
        entry
    })
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
pub(super) fn normalize(path: SharedString) -> SharedString {
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

#[cfg(test)]
mod tests {
    use super::*;

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
