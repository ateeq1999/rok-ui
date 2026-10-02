//! InputGroup: a text field with addons (icons, text, buttons) inside its border.

use gpui::{div, prelude::*, px, AnyElement, App, Entity, SharedString, StyleRefinement, Window};

use super::direction::DirectionalStyled;
use super::{
    focus_ring_outline,
    input::{Input, InputState, Textarea},
};
use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// ```ignore
/// InputGroup::new(&url)
///     .leading_text("https://")
///     .trailing_text(".com")
///
/// InputGroup::new(&query)
///     .leading_icon(IconName::Search)
///     .trailing(Kbd::new("⌘K"))
///
/// // A multi-line state renders a textarea; `block_end` adds a toolbar under it.
/// InputGroup::new(&prompt)
///     .block_end(Button::new("send").small().icon_only(IconName::Send).ml_auto())
/// ```
#[derive(IntoElement)]
pub struct InputGroup {
    state: Entity<InputState>,
    leading: Vec<AnyElement>,
    trailing: Vec<AnyElement>,
    block_start: Vec<AnyElement>,
    block_end: Vec<AnyElement>,
    disabled: bool,
    invalid: bool,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(InputGroup);

impl InputGroup {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            leading: Vec::new(),
            trailing: Vec::new(),
            block_start: Vec::new(),
            block_end: Vec::new(),
            disabled: false,
            invalid: false,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Any element before the text, such as a button or a spinner.
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading.push(element.into_any_element());
        self
    }

    /// Any element after the text.
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing.push(element.into_any_element());
        self
    }

    pub fn leading_icon(self, icon: IconName) -> Self {
        self.leading(AddonIcon(icon))
    }

    pub fn trailing_icon(self, icon: IconName) -> Self {
        self.trailing(AddonIcon(icon))
    }

    /// Muted text before the field, like `https://` or `$`.
    pub fn leading_text(self, text: impl Into<SharedString>) -> Self {
        self.leading(AddonText(text.into()))
    }

    /// Muted text after the field, like `.com` or `USD`.
    pub fn trailing_text(self, text: impl Into<SharedString>) -> Self {
        self.trailing(AddonText(text.into()))
    }

    /// A full-width row above the field.
    pub fn block_start(mut self, element: impl IntoElement) -> Self {
        self.block_start.push(element.into_any_element());
        self
    }

    /// A full-width row below the field (a textarea toolbar, a character count).
    pub fn block_end(mut self, element: impl IntoElement) -> Self {
        self.block_end.push(element.into_any_element());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

#[derive(IntoElement)]
struct AddonIcon(IconName);

impl RenderOnce for AddonIcon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        Icon::new(self.0)
            .size(px(16.))
            .color(cx.theme().colors.muted_foreground)
    }
}

#[derive(IntoElement)]
struct AddonText(SharedString);

impl RenderOnce for AddonText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_sm()
            .text_color(cx.theme().colors.muted_foreground)
            .whitespace_nowrap()
            .child(self.0)
    }
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let state = self.state.read(cx);
        let is_focused = state.focus_handle_ref().contains_focused(window, cx);
        let is_multiline = state.is_multiline();
        let accent = if self.invalid {
            colors.destructive_text
        } else {
            colors.ring
        };

        let control = if is_multiline {
            Textarea::new(&self.state)
                .disabled(self.disabled)
                .without_focus_ring()
                .border_0()
                .rounded_none()
                .into_any_element()
        } else {
            Input::new(&self.state)
                .disabled(self.disabled)
                .without_focus_ring()
                .border_0()
                .rounded_none()
                .flex_1()
                .when(!self.leading.is_empty(), |input| input.ps(px(0.)))
                .when(!self.trailing.is_empty(), |input| input.pe(px(0.)))
                .into_any_element()
        };

        let addon_row = |children: Vec<AnyElement>| {
            div()
                .flex_dir()
                .items_center()
                .gap(px(8.))
                .px(px(12.))
                .children(children)
        };
        let has_leading = !self.leading.is_empty();
        let has_trailing = !self.trailing.is_empty();
        let main_row = div()
            .flex_dir()
            .items_center()
            .w_full()
            .when(has_leading, |row| {
                row.child(addon_row(self.leading).pe(px(8.)))
            })
            .child(div().flex_1().min_w_0().child(control))
            .when(has_trailing, |row| {
                row.child(addon_row(self.trailing).ps(px(8.)))
            });

        div()
            .flex_dir()
            .flex_col()
            .w_full()
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(if self.invalid || is_focused {
                accent
            } else {
                colors.input
            })
            .relative()
            .when(is_focused, |group| {
                group.child(focus_ring_outline(accent, theme.radius_medium()))
            })
            .when(self.disabled, |group| group.opacity(0.5))
            .when(!self.block_start.is_empty(), |group| {
                group.child(addon_row(self.block_start).pt(px(8.)))
            })
            .child(main_row)
            .when(!self.block_end.is_empty(), |group| {
                group.child(addon_row(self.block_end).pb(px(8.)))
            })
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}
