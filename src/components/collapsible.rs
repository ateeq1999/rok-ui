//! Collapsible: a trigger that shows and hides a panel.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ElementId, StyleRefinement, Window};

use super::{interaction::on_activate, overlay::child_id};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler},
    styles,
    styles::ApplyStyleOverrides,
};

/// Uncontrolled by default; pass `.open(..)` and `.on_open_change(..)` to control it.
/// `.always_visible(..)` content stays shown in both states (shadcn's peek row).
///
/// ```ignore
/// Collapsible::new("repos")
///     .trigger(Button::new("toggle").ghost().icon_only(IconName::ChevronsUpDown))
///     .always_visible(div().child("@radix-ui/primitives"))
///     .child(div().child("@radix-ui/colors"))
/// ```
#[derive(IntoElement)]
pub struct Collapsible {
    id: ElementId,
    trigger: Option<AnyElement>,
    always_visible: Vec<AnyElement>,
    children: Vec<AnyElement>,
    default_open: bool,
    open: Option<bool>,
    on_open_change: Option<EventHandler<bool>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Collapsible);

impl Collapsible {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            always_visible: Vec::new(),
            children: Vec::new(),
            default_open: false,
            open: None,
            on_open_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The element that toggles the panel. Clicking anywhere on it toggles.
    #[must_use]
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Content shown whether open or closed, between the trigger and the panel.
    #[must_use]
    pub fn always_visible(mut self, element: impl IntoElement) -> Self {
        self.always_visible.push(element.into_any_element());
        self
    }

    /// Start open when uncontrolled.
    #[must_use]
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
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

impl ParentElement for Collapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

styles! {
    COLLAPSIBLE = { root: { display: flex, direction: column, gap: 2 } }
}

impl RenderOnce for Collapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_open = self.default_open;
        let internal = use_keyed_state(child_id(&self.id, "open"), window, cx, || default_open);
        let is_open = self.open.unwrap_or_else(|| internal.get(cx));
        let on_open_change = self.on_open_change;
        let toggle = Rc::new(move |window: &mut Window, cx: &mut App| {
            internal.set(!is_open, cx);
            if let Some(handler) = on_open_change.as_ref() {
                handler(&!is_open, window, cx);
            }
        });

        div()
            .sx((&COLLAPSIBLE.root, &self.sx))
            .when_some(self.trigger, |collapsible, trigger| {
                collapsible.child(on_activate(div().id(self.id.clone()), toggle).child(trigger))
            })
            .children(self.always_visible)
            .when(is_open, |collapsible| collapsible.children(self.children))
            .apply_style_overrides(&self.style_overrides)
    }
}
