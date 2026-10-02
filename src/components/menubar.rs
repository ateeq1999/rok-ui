//! Menubar and NavigationMenu: horizontal bars of menu triggers.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, FontWeight, MouseButton,
    SharedString, StyleRefinement, Window,
};

use super::{
    menu::{render_menu_panel, Menu},
    overlay::{child_id, floating, popover_surface, Align, Side},
};
use crate::{
    hooks::{use_keyed_state, EventHandler, State},
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

struct MenubarMemory {
    open_index: Option<usize>,
    focus_handle: gpui::FocusHandle,
}

/// A desktop-style menu bar. Click a title to open its menu; while one is open,
/// hovering another title switches to it.
///
/// ```ignore
/// Menubar::new("app-menu")
///     .menu("File", Menu::new()
///         .item(MenuItem::new("New Tab").shortcut("⌘T"))
///         .item(MenuItem::new("Print…").shortcut("⌘P")))
///     .menu("Edit", Menu::new().item(MenuItem::new("Undo").shortcut("⌘Z")))
/// ```
#[derive(IntoElement)]
pub struct Menubar {
    id: ElementId,
    menus: Vec<(SharedString, Menu)>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Menubar);

impl Menubar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            menus: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn menu(mut self, title: impl Into<SharedString>, menu: Menu) -> Self {
        self.menus.push((title.into(), menu));
        self
    }
}

impl RenderOnce for Menubar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let memory: State<MenubarMemory> = {
            let entity =
                window.use_keyed_state(child_id(&self.id, "menubar"), cx, |_, cx| MenubarMemory {
                    open_index: None,
                    focus_handle: cx.focus_handle(),
                });
            State::from_entity(entity)
        };
        let open_index = memory.read(cx).open_index;
        let focus_handle = memory.read(cx).focus_handle.clone();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let radius = theme.radius_small();
        let bar_radius = theme.radius_medium();
        let ring_color = colors.ring;
        let menu_count = self.menus.len();
        let menu_id = child_id(&self.id, "menu");

        let mut triggers = Vec::with_capacity(menu_count);
        for (index, (title, menu)) in self.menus.into_iter().enumerate() {
            let is_open = open_index == Some(index);
            let click_memory = memory.clone();
            let hover_memory = memory.clone();
            let click_focus = focus_handle.clone();
            let mut trigger = div()
                .id(("menubar-trigger", index))
                .relative()
                .px(px(8.))
                .py(px(4.))
                .rounded(radius)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .cursor(CursorStyle::PointingHand)
                .tab_index(0)
                .border_1()
                .border_color(gpui::transparent_black())
                .focus(move |style| style.border_color(ring_color))
                .when(is_open, |trigger| {
                    trigger
                        .bg(colors.accent)
                        .text_color(colors.accent_foreground)
                })
                .hover(|style| style.bg(colors.accent))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    click_memory.update(cx, |memory| {
                        memory.open_index = if is_open { None } else { Some(index) }
                    });
                    if !is_open {
                        let focus = click_focus.clone();
                        window.defer(cx, move |window, _| window.focus(&focus));
                    }
                })
                .on_hover(move |hovered, _, cx| {
                    let switching = hover_memory
                        .read(cx)
                        .open_index
                        .is_some_and(|open| open != index);
                    if *hovered && switching {
                        hover_memory.update(cx, |memory| memory.open_index = Some(index));
                    }
                })
                .child(title);
            if is_open {
                let close_memory = memory.clone();
                let panel = render_menu_panel(
                    menu_id.clone(),
                    menu,
                    Rc::new(move |_, cx| {
                        close_memory.update(cx, |memory| memory.open_index = None)
                    }),
                    px(192.),
                    true,
                    window,
                    cx,
                );
                trigger = trigger.child(floating(Side::Bottom, Align::Start, panel.occlude(), cx));
            }
            triggers.push(trigger);
        }

        let outside_memory = memory.clone();
        let key_memory = memory.clone();
        div()
            .id(self.id)
            .track_focus(&focus_handle)
            .flex()
            .items_center()
            .gap(px(4.))
            .h(px(36.))
            .p(px(4.))
            .rounded(bar_radius)
            .border_1()
            .border_color(colors.border)
            .bg(colors.background)
            .shadow(super::extra_small_shadow())
            .children(triggers)
            .when(open_index.is_some(), |bar| {
                bar.on_mouse_down_out(move |_, _, cx| {
                    outside_memory.update(cx, |memory| memory.open_index = None)
                })
            })
            .on_key_down(move |event, _, cx| {
                let Some(open) = key_memory.read(cx).open_index else {
                    return;
                };
                let next = match event.keystroke.key.as_str() {
                    "escape" => None,
                    "right" => Some((open + 1) % menu_count),
                    "left" => Some((open + menu_count - 1) % menu_count),
                    _ => return,
                };
                cx.stop_propagation();
                key_memory.update(cx, |memory| memory.open_index = next);
            })
            .apply_style_overrides(&self.style_overrides)
    }
}

enum NavigationEntry {
    Link {
        label: SharedString,
        on_click: Option<EventHandler<()>>,
        active: bool,
    },
    Panel {
        label: SharedString,
        content: AnyElement,
    },
}

/// A website-style navigation bar: plain links plus titles that reveal a content
/// panel on hover (shadcn/ui's `<NavigationMenu>`).
///
/// ```ignore
/// NavigationMenu::new("site-nav")
///     .panel("Getting started", getting_started_grid)
///     .panel("Components", component_grid)
///     .link("Docs", |_, _, cx| open_docs(cx))
/// ```
#[derive(IntoElement)]
pub struct NavigationMenu {
    id: ElementId,
    entries: Vec<NavigationEntry>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(NavigationMenu);

impl NavigationMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A plain link.
    pub fn link(
        mut self,
        label: impl Into<SharedString>,
        on_click: impl Fn(&(), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.entries.push(NavigationEntry::Link {
            label: label.into(),
            on_click: Some(Rc::new(on_click)),
            active: false,
        });
        self
    }

    /// A link marked as the current page.
    pub fn active_link(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(NavigationEntry::Link {
            label: label.into(),
            on_click: None,
            active: true,
        });
        self
    }

    /// A title that shows `content` in a panel below it while hovered.
    pub fn panel(mut self, label: impl Into<SharedString>, content: impl IntoElement) -> Self {
        self.entries.push(NavigationEntry::Panel {
            label: label.into(),
            content: content.into_any_element(),
        });
        self
    }
}

/// A titled link with a description, for the grids inside navigation panels.
#[derive(IntoElement)]
pub struct NavigationMenuLink {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    on_click: Option<EventHandler<()>>,
}

impl NavigationMenuLink {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            on_click: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavigationMenuLink {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(8.))
            .rounded(theme.radius_small())
            .cursor(CursorStyle::PointingHand)
            .hover(|style| style.bg(colors.accent))
            .when_some(self.on_click, |link, handler| {
                link.on_click(move |_, window, cx| handler(&(), window, cx))
            })
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(self.title),
            )
            .when_some(self.description, |link, description| {
                link.child(
                    div()
                        .text_sm()
                        .text_color(colors.muted_foreground)
                        .line_clamp(2)
                        .child(description),
                )
            })
    }
}

impl RenderOnce for NavigationMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_panel = use_keyed_state(child_id(&self.id, "open"), window, cx, || None::<usize>);
        let open_index = open_panel.get(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let radius = theme.radius_medium();
        let ring_color = colors.ring;

        let entries = self.entries.into_iter().enumerate().map(|(index, entry)| {
            let trigger = div()
                .id(("navigation-trigger", index))
                .relative()
                .flex()
                .items_center()
                .gap(px(4.))
                .h(px(36.))
                .px(px(16.))
                .rounded(radius)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .cursor(CursorStyle::PointingHand)
                .tab_index(0)
                .border_1()
                .border_color(gpui::transparent_black())
                .focus(move |style| style.border_color(ring_color))
                .hover(|style| style.bg(colors.accent));
            match entry {
                NavigationEntry::Link {
                    label,
                    on_click,
                    active,
                } => trigger
                    .on_hover({
                        let close_state = open_panel.clone();
                        move |hovered, _, cx| {
                            if *hovered {
                                close_state.set(None, cx);
                            }
                        }
                    })
                    .when(active, |trigger| trigger.bg(colors.accent))
                    .when_some(on_click, |trigger, handler| {
                        trigger.on_click(move |_, window, cx| handler(&(), window, cx))
                    })
                    .child(label)
                    .into_any_element(),
                NavigationEntry::Panel { label, content } => {
                    let is_open = open_index == Some(index);
                    let hover_state = open_panel.clone();
                    let key_state = open_panel.clone();
                    trigger
                        .when(is_open, |trigger| trigger.bg(colors.accent))
                        .on_hover(move |hovered, _, cx| {
                            if *hovered {
                                hover_state.set(Some(index), cx);
                            }
                        })
                        .on_key_down(move |event, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space" | "down") {
                                cx.stop_propagation();
                                key_state.set(if is_open { None } else { Some(index) }, cx);
                            }
                        })
                        .child(label)
                        .child(
                            Icon::new(if is_open {
                                IconName::ChevronUp
                            } else {
                                IconName::ChevronDown
                            })
                            .size(px(12.))
                            .color(colors.muted_foreground),
                        )
                        .when(is_open, |trigger| {
                            let panel = popover_surface(cx.theme())
                                .id(("navigation-panel", index))
                                .occlude()
                                .p(px(8.))
                                // Hovering the panel keeps it open; it is not inside the trigger's bounds.
                                .on_hover({
                                    let panel_state = open_panel.clone();
                                    move |hovered, _, cx| {
                                        if *hovered {
                                            panel_state.set(Some(index), cx);
                                        }
                                    }
                                })
                                .child(content);
                            trigger.child(floating(Side::Bottom, Align::Start, panel, cx))
                        })
                        .into_any_element()
                }
            }
        });

        let leave_state = open_panel.clone();
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(4.))
            .children(entries)
            .when(open_index.is_some(), |bar| {
                bar.on_mouse_down_out(move |_, _, cx| leave_state.set(None, cx))
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
