//! AspectRatio: keeps its content at a fixed width-to-height ratio.

use gpui::{div, prelude::*, AnyElement, StyleRefinement};

use super::direction::DirectionalStyled;
use crate::{component, styles::ApplyStyleOverrides};

/// Fills the available width and sets its height from `ratio` (width / height).
/// Children are stretched over the whole box.
///
/// ```ignore
/// AspectRatio::new(16. / 9.).child(img("photo.jpg").size_full().rounded_md())
/// ```
#[component]
pub fn AspectRatio(
    ratio: f32,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    let mut frame = div().relative().w_full().overflow_hidden();
    frame.style().aspect_ratio = Some(ratio);
    frame
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex_dir()
                .children(children),
        )
        .apply_style_overrides(&style_overrides)
}
