//! InputGroup: a text field with addons (icons, text, buttons) inside its border.

use gpui::{div, prelude::*, px, AnyElement, App, Entity, SharedString, StyleRefinement, Window};

use super::{
    focus_ring_outline,
    input::{Input, InputState, Textarea},
};
use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
    styles,
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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().sx(&INPUT_GROUP.addon_text).child(self.0)
    }
}

styles! {
    INPUT_GROUP = {
        group: {
            position: relative,
            display: flex,
            direction: column,
            width: full,
            radius: md,
            border: 1,
            border_color: input,
        },
        focused: { border_color: ring },
        invalid: { border_color: destructive_text },
        disabled: { opacity: 0.5 },
        // The field drops its own frame and sits flush against the addons.
        control: { border: 0, radius: none },
        input: { flex: 1 },
        input_after_leading: { padding_start: 0 },
        input_before_trailing: { padding_end: 0 },
        main_row: { display: flex, align: center, width: full },
        control_slot: { flex: 1, min_width: 0 },
        addons: { display: flex, align: center, gap: 2, padding_x: 3 },
        leading: { padding_end: 2 },
        trailing: { padding_start: 2 },
        block_start: { padding_top: 2 },
        block_end: { padding_bottom: 2 },
        addon_text: { text: sm, color: muted_foreground, whitespace: nowrap },
    }
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let state = self.state.read(cx);
        let is_focused = state.focus_handle_ref().contains_focused(window, cx);
        let is_multiline = state.is_multiline();
        let accent = if self.invalid {
            theme.colors.destructive_text
        } else {
            theme.colors.ring
        };
        let has_leading = !self.leading.is_empty();
        let has_trailing = !self.trailing.is_empty();

        let control = if is_multiline {
            Textarea::new(&self.state)
                .disabled(self.disabled)
                .without_focus_ring()
                .sx(&INPUT_GROUP.control)
                .into_any_element()
        } else {
            Input::new(&self.state)
                .disabled(self.disabled)
                .without_focus_ring()
                .sx((
                    &INPUT_GROUP.control,
                    &INPUT_GROUP.input,
                    has_leading.then_some(&INPUT_GROUP.input_after_leading),
                    has_trailing.then_some(&INPUT_GROUP.input_before_trailing),
                ))
                .into_any_element()
        };

        let addon_row = |children: Vec<AnyElement>, edge: &crate::sx::Sx| {
            div().sx((&INPUT_GROUP.addons, edge)).children(children)
        };
        let main_row = div()
            .sx(&INPUT_GROUP.main_row)
            .when(has_leading, |row| {
                row.child(addon_row(self.leading, &INPUT_GROUP.leading))
            })
            .child(div().sx(&INPUT_GROUP.control_slot).child(control))
            .when(has_trailing, |row| {
                row.child(addon_row(self.trailing, &INPUT_GROUP.trailing))
            });

        div()
            .sx((
                &INPUT_GROUP.group,
                is_focused.then_some(&INPUT_GROUP.focused),
                self.invalid.then_some(&INPUT_GROUP.invalid),
                self.disabled.then_some(&INPUT_GROUP.disabled),
                &self.sx,
            ))
            .when(is_focused, |group| {
                group.child(focus_ring_outline(accent, theme.radius_medium()))
            })
            .when(!self.block_start.is_empty(), |group| {
                group.child(addon_row(self.block_start, &INPUT_GROUP.block_start))
            })
            .child(main_row)
            .when(!self.block_end.is_empty(), |group| {
                group.child(addon_row(self.block_end, &INPUT_GROUP.block_end))
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
