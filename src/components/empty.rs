//! Empty: the placeholder shown when there is nothing to display yet.

use gpui::{
    div, prelude::*, px, AnyElement, App, FontWeight, SharedString, StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use crate::{
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// ```ignore
/// Empty::new()
///     .icon(IconName::Folder)
///     .title("No projects yet")
///     .description("You haven't created any projects yet.")
///     .child(Button::new("create").label("Create project"))
/// ```
#[derive(IntoElement)]
pub struct Empty {
    icon: Option<IconName>,
    media: Option<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    bordered: bool,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Empty);

impl Empty {
    pub fn new() -> Self {
        Self {
            icon: None,
            media: None,
            title: None,
            description: None,
            children: Vec::new(),
            bordered: false,
            style_overrides: StyleRefinement::default(),
        }
    }

    /// An icon in a muted tile above the title.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Any element above the title instead of an icon, such as an avatar.
    pub fn media(mut self, media: impl IntoElement) -> Self {
        self.media = Some(media.into_any_element());
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Dashed outline around the whole area.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
}

impl Default for Empty {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Empty {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Empty {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex_dir()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(24.))
            .w_full()
            .p(px(48.))
            .rounded(theme.radius_large())
            .when(self.bordered, |empty| {
                empty.border_1().border_dashed().border_color(colors.border)
            })
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .items_center()
                    .gap(px(8.))
                    .max_w(px(384.))
                    .when_some(self.icon, |header, icon| {
                        header.child(
                            div()
                                .mb(px(8.))
                                .size(px(40.))
                                .flex_dir()
                                .items_center()
                                .justify_center()
                                .rounded(theme.radius_large())
                                .bg(colors.muted)
                                .child(Icon::new(icon).size(px(24.)).color(colors.foreground)),
                        )
                    })
                    .when_some(self.media, |header, media| {
                        header.child(div().mb(px(8.)).child(media))
                    })
                    .when_some(self.title, |header, title| {
                        header.child(div().text_lg().font_weight(FontWeight::MEDIUM).child(title))
                    })
                    .when_some(self.description, |header, description| {
                        header.child(
                            div()
                                .text_sm()
                                .text_center()
                                .text_color(colors.muted_foreground)
                                .child(description),
                        )
                    }),
            )
            .when(!self.children.is_empty(), |empty| {
                empty.child(
                    div()
                        .flex_dir()
                        .items_center()
                        .gap(px(8.))
                        .children(self.children),
                )
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
