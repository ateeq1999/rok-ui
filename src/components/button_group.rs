//! `ButtonGroup`: related buttons joined into one control.

use gpui::{div, prelude::*, AnyElement, App, SharedString, StyleRefinement, Styled, Window};

use super::direction::{is_rtl, DirectionalStyled};
use crate::sx::SxStyled;
use crate::{styles, styles::ApplyStyleOverrides};

/// Direction of a [`ButtonGroup`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonGroupOrientation {
    /// Buttons side by side.
    #[default]
    Horizontal,
    /// Buttons stacked.
    Vertical,
}

/// Where an item sits in the group, which decides which corners stay rounded.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupPosition {
    Only,
    First,
    Middle,
    Last,
}

type GroupItem = Box<dyn FnOnce(GroupPosition, ButtonGroupOrientation) -> AnyElement>;

/// Joins buttons (or any styled control: inputs, selects) so they share borders
/// and only the outer corners are rounded.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let actions = ButtonGroup::new()
///     .item(Button::new("archive").outline().label("Archive"))
///     .item(Button::new("report").outline().label("Report"))
///     .item(Button::new("snooze").outline().label("Snooze"));
/// # }
/// ```
#[derive(IntoElement)]
pub struct ButtonGroup {
    items: Vec<GroupItem>,
    orientation: ButtonGroupOrientation,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ButtonGroup);

impl ButtonGroup {
    /// An empty `ButtonGroup`; add content with the builder methods.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            orientation: ButtonGroupOrientation::Horizontal,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Horizontal or vertical.
    #[must_use]
    pub fn orientation(mut self, orientation: ButtonGroupOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Stack the items vertically.
    #[must_use]
    pub fn vertical(self) -> Self {
        self.orientation(ButtonGroupOrientation::Vertical)
    }

    /// Add a control. Its inner corners are squared off to join its neighbours.
    #[must_use]
    pub fn item<Control: IntoElement + Styled + 'static>(mut self, control: Control) -> Self {
        self.items.push(Box::new(move |position, orientation| {
            let control = match (orientation, position) {
                (_, GroupPosition::Only) => control,
                (_, GroupPosition::Middle) => control.rounded_none(),
                (ButtonGroupOrientation::Horizontal, GroupPosition::First) => {
                    control.rounded_e_none()
                }
                (ButtonGroupOrientation::Horizontal, GroupPosition::Last) => {
                    control.rounded_s_none()
                }
                (ButtonGroupOrientation::Vertical, GroupPosition::First) => {
                    control.rounded_b_none()
                }
                (ButtonGroupOrientation::Vertical, GroupPosition::Last) => control.rounded_t_none(),
            };
            // Drop the border facing the previous item so neighbours share one line.
            // (A -1px overlap would do the same, but taffy then undersizes the
            // group and later siblings overlap it.)
            let control = match (orientation, position) {
                (_, GroupPosition::Only | GroupPosition::First) => control,
                (ButtonGroupOrientation::Horizontal, _) if is_rtl() => control.border_r_0(),
                (ButtonGroupOrientation::Horizontal, _) => control.border_l_0(),
                (ButtonGroupOrientation::Vertical, _) => control.border_t_0(),
            };
            control.into_any_element()
        }));
        self
    }

    /// Add a non-interactive text segment, like a unit or a prefix.
    #[must_use]
    pub fn text(self, text: impl Into<SharedString>) -> Self {
        self.item(ButtonGroupText::new(text))
    }
}

impl Default for ButtonGroup {
    fn default() -> Self {
        Self::new()
    }
}

styles! {
    BUTTON_GROUP = {
        group: { display: flex, align: stretch },
        orientation(ButtonGroupOrientation): {
            Horizontal: {},
            Vertical: { direction: column },
        },
        text: {
            display: flex,
            align: center,
            padding_x: 4,
            radius: md,
            border: 1,
            border_color: input,
            background: muted,
            text: sm,
            font: medium,
        },
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let count = self.items.len();
        let orientation = self.orientation;
        let children = self.items.into_iter().enumerate().map(|(index, item)| {
            let position = match (index, count) {
                (_, 1) => GroupPosition::Only,
                (0, _) => GroupPosition::First,
                (index, count) if index + 1 == count => GroupPosition::Last,
                _ => GroupPosition::Middle,
            };
            item(position, orientation)
        });
        div()
            .sx((
                &BUTTON_GROUP.group,
                BUTTON_GROUP.orientation(orientation),
                &self.sx,
            ))
            .children(children)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// A muted text box that sits inside a [`ButtonGroup`].
#[derive(IntoElement)]
pub struct ButtonGroupText {
    text: SharedString,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ButtonGroupText);

impl ButtonGroupText {
    /// A text segment inside a button group.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ButtonGroupText {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .sx((&BUTTON_GROUP.text, &self.sx))
            .child(crate::components::bidi_text::text(self.text))
            .apply_style_overrides(&self.style_overrides)
    }
}
