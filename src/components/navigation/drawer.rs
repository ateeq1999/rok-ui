//! [`NavigationDrawer`]: destinations grouped under headings in a side panel, and the scope
//! that lets a scaffold's drawer close when one is picked.

use std::{cell::RefCell, rc::Rc};

use gpui::{div, prelude::*, AnyElement, App, ElementId, SharedString, StyleRefinement, Window};

use super::{row_item, NavigationDestination, NAVIGATION};
use crate::components::interaction::Callback;
use crate::{
    hooks::EventHandler,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// An entry in a [`NavigationDrawer`].
enum DrawerEntry {
    Section(SharedString),
    Divider,
    Destination(NavigationDestination),
}

/// A list of destinations, grouped under section headings, for a side panel:
/// as the `drawer` of a [`crate::components::Scaffold`] (it closes when a destination is
/// picked) or as a permanent panel in wide layouts.
///
/// Destinations are numbered in order across sections, ignoring headings and
/// dividers.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let page = 0;
/// # fn set_page(_: usize, _: &mut App) {}
/// let drawer = NavigationDrawer::new("drawer")
///     .header(H4::new("Mail"))
///     .destination(NavigationDestination::new(IconName::Inbox, "Inbox").badge("24"))
///     .destination(NavigationDestination::new(IconName::Send, "Outbox"))
///     .divider()
///     .section("Labels")
///     .destination(NavigationDestination::new(IconName::Star, "Starred"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx));
/// # }
/// ```
#[derive(IntoElement)]
pub struct NavigationDrawer {
    id: ElementId,
    header: Vec<AnyElement>,
    entries: Vec<DrawerEntry>,
    selected_index: usize,
    on_change: Option<EventHandler<usize>>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(NavigationDrawer);

impl NavigationDrawer {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            header: Vec::new(),
            entries: Vec::new(),
            selected_index: 0,
            on_change: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Content above the destinations, such as the app name.
    #[must_use]
    pub fn header(mut self, element: impl IntoElement) -> Self {
        self.header.push(element.into_any_element());
        self
    }

    /// A heading over the destinations that follow.
    #[must_use]
    pub fn section(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(DrawerEntry::Section(label.into()));
        self
    }

    /// Add a divider after the destinations so far.
    #[must_use]
    pub fn divider(mut self) -> Self {
        self.entries.push(DrawerEntry::Divider);
        self
    }

    /// Add a destination.
    #[must_use]
    pub fn destination(mut self, destination: NavigationDestination) -> Self {
        self.entries.push(DrawerEntry::Destination(destination));
        self
    }

    /// Add several destinations.
    #[must_use]
    pub fn destinations(
        mut self,
        destinations: impl IntoIterator<Item = NavigationDestination>,
    ) -> Self {
        self.entries
            .extend(destinations.into_iter().map(DrawerEntry::Destination));
        self
    }

    /// The selected destination.
    #[must_use]
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// Called with the new value.
    #[must_use]
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavigationDrawer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Inside a scaffold's modal drawer, picking a destination closes it.
        let close_modal = current_drawer_closer();
        let mut index = 0;
        let mut rows: Vec<AnyElement> = Vec::new();
        for entry in &self.entries {
            match entry {
                DrawerEntry::Section(label) => rows.push(
                    div()
                        .sx(&NAVIGATION.drawer_section)
                        .child(crate::components::bidi_text::text(label.clone()))
                        .into_any_element(),
                ),
                DrawerEntry::Divider => {
                    rows.push(div().sx(&NAVIGATION.drawer_divider).into_any_element());
                }
                DrawerEntry::Destination(destination) => {
                    rows.push(row_item(
                        ElementId::NamedChild(Box::new(self.id.clone()), index.to_string().into()),
                        index,
                        destination,
                        index == self.selected_index,
                        self.on_change.as_ref(),
                        close_modal.clone(),
                        cx,
                    ));
                    index += 1;
                }
            }
        }
        div()
            .id(self.id)
            .sx((&NAVIGATION.drawer, &self.sx))
            .when(!self.header.is_empty(), |drawer| {
                drawer.child(div().sx(&NAVIGATION.drawer_header).children(self.header))
            })
            .children(rows)
            .apply_style_overrides(&self.style_overrides)
    }
}

thread_local! {
    /// Closes the modal drawer being rendered, set by `Scaffold` around it.
    static DRAWER_CLOSER: RefCell<Option<Callback>> = const { RefCell::new(None) };
}

/// Wraps the content of a modal drawer so a [`NavigationDrawer`] inside it closes
/// the drawer when a destination is picked. Components render while their parent
/// lays out, so the closer is set during layout.
#[cfg_attr(not(feature = "scaffold"), allow(dead_code))]
#[doc(hidden)]
pub struct DrawerScope {
    pub closer: Callback,
    pub child: AnyElement,
}

impl IntoElement for DrawerScope {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for DrawerScope {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let previous = DRAWER_CLOSER.with(|current| current.replace(Some(self.closer.clone())));
        let layout_id = self.child.request_layout(window, cx);
        DRAWER_CLOSER.with(|current| *current.borrow_mut() = previous);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: gpui::Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: gpui::Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

fn current_drawer_closer() -> Option<Callback> {
    DRAWER_CLOSER.with(|current| current.borrow().clone())
}
