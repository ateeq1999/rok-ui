//! What opens a menu: [`DropdownMenu`] (a trigger element) and [`ContextMenu`] (a right
//! click).

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, Corner, ElementId, MouseButton, Pixels, Point,
    StyleRefinement, Window,
};

use super::{close_handler, render_menu_panel, Menu};
use crate::components::layer::layer_at;
use crate::components::overlay::{
    child_id, dismissable, floating, trigger_wrapper, use_open_state, Align, Side,
};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler},
    motion::{presets, MotionExt},
    styles::ApplyStyleOverrides,
};

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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(DropdownMenu);

impl DropdownMenu {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            menu: Menu::new(),
            side: Side::Bottom,
            align: Align::Start,
            open: None,
            on_open_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The element that opens the menu on click.
    #[must_use]
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// The menu's items.
    #[must_use]
    pub fn menu(mut self, menu: Menu) -> Self {
        self.menu = menu;
        self
    }

    /// Which side it opens on.
    #[must_use]
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// How it lines up with its trigger.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Whether it is open (controlled).
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Called with the new open state.
    #[must_use]
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
        .sx(&self.sx)
        .apply_style_overrides(&self.style_overrides);
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(self.side, self.align, panel, cx))
    }
}

/// A menu opened by right-clicking an area (shadcn/ui's `<ContextMenu>`).
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let canvas = ContextMenu::new("canvas-menu")
///     .menu(Menu::new().item(MenuItem::new("Back").shortcut("Cmd-[")))
///     .child(div().size(px(300.)).child("Right click here"));
/// # }
/// ```
#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    menu: Menu,
    children: Vec<AnyElement>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ContextMenu);

impl ContextMenu {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            menu: Menu::new(),
            children: Vec::new(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The menu's items.
    #[must_use]
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
            .sx(&self.sx)
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
