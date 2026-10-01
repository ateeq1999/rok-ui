//! Progress: a horizontal bar showing completion.

use gpui::{div, prelude::*, px, relative, App, StyleRefinement};

use crate::{component, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// `Progress::new(66.0)`: `value` is a percentage from 0 to 100 (clamped).
#[component]
pub fn Progress(
    value: f32,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    let colors = &cx.theme().colors;
    let fraction = (value / 100.).clamp(0., 1.);
    div()
        .relative()
        .w_full()
        .h(px(8.))
        .rounded_full()
        .overflow_hidden()
        .bg(colors.primary.opacity(0.2))
        .child(
            div()
                .h_full()
                .w(relative(fraction))
                .rounded_full()
                .bg(colors.primary),
        )
        .apply_style_overrides(&style_overrides)
}
