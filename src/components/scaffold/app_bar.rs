//! [`AppBar`]: the bar at the top of a screen, with a title, leading element and actions.

use gpui::{div, prelude::*, AnyElement, App, SharedString, StyleRefinement, Window};

#[cfg(doc)]
use super::Scaffold;
use crate::{
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

styles! {
    APP_BAR = {
        root: {
            display: flex,
            direction: column,
            flex: none,
            width: full,
            background: background,
            border_bottom: 1,
            border_color: border,
        },
        row: { display: flex, direction: row, align: center, gap: 1, height: 16, padding_x: 2 },
        title: { flex: 1, min_width: 0, padding_x: 2, text: lg, font: semibold, truncate: true },
        title_centered: { flex: none, max_width: 120 },
        side: { display: flex, direction: row, align: center, gap: 1, flex: none },
        side_centered: { flex: 1, min_width: 0 },
        actions_centered: { justify: end },
    }
}

/// The bar across the top of a [`Scaffold`]: a leading button, a title and actions,
/// with an optional `bottom` row such as tabs.
#[derive(IntoElement)]
pub struct AppBar {
    pub(super) leading: Option<AnyElement>,
    title: Option<AnyElement>,
    pub(super) actions: Vec<AnyElement>,
    center_title: bool,
    bottom: Option<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(AppBar);

impl AppBar {
    /// An empty `AppBar`; add content with the builder methods.
    #[must_use]
    pub fn new() -> Self {
        Self {
            leading: None,
            title: None,
            actions: Vec::new(),
            center_title: false,
            bottom: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Before the title. Inside a [`Scaffold`] with a drawer it defaults to a menu
    /// button that opens the drawer.
    #[must_use]
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_any_element());
        self
    }

    /// The title.
    #[must_use]
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(crate::components::bidi_text::text(title));
        self
    }

    /// A custom title, such as a search field or a logo.
    #[must_use]
    pub fn title_element(mut self, element: impl IntoElement) -> Self {
        self.title = Some(element.into_any_element());
        self
    }

    /// An element at the end of the bar, usually a ghost icon button.
    #[must_use]
    pub fn action(mut self, element: impl IntoElement) -> Self {
        self.actions.push(element.into_any_element());
        self
    }

    /// Center the title between the leading element and the actions.
    #[must_use]
    pub fn center_title(mut self, center: bool) -> Self {
        self.center_title = center;
        self
    }

    /// A second row under the bar, such as [`crate::components::Tabs`].
    #[must_use]
    pub fn bottom(mut self, element: impl IntoElement) -> Self {
        self.bottom = Some(element.into_any_element());
        self
    }
}

impl Default for AppBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for AppBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let centered = self.center_title;
        let leading = div()
            .sx((&APP_BAR.side, centered.then_some(&APP_BAR.side_centered)))
            .children(self.leading);
        let title = div()
            .sx((&APP_BAR.title, centered.then_some(&APP_BAR.title_centered)))
            .children(self.title);
        let actions = div()
            .sx((
                &APP_BAR.side,
                centered.then_some(&APP_BAR.side_centered),
                centered.then_some(&APP_BAR.actions_centered),
            ))
            .children(self.actions);
        div()
            .sx((&APP_BAR.root, &self.sx))
            .child(
                div()
                    .sx(&APP_BAR.row)
                    .child(leading)
                    .child(title)
                    .child(actions),
            )
            .children(self.bottom)
            .apply_style_overrides(&self.style_overrides)
    }
}
