//! Popover: rich floating content opened by clicking a trigger.

use std::rc::Rc;

use gpui::{prelude::*, AnyElement, App, ElementId, StyleRefinement, Window};

use super::overlay::{
    dismissable, floating, popover_surface, trigger_wrapper, use_open_state, Align, Side,
};
use crate::sx::SxStyled;
use crate::{hooks::EventHandler, styles, styles::ApplyStyleOverrides};

/// Uncontrolled by default: clicking the trigger opens it, Escape or a click
/// outside closes it. Pass `.open(..)` with `.on_open_change(..)` to control it.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let width = use_input_state("width", window, cx, |state| state);
/// let popover = Popover::new("dimensions")
///     .trigger(Button::new("open").outline().label("Open popover"))
///     .child(Label::new("Width"))
///     .child(Input::new(&width));
/// # }
/// ```
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    trigger: Option<AnyElement>,
    children: Vec<AnyElement>,
    side: Side,
    align: Align,
    open: Option<bool>,
    on_open_change: Option<EventHandler<bool>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Popover);

impl Popover {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            children: Vec::new(),
            side: Side::Bottom,
            align: Align::Start,
            open: None,
            on_open_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The element that opens the popover, usually a [`super::Button`].
    #[must_use]
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Which side of the trigger it opens on. Defaults to below.
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

    /// Control the open state yourself.
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Called whenever the popover wants to open or close.
    #[must_use]
    pub fn on_open_change(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

styles! {
    POPOVER = { panel: { width: 72, padding: 4, gap: 3 } }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state = use_open_state(&self.id, self.open, self.on_open_change, window, cx);
        let is_open = open_state.is_open(cx);
        let wrapper = trigger_wrapper(self.id, &open_state, false, cx).children(self.trigger);
        if !is_open {
            return wrapper;
        }
        let panel = popover_surface()
            .sx((&POPOVER.panel, &self.sx))
            .children(self.children)
            .apply_style_overrides(&self.style_overrides);
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(self.side, self.align, panel, cx))
    }
}
