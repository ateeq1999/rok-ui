//! Label: the caption for a form control.

use gpui::{div, prelude::*, SharedString, StyleRefinement};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles, styles::ApplyStyleOverrides};

styles! {
    LABEL = {
        root: { text: sm, font: medium, line_height: 3.5, color: foreground },
        disabled: { opacity: 0.5 },
    }
}

/// `Label::new("Email")`. Use `.disabled(true)` to dim it with its control.
#[component]
pub fn Label(
    text: SharedString,
    #[prop(optional)] disabled: bool,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&LABEL.root, disabled.then_some(&LABEL.disabled), &sx))
        .child(crate::components::bidi_text::text(text))
        .apply_style_overrides(&style_overrides)
}
