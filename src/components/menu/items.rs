//! Menu contents: [`MenuItem`]s, separators and labels in a [`Menu`].

use std::rc::Rc;

use gpui::{App, SharedString, Window};

use crate::{hooks::EventHandler, icon::IconName};

#[cfg_attr(not(feature = "full"), allow(dead_code))]
#[derive(Clone)]
pub(super) enum MenuItemKind {
    Action,
    Checkbox(bool),
    Radio(bool),
    /// A [`crate::components::Select`] option: a check on the right when selected.
    SelectOption(bool),
    Submenu(Menu),
}

/// One row of a [`Menu`].
#[derive(Clone)]
pub struct MenuItem {
    pub(super) label: SharedString,
    pub(super) icon: Option<IconName>,
    pub(super) shortcut: Option<SharedString>,
    pub(super) disabled: bool,
    pub(super) destructive: bool,
    pub(super) inset: bool,
    pub(super) kind: MenuItemKind,
    on_select: Option<EventHandler<()>>,
}

impl MenuItem {
    /// Create it with its label.
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

    /// An icon shown with the label.
    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Keyboard shortcut hint shown on the right, like `"⌘K"`.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Red text, for destructive actions.
    #[must_use]
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// Indent the label to line up with checkbox and radio items.
    #[must_use]
    pub fn inset(mut self) -> Self {
        self.inset = true;
        self
    }

    /// Make this a checkbox item showing a check when `checked`.
    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        self.kind = MenuItemKind::Checkbox(checked);
        self
    }

    /// Make this a radio item showing a dot when `selected`.
    #[must_use]
    pub fn radio(mut self, selected: bool) -> Self {
        self.kind = MenuItemKind::Radio(selected);
        self
    }

    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    #[doc(hidden)]
    #[must_use]
    pub fn select_option(mut self, selected: bool) -> Self {
        self.kind = MenuItemKind::SelectOption(selected);
        self
    }

    /// Open `menu` to the side when this item is hovered.
    #[must_use]
    pub fn submenu(mut self, menu: Menu) -> Self {
        self.kind = MenuItemKind::Submenu(menu);
        self
    }

    /// Called when the item is chosen. The menu closes afterwards.
    #[must_use]
    pub fn on_select(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    fn is_selectable(&self) -> bool {
        !self.disabled
    }
}

#[derive(Clone)]
pub(super) enum MenuEntry {
    Item(MenuItem),
    Label(SharedString),
    Separator,
}

/// The contents of a menu: items, group labels and separators.
#[derive(Clone, Default)]
pub struct Menu {
    pub(super) entries: Vec<MenuEntry>,
}

impl Menu {
    /// An empty `Menu`; add content with the builder methods.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an item.
    #[must_use]
    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(MenuEntry::Item(item));
        self
    }

    /// A non-interactive group heading.
    #[must_use]
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(MenuEntry::Label(label.into()));
        self
    }

    /// Add a separator line after the items so far.
    #[must_use]
    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    /// Whether the menu has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn selectable_indices(&self) -> Vec<usize> {
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
#[doc(hidden)]
pub type CloseMenu = Rc<dyn Fn(&mut Window, &mut App)>;

pub(super) fn choose(item: &MenuItem, close_menu: &CloseMenu, window: &mut Window, cx: &mut App) {
    if let Some(handler) = item.on_select.as_ref() {
        handler(&(), window, cx);
    }
    close_menu(window, cx);
}
