//! The root of a rok-ui window: applies the theme's background, text color and
//! font, and wires Tab / Shift-Tab focus navigation. The `ThemeProvider` of rok-ui.

use gpui::{actions, div, prelude::*, AnyElement, App, KeyBinding, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{styles, styles::ApplyStyleOverrides, theme::ActiveTheme};

styles! {
    APP_ROOT = {
        root: {
            size: full,
            display: flex,
            direction: column,
            background: background,
            color: foreground,
            font_family: sans,
            text_align: start,
        },
    }
}

actions!(rok_ui, [FocusNextElement, FocusPreviousElement]);

pub(crate) fn bind_focus_navigation_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", FocusNextElement, None),
        KeyBinding::new("shift-tab", FocusPreviousElement, None),
    ]);
}

/// Wrap each window's content in `AppRoot` so components inherit the theme.
/// With the `toast` feature it also draws the toasts shown with `toast(cx, ..)`.
///
/// ```ignore
/// impl Render for MyView {
///     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
///         AppRoot::new().child(Button::new("hello").label("Hello"))
///     }
/// }
/// ```
#[derive(IntoElement)]
pub struct AppRoot {
    children: Vec<AnyElement>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(AppRoot);

impl AppRoot {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl Default for AppRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AppRoot {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for AppRoot {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Keyboard actions dispatch from the focused element upward, so the root
        // holds focus whenever nothing else does; that keeps Tab navigation working.
        let root_focus_handle = window
            .use_keyed_state("rok-ui-app-root-focus", cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        // Also reclaim focus when the focused element left the tree (a closed dialog).
        if !root_focus_handle.contains_focused(window, cx) {
            let focus_handle_to_focus = root_focus_handle.clone();
            window.defer(cx, move |window, _| window.focus(&focus_handle_to_focus));
        }

        let theme = cx.theme();
        div()
            .id("rok-ui-app-root")
            .track_focus(&root_focus_handle)
            .sx((&APP_ROOT.root, &self.sx))
            .text_size(theme.font_size)
            .on_action(|_: &FocusNextElement, window, _| window.focus_next())
            .on_action(|_: &FocusPreviousElement, window, _| window.focus_prev())
            .children(self.children)
            .children(toaster())
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(feature = "toast")]
fn toaster() -> Option<gpui::AnyElement> {
    Some(super::toast::Toaster.into_any_element())
}

#[cfg(not(feature = "toast"))]
fn toaster() -> Option<gpui::AnyElement> {
    None
}
