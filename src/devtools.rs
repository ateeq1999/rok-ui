//! Devtools (feature `devtools`): an overlay that shows the router and the query cache.
//!
//! Add [`Devtools`] once, last in the window's root, and toggle it with Ctrl-Shift-D:
//!
//! ```no_run
//! # use rok_ui::{prelude::*, devtools::Devtools};
//! # struct Main;
//! impl Render for Main {
//!     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
//!         AppRoot::new().child(div().child("The app")).child(Devtools::new())
//!     }
//! }
//! ```

#[cfg(any(feature = "router", feature = "query"))]
use gpui::SharedString;
use gpui::{actions, div, prelude::*, App, Global, KeyBinding, Window};

use crate::{styles, sx::SxStyled};

actions!(
    rok_devtools,
    [
        /// Show or hide the devtools overlay (Ctrl-Shift-D).
        ToggleDevtools,
    ]
);

#[derive(Default)]
struct DevtoolsState {
    open: bool,
}

impl Global for DevtoolsState {}

/// Register the toggle shortcut. Called by [`crate::init`].
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-d", ToggleDevtools, None)]);
    cx.on_action(|_: &ToggleDevtools, cx| toggle(cx));
}

/// Show or hide the overlay.
pub fn toggle(cx: &mut App) {
    let state = cx.default_global::<DevtoolsState>();
    state.open = !state.open;
    cx.refresh_windows();
}

/// Whether the overlay is shown.
#[must_use]
pub fn is_open(cx: &App) -> bool {
    cx.try_global::<DevtoolsState>()
        .is_some_and(|state| state.open)
}

styles! {
    DEVTOOLS = {
        panel: {
            position: absolute,
            bottom: 3,
            right: 3,
            width: 96,
            max_height: 120,
            display: flex,
            direction: column,
            gap: 3,
            padding: 3,
            radius: lg,
            border: 1,
            border_color: border,
            background: popover,
            color: popover_foreground,
            shadow: lg,
            text: xs,
            overflow: hidden,
        },
        title: { font: semibold, text: sm },
        section: { display: flex, direction: column, gap: 1 },
        heading: { font: semibold, color: muted_foreground },
        row: { display: flex, gap: 2, justify: between },
        mono: { font_family: mono, truncate: true },
        current: { color: primary, font: semibold },
        bad: { color: destructive_text },
        muted: { color: muted_foreground },
    }
}

/// The devtools overlay. Renders nothing until toggled open.
#[derive(IntoElement, Default)]
pub struct Devtools;

impl Devtools {
    /// The overlay.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[cfg(any(feature = "router", feature = "query"))]
fn line(text: impl Into<SharedString>, style: &crate::sx::Sx) -> gpui::Div {
    div()
        .sx((&DEVTOOLS.mono, style))
        .child(crate::components::BidiText::new(text.into()))
}

#[cfg(feature = "router")]
fn router_section(cx: &mut App) -> gpui::AnyElement {
    let (entries, index) = crate::router::history_entries(cx);
    div()
        .sx(&DEVTOOLS.section)
        .child(div().sx(&DEVTOOLS.heading).child("Router"))
        .children(
            entries
                .into_iter()
                .enumerate()
                .rev()
                .take(8)
                .map(|(position, entry)| {
                    let style = if position == index {
                        &DEVTOOLS.current
                    } else {
                        &DEVTOOLS.muted
                    };
                    line(entry, style)
                }),
        )
        .into_any_element()
}

#[cfg(not(feature = "router"))]
fn router_section(_: &mut App) -> gpui::AnyElement {
    div().into_any_element()
}

#[cfg(feature = "query")]
fn query_section(cx: &mut App) -> gpui::AnyElement {
    let queries = crate::query::queries(cx);
    let count = queries.len();
    div()
        .sx(&DEVTOOLS.section)
        .child(
            div()
                .sx(&DEVTOOLS.heading)
                .child(format!("Queries ({count})")),
        )
        .children(queries.into_iter().take(20).map(|query| {
            let status = if query.is_fetching {
                "fetching"
            } else if query.error.is_some() {
                "error"
            } else if query.is_invalidated {
                "stale"
            } else if query.has_data {
                "fresh"
            } else {
                "pending"
            };
            let age = query
                .age
                .map(|age| format!("{:.1}s", age.as_secs_f32()))
                .unwrap_or_default();
            div()
                .sx(&DEVTOOLS.row)
                .child(line(query.key.to_string(), &DEVTOOLS.mono))
                .child(line(
                    format!("{status} {age}"),
                    if query.error.is_some() {
                        &DEVTOOLS.bad
                    } else {
                        &DEVTOOLS.muted
                    },
                ))
        }))
        .into_any_element()
}

#[cfg(not(feature = "query"))]
fn query_section(_: &mut App) -> gpui::AnyElement {
    div().into_any_element()
}

impl RenderOnce for Devtools {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if !is_open(cx) {
            return div().into_any_element();
        }
        div()
            .id("rok-devtools")
            .sx(&DEVTOOLS.panel)
            .child(div().sx(&DEVTOOLS.title).child("rok-ui devtools"))
            .child(router_section(cx))
            .child(query_section(cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn the_shortcut_action_toggles_the_overlay(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            assert!(!is_open(cx));
            toggle(cx);
            assert!(is_open(cx));
            cx.dispatch_action(&ToggleDevtools);
            assert!(!is_open(cx));
        });
    }

    struct Root;

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            crate::components::AppRoot::new().child(Devtools::new())
        }
    }

    #[gpui::test]
    fn the_open_overlay_renders_queries_and_history(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            #[cfg(feature = "query")]
            crate::query::set_query_data(cx, &crate::query_key!["notes", 1], "Groceries");
            #[cfg(feature = "router")]
            crate::router::navigate("/notes/1", cx);
            toggle(cx);
        });
        let (_, window) = cx.add_window_view(|_, _| Root);
        window.run_until_parked();
        #[cfg(feature = "router")]
        window.update(|_, cx| {
            let (entries, index) = crate::router::history_entries(cx);
            assert_eq!((entries.len(), index), (2, 1));
        });
    }
}
