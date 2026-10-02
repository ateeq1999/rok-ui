//! Alert: an inline callout for important messages.

use gpui::{div, prelude::*, px, App, FontWeight, SharedString, StyleRefinement, Window};

use super::direction::DirectionalStyled;
use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
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

impl RenderOnce for Alert {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (title_color, description_color, border_color) = match self.variant {
            AlertVariant::Default => (
                colors.card_foreground,
                colors.muted_foreground,
                colors.border,
            ),
            AlertVariant::Destructive => (
                colors.destructive_text,
                colors.destructive_text,
                colors.destructive.opacity(0.5),
            ),
        };

        div()
            .flex_dir()
            .items_start()
            .gap(px(12.))
            .w_full()
            .px(px(16.))
            .py(px(12.))
            .rounded(theme.radius_large())
            .border_1()
            .border_color(border_color)
            .bg(colors.card)
            .when_some(self.icon, |alert, icon| {
                alert.child(Icon::new(icon).size(px(16.)).color(title_color).mt(px(2.)))
            })
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .gap(px(4.))
                    .flex_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(title_color)
                            .child(self.title),
                    )
                    .when_some(self.description, |content, description| {
                        content.child(
                            div()
                                .text_sm()
                                .text_color(description_color)
                                .child(description),
                        )
                    }),
            )
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}
