//! Menubar and `NavigationMenu`: horizontal bars of menu triggers.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ElementId, MouseButton, SharedString, StyleRefinement,
    Window,
};

use super::{
    menu::{render_menu_panel, Menu},
    overlay::{child_id, floating, popover_surface, Align, Side},
};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler, State},
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

styles! {
    MENUBAR = {
        bar: {
            display: flex,
            align: center,
            gap: 1,
            height: 9,
            padding: 1,
            radius: md,
            border: 1,
            border_color: border,
            background: background,
            shadow: xs,
        },
        trigger: {
            position: relative,
            padding_x: 2,
            padding_y: 1,
            radius: sm,
            text: sm,
            font: medium,
            cursor: pointer,
            border: 1,
            border_color: transparent,
            hover: { background: accent },
            focus: { border_color: ring },
        },
        trigger_open: { background: accent, color: accent_foreground },
    }
}

styles! {
    NAVIGATION = {
        bar: { display: flex, align: center, gap: 1 },
        trigger: {
            position: relative,
            display: flex,
            align: center,
            gap: 1,
            height: 9,
            padding_x: 4,
            radius: md,
            text: sm,
            font: medium,
            cursor: pointer,
            border: 1,
            border_color: transparent,
            hover: { background: accent },
            focus: { border_color: ring },
        },
        trigger_highlighted: { background: accent },
        panel: { padding: 2 },
        link: {
            display: flex,
            direction: column,
            gap: 1,
            padding: 2,
            radius: sm,
            cursor: pointer,
            hover: { background: accent },
        },
        link_title: { text: sm, font: medium },
        link_description: { text: sm, color: muted_foreground, line_clamp: 2 },
    }
}

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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Menubar);

impl Menubar {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            menus: Vec::new(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add a top-level menu titled `title`.
    #[must_use]
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
                .tab_index(0)
                .sx((&MENUBAR.trigger, is_open.then_some(&MENUBAR.trigger_open)))
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
                .child(crate::components::bidi_text::text(title));
            if is_open {
                let close_memory = memory.clone();
                let panel = render_menu_panel(
                    menu_id.clone(),
                    menu,
                    Rc::new(move |_, cx| {
                        close_memory.update(cx, |memory| memory.open_index = None);
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
        let rtl = super::direction::is_rtl();
        div()
            .id(self.id)
            .track_focus(&focus_handle)
            .sx((&MENUBAR.bar, &self.sx))
            .children(triggers)
            .when(open_index.is_some(), |bar| {
                bar.on_mouse_down_out(move |_, _, cx| {
                    outside_memory.update(cx, |memory| memory.open_index = None);
                })
            })
            .on_key_down(move |event, _, cx| {
                let Some(open) = key_memory.read(cx).open_index else {
                    return;
                };
                let next = match event.keystroke.key.as_str() {
                    "escape" => None,
                    // Menus run right to left in RTL.
                    "right" | "left" if (event.keystroke.key == "right") != rtl => {
                        Some((open + 1) % menu_count)
                    }
                    "right" | "left" => Some((open + menu_count - 1) % menu_count),
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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(NavigationMenu);

impl NavigationMenu {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A plain link.
    #[must_use]
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
    #[must_use]
    pub fn active_link(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(NavigationEntry::Link {
            label: label.into(),
            on_click: None,
            active: true,
        });
        self
    }

    /// A title that shows `content` in a panel below it while hovered.
    #[must_use]
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
    /// A link with a `title`. `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            on_click: None,
        }
    }

    /// Text below the title.
    #[must_use]
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Called when clicked or activated with the keyboard.
    #[must_use]
    pub fn on_click(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavigationMenuLink {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .sx(&NAVIGATION.link)
            .when_some(self.on_click, |link, handler| {
                link.on_click(move |_, window, cx| handler(&(), window, cx))
            })
            .child(
                div()
                    .sx(&NAVIGATION.link_title)
                    .child(crate::components::bidi_text::text(self.title)),
            )
            .when_some(self.description, |link, description| {
                link.child(
                    div()
                        .sx(&NAVIGATION.link_description)
                        .child(crate::components::bidi_text::text(description)),
                )
            })
    }
}

impl RenderOnce for NavigationMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_panel = use_keyed_state(child_id(&self.id, "open"), window, cx, || None::<usize>);
        let open_index = open_panel.get(cx);
        let muted_foreground = cx.theme().colors.muted_foreground;

        let entries = self.entries.into_iter().enumerate().map(|(index, entry)| {
            let highlighted = match &entry {
                NavigationEntry::Link { active, .. } => *active,
                NavigationEntry::Panel { .. } => open_index == Some(index),
            };
            let trigger = div().id(("navigation-trigger", index)).tab_index(0).sx((
                &NAVIGATION.trigger,
                highlighted.then_some(&NAVIGATION.trigger_highlighted),
            ));
            match entry {
                NavigationEntry::Link {
                    label, on_click, ..
                } => trigger
                    .on_hover({
                        let close_state = open_panel.clone();
                        move |hovered, _, cx| {
                            if *hovered {
                                close_state.set(None, cx);
                            }
                        }
                    })
                    .when_some(on_click, |trigger, handler| {
                        trigger.on_click(move |_, window, cx| handler(&(), window, cx))
                    })
                    .child(crate::components::bidi_text::text(label))
                    .into_any_element(),
                NavigationEntry::Panel { label, content } => {
                    let is_open = open_index == Some(index);
                    let hover_state = open_panel.clone();
                    let key_state = open_panel.clone();
                    trigger
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
                        .child(crate::components::bidi_text::text(label))
                        .child(
                            Icon::new(if is_open {
                                IconName::ChevronUp
                            } else {
                                IconName::ChevronDown
                            })
                            .size(px(12.))
                            .color(muted_foreground),
                        )
                        .when(is_open, |trigger| {
                            let panel = popover_surface()
                                .id(("navigation-panel", index))
                                .occlude()
                                .sx(&NAVIGATION.panel)
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
            .sx((&NAVIGATION.bar, &self.sx))
            .children(entries)
            .when(open_index.is_some(), |bar| {
                bar.on_mouse_down_out(move |_, _, cx| leave_state.set(None, cx))
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
