//! Card and its parts, composed exactly like shadcn/ui:
//!
//! ```ignore
//! Card::new()
//!     .child(CardHeader::new()
//!         .child(CardTitle::new("Create project"))
//!         .child(CardDescription::new("Deploy your new project in one click.")))
//!     .child(CardContent::new().child(/* form */))
//!     .child(CardFooter::new().child(Button::new("deploy").label("Deploy")))
//! ```
//!
//! These parts are written with `#[component]`, the same macro your own components use.

use gpui::{div, prelude::*, px, AnyElement, App, FontWeight, SharedString, StyleRefinement};

use crate::{component, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// A bordered surface that groups related content.
#[component]
pub fn Card(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(px(24.))
        .py(px(24.))
        .rounded(theme.radius_extra_large())
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.card)
        .text_color(theme.colors.card_foreground)
        .shadow(super::extra_small_shadow())
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// Title and description area at the top of a card.
#[component]
pub fn CardHeader(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .px(px(24.))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The card's heading.
#[component]
pub fn CardTitle(text: SharedString) -> impl IntoElement {
    div()
        .text_base()
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(px(20.))
        .child(text)
}

/// Secondary text under the title.
#[component]
pub fn CardDescription(text: SharedString, cx: &mut App) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(cx.theme().colors.muted_foreground)
        .child(text)
}

/// The card's main content.
#[component]
pub fn CardContent(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .px(px(24.))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// Actions row at the bottom of a card.
#[component]
pub fn CardFooter(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(24.))
        .children(children)
        .apply_style_overrides(&style_overrides)
}
