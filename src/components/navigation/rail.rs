//! [`NavigationRail`]: destinations down the side of the window, compact or extended.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ElementId, StyleRefinement, Window};

use super::{
    indicator, row_item, selectable, NavigationDestination, NavigationLabelBehavior,
    NavigationRailAlignment, NAVIGATION,
};
use crate::components::{overlay::child_id, tooltip::Tooltip};
use crate::{
    hooks::EventHandler,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// A narrow column of destinations along the start edge, for medium and
/// expanded layouts. `extended` widens it to show labels beside the icons.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let page = 0;
/// # fn set_page(_: usize, _: &mut App) {}
/// let rail = NavigationRail::new("rail")
///     .leading(FloatingActionButton::new("compose", IconName::Pencil))
///     .destination(NavigationDestination::new(IconName::Inbox, "Inbox"))
///     .destination(NavigationDestination::new(IconName::Send, "Sent"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx));
/// # }
/// ```
#[derive(IntoElement)]
pub struct NavigationRail {
    id: ElementId,
    destinations: Vec<NavigationDestination>,
    selected_index: usize,
    extended: bool,
    label_behavior: NavigationLabelBehavior,
    alignment: NavigationRailAlignment,
    leading: Vec<AnyElement>,
    trailing: Vec<AnyElement>,
    on_change: Option<EventHandler<usize>>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(NavigationRail);

impl NavigationRail {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            destinations: Vec::new(),
            selected_index: 0,
            extended: false,
            label_behavior: NavigationLabelBehavior::AlwaysShow,
            alignment: NavigationRailAlignment::Top,
            leading: Vec::new(),
            trailing: Vec::new(),
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

    /// Show labels beside the icons in a wider rail.
    #[must_use]
    pub fn extended(mut self, extended: bool) -> Self {
        self.extended = extended;
        self
    }

    /// Labels under the icons (ignored while extended).
    #[must_use]
    pub fn label_behavior(mut self, behavior: NavigationLabelBehavior) -> Self {
        self.label_behavior = behavior;
        self
    }

    /// Where the destinations sit vertically. Default: `Top`.
    #[must_use]
    pub fn alignment(mut self, alignment: NavigationRailAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Above the destinations: a menu button or a floating action button.
    #[must_use]
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading.push(element.into_any_element());
        self
    }

    /// Below the destinations: settings or the user's avatar.
    #[must_use]
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing.push(element.into_any_element());
        self
    }

    /// Called with the new value.
    #[must_use]
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavigationRail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let extended = self.extended;
        let item_id = |index: usize| {
            ElementId::NamedChild(Box::new(self.id.clone()), index.to_string().into())
        };
        let items: Vec<AnyElement> = self
            .destinations
            .iter()
            .enumerate()
            .map(|(index, destination)| {
                let selected = index == self.selected_index;
                if extended {
                    return row_item(
                        item_id(index),
                        index,
                        destination,
                        selected,
                        self.on_change.as_ref(),
                        None,
                        cx,
                    );
                }
                let shows_label = self.label_behavior.shows(selected);
                let item = div()
                    .id(item_id(index))
                    .sx(&NAVIGATION.rail_item)
                    .child(indicator(destination, selected, cx))
                    .when(shows_label, |item| {
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
                    .when(!shows_label, |item| {
                        item.tooltip(Tooltip::text(destination.label.clone()))
                    });
                selectable(item, index, destination, self.on_change.as_ref(), None)
                    .into_any_element()
            })
            .collect();

        let section = |children: Vec<AnyElement>| {
            div()
                .sx((
                    &NAVIGATION.rail_section,
                    extended.then_some(&NAVIGATION.rail_section_extended),
                ))
                .children(children)
        };
        let mut destinations = div()
            .id(child_id(&self.id, "destinations"))
            .overflow_y_scroll()
            .sx((
                &NAVIGATION.rail_destinations,
                extended.then_some(&NAVIGATION.rail_extended_destinations),
            ))
            .children(items);
        destinations.style().justify_content = Some(match self.alignment {
            NavigationRailAlignment::Top => gpui::JustifyContent::FlexStart,
            NavigationRailAlignment::Center => gpui::JustifyContent::Center,
            NavigationRailAlignment::Bottom => gpui::JustifyContent::FlexEnd,
        });
        if !extended {
            destinations.style().align_items = Some(gpui::AlignItems::Center);
        }

        let border_end = crate::components::direction::is_rtl();
        div()
            .id(self.id.clone())
            .sx((
                &NAVIGATION.rail,
                extended.then_some(&NAVIGATION.rail_extended),
                &self.sx,
            ))
            .map(|rail| {
                // The border faces the content, on the end side.
                if border_end {
                    rail.border_l_1()
                } else {
                    rail.border_r_1()
                }
            })
            .when(!self.leading.is_empty(), |rail| {
                rail.child(section(self.leading))
            })
            .child(destinations)
            .when(!self.trailing.is_empty(), |rail| {
                rail.child(section(self.trailing))
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
