//! Separator: a 1px divider.

use gpui::{div, prelude::*};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles};

/// Direction of a [`Separator`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SeparatorOrientation {
    /// A horizontal line between stacked content.
    #[default]
    Horizontal,
    /// A vertical line between side-by-side content.
    Vertical,
}

styles! {
    SEPARATOR = {
        base: { flex: none, background: border },
        orientation(SeparatorOrientation): {
            Horizontal: { height: 0.25, width: full },
            Vertical: { width: 0.25, height: full },
        },
    }
}

/// `Separator::new()` for a horizontal rule; `.orientation(SeparatorOrientation::Vertical)`
/// inside a row.
#[component]
pub fn Separator(
    #[prop(optional)] orientation: SeparatorOrientation,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div().sx((&SEPARATOR.base, SEPARATOR.orientation(orientation), &sx))
}
