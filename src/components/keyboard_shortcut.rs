//! KeyboardShortcut: shadcn/ui's `<Kbd>`, a key cap for shortcuts.

use gpui::{div, prelude::*, px, App, FontWeight, SharedString};

use super::direction::DirectionalStyled;
use crate::sx::SxStyled;
use crate::{component, theme::ActiveTheme};

/// `KeyboardShortcut::new("⌘K")`.
#[component]
pub fn KeyboardShortcut(
    keys: SharedString,
    cx: &mut App,
    #[sx] sx: crate::sx::Sx,
) -> impl IntoElement {
    let element = {
        let theme = cx.theme();
        div()
            .flex_dir()
            .flex_none()
            .items_center()
            .justify_center()
            .h(px(20.))
            .min_w(px(20.))
            .px(px(4.))
            .rounded(theme.radius_small().max(px(3.)))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.muted)
            .text_color(theme.colors.muted_foreground)
            .font_family(theme.monospace_font_family.clone())
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .child(keys)
    };
    element.sx(&sx)
}

/// shadcn/ui's name for [`KeyboardShortcut`]: `Kbd::new("⌘")`.
pub type Kbd = KeyboardShortcut;

/// Key caps for a chord, joined with `+`: `KbdGroup::new(["Ctrl", "Shift", "P"])`.
#[component]
pub fn KbdGroup(
    keys: Vec<SharedString>,
    cx: &mut App,
    #[sx] sx: crate::sx::Sx,
) -> impl IntoElement {
    let element = {
        let muted_foreground = cx.theme().colors.muted_foreground;
        let key_count = keys.len();
        div()
            .flex_dir()
            .items_center()
            .gap(px(4.))
            .text_xs()
            .text_color(muted_foreground)
            .children(keys.into_iter().enumerate().flat_map(move |(index, key)| {
                let separator =
                    (index + 1 < key_count).then(|| div().child("+").into_any_element());
                std::iter::once(KeyboardShortcut::new(key).into_any_element()).chain(separator)
            }))
    };
    element.sx(&sx)
}
