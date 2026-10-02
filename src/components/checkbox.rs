//! Checkbox: a controlled boolean with an optional label.

use std::rc::Rc;

use gpui::{div, prelude::*, px, App, ElementId, SharedString, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
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
    sx: crate::sx::Sx,
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
            sx: crate::sx::Sx::new(),
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

styles! {
    CHECKBOX = {
        row: { display: flex, align: center, gap: 2, text: sm, radius: 1 },
        interactive: { cursor: pointer, focus: { shadow: ring } },
        inert: { opacity: 0.5 },
        check_box: {
            display: flex,
            flex: none,
            align: center,
            justify: center,
            size: 4,
            border: 1,
            border_color: input,
            background: background,
            shadow: xs,
        },
        checked: { border_color: primary, background: primary },
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let checked = self.checked;
        let is_interactive = !self.disabled;

        let check_box = div()
            .rounded(px(4.).min(theme.radius))
            .sx((&CHECKBOX.check_box, checked.then_some(&CHECKBOX.checked)))
            .when(checked, |check_box| {
                check_box.child(
                    Icon::new(IconName::Check)
                        .size(px(14.))
                        .color(theme.colors.primary_foreground),
                )
            });

        div()
            .id(self.id)
            .child(check_box)
            .when_some(self.label, |row, label| row.child(label))
            .when(is_interactive, |row| {
                row.tab_index(0).when_some(self.on_change, |row, handler| {
                    row.on_click(move |_, window, cx| handler(&!checked, window, cx))
                })
            })
            // The caller's `sx` is merged into the same call: GPUI allows a single
            // hover / focus style per element.
            .sx((
                &CHECKBOX.row,
                if is_interactive {
                    &CHECKBOX.interactive
                } else {
                    &CHECKBOX.inert
                },
                &self.sx,
            ))
            .apply_style_overrides(&self.style_overrides)
    }
}
