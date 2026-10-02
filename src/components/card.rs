//! Card and its parts, composed exactly like shadcn/ui:
//!
//! ```ignore
//! view! {
//!     Card {
//!         CardHeader {
//!             CardTitle("Create project")
//!             CardDescription("Deploy your new project in one click.")
//!         }
//!         CardContent { /* form */ }
//!         CardFooter { Button("deploy", label = "Deploy") }
//!     }
//! }
//! ```
//!
//! These parts are written with `#[component]` and `styles!`, the same tools
//! your own components use. Every part takes `.sx(..)` overrides.

use gpui::{div, prelude::*, AnyElement, SharedString, StyleRefinement};

use crate::{
    component, styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

styles! {
    pub(crate) CARD = {
        root: {
            display: flex,
            direction: column,
            gap: 6,
            padding_y: 6,
            radius: xl,
            border: 1,
            border_color: border,
            background: card,
            color: card_foreground,
            shadow: xs,
        },
        header: { display: flex, direction: column, gap: 1.5, padding_x: 6 },
        title: { text: base, font: semibold, line_height: 5 },
        description: { text: sm, color: muted_foreground },
        content: { display: flex, direction: column, gap: 4, padding_x: 6 },
        footer: { display: flex, align: center, gap: 2, padding_x: 6 },
    }
}

/// A bordered surface that groups related content.
#[component]
pub fn Card(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&CARD.root, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// Title and description area at the top of a card.
#[component]
pub fn CardHeader(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&CARD.header, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The card's heading.
#[component]
pub fn CardTitle(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&CARD.title, &sx)).child(text)
}

/// Secondary text under the title.
#[component]
pub fn CardDescription(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&CARD.description, &sx)).child(text)
}

/// The card's main content.
#[component]
pub fn CardContent(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&CARD.content, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// Actions row at the bottom of a card.
#[component]
pub fn CardFooter(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&CARD.footer, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}
