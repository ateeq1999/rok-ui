//! Switch: an on/off toggle.

use std::rc::Rc;

use gpui::{div, prelude::*, App, ElementId, SharedString, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{hooks::EventHandler, styles, styles::ApplyStyleOverrides, theme::ActiveTheme};

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
    sx: crate::sx::Sx,
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
            sx: crate::sx::Sx::new(),
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

styles! {
    SWITCH = {
        row: { display: flex, align: center, gap: 2, text: sm, radius: full },
        interactive: { cursor: pointer, focus: { shadow: ring } },
        inert: { opacity: 0.5 },
        track: {
            display: flex,
            flex: none,
            align: center,
            width: 8,
            height: 4.5,
            padding_x: 0.25,
            radius: full,
            border: 1,
            border_color: transparent,
            background: input,
        },
        track_checked: { background: primary, justify: end },
        thumb: { size: 3.5, radius: full },
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let checked = self.checked;
        let is_interactive = !self.disabled;
        let thumb_color = if theme.mode.is_dark() && !checked {
            theme.colors.foreground
        } else {
            theme.colors.background
        };

        let track = div()
            .sx((&SWITCH.track, checked.then_some(&SWITCH.track_checked)))
            .child(div().sx(&SWITCH.thumb).bg(thumb_color));

        div()
            .id(self.id)
            .child(track)
            .when_some(self.label, |row, label| {
                row.child(crate::components::bidi_text::text(label))
            })
            .when(is_interactive, |row| {
                row.tab_index(0).when_some(self.on_change, |row, handler| {
                    row.on_click(move |_, window, cx| handler(&!checked, window, cx))
                })
            })
            // The caller's `sx` is merged into the same call: GPUI allows a single
            // hover / focus style per element.
            .sx((
                &SWITCH.row,
                if is_interactive {
                    &SWITCH.interactive
                } else {
                    &SWITCH.inert
                },
                &self.sx,
            ))
            .apply_style_overrides(&self.style_overrides)
    }
}
