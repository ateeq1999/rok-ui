//! Progress: a horizontal bar showing completion.

use gpui::{div, prelude::*, relative, StyleRefinement};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles, styles::ApplyStyleOverrides};

styles! {
    PROGRESS = {
        // A flex row, so the bar fills from the starting side (the right in RTL).
        track: {
            position: relative,
            display: flex,
            width: full,
            height: 2,
            radius: full,
            overflow: hidden,
            background: primary/20,
        },
        bar: { height: full, radius: full, background: primary },
    }
}

/// `Progress::new(66.0)`: `value` is a percentage from 0 to 100 (clamped).
#[component]
pub fn Progress(
    value: f32,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    let fraction = (value / 100.).clamp(0., 1.);
    div()
        .sx((&PROGRESS.track, &sx))
        .child(div().sx(&PROGRESS.bar).w(relative(fraction)))
        .apply_style_overrides(&style_overrides)
}
