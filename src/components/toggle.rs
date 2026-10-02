//! Toggle and ToggleGroup: buttons that stay pressed.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, FontWeight, Pixels, SharedString,
    StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::extra_small_shadow;
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::{ApplyStyleOverrides, ComponentSize},
    theme::ActiveTheme,
};

/// Visual style of a toggle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToggleVariant {
    /// Transparent until pressed or hovered.
    #[default]
    Default,
    /// Bordered.
    Outline,
}

fn toggle_height(size: ComponentSize) -> Pixels {
    match size {
        ComponentSize::Small => px(32.),
        ComponentSize::Medium => px(36.),
        ComponentSize::Large => px(40.),
    }
}

/// A two-state button. Controlled: pass `pressed`, update it in `on_change`.
///
/// ```ignore
/// Toggle::new("bold").icon(IconName::Bold).tooltip("Bold")
///     .pressed(bold).on_change(move |pressed, _, cx| set_bold(*pressed, cx))
/// ```
#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    pressed: bool,
    icon: Option<IconName>,
    label: Option<SharedString>,
    variant: ToggleVariant,
    size: ComponentSize,
    disabled: bool,
    tooltip_text: Option<SharedString>,
    on_change: Option<EventHandler<bool>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Toggle);

impl Toggle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            pressed: false,
            icon: None,
            label: None,
            variant: ToggleVariant::Default,
            size: ComponentSize::Medium,
            disabled: false,
            tooltip_text: None,
            on_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Shorthand for `.variant(ToggleVariant::Outline)`.
    pub fn outline(self) -> Self {
        self.variant(ToggleVariant::Outline)
    }

    pub fn size(mut self, size: ComponentSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip_text = Some(text.into());
        self
    }

    /// Receives the new pressed value.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // The component's own interaction styles, merged with the caller's `sx` in one
        // call (GPUI allows a single hover / focus style per element).
        let own_states = if self.disabled {
            crate::sx::Sx::new()
        } else {
            crate::sx::Sx::new()
                .hover(|state| {
                    state
                        .bg(crate::sx::ColorToken::Muted)
                        .text_color(crate::sx::ColorToken::MutedForeground)
                })
                .focus(|state| state.border_color(crate::sx::ColorToken::Ring))
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let height = toggle_height(self.size);
        let pressed = self.pressed;
        let text_color = if pressed {
            colors.accent_foreground
        } else {
            colors.foreground
        };
        let icon_only = self.label.is_none();

        div()
            .id(self.id)
            .flex_dir()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .h(height)
            .min_w(height)
            .px(if icon_only { px(0.) } else { px(10.) })
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(match self.variant {
                ToggleVariant::Default => gpui::transparent_black(),
                ToggleVariant::Outline => colors.input,
            })
            .when(self.variant == ToggleVariant::Outline, |toggle| {
                toggle.shadow(extra_small_shadow())
            })
            .when(pressed, |toggle| toggle.bg(colors.accent))
            .text_color(text_color)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .when(!self.disabled, |toggle| {
                toggle
                    .tab_index(0)
                    .cursor(CursorStyle::PointingHand)
                    .when_some(self.on_change, |toggle, handler| {
                        toggle.on_click(move |_, window, cx| handler(&!pressed, window, cx))
                    })
            })
            .when(self.disabled, |toggle| toggle.opacity(0.5))
            .when_some(self.tooltip_text, |toggle, text| {
                toggle.tooltip(super::Tooltip::text(text))
            })
            .when_some(self.icon, |toggle, icon| {
                toggle.child(Icon::new(icon).size(px(16.)).color(text_color))
            })
            .when_some(self.label, |toggle, label| toggle.child(label))
            .sx((&own_states, &self.sx))
            .apply_style_overrides(&self.style_overrides)
    }
}

struct ToggleGroupItem {
    value: SharedString,
    icon: Option<IconName>,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
    disabled: bool,
}

/// A row of [`Toggle`]s. With `.multiple(true)` any number can be pressed;
/// otherwise pressing one releases the others (pressing it again clears it).
///
/// ```ignore
/// ToggleGroup::new("formatting")
///     .multiple(true)
///     .outline()
///     .icon_item("bold", IconName::Bold, "Bold")
///     .icon_item("italic", IconName::Italic, "Italic")
///     .value(pressed_values.clone())
///     .on_change(move |values, _, cx| set_pressed_values(values.clone(), cx))
/// ```
#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    items: Vec<ToggleGroupItem>,
    value: Vec<SharedString>,
    multiple: bool,
    variant: ToggleVariant,
    size: ComponentSize,
    disabled: bool,
    on_change: Option<EventHandler<Vec<SharedString>>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ToggleGroup);

impl ToggleGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            value: Vec::new(),
            multiple: false,
            variant: ToggleVariant::Default,
            size: ComponentSize::Medium,
            disabled: false,
            on_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A text item.
    pub fn item(mut self, value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        self.items.push(ToggleGroupItem {
            value: value.into(),
            icon: None,
            label: Some(label.into()),
            tooltip: None,
            disabled: false,
        });
        self
    }

    /// An icon-only item; `tooltip` doubles as its name.
    pub fn icon_item(
        mut self,
        value: impl Into<SharedString>,
        icon: IconName,
        tooltip: impl Into<SharedString>,
    ) -> Self {
        self.items.push(ToggleGroupItem {
            value: value.into(),
            icon: Some(icon),
            label: None,
            tooltip: Some(tooltip.into()),
            disabled: false,
        });
        self
    }

    /// Disable the item added last.
    pub fn disable_last(mut self) -> Self {
        if let Some(item) = self.items.last_mut() {
            item.disabled = true;
        }
        self
    }

    /// The pressed items' values.
    pub fn value(mut self, value: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.value = value.into_iter().map(Into::into).collect();
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = ToggleVariant::Outline;
        self
    }

    pub fn size(mut self, size: ComponentSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Receives the pressed values after a change.
    pub fn on_change(
        mut self,
        handler: impl Fn(&Vec<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let radius = theme.radius_medium();
        let is_outline = self.variant == ToggleVariant::Outline;
        let count = self.items.len();
        let value = self.value;
        let multiple = self.multiple;

        let toggles: Vec<AnyElement> = self
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let is_pressed = value.contains(&item.value);
                let on_change = self.on_change.clone();
                let current = value.clone();
                let item_value = item.value.clone();
                let toggle = Toggle::new(index)
                    .pressed(is_pressed)
                    .variant(self.variant)
                    .size(self.size)
                    .disabled(self.disabled || item.disabled)
                    .when_some(item.icon, |toggle, icon| toggle.icon(icon))
                    .when_some(item.label, |toggle, label| toggle.label(label))
                    .when_some(item.tooltip, |toggle, tooltip| toggle.tooltip(tooltip))
                    .on_change(move |pressed, window, cx| {
                        let next: Vec<SharedString> = match (multiple, *pressed) {
                            (true, true) => current
                                .iter()
                                .cloned()
                                .chain(std::iter::once(item_value.clone()))
                                .collect(),
                            (true, false) => current
                                .iter()
                                .filter(|value| **value != item_value)
                                .cloned()
                                .collect(),
                            (false, true) => vec![item_value.clone()],
                            (false, false) => Vec::new(),
                        };
                        if let Some(handler) = on_change.as_ref() {
                            handler(&next, window, cx);
                        }
                    });
                // Outlined groups join into one bordered strip.
                let toggle = if is_outline && count > 1 {
                    let toggle = if index == 0 {
                        toggle.rounded_e_none()
                    } else if index + 1 == count {
                        toggle.rounded_s_none().ms(px(-1.))
                    } else {
                        toggle.rounded_none().ms(px(-1.))
                    };
                    toggle.shadow(Vec::new())
                } else {
                    toggle
                };
                toggle.into_any_element()
            })
            .collect();

        div()
            .id(self.id)
            .flex_dir()
            .items_center()
            .when(!is_outline, |group| group.gap(px(4.)))
            .when(is_outline, |group| {
                group.rounded(radius).shadow(extra_small_shadow())
            })
            .children(toggles)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}
