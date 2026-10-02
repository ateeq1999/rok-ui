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

use gpui::{div, prelude::*, px, App, SharedString, StyleRefinement};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles, styles::ApplyStyleOverrides, theme::ActiveTheme};

styles! {
    TYPOGRAPHY = {
        h1: { text: 36.0, line_height: 10, font: extrabold },
        h2: {
            padding_bottom: 2,
            border_bottom: 1,
            border_color: border,
            text: 30.0,
            line_height: 9,
            font: semibold,
        },
        h3: { text: 24.0, line_height: 8, font: semibold },
        h4: { text: 20.0, line_height: 7, font: semibold },
        p: { text: base, line_height: 7 },
        blockquote: { padding_start: 6, border_start: 2, border_color: border, italic: true },
        list: { display: flex, direction: column, gap: 2, padding_start: 6 },
        list_item: { display: flex, gap: 2 },
        list_marker: { flex: none, min_width: 3 },
        inline_code: {
            padding_x: 1.25,
            padding_y: 0.5,
            background: muted,
            font_family: mono,
            text: sm,
            font: semibold,
        },
        lead: { text: xl, color: muted_foreground },
        large: { text: lg, font: semibold },
        small: { text: sm, font: medium, line_height: 3.5 },
        muted: { text: sm, color: muted_foreground },
    }
}

/// Page title: 36px extra-bold.
#[component]
pub fn H1(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TYPOGRAPHY.h1, &sx))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Section title: 30px semibold with a rule underneath.
#[component]
pub fn H2(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TYPOGRAPHY.h2, &sx))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// 24px semibold.
#[component]
pub fn H3(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TYPOGRAPHY.h3, &sx))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// 20px semibold.
#[component]
pub fn H4(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TYPOGRAPHY.h4, &sx))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Body paragraph with relaxed line height.
#[component]
pub fn P(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TYPOGRAPHY.p, &sx))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Indented italic quote with a rule on the starting side.
#[component]
pub fn Blockquote(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TYPOGRAPHY.blockquote, &sx))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Bulleted list. Pass `ordered(true)` for numbers.
#[component]
pub fn List(
    items: Vec<SharedString>,
    #[prop(optional)] ordered: bool,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TYPOGRAPHY.list, &sx))
        .children(items.into_iter().enumerate().map(move |(index, item)| {
            let marker: SharedString = if ordered {
                format!("{}.", index + 1).into()
            } else {
                "•".into()
            };
            div()
                .sx(&TYPOGRAPHY.list_item)
                .child(div().sx(&TYPOGRAPHY.list_marker).child(marker))
                .child(item)
        }))
        .apply_style_overrides(&style_overrides)
}

/// Monospace code inside running text.
#[component]
pub fn InlineCode(text: SharedString, cx: &mut App, #[sx] sx: Sx) -> impl IntoElement {
    // Keeps a little rounding even in square themes.
    let radius = cx.theme().radius_small().max(px(3.));
    div()
        .rounded(radius)
        .sx((&TYPOGRAPHY.inline_code, &sx))
        .child(text)
}

/// Large muted intro text.
#[component]
pub fn Lead(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&TYPOGRAPHY.lead, &sx)).child(text)
}

/// 18px semibold.
#[component]
pub fn Large(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&TYPOGRAPHY.large, &sx)).child(text)
}

/// 14px medium, tight line height.
#[component]
pub fn Small(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&TYPOGRAPHY.small, &sx)).child(text)
}

/// 14px muted text.
#[component]
pub fn Muted(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&TYPOGRAPHY.muted, &sx)).child(text)
}
