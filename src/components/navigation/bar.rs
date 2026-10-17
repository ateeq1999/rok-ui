//! [`NavigationBar`]: destinations along the bottom of the window.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ElementId, StyleRefinement, Window};

use super::{indicator, selectable, NavigationDestination, NavigationLabelBehavior, NAVIGATION};
use crate::components::tooltip::Tooltip;
use crate::{
    hooks::EventHandler,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// A bar of three to five destinations along the bottom of the window, for
/// compact (phone-sized) layouts.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let page = 0;
/// # fn set_page(_: usize, _: &mut App) {}
/// let bar = NavigationBar::new("main-nav")
///     .destination(NavigationDestination::new(IconName::Home, "Home"))
///     .destination(NavigationDestination::new(IconName::Inbox, "Inbox").badge("3"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx));
/// # }
/// ```
#[derive(IntoElement)]
pub struct NavigationBar {
    id: ElementId,
    destinations: Vec<NavigationDestination>,
    selected_index: usize,
    label_behavior: NavigationLabelBehavior,
    on_change: Option<EventHandler<usize>>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(NavigationBar);

impl NavigationBar {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            destinations: Vec::new(),
            selected_index: 0,
            label_behavior: NavigationLabelBehavior::AlwaysShow,
            on_change: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add a destination.
    #[must_use]
    pub fn destination(mut self, destination: NavigationDestination) -> Self {
        self.destinations.push(destination);
        self
    }

    /// Add several destinations.
    #[must_use]
    pub fn destinations(
        mut self,
        destinations: impl IntoIterator<Item = NavigationDestination>,
    ) -> Self {
        self.destinations.extend(destinations);
        self
    }

    /// The selected destination.
    #[must_use]
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// When destination labels are shown.
    #[must_use]
    pub fn label_behavior(mut self, behavior: NavigationLabelBehavior) -> Self {
        self.label_behavior = behavior;
        self
    }

    /// Called with the new value.
    #[must_use]
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavigationBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let items: Vec<AnyElement> = self
            .destinations
            .iter()
            .enumerate()
            .map(|(index, destination)| {
                let selected = index == self.selected_index;
                let item = div()
                    .id(ElementId::NamedChild(
                        Box::new(self.id.clone()),
                        index.to_string().into(),
                    ))
                    .sx(&NAVIGATION.bar_item)
                    .child(indicator(destination, selected, cx))
                    .when(self.label_behavior.shows(selected), |item| {
                        item.child(
                            div()
                                .sx((
                                    &NAVIGATION.label,
                                    selected.then_some(&NAVIGATION.selected_label),
                                ))
                                .child(crate::components::bidi_text::text(
                                    destination.label.clone(),
                                )),
                        )
                    })
                    .when(!self.label_behavior.shows(selected), |item| {
                        item.tooltip(Tooltip::text(destination.label.clone()))
                    });
                selectable(item, index, destination, self.on_change.as_ref(), None)
                    .into_any_element()
            })
            .collect();
        div()
            .id(self.id)
            .sx((&NAVIGATION.bar, &self.sx))
            .children(items)
            .apply_style_overrides(&self.style_overrides)
    }
}
