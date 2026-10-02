//! KeyboardShortcut: shadcn/ui's `<Kbd>`, a key cap for shortcuts.

use gpui::{div, prelude::*, px, App, SharedString};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles, theme::ActiveTheme};

styles! {
    KBD = {
        key: {
            display: flex,
            flex: none,
            align: center,
            justify: center,
            height: 5,
            min_width: 5,
            padding_x: 1,
            border: 1,
            border_color: border,
            background: muted,
            color: muted_foreground,
            font_family: mono,
            text: xs,
            font: medium,
        },
        group: { display: flex, align: center, gap: 1, text: xs, color: muted_foreground },
    }
}

/// `KeyboardShortcut::new("⌘K")`.
#[component]
pub fn KeyboardShortcut(keys: SharedString, cx: &mut App, #[sx] sx: Sx) -> impl IntoElement {
    // Key caps keep a little rounding even in square themes.
    let radius = cx.theme().radius_small().max(px(3.));
    div()
        .rounded(radius)
        .sx((&KBD.key, &sx))
        .child(crate::components::bidi_text::text(keys))
}

/// shadcn/ui's name for [`KeyboardShortcut`]: `Kbd::new("⌘")`.
pub type Kbd = KeyboardShortcut;

/// Key caps for a chord, joined with `+`: `KbdGroup::new(["Ctrl", "Shift", "P"])`.
#[component]
pub fn KbdGroup(keys: Vec<SharedString>, #[sx] sx: Sx) -> impl IntoElement {
    let key_count = keys.len();
    div()
        .sx((&KBD.group, &sx))
        .children(keys.into_iter().enumerate().flat_map(move |(index, key)| {
            let separator = (index + 1 < key_count).then(|| div().child("+").into_any_element());
            std::iter::once(KeyboardShortcut::new(key).into_any_element()).chain(separator)
        }))
}
