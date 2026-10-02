//! Button: shadcn/ui's `<Button variant size>` with icons, loading state and tooltips.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ClickEvent, ElementId, Hsla, Pixels, SharedString,
    StyleRefinement, Window,
};

use super::{spinner::Spinner, tooltip::Tooltip};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
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
    sx: Sx,
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
            sx: Sx::new(),
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

styles! {
    BUTTON = {
        base: {
            display: flex,
            flex: none,
            align: center,
            justify: center,
            radius: md,
            border: 1,
            text: sm,
            font: medium,
            whitespace: nowrap,
        },
        interactive: { cursor: pointer, focus: { border_color: ring, shadow: ring } },
        inert: { opacity: 0.5, cursor: default },
        variant(ButtonVariant): {
            Primary: { background: primary, color: primary_foreground, border_color: primary },
            Destructive: {
                background: destructive,
                color: destructive_foreground,
                border_color: destructive,
            },
            Outline: { background: background, color: foreground, border_color: input, shadow: xs },
            Secondary: { background: secondary, color: secondary_foreground, border_color: secondary },
            Ghost: { background: transparent, color: foreground, border_color: transparent },
            Link: { background: transparent, color: primary, border_color: transparent },
        },
        // Hover styles apply only while the button is interactive.
        hover(ButtonVariant): {
            Primary: { hover: { background: primary/90 } },
            Destructive: { hover: { background: destructive/90 } },
            Outline: { hover: { background: accent } },
            Secondary: { hover: { background: secondary/80 } },
            Ghost: { hover: { background: accent } },
            Link: { hover: { underline: true } },
        },
        size(ButtonSize): {
            Small: { height: 8, padding_x: 3, gap: 1.5 },
            Medium: { height: 9, padding_x: 4, gap: 2 },
            Large: { height: 10, padding_x: 6, gap: 2 },
            Icon: { size: 9, padding_x: 0, gap: 0 },
        },
    }
}

/// Icons do not inherit text color in GPUI, so they get the variant's text color.
fn icon_color(variant: ButtonVariant, theme: &Theme) -> Hsla {
    let colors = &theme.colors;
    match variant {
        ButtonVariant::Primary => colors.primary_foreground,
        ButtonVariant::Destructive => colors.destructive_foreground,
        ButtonVariant::Outline | ButtonVariant::Ghost => colors.foreground,
        ButtonVariant::Secondary => colors.secondary_foreground,
        ButtonVariant::Link => colors.primary,
    }
}

fn icon_size(size: ButtonSize) -> Pixels {
    match size {
        ButtonSize::Small => px(14.),
        _ => px(16.),
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_interactive = !self.disabled && !self.loading;
        let text_color = icon_color(self.variant, cx.theme());
        let icon_size = icon_size(self.size);

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

        let on_click = self.on_click;
        let tooltip_text = self.tooltip_text;

        div()
            .id(self.id)
            .sx((
                &BUTTON.base,
                BUTTON.variant(self.variant),
                BUTTON.size(self.size),
                is_interactive.then(|| (BUTTON.hover(self.variant), &BUTTON.interactive)),
                (!is_interactive).then_some(&BUTTON.inert),
                &self.sx,
            ))
            .when(is_interactive, |button| {
                button.tab_index(0).when_some(on_click, |button, handler| {
                    button.on_click(move |event, window, cx| handler(event, window, cx))
                })
            })
            .when_some(tooltip_text, |button, text| {
                button.tooltip(Tooltip::text(text))
            })
            .children(leading_visual)
            .when_some(self.label, |button, label| {
                button.child(crate::components::bidi_text::text(label))
            })
            .children(self.children)
            .children(trailing_visual)
            .apply_style_overrides(&self.style_overrides)
    }
}
