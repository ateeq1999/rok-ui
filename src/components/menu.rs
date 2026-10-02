//! Menus: the item list shared by [`DropdownMenu`], [`ContextMenu`],
//! [`super::Menubar`] and [`super::Select`].
//!
//! ```ignore
//! DropdownMenu::new("account")
//!     .trigger(Button::new("account-trigger").outline().label("Open"))
//!     .menu(Menu::new()
//!         .label("My Account")
//!         .separator()
//!         .item(MenuItem::new("Profile").icon(IconName::User).shortcut("⇧⌘P").on_select(|_, _, cx| ..))
//!         .item(MenuItem::new("Status bar").checked(show_status_bar).on_select(..))
//!         .item(MenuItem::new("Invite users").submenu(Menu::new().item(MenuItem::new("Email"))))
//!         .separator()
//!         .item(MenuItem::new("Log out").destructive()))
//! ```

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, Corner, CursorStyle, Div, ElementId, FontWeight,
    MouseButton, Pixels, Point, SharedString, StyleRefinement, Window,
};

use super::layer::layer_at;
use super::overlay::{
    child_id, dismissable, floating, popover_surface, trigger_wrapper, use_open_state, Align,
    OpenState, Side,
};
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::{Icon, IconName},
    motion::{presets, MotionExt},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

#[cfg_attr(not(feature = "full"), allow(dead_code))]
#[derive(Clone)]
enum MenuItemKind {
    Action,
    Checkbox(bool),
    Radio(bool),
    /// A [`super::Select`] option: a check on the right when selected.
    SelectOption(bool),
    Submenu(Menu),
}

/// One row of a [`Menu`].
#[derive(Clone)]
pub struct MenuItem {
    label: SharedString,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    disabled: bool,
    destructive: bool,
    inset: bool,
    kind: MenuItemKind,
    on_select: Option<EventHandler<()>>,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            disabled: false,
            destructive: false,
            inset: false,
            kind: MenuItemKind::Action,
            on_select: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Keyboard shortcut hint shown on the right, like `"⌘K"`.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Red text, for destructive actions.
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// Indent the label to line up with checkbox and radio items.
    pub fn inset(mut self) -> Self {
        self.inset = true;
        self
    }

    /// Make this a checkbox item showing a check when `checked`.
    pub fn checked(mut self, checked: bool) -> Self {
        self.kind = MenuItemKind::Checkbox(checked);
        self
    }

    /// Make this a radio item showing a dot when `selected`.
    pub fn radio(mut self, selected: bool) -> Self {
        self.kind = MenuItemKind::Radio(selected);
        self
    }

    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    pub(crate) fn select_option(mut self, selected: bool) -> Self {
        self.kind = MenuItemKind::SelectOption(selected);
        self
    }

    /// Open `menu` to the side when this item is hovered.
    pub fn submenu(mut self, menu: Menu) -> Self {
        self.kind = MenuItemKind::Submenu(menu);
        self
    }

    /// Called when the item is chosen. The menu closes afterwards.
    pub fn on_select(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    fn is_selectable(&self) -> bool {
        !self.disabled
    }
}

#[derive(Clone)]
enum MenuEntry {
    Item(MenuItem),
    Label(SharedString),
    Separator,
}

/// The contents of a menu: items, group labels and separators.
#[derive(Clone, Default)]
pub struct Menu {
    entries: Vec<MenuEntry>,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(MenuEntry::Item(item));
        self
    }

    /// A non-interactive group heading.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(MenuEntry::Label(label.into()));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn selectable_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| match entry {
                MenuEntry::Item(item) if item.is_selectable() => Some(index),
                _ => None,
            })
            .collect()
    }
}

/// Closes the whole menu (all submenus included).
pub(crate) type CloseMenu = Rc<dyn Fn(&mut Window, &mut App)>;

fn choose(item: &MenuItem, close_menu: &CloseMenu, window: &mut Window, cx: &mut App) {
    if let Some(handler) = item.on_select.as_ref() {
        handler(&(), window, cx);
    }
    close_menu(window, cx);
}

/// Render `menu` as a panel. `id` keys the highlighted row. Keyboard handling is
/// attached when `keyboard` is true (the panel must hold focus for it to work).
pub(crate) fn render_menu_panel(
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
    let theme = cx.theme();
    let colors = theme.colors.clone();
    let item_radius = theme.radius_small();
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
            MenuEntry::Separator => rows.push(
                div()
                    .h(px(1.))
                    .mx(px(-4.))
                    .my(px(4.))
                    .bg(colors.border)
                    .into_any_element(),
            ),
            MenuEntry::Label(label) => rows.push(
                div()
                    .px(px(8.))
                    .py(px(6.))
                    .when(has_indicators, |row| row.pl(px(32.)))
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(label)
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
                    MenuItemKind::Radio(true) => Some(
                        div()
                            .size(px(8.))
                            .rounded_full()
                            .bg(text_color)
                            .into_any_element(),
                    ),
                    _ => None,
                };
                let trailing: Option<AnyElement> = match &item.kind {
                    MenuItemKind::Submenu(_) => Some(
                        Icon::new(IconName::ChevronRight)
                            .size(px(16.))
                            .color(icon_color)
                            .into_any_element(),
                    ),
                    MenuItemKind::SelectOption(selected) => Some(
                        div()
                            .size(px(16.))
                            .when(*selected, |slot| {
                                slot.child(
                                    Icon::new(IconName::Check).size(px(16.)).color(text_color),
                                )
                            })
                            .into_any_element(),
                    ),
                    _ => item.shortcut.clone().map(|shortcut| {
                        div()
                            .pl(px(16.))
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(shortcut)
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
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(item_radius)
                    .text_sm()
                    .text_color(text_color)
                    .when(has_indicators || item.inset, |row| row.pl(px(32.)))
                    .when(is_highlighted, |row| {
                        row.bg(if item.destructive {
                            colors.destructive.opacity(0.1)
                        } else {
                            colors.accent
                        })
                    })
                    .when(item_disabled, |row| row.opacity(0.5))
                    .when(!item_disabled, |row| {
                        row.cursor(CursorStyle::PointingHand)
                            .on_hover(move |hovered, _, cx| {
                                if *hovered {
                                    hover_state.set(Some(index), cx)
                                }
                            })
                            .when(!is_submenu, |row| {
                                let item = item.clone();
                                // Mouse down, not click: a click outside the root panel
                                // (inside a submenu) closes the menu before mouse up.
                                row.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    cx.stop_propagation();
                                    choose(&item, &close_menu, window, cx)
                                })
                            })
                    })
                    .when_some(indicator, |row, indicator| {
                        row.child(
                            div()
                                .absolute()
                                .left(px(8.))
                                .size(px(16.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(indicator),
                        )
                    })
                    .when_some(item.icon, |row, icon| {
                        row.child(Icon::new(icon).size(px(16.)).color(icon_color))
                    })
                    // The label takes the free space; an auto margin on the trailing slot
                    // would also swallow the row's gap in GPUI's layout.
                    .child(div().flex_1().child(item.label.clone()))
                    .children(trailing)
                    .children(submenu_panel);
                rows.push(row.into_any_element());
            }
        }
    }

    let selectable = menu.selectable_indices();
    let panel = popover_surface(cx.theme())
        .id(child_id(&id, "panel"))
        .min_w(min_width)
        .max_h(px(384.))
        .overflow_y_scroll()
        .p(px(4.))
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

pub(crate) fn close_handler(open_state: &OpenState) -> CloseMenu {
    let open_state = open_state.clone();
    Rc::new(move |window, cx| open_state.set_open(false, window, cx))
}

/// A menu of actions opened by a trigger (shadcn/ui's `<DropdownMenu>`).
/// Uncontrolled by default; Up / Down / Enter / Escape work while it is open.
#[derive(IntoElement)]
pub struct DropdownMenu {
    id: ElementId,
    trigger: Option<AnyElement>,
    menu: Menu,
    side: Side,
    align: Align,
    open: Option<bool>,
    on_open_change: Option<EventHandler<bool>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(DropdownMenu);

impl DropdownMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            menu: Menu::new(),
            side: Side::Bottom,
            align: Align::Start,
            open: None,
            on_open_change: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn menu(mut self, menu: Menu) -> Self {
        self.menu = menu;
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state = use_open_state(&self.id, self.open, self.on_open_change, window, cx);
        let wrapper =
            trigger_wrapper(self.id.clone(), &open_state, false, cx).children(self.trigger);
        if !open_state.is_open(cx) {
            return wrapper;
        }
        let panel = render_menu_panel(
            self.id,
            self.menu,
            close_handler(&open_state),
            px(128.),
            true,
            window,
            cx,
        )
        .apply_style_overrides(&self.style_overrides);
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(self.side, self.align, panel, cx))
    }
}

/// A menu opened by right-clicking an area (shadcn/ui's `<ContextMenu>`).
///
/// ```ignore
/// ContextMenu::new("canvas-menu")
///     .menu(Menu::new().item(MenuItem::new("Back").shortcut("⌘[")))
///     .child(div().size(px(300.)).child("Right click here"))
/// ```
#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    menu: Menu,
    children: Vec<AnyElement>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ContextMenu);

impl ContextMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            menu: Menu::new(),
            children: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn menu(mut self, menu: Menu) -> Self {
        self.menu = menu;
        self
    }
}

impl ParentElement for ContextMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state = use_open_state(&self.id, None, None, window, cx);
        let position = use_keyed_state(child_id(&self.id, "position"), window, cx, || {
            Point::<Pixels>::default()
        });
        let is_open = open_state.is_open(cx);

        let opener = open_state.clone();
        let menu_position = position.clone();
        let area = div()
            .id(self.id.clone())
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                menu_position.set(event.position, cx);
                opener.set_open(true, window, cx);
            })
            .children(self.children)
            .apply_style_overrides(&self.style_overrides);
        if !is_open {
            return area;
        }
        let panel = render_menu_panel(
            self.id,
            self.menu,
            close_handler(&open_state),
            px(128.),
            true,
            window,
            cx,
        );
        let panel = dismissable(panel, &open_state, window, cx);
        let position = position.get(cx);
        let panel = panel.motion("rok-ui-context-enter", presets::fade_in().duration_ms(120));
        area.child(layer_at(position, Corner::TopLeft, px(8.), panel, 1, cx))
    }
}
