//! `AspectRatio`: keeps its content at a fixed width-to-height ratio.

use gpui::{div, prelude::*, AnyElement, StyleRefinement};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles, styles::ApplyStyleOverrides};

styles! {
    ASPECT_RATIO = {
        frame: { position: relative, width: full, overflow: hidden },
        fill: { position: absolute, top: 0, left: 0, size: full, display: flex },
    }
}

/// Fills the available width and sets its height from `ratio` (width / height).
/// Children are stretched over the whole box.
///
/// ```no_run
/// # use rok_ui::{gpui, prelude::*};
/// # fn example(window: &mut Window, cx: &mut App) {
/// let photo = AspectRatio::new(16. / 9.).child(gpui::img("photo.jpg").size_full().rounded_md());
/// # }
/// ```
#[component]
pub fn AspectRatio(
    ratio: f32,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    let mut frame = div();
    frame.style().aspect_ratio = Some(ratio);
    frame
        .sx((&ASPECT_RATIO.frame, &sx))
        .child(div().sx(&ASPECT_RATIO.fill).children(children))
        .apply_style_overrides(&style_overrides)
}
