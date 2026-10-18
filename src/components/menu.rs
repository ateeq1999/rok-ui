//! Menus: the item list shared by [`DropdownMenu`], [`ContextMenu`],
//! [`super::Menubar`] and [`super::Select`].
//!
//! ```no_run
//! # use rok_ui::prelude::*;
//! # fn example(window: &mut Window, cx: &mut App) {
//! # let show_status_bar = true;
//! let account = DropdownMenu::new("account")
//!     .trigger(Button::new("account-trigger").outline().label("Open"))
//!     .menu(
//!         Menu::new()
//!             .label("My Account")
//!             .separator()
//!             .item(MenuItem::new("Profile").icon(IconName::User).shortcut("Shift-Cmd-P").on_select(|_, _, _| {}))
//!             .item(MenuItem::new("Status bar").checked(show_status_bar).on_select(|_, _, _| {}))
//!             .item(MenuItem::new("Invite users").submenu(Menu::new().item(MenuItem::new("Email"))))
//!             .separator()
//!             .item(MenuItem::new("Log out").destructive()),
//!     );
//! # }
//! ```

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, Div, ElementId, MouseButton, Pixels, Window};

use super::overlay::{child_id, floating, popover_surface, Align, OpenState, Side};
use crate::sx::SxStyled;
use crate::{
    hooks::use_keyed_state,
    icon::{Icon, IconName},
    styles,
    theme::ActiveTheme,
};

mod items;
mod triggers;

use items::{choose, MenuEntry, MenuItemKind};
pub use items::{CloseMenu, Menu, MenuItem};
pub use triggers::{ContextMenu, DropdownMenu};

styles! {
    MENU = {
        panel: { max_height: 96, padding: 1 },
        separator: { height: 0.25, margin_x: -1, margin_y: 1, background: border },
        label: { padding_x: 2, padding_y: 1.5, text: sm, font: medium },
        // Leaves room for the check or radio indicator.
        inset: { padding_start: 8 },
        item: {
            position: relative,
            display: flex,
            align: center,
            gap: 2,
            padding_x: 2,
            padding_y: 1.5,
            radius: sm,
            text: sm,
            color: popover_foreground,
        },
        highlighted: { background: accent, color: accent_foreground },
        destructive: { color: destructive_text },
        destructive_highlighted: { background: destructive/10 },
        interactive: { cursor: pointer },
        inert: { opacity: 0.5 },
        indicator: {
            position: absolute,
            inset_start: 2,
            size: 4,
            display: flex,
            align: center,
            justify: center,
        },
        radio_dot: { size: 2, radius: full },
        icon_slot: { size: 4 },
        // The label takes the free space; an auto margin on the trailing slot
        // would also swallow the row's gap in GPUI's layout.
        item_label: { flex: 1 },
        shortcut: { padding_start: 4, text: xs, color: muted_foreground },
    }
}

/// Render `menu` as a panel. `id` keys the highlighted row. Keyboard handling is
/// attached when `keyboard` is true (the panel must hold focus for it to work).
#[doc(hidden)]
pub fn render_menu_panel(
    id: ElementId,
    menu: Menu,
    close_menu: CloseMenu,
    min_width: Pixels,
    keyboard: bool,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let highlighted = use_keyed_state(child_id(&id, "highlighted"), window, cx, || None::<usize>);
    let highlighted_index = highlighted.get(cx);
    let colors = cx.theme().colors.clone();
    let has_indicators = menu.entries.iter().any(|entry| {
        matches!(
            entry,
            MenuEntry::Item(MenuItem {
                kind: MenuItemKind::Checkbox(_) | MenuItemKind::Radio(_),
                ..
            })
        )
    });

    let mut rows: Vec<AnyElement> = Vec::with_capacity(menu.entries.len());
    for (index, entry) in menu.entries.iter().cloned().enumerate() {
        match entry {
            MenuEntry::Separator => rows.push(div().sx(&MENU.separator).into_any_element()),
            MenuEntry::Label(label) => rows.push(
                div()
                    .sx((&MENU.label, has_indicators.then_some(&MENU.inset)))
                    .child(crate::components::bidi_text::text(label))
                    .into_any_element(),
            ),
            MenuEntry::Item(item) => {
                let is_highlighted = highlighted_index == Some(index) && !item.disabled;
                let text_color = if item.destructive {
                    colors.destructive_text
                } else if is_highlighted {
                    colors.accent_foreground
                } else {
                    colors.popover_foreground
                };
                let icon_color = if item.destructive {
                    colors.destructive_text
                } else {
                    colors.muted_foreground
                };
                let indicator = match item.kind {
                    MenuItemKind::Checkbox(true) => Some(
                        Icon::new(IconName::Check)
                            .size(px(16.))
                            .color(text_color)
                            .into_any_element(),
                    ),
                    MenuItemKind::Radio(true) => {
                        Some(div().sx(&MENU.radio_dot).bg(text_color).into_any_element())
                    }
                    _ => None,
                };
                let trailing: Option<AnyElement> = match &item.kind {
                    MenuItemKind::Submenu(_) => Some(
                        Icon::new(IconName::ChevronRight.for_direction())
                            .size(px(16.))
                            .color(icon_color)
                            .into_any_element(),
                    ),
                    MenuItemKind::SelectOption(selected) => Some(
                        div()
                            .sx(&MENU.icon_slot)
                            .when(*selected, |slot| {
                                slot.child(
                                    Icon::new(IconName::Check).size(px(16.)).color(text_color),
                                )
                            })
                            .into_any_element(),
                    ),
                    _ => item.shortcut.clone().map(|shortcut| {
                        div()
                            .sx(&MENU.shortcut)
                            .child(crate::components::bidi_text::text(shortcut))
                            .into_any_element()
                    }),
                };

                let submenu_panel = match (&item.kind, is_highlighted) {
                    (MenuItemKind::Submenu(submenu), true) => {
                        let submenu_panel = render_menu_panel(
                            child_id(&id, "submenu").with_index(index),
                            submenu.clone(),
                            close_menu.clone(),
                            px(128.),
                            false,
                            window,
                            cx,
                        )
                        .occlude();
                        Some(floating(Side::Right, Align::Start, submenu_panel, cx))
                    }
                    _ => None,
                };

                let hover_state = highlighted.clone();
                let close_menu = close_menu.clone();
                let is_submenu = matches!(item.kind, MenuItemKind::Submenu(_));
                let item_disabled = item.disabled;
                let row = div()
                    .id(index)
                    .sx((
                        &MENU.item,
                        (has_indicators || item.inset).then_some(&MENU.inset),
                        is_highlighted.then_some(&MENU.highlighted),
                        item.destructive.then_some(&MENU.destructive),
                        (item.destructive && is_highlighted)
                            .then_some(&MENU.destructive_highlighted),
                        if item_disabled {
                            &MENU.inert
                        } else {
                            &MENU.interactive
                        },
                    ))
                    .when(!item_disabled, |row| {
                        row.on_hover(move |hovered, _, cx| {
                            if *hovered {
                                hover_state.set(Some(index), cx);
                            }
                        })
                        .when(!is_submenu, |row| {
                            let item = item.clone();
                            // Mouse down, not click: a click outside the root panel
                            // (inside a submenu) closes the menu before mouse up.
                            row.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                cx.stop_propagation();
                                choose(&item, &close_menu, window, cx);
                            })
                        })
                    })
                    .when_some(indicator, |row, indicator| {
                        row.child(div().sx(&MENU.indicator).child(indicator))
                    })
                    .when_some(item.icon, |row, icon| {
                        row.child(Icon::new(icon).size(px(16.)).color(icon_color))
                    })
                    .child(
                        div()
                            .sx(&MENU.item_label)
                            .child(crate::components::bidi_text::text(item.label.clone())),
                    )
                    .children(trailing)
                    .children(submenu_panel);
                rows.push(row.into_any_element());
            }
        }
    }

    let selectable = menu.selectable_indices();
    let panel = popover_surface()
        .sx(&MENU.panel)
        .id(child_id(&id, "panel"))
        .min_w(min_width)
        .overflow_y_scroll()
        .children(rows);

    let panel = div().child(panel);
    if !keyboard {
        return panel;
    }
    let entries = menu.entries;
    panel.on_key_down(move |event, window, cx| {
        let key = event.keystroke.key.as_str();
        let current = highlighted.get(cx);
        let position = current.and_then(|index| selectable.iter().position(|&i| i == index));
        match key {
            "down" | "up" if !selectable.is_empty() => {
                cx.stop_propagation();
                let next = match (key, position) {
                    ("down", None) => 0,
                    ("down", Some(position)) => (position + 1) % selectable.len(),
                    (_, None) => selectable.len() - 1,
                    (_, Some(position)) => (position + selectable.len() - 1) % selectable.len(),
                };
                highlighted.set(Some(selectable[next]), cx);
            }
            "home" if !selectable.is_empty() => highlighted.set(Some(selectable[0]), cx),
            "end" if !selectable.is_empty() => highlighted.set(selectable.last().copied(), cx),
            "enter" | "space" => {
                if let Some(MenuEntry::Item(item)) = current.and_then(|index| entries.get(index)) {
                    cx.stop_propagation();
                    if !matches!(item.kind, MenuItemKind::Submenu(_)) {
                        choose(item, &close_menu, window, cx);
                    }
                }
            }
            _ => {}
        }
    })
}

trait WithIndex {
    fn with_index(self, index: usize) -> ElementId;
}

impl WithIndex for ElementId {
    fn with_index(self, index: usize) -> ElementId {
        ElementId::NamedChild(Box::new(self), index.to_string().into())
    }
}

#[doc(hidden)]
#[must_use]
pub fn close_handler(open_state: &OpenState) -> CloseMenu {
    let open_state = open_state.clone();
    Rc::new(move |window, cx| open_state.set_open(false, window, cx))
}
