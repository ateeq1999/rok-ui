//! KeyboardShortcut: shadcn/ui's `<Kbd>`, a key cap for shortcuts.

use gpui::{div, prelude::*, px, App, FontWeight, SharedString};

use crate::{component, theme::ActiveTheme};

/// `KeyboardShortcut::new("⌘K")`.
#[component]
pub fn KeyboardShortcut(keys: SharedString, cx: &mut App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
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
}
