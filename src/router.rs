#![doc = include_str!("../docs/guide/router.md")]

use std::{collections::HashMap, rc::Rc};

use gpui::{
    actions, div, prelude::*, AnyElement, App, ElementId, KeyBinding, SharedString,
    StyleRefinement, Task, Window,
};

use crate::{
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
mod guards;
mod history;
mod link;
mod loading;
mod location;
mod pattern;
mod search;
#[doc(hidden)]
pub mod typed;

pub use blocker::{use_blocker, Blocker, PendingNavigation};
pub use guards::{AsyncGuard, Guard, RouteControl};
#[cfg(feature = "persist")]
pub use history::persist_location;
#[cfg(any(feature = "scroll-area", feature = "scaffold"))]
pub(crate) use history::restore_scroll;
pub use history::{
    back, can_go_back, can_go_forward, forward, history_entries, is_active, location, navigate,
    on_navigate, replace, set_per_window_history, with_window,
};
use history::{normalize, replace_unblocked, WindowScope};
pub use link::Link;
#[cfg(feature = "query")]
use loading::cancel_stale_loads;
#[cfg(feature = "query")]
pub use loading::{load_state, use_pending, LoadState};
pub use loading::{preload, Loader};
use loading::{preload_once_visible, run_loader_once};
use location::decode_path;
pub use location::Location;
use pattern::Pattern;
pub use pattern::RouteMatch;
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

/// Register the back / forward key bindings. Called by [`crate::init`].
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("alt-left", GoBack, None),
        KeyBinding::new("alt-right", GoForward, None),
    ]);
    cx.on_action(|_: &GoBack, cx| back(cx));
    cx.on_action(|_: &GoForward, cx| forward(cx));
}

/// Builds a route's element; `None` falls through to the "not found" route (a typed route whose
/// parameters do not parse).
type RouteBuilder = Rc<dyn Fn(&RouteMatch, &mut Window, &mut App) -> Option<AnyElement>>;
type NotFoundBuilder = Rc<dyn Fn(&RouteMatch, &mut Window, &mut App) -> AnyElement>;

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

/// Renders the element of the route that matches the current location.
#[derive(IntoElement)]
pub struct Router {
    routes: Vec<RouteEntry>,
    redirects: Vec<(Pattern, SharedString)>,
    guards: Vec<(Pattern, Guard)>,
    async_guards: Vec<(Pattern, SharedString, AsyncGuard)>,
    not_found: Option<NotFoundBuilder>,
    pending: Option<NotFoundBuilder>,
    transition: Option<crate::motion::Motion>,
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
            async_guards: Vec::new(),
            not_found: None,
            pending: None,
            transition: None,
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

    /// Like [`Router::guard`], for a guard that decides in the background: checking a session
    /// with a server, loading permissions. While it runs the router renders
    /// [`Router::pending`] (or nothing), never the guarded route. It runs once each time the
    /// window arrives at a location below `prefix`, and again after the user leaves and comes
    /// back.
    ///
    /// ```no_run
    /// # use rok_ui::{prelude::*, router::{Router, RouteControl}};
    /// # async fn session_is_valid() -> bool { true }
    /// let router = Router::new()
    ///     .route("/login", |_, _, _| div().child("Sign in"))
    ///     .route("/account", |_, _, _| div().child("Your account"))
    ///     .guard_async("/account", |_, cx| {
    ///         cx.spawn(async |_| {
    ///             if session_is_valid().await {
    ///                 Ok(())
    ///             } else {
    ///                 Err(RouteControl::redirect("/login"))
    ///             }
    ///         })
    ///     })
    ///     .pending(|_, _, _| div().child("Checking your session"));
    /// # let _ = router;
    /// ```
    #[must_use]
    pub fn guard_async(
        mut self,
        prefix: &str,
        guard: impl Fn(&Location, &mut App) -> Task<Result<(), RouteControl>> + 'static,
    ) -> Self {
        let pattern = format!("{}/*__rok_rest", prefix.trim_end_matches('/'));
        let key = format!("{}#{}", pattern, self.async_guards.len()).into();
        self.async_guards
            .push((Pattern::parse(&pattern), key, Rc::new(guard)));
        self
    }

    /// Animate each page in when the location changes, with any [`Motion`](crate::motion::Motion)
    /// (`motion::fade_in()`, `motion::slide_in(..)`, your own keyframes). The new page plays
    /// it once per location; the old page is replaced at once (there is no exit animation).
    /// Reduced motion skips it.
    ///
    /// ```
    /// # use rok_ui::prelude::*;
    /// // `motion` is the prelude's name for `rok_ui::motion::presets`.
    /// let router = Router::new()
    ///     .route("/", |_, _, _| div().child("Home"))
    ///     .transition(motion::fade_in().duration_ms(150));
    /// # let _ = router;
    /// ```
    #[must_use]
    pub fn transition(mut self, motion: crate::motion::Motion) -> Self {
        self.transition = Some(motion);
        self
    }

    /// What to render while an async guard decides (TanStack Router's `pendingComponent`).
    /// Default: nothing.
    #[must_use]
    pub fn pending<E: IntoElement>(
        mut self,
        build: impl Fn(&RouteMatch, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.pending = Some(Rc::new(move |route, window, cx| {
            build(route, window, cx).into_any_element()
        }));
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

    /// The target of the first redirect matching `location`, parameters filled in.
    fn redirect_for(&self, location: &Location) -> Option<SharedString> {
        self.redirects.iter().find_map(|(pattern, to)| {
            let (params, _) = pattern.matches(location.path())?;
            Some(fill_params(to, &params).into())
        })
    }
}

/// `to` with each `:name` and `*name` segment of its path replaced by that parameter's value,
/// encoded again. Segments naming no parameter, and the query, are kept as written.
fn fill_params(to: &str, params: &HashMap<SharedString, SharedString>) -> String {
    let (path, rest) = to.split_at(to.find(['?', '#']).unwrap_or(to.len()));
    let segments: Vec<String> = path
        .split('/')
        .map(|segment| {
            let param = segment
                .strip_prefix(':')
                .map(|name| (name, false))
                .or_else(|| segment.strip_prefix('*').map(|name| (name, true)));
            match param.and_then(|(name, wildcard)| Some((params.get(name)?, wildcard))) {
                Some((value, false)) => typed::encode(value),
                Some((value, true)) => value
                    .split('/')
                    .map(typed::encode)
                    .collect::<Vec<_>>()
                    .join("/"),
                None => segment.to_string(),
            }
        })
        .collect();
    segments.join("/") + rest
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
        let mut guarding = false;
        // Follow redirects and guard redirects (a few hops at most, so a cycle cannot hang
        // the app).
        for _ in 0..8 {
            let target = match self.redirect_for(&location) {
                Some(target) => Some(target),
                None => match self
                    .check_guards(&location, cx)
                    .map(|()| self.check_async_guards(&location, cx))
                {
                    Ok(Some(Ok(()))) => None,
                    Ok(None) => {
                        guarding = true;
                        None
                    }
                    Err(RouteControl::Redirect(target))
                    | Ok(Some(Err(RouteControl::Redirect(target)))) => Some(target),
                    Err(RouteControl::NotFound) | Ok(Some(Err(RouteControl::NotFound))) => {
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
        #[cfg(feature = "query")]
        cancel_stale_loads(&location, cx);
        if guarding {
            let content = match &self.pending {
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
            };
            return div()
                .flex()
                .flex_col()
                .size_full()
                .child(content)
                .sx(&self.sx)
                .apply_style_overrides(&self.style_overrides);
        }
        let href = location.href();
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
        let content = match self.transition.clone() {
            Some(motion) => {
                use crate::motion::MotionExt;
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(content)
                    .motion(ElementId::Name(format!("rok-route:{href}").into()), motion)
                    .into_any_element()
            }
            None => content,
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

        // Each parameter fills its own segment, encoded again.
        let router = Router::new().redirect("/old/:id/:identity", "/new/:identity/:id");
        assert_eq!(
            router
                .redirect_for(&Location::parse("/old/1/a%20b"))
                .as_ref()
                .map(std::convert::AsRef::as_ref),
            Some("/new/a%20b/1")
        );
        let router = Router::new().redirect("/docs/*rest", "/manual/*rest?from=docs");
        assert_eq!(
            router
                .redirect_for(&Location::parse("/docs/a/b"))
                .as_ref()
                .map(std::convert::AsRef::as_ref),
            Some("/manual/a/b?from=docs")
        );
        assert_eq!(normalize("/".into()).as_ref(), "/");
    }
}
