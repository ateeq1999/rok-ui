//! Badge: a small status label.

use gpui::{div, prelude::*, px, App, FontWeight, SharedString, StyleRefinement, Window};

use super::direction::DirectionalStyled;
use crate::{
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Visual style, matching shadcn/ui's badge variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeVariant {
    #[default]
    Primary,
    Secondary,
    Destructive,
    Outline,
}

/// `Badge::new("New")`, `Badge::new("Failed").variant(BadgeVariant::Destructive)`.
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    icon: Option<IconName>,
    variant: BadgeVariant,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Badge);

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            variant: BadgeVariant::Primary,
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (background, text, border) = match self.variant {
            BadgeVariant::Primary => (colors.primary, colors.primary_foreground, colors.primary),
            BadgeVariant::Secondary => (
                colors.secondary,
                colors.secondary_foreground,
                colors.secondary,
            ),
            BadgeVariant::Destructive => (
                colors.destructive,
                colors.destructive_foreground,
                colors.destructive,
            ),
            BadgeVariant::Outline => (gpui::transparent_black(), colors.foreground, colors.border),
        };
        div()
            .flex_dir()
            .flex_none()
            .items_center()
            .gap(px(4.))
            .px(px(8.))
            .py(px(2.))
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(border)
            .bg(background)
            .text_color(text)
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .when_some(self.icon, |badge, icon| {
                badge.child(Icon::new(icon).size(px(12.)).color(text))
            })
            .child(self.label)
            .apply_style_overrides(&self.style_overrides)
    }
}
