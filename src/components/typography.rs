//! Typography: shadcn/ui's text styles as components.
//!
//! ```ignore
//! div()
//!     .child(H1::new("The Joke Tax Chronicles"))
//!     .child(Lead::new("A modal dialog that interrupts the user."))
//!     .child(H2::new("The King's Plan"))
//!     .child(P::new("The king thought long and hard…"))
//!     .child(Blockquote::new("\"After all,\" he said, \"everyone enjoys a good joke.\""))
//!     .child(List::new(["1st level of puns: 5 gold coins", "2nd level of jokes: 10 gold coins"]))
//!     .child(InlineCode::new("@radix-ui/react-alert-dialog"))
//! ```

use gpui::{div, prelude::*, px, App, FontWeight, SharedString, StyleRefinement};

use crate::{component, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// Page title: 36px extra-bold.
#[component]
pub fn H1(text: SharedString, #[style] style_overrides: StyleRefinement) -> impl IntoElement {
    div()
        .text_size(px(36.))
        .line_height(px(40.))
        .font_weight(FontWeight::EXTRA_BOLD)
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Section title: 30px semibold with a rule underneath.
#[component]
pub fn H2(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    div()
        .pb(px(8.))
        .border_b_1()
        .border_color(cx.theme().colors.border)
        .text_size(px(30.))
        .line_height(px(36.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// 24px semibold.
#[component]
pub fn H3(text: SharedString, #[style] style_overrides: StyleRefinement) -> impl IntoElement {
    div()
        .text_size(px(24.))
        .line_height(px(32.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// 20px semibold.
#[component]
pub fn H4(text: SharedString, #[style] style_overrides: StyleRefinement) -> impl IntoElement {
    div()
        .text_size(px(20.))
        .line_height(px(28.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Body paragraph with relaxed line height.
#[component]
pub fn P(text: SharedString, #[style] style_overrides: StyleRefinement) -> impl IntoElement {
    div()
        .text_base()
        .line_height(px(28.))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Indented italic quote with a left rule.
#[component]
pub fn Blockquote(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    div()
        .pl(px(24.))
        .border_l_2()
        .border_color(cx.theme().colors.border)
        .italic()
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Bulleted list. Pass `ordered(true)` for numbers.
#[component]
pub fn List(
    items: Vec<SharedString>,
    #[prop(optional)] ordered: bool,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .pl(px(24.))
        .children(items.into_iter().enumerate().map(move |(index, item)| {
            let marker: SharedString = if ordered {
                format!("{}.", index + 1).into()
            } else {
                "•".into()
            };
            div()
                .flex()
                .gap(px(8.))
                .child(div().flex_none().min_w(px(12.)).child(marker))
                .child(item)
        }))
        .apply_style_overrides(&style_overrides)
}

/// Monospace code inside running text.
#[component]
pub fn InlineCode(text: SharedString, cx: &mut App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .px(px(5.))
        .py(px(2.))
        .rounded(theme.radius_small().max(px(3.)))
        .bg(theme.colors.muted)
        .font_family(theme.monospace_font_family.clone())
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
}

/// Large muted intro text.
#[component]
pub fn Lead(text: SharedString, cx: &mut App) -> impl IntoElement {
    div()
        .text_xl()
        .text_color(cx.theme().colors.muted_foreground)
        .child(text)
}

/// 18px semibold.
#[component]
pub fn Large(text: SharedString) -> impl IntoElement {
    div()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
}

/// 14px medium, tight line height.
#[component]
pub fn Small(text: SharedString) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .line_height(px(14.))
        .child(text)
}

/// 14px muted text.
#[component]
pub fn Muted(text: SharedString, cx: &mut App) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(cx.theme().colors.muted_foreground)
        .child(text)
}
