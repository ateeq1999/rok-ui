//! Checkbox: a controlled boolean with an optional label.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, CursorStyle, ElementId, SharedString, StyleRefinement, Window,
};

use super::focus_ring_shadow;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Controlled like a React input: pass `checked`, update it in `on_change`.
///
/// ```ignore
/// let accepted = use_state(window, cx, || false);
/// Checkbox::new("terms")
///     .checked(accepted.get(cx))
///     .label("Accept terms and conditions")
///     .on_change(move |checked, _, cx| accepted.set(*checked, cx))
/// ```
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    checked: bool,
    label: Option<SharedString>,
    disabled: bool,
    on_change: Option<EventHandler<bool>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Checkbox);

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            label: None,
            disabled: false,
            on_change: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Text next to the box. Clicking it toggles the box too.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Receives the new checked value.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let checked = self.checked;
        let ring_color = colors.ring;
        let is_interactive = !self.disabled;

        let check_box = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(16.))
            .rounded(px(4.).min(theme.radius))
            .border_1()
            .border_color(if checked {
                colors.primary
            } else {
                colors.input
            })
            .bg(if checked {
                colors.primary
            } else {
                colors.background
            })
            .shadow(super::extra_small_shadow())
            .when(checked, |check_box| {
                check_box.child(
                    Icon::new(IconName::Check)
                        .size(px(14.))
                        .color(colors.primary_foreground),
                )
            });

        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(8.))
            .text_sm()
            .rounded(px(4.))
            .child(check_box)
            .when_some(self.label, |row, label| row.child(label))
            .when(is_interactive, |row| {
                row.tab_index(0)
                    .cursor(CursorStyle::PointingHand)
                    .focus(move |style| style.shadow(focus_ring_shadow(ring_color)))
                    .when_some(self.on_change, |row, handler| {
                        row.on_click(move |_, window, cx| handler(&!checked, window, cx))
                    })
            })
            .when(!is_interactive, |row| row.opacity(0.5))
            .apply_style_overrides(&self.style_overrides)
    }
}
