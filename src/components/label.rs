//! Label: the caption for a form control.

use gpui::{div, prelude::*, App, FontWeight, SharedString, StyleRefinement};

use crate::sx::SxStyled;
use crate::{component, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// `Label::new("Email")`. Use `.disabled(true)` to dim it with its control.
#[component]
pub fn Label(
    text: SharedString,
    #[prop(optional)] disabled: bool,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: crate::sx::Sx,
    cx: &mut App,
) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .line_height(gpui::px(14.))
        .text_color(cx.theme().colors.foreground)
        .when(disabled, |label| label.opacity(0.5))
        .child(text)
        .sx(&sx)
        .apply_style_overrides(&style_overrides)
}
