//! Alert: an inline callout for important messages.

use gpui::{div, prelude::*, px, App, SharedString, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Visual style, matching shadcn/ui's alert variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AlertVariant {
    #[default]
    Default,
    Destructive,
}

/// ```ignore
/// Alert::new("Heads up!")
///     .icon(IconName::Info)
///     .description("You can add components to your app using the CLI.")
/// ```
#[derive(IntoElement)]
pub struct Alert {
    title: SharedString,
    description: Option<SharedString>,
    icon: Option<IconName>,
    variant: AlertVariant,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Alert);

impl Alert {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
            variant: AlertVariant::Default,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Shorthand for `.variant(AlertVariant::Destructive)`.
    pub fn destructive(self) -> Self {
        self.variant(AlertVariant::Destructive)
    }
}

styles! {
    ALERT = {
        root: {
            display: flex,
            align: start,
            gap: 3,
            width: full,
            padding_x: 4,
            padding_y: 3,
            radius: lg,
            border: 1,
            background: card,
        },
        content: { display: flex, direction: column, gap: 1, flex: 1 },
        title: { text: sm, font: medium },
        description: { text: sm },
        variant(AlertVariant): {
            Default: { border_color: border, color: card_foreground },
            Destructive: { border_color: destructive/50, color: destructive_text },
        },
        description_color(AlertVariant): {
            Default: { color: muted_foreground },
            Destructive: { color: destructive_text },
        },
    }
}

impl RenderOnce for Alert {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Icons do not inherit text color in GPUI.
        let colors = &cx.theme().colors;
        let icon_color = match self.variant {
            AlertVariant::Default => colors.card_foreground,
            AlertVariant::Destructive => colors.destructive_text,
        };

        div()
            .sx((&ALERT.root, ALERT.variant(self.variant), &self.sx))
            .when_some(self.icon, |alert, icon| {
                alert.child(Icon::new(icon).size(px(16.)).color(icon_color).mt(px(2.)))
            })
            .child(
                div()
                    .sx(&ALERT.content)
                    .child(
                        div()
                            .sx(&ALERT.title)
                            .child(crate::components::bidi_text::text(self.title)),
                    )
                    .when_some(self.description, |content, description| {
                        content.child(
                            div()
                                .sx((&ALERT.description, ALERT.description_color(self.variant)))
                                .child(crate::components::bidi_text::text(description)),
                        )
                    }),
            )
            .apply_style_overrides(&self.style_overrides)
    }
}
