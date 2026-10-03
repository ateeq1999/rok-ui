#![doc = include_str!("../docs/guide/router.md")]

use std::{collections::HashMap, rc::Rc};

use gpui::{
    actions, div, prelude::*, AnyElement, App, ElementId, Global, KeyBinding, SharedString,
    StyleRefinement, Window,
};

use crate::{
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

actions!(rok_router, [GoBack, GoForward]);

/// Called after every navigation with the new location.
type NavigateListener = Rc<dyn Fn(&Location, &mut App)>;

/// The app's navigation history.
struct History {
    entries: Vec<SharedString>,
    index: usize,
    listeners: Vec<NavigateListener>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: vec!["/".into()],
            index: 0,
            listeners: Vec::new(),
        }
    }
}

impl Global for History {}

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
    cx.default_global::<History>()
}

/// Re-render every window and tell listeners where the app is now.
fn changed(cx: &mut App) {
    let location = location(cx);
    let listeners = history(cx).listeners.clone();
    for listener in listeners {
        listener(&location, cx);
    }
    cx.refresh_windows();
}

/// Go to `path`, adding it to the history. Forward entries are dropped, as in a
/// browser. Navigating to the current location does nothing.
pub fn navigate(path: impl Into<SharedString>, cx: &mut App) {
    let path = normalize(path.into());
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
    let history = history(cx);
    let index = history.index;
    history.entries[index] = path;
    changed(cx);
}

/// Go back one entry, if there is one.
pub fn back(cx: &mut App) {
    if can_go_back(cx) {
        history(cx).index -= 1;
        changed(cx);
    }
}

/// Go forward one entry, if there is one.
pub fn forward(cx: &mut App) {
    if can_go_forward(cx) {
        history(cx).index += 1;
        changed(cx);
    }
}

pub fn can_go_back(cx: &mut App) -> bool {
    history(cx).index > 0
}

pub fn can_go_forward(cx: &mut App) -> bool {
    let history = history(cx);
    history.index + 1 < history.entries.len()
}

/// The current location.
pub fn location(cx: &mut App) -> Location {
    let history = history(cx);
    Location::parse(&history.entries[history.index])
}

/// Call `listener` after every navigation (for analytics, saving the last page,
/// or syncing other state).
pub fn on_navigate(listener: impl Fn(&Location, &mut App) + 'static, cx: &mut App) {
    history(cx).listeners.push(Rc::new(listener));
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
    /// Split `"/users/42?tab=posts"` into its path and decoded query pairs.
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

    pub fn path(&self) -> &str {
        &self.path
    }

    /// The first value of query parameter `key`.
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(name, _)| name.as_ref() == key)
            .map(|(_, value)| value.as_ref())
    }

    pub fn query_pairs(&self) -> &[(SharedString, SharedString)] {
        &self.query
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
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// A `:name` or `*name` parameter.
    pub fn param(&self, name: &str) -> Option<SharedString> {
        self.params.get(name).cloned()
    }

    /// A parameter parsed as `T` (numbers, ids).
    pub fn param_as<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.params.get(name)?.parse().ok()
    }

    pub fn path(&self) -> SharedString {
        self.location.path.clone()
    }

    pub fn query(&self, key: &str) -> Option<&str> {
        self.location.query(key)
    }

    pub fn location(&self) -> &Location {
        &self.location
    }
}

type RouteBuilder = Rc<dyn Fn(&RouteMatch, &mut Window, &mut App) -> AnyElement>;

/// Renders the element of the route that matches the current location.
#[derive(IntoElement)]
pub struct Router {
    routes: Vec<(Pattern, RouteBuilder)>,
    redirects: Vec<(Pattern, SharedString)>,
    not_found: Option<RouteBuilder>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Router);

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            redirects: Vec::new(),
            not_found: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Show `build`'s element when the location matches `pattern`.
    pub fn route<E: IntoElement>(
        mut self,
        pattern: &str,
        build: impl Fn(&RouteMatch, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.routes.push((
            Pattern::parse(pattern),
            Rc::new(move |route, window, cx| build(route, window, cx).into_any_element()),
        ));
        self
    }

    /// Replace the location with `to` when it matches `from`. Parameters in
    /// `from` can be used in `to`: `.redirect("/u/:id", "/users/:id")`.
    pub fn redirect(mut self, from: &str, to: impl Into<SharedString>) -> Self {
        self.redirects.push((Pattern::parse(from), to.into()));
        self
    }

    /// Shown when no route matches. Without one, nothing is shown.
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
    fn resolve(&self, location: &Location) -> Option<(RouteMatch, RouteBuilder)> {
        self.routes
            .iter()
            .enumerate()
            .filter_map(|(index, (pattern, build))| {
                let (params, score) = pattern.matches(location.path())?;
                Some((score, index, pattern, params, build))
            })
            // Highest score first; earlier declarations win ties.
            .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
            .map(|(_, _, pattern, params, build)| {
                (
                    RouteMatch {
                        pattern: pattern.source.clone(),
                        params,
                        location: location.clone(),
                    },
                    build.clone(),
                )
            })
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
        let mut location = location(cx);
        // Follow redirects (a few hops at most, so a cycle cannot hang the app).
        for _ in 0..8 {
            let Some(target) = self.redirect_for(&location) else {
                break;
            };
            replace(target, cx);
            location = self::location(cx);
        }
        let content = match self.resolve(&location) {
            Some((route, build)) => build(&route, window, cx),
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
    exact: bool,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Link);

impl Link {
    pub fn new(id: impl Into<ElementId>, to: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            to: to.into(),
            replace: false,
            exact: false,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Replace the current history entry instead of adding one.
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// Only count as active on exactly this path, not on paths below it.
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
        let element = div()
            .id(self.id)
            .tab_index(0)
            .sx((&LINK.root, active.then_some(&LINK.active), &self.sx))
            .children(self.children)
            .apply_style_overrides(&self.style_overrides);
        crate::components::interaction::on_activate(
            element,
            Rc::new(move |_, cx| {
                if replace_entry {
                    replace(to.clone(), cx);
                } else {
                    navigate(to.clone(), cx);
                }
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
                .map(|target| target.as_ref()),
            Some("/users/5")
        );
        assert_eq!(normalize("users/".into()).as_ref(), "/users");
        assert_eq!(normalize("/".into()).as_ref(), "/");
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
