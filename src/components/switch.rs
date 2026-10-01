//! Switch: an on/off toggle.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, CursorStyle, ElementId, SharedString, StyleRefinement, Window,
};

use super::focus_ring_shadow;
use crate::{hooks::EventHandler, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// Controlled like [`super::Checkbox`]: pass `checked`, update it in `on_change`.
///
/// ```ignore
/// Switch::new("airplane-mode")
///     .checked(enabled)
///     .label("Airplane mode")
///     .on_change(move |checked, _, cx| airplane_mode.set(*checked, cx))
/// ```
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    label: Option<SharedString>,
    disabled: bool,
    on_change: Option<EventHandler<bool>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Switch);

impl Switch {
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

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let checked = self.checked;
        let ring_color = colors.ring;
        let is_interactive = !self.disabled;

        let track_width = px(32.);
        let track_height = px(18.);
        let thumb_size = px(14.);
        let thumb_inset = px(1.);

        let thumb_color = if theme.mode.is_dark() && !checked {
            colors.foreground
        } else {
            colors.background
        };

        let track = div()
            .flex()
            .flex_none()
            .items_center()
            .w(track_width)
            .h(track_height)
            .px(thumb_inset)
            .rounded_full()
            .border_1()
            .border_color(gpui::transparent_black())
            .bg(if checked {
                colors.primary
            } else {
                colors.input
            })
            .when(checked, |track| track.justify_end())
            .child(div().size(thumb_size).rounded_full().bg(thumb_color));

        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(8.))
            .text_sm()
            .child(track)
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
            .rounded_full()
            .apply_style_overrides(&self.style_overrides)
    }
}
