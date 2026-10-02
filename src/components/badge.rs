//! Badge: a small status label.

use gpui::{div, prelude::*, px, App, SharedString, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
    styles,
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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Badge);

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            variant: BadgeVariant::Primary,
            sx: crate::sx::Sx::new(),
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

styles! {
    BADGE = {
        root: {
            display: flex,
            flex: none,
            align: center,
            gap: 1,
            padding_x: 2,
            padding_y: 0.5,
            radius: md,
            border: 1,
            text: xs,
            font: medium,
            whitespace: nowrap,
        },
        variant(BadgeVariant): {
            Primary: { background: primary, color: primary_foreground, border_color: primary },
            Secondary: {
                background: secondary,
                color: secondary_foreground,
                border_color: secondary,
            },
            Destructive: {
                background: destructive,
                color: destructive_foreground,
                border_color: destructive,
            },
            Outline: { background: transparent, color: foreground, border_color: border },
        },
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Icons do not inherit text color in GPUI.
        let colors = &cx.theme().colors;
        let icon_color = match self.variant {
            BadgeVariant::Primary => colors.primary_foreground,
            BadgeVariant::Secondary => colors.secondary_foreground,
            BadgeVariant::Destructive => colors.destructive_foreground,
            BadgeVariant::Outline => colors.foreground,
        };
        div()
            .sx((&BADGE.root, BADGE.variant(self.variant), &self.sx))
            .when_some(self.icon, |badge, icon| {
                badge.child(Icon::new(icon).size(px(12.)).color(icon_color))
            })
            .child(self.label)
            .apply_style_overrides(&self.style_overrides)
    }
}
