//! Separator: a 1px divider.

use gpui::{div, prelude::*, px, App};

use crate::sx::SxStyled;
use crate::{component, theme::ActiveTheme};

/// Direction of a [`Separator`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SeparatorOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// `Separator::new()` for a horizontal rule; `.orientation(SeparatorOrientation::Vertical)`
/// inside a row.
#[component]
pub fn Separator(
    #[prop(optional)] orientation: SeparatorOrientation,
    cx: &mut App,
    #[sx] sx: crate::sx::Sx,
) -> impl IntoElement {
    let element = {
        let line = div().flex_none().bg(cx.theme().colors.border);
        match orientation {
            SeparatorOrientation::Horizontal => line.h(px(1.)).w_full(),
            SeparatorOrientation::Vertical => line.w(px(1.)).h_full(),
        }
    };
    element.sx(&sx)
}
