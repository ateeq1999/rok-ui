//! Button: shadcn/ui's `<Button variant size>` with icons, loading state and tooltips.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ClickEvent, CursorStyle, ElementId, FontWeight, Hsla,
    Pixels, SharedString, StyleRefinement, Window,
};

use super::{extra_small_shadow, focus_ring_shadow, spinner::Spinner, tooltip::Tooltip};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::{ActiveTheme, Theme},
};

/// Visual style, matching shadcn/ui's `variant` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    /// Filled with the primary color. The main action on a screen.
    #[default]
    Primary,
    /// Filled red, for irreversible actions.
    Destructive,
    /// Bordered, transparent fill.
    Outline,
    /// Low-emphasis filled.
    Secondary,
    /// No fill until hovered.
    Ghost,
    /// Looks like a text link.
    Link,
}

/// Height and padding, matching shadcn/ui's `size` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonSize {
    /// 32px tall.
    Small,
    /// 36px tall.
    #[default]
    Medium,
    /// 40px tall.
    Large,
    /// 36px square, for icon-only buttons.
    Icon,
}

/// Where the icon sits relative to the label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconPosition {
    #[default]
    Start,
    End,
}

/// A clickable button. Focusable with Tab, activated with Enter or Space.
///
/// ```ignore
/// Button::new("save").label("Save changes").on_click(|_, _, cx| save(cx))
/// Button::new("delete").destructive().icon(IconName::Close).label("Delete")
/// Button::new("theme").ghost().icon_only(IconName::Moon).tooltip("Toggle theme")
/// ```
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<IconName>,
    icon_position: IconPosition,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    tooltip_text: Option<SharedString>,
    on_click: Option<EventHandler<ClickEvent>>,
    children: Vec<AnyElement>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Button);

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon: None,
            icon_position: IconPosition::Start,
            variant: ButtonVariant::Primary,
            size: ButtonSize::Medium,
            disabled: false,
            loading: false,
            tooltip_text: None,
            on_click: None,
            children: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Text shown on the button.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Icon shown next to the label.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Put the icon after the label.
    pub fn icon_position(mut self, icon_position: IconPosition) -> Self {
        self.icon_position = icon_position;
        self
    }

    /// Icon-only square button. Pair it with [`Button::tooltip`] so it has a name.
    pub fn icon_only(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self.size = ButtonSize::Icon;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Shorthand for `.variant(ButtonVariant::Destructive)`.
    pub fn destructive(self) -> Self {
        self.variant(ButtonVariant::Destructive)
    }

    /// Shorthand for `.variant(ButtonVariant::Outline)`.
    pub fn outline(self) -> Self {
        self.variant(ButtonVariant::Outline)
    }

    /// Shorthand for `.variant(ButtonVariant::Secondary)`.
    pub fn secondary(self) -> Self {
        self.variant(ButtonVariant::Secondary)
    }

    /// Shorthand for `.variant(ButtonVariant::Ghost)`.
    pub fn ghost(self) -> Self {
        self.variant(ButtonVariant::Ghost)
    }

    /// Shorthand for `.variant(ButtonVariant::Link)`.
    pub fn link(self) -> Self {
        self.variant(ButtonVariant::Link)
    }

    /// Shorthand for `.size(ButtonSize::Small)`.
    pub fn small(self) -> Self {
        self.size(ButtonSize::Small)
    }

    /// Shorthand for `.size(ButtonSize::Large)`.
    pub fn large(self) -> Self {
        self.size(ButtonSize::Large)
    }

    /// Dim the button and ignore clicks.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show a spinner in place of the icon and ignore clicks.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Hover tooltip. Also the accessible name for icon-only buttons.
    pub fn tooltip(mut self, tooltip_text: impl Into<SharedString>) -> Self {
        self.tooltip_text = Some(tooltip_text.into());
        self
    }

    /// Called on mouse click, or Enter / Space while focused.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct ButtonColors {
    background: Hsla,
    text: Hsla,
    hover_background: Hsla,
    border: Hsla,
}

fn button_colors(variant: ButtonVariant, theme: &Theme) -> ButtonColors {
    let colors = &theme.colors;
    let transparent = gpui::transparent_black();
    match variant {
        ButtonVariant::Primary => ButtonColors {
            background: colors.primary,
            text: colors.primary_foreground,
            hover_background: colors.primary.opacity(0.9),
            border: colors.primary,
        },
        ButtonVariant::Destructive => ButtonColors {
            background: colors.destructive,
            text: colors.destructive_foreground,
            hover_background: colors.destructive.opacity(0.9),
            border: colors.destructive,
        },
        ButtonVariant::Outline => ButtonColors {
            background: colors.background,
            text: colors.foreground,
            hover_background: colors.accent,
            border: colors.input,
        },
        ButtonVariant::Secondary => ButtonColors {
            background: colors.secondary,
            text: colors.secondary_foreground,
            hover_background: colors.secondary.opacity(0.8),
            border: colors.secondary,
        },
        ButtonVariant::Ghost => ButtonColors {
            background: transparent,
            text: colors.foreground,
            hover_background: colors.accent,
            border: transparent,
        },
        ButtonVariant::Link => ButtonColors {
            background: transparent,
            text: colors.primary,
            hover_background: transparent,
            border: transparent,
        },
    }
}

/// (height, horizontal padding, gap, icon size)
fn button_dimensions(size: ButtonSize) -> (Pixels, Pixels, Pixels, Pixels) {
    match size {
        ButtonSize::Small => (px(32.), px(12.), px(6.), px(14.)),
        ButtonSize::Medium => (px(36.), px(16.), px(8.), px(16.)),
        ButtonSize::Large => (px(40.), px(24.), px(8.), px(16.)),
        ButtonSize::Icon => (px(36.), px(0.), px(0.), px(16.)),
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let button_colors = button_colors(self.variant, theme);
        let (height, horizontal_padding, gap, icon_size) = button_dimensions(self.size);
        let is_interactive = !self.disabled && !self.loading;
        let is_link = self.variant == ButtonVariant::Link;
        let ring_color = theme.colors.ring;
        let text_color = button_colors.text;

        let leading_visual = if self.loading {
            Some(
                Spinner::new()
                    .size(icon_size)
                    .color(text_color)
                    .into_any_element(),
            )
        } else if self.icon_position == IconPosition::Start {
            self.icon.map(|icon| {
                Icon::new(icon)
                    .size(icon_size)
                    .color(text_color)
                    .into_any_element()
            })
        } else {
            None
        };
        let trailing_visual = (!self.loading && self.icon_position == IconPosition::End)
            .then_some(self.icon)
            .flatten()
            .map(|icon| Icon::new(icon).size(icon_size).color(text_color));

        let hover_background = button_colors.hover_background;
        let on_click = self.on_click;
        let tooltip_text = self.tooltip_text;

        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(gap)
            .h(height)
            .when(self.size == ButtonSize::Icon, |button| button.w(height))
            .px(horizontal_padding)
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(button_colors.border)
            .bg(button_colors.background)
            .text_color(text_color)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .when(self.variant == ButtonVariant::Outline, |button| {
                button.shadow(extra_small_shadow())
            })
            .when(is_interactive, |button| {
                button
                    .tab_index(0)
                    .cursor(CursorStyle::PointingHand)
                    .hover(move |style| {
                        if is_link {
                            style.underline()
                        } else {
                            style.bg(hover_background)
                        }
                    })
                    .focus(move |style| {
                        style
                            .border_color(ring_color)
                            .shadow(focus_ring_shadow(ring_color))
                    })
                    .when_some(on_click, |button, handler| {
                        button.on_click(move |event, window, cx| handler(event, window, cx))
                    })
            })
            .when(!is_interactive, |button| {
                button.opacity(0.5).cursor(CursorStyle::Arrow)
            })
            .when_some(tooltip_text, |button, text| {
                button.tooltip(Tooltip::text(text))
            })
            .children(leading_visual)
            .when_some(self.label, |button, label| button.child(label))
            .children(self.children)
            .children(trailing_visual)
            .apply_style_overrides(&self.style_overrides)
    }
}
