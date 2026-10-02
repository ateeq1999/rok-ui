//! ButtonGroup: related buttons joined into one control.

use gpui::{div, prelude::*, px, AnyElement, App, SharedString, StyleRefinement, Styled, Window};

use super::direction::DirectionalStyled;
use crate::sx::SxStyled;
use crate::{styles::ApplyStyleOverrides, theme::ActiveTheme};

/// Direction of a [`ButtonGroup`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonGroupOrientation {
    #[default]
    Horizontal,
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
/// ```ignore
/// ButtonGroup::new()
///     .item(Button::new("archive").outline().label("Archive"))
///     .item(Button::new("report").outline().label("Report"))
///     .item(Button::new("snooze").outline().label("Snooze"))
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
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            orientation: ButtonGroupOrientation::Horizontal,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn orientation(mut self, orientation: ButtonGroupOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Stack the items vertically.
    pub fn vertical(self) -> Self {
        self.orientation(ButtonGroupOrientation::Vertical)
    }

    /// Add a control. Its inner corners are squared off to join its neighbours.
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
            // Overlap borders so adjacent outlines read as one line.
            let control = match (orientation, position) {
                (_, GroupPosition::Only | GroupPosition::First) => control,
                (ButtonGroupOrientation::Horizontal, _) => control.ms(px(-1.)),
                (ButtonGroupOrientation::Vertical, _) => control.mt(px(-1.)),
            };
            control.into_any_element()
        }));
        self
    }

    /// Add a non-interactive text segment, like a unit or a prefix.
    pub fn text(self, text: impl Into<SharedString>) -> Self {
        self.item(ButtonGroupText::new(text))
    }
}

impl Default for ButtonGroup {
    fn default() -> Self {
        Self::new()
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
            .flex_dir()
            .when(orientation == ButtonGroupOrientation::Vertical, |group| {
                group.flex_col()
            })
            .map(|group| {
                let mut group = group;
                group.style().align_items = Some(gpui::AlignItems::Stretch);
                group
            })
            .children(children)
            .sx(&self.sx)
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
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ButtonGroupText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex_dir()
            .items_center()
            .px(px(16.))
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(theme.colors.input)
            .bg(theme.colors.muted)
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .child(self.text)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}
