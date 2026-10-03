//! Material-style app navigation: a bottom [`NavigationBar`], a side
//! [`NavigationRail`] and a [`NavigationDrawer`], all built from the same
//! [`NavigationDestination`]s. [`super::AdaptiveScaffold`] switches between them
//! by window width.
//!
//! Each one is controlled: pass `selected_index` and update it from `on_change`.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    div, prelude::*, px, AnyElement, App, ElementId, SharedString, StyleRefinement, Window,
};

use super::{
    interaction::{on_activate, Callback},
    overlay::child_id,
    tooltip::Tooltip,
};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
    theme::ActiveTheme,
};

/// A place in the app: an icon, a label and an optional badge.
#[derive(Clone, Debug)]
pub struct NavigationDestination {
    icon: IconName,
    selected_icon: Option<IconName>,
    label: SharedString,
    badge: Option<SharedString>,
    disabled: bool,
}

impl NavigationDestination {
    /// Create it with its label.
    pub fn new(icon: IconName, label: impl Into<SharedString>) -> Self {
        Self {
            icon,
            selected_icon: None,
            label: label.into(),
            badge: None,
            disabled: false,
        }
    }

    /// The icon shown while this destination is selected.
    #[must_use]
    pub fn selected_icon(mut self, icon: IconName) -> Self {
        self.selected_icon = Some(icon);
        self
    }

    /// A count on the icon, like unread messages. An empty string shows a dot.
    #[must_use]
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The destination's label.
    #[must_use]
    pub fn label(&self) -> &SharedString {
        &self.label
    }
}

/// When a [`NavigationBar`] or [`NavigationRail`] shows destination labels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NavigationLabelBehavior {
    /// Every destination shows its label.
    #[default]
    AlwaysShow,
    /// Only the selected destination shows its label.
    OnlyShowSelected,
    /// Icons only.
    AlwaysHide,
}

impl NavigationLabelBehavior {
    fn shows(self, selected: bool) -> bool {
        match self {
            Self::AlwaysShow => true,
            Self::OnlyShowSelected => selected,
            Self::AlwaysHide => false,
        }
    }
}

/// Where a [`NavigationRail`] places its destinations vertically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NavigationRailAlignment {
    /// Destinations start at the top.
    #[default]
    Top,
    /// Destinations are centered vertically.
    Center,
    /// Destinations sit at the bottom.
    Bottom,
}

styles! {
    NAVIGATION = {
        bar: {
            display: flex,
            direction: row,
            flex: none,
            width: full,
            height: 20,
            padding_x: 2,
            gap: 2,
            background: muted/40,
            border_top: 1,
            border_color: border,
        },
        bar_item: {
            display: flex,
            direction: column,
            align: center,
            justify: center,
            gap: 1,
            flex: 1,
            min_width: 0,
            text: xs,
            font: medium,
            color: muted_foreground,
            cursor: pointer,
            border: 1,
            border_color: transparent,
            radius: md,
            focus: { border_color: ring },
        },
        selected_label: { color: foreground, font: semibold },
        indicator: {
            position: relative,
            display: flex,
            align: center,
            justify: center,
            width: 16,
            height: 8,
            radius: full,
            hover: { background: accent },
        },
        indicator_selected: { background: primary/15, hover: { background: primary/20 } },
        label: { truncate: true, max_width: full },
        badge: {
            position: absolute,
            display: flex,
            align: center,
            justify: center,
            min_width: 4,
            height: 4,
            padding_x: 1,
            radius: full,
            background: destructive,
            color: destructive_foreground,
            text: xs,
            font: medium,
            line_height: 4,
        },
        badge_dot: { min_width: 2, width: 2, height: 2, padding_x: 0 },
        rail: {
            display: flex,
            direction: column,
            align: center,
            flex: none,
            height: full,
            width: 20,
            padding_y: 3,
            gap: 3,
            background: muted/40,
            border_color: border,
        },
        rail_extended: { width: 64, align: stretch, padding_x: 3 },
        rail_section: { display: flex, direction: column, align: center, gap: 2 },
        rail_section_extended: { align: stretch },
        rail_destinations: { display: flex, direction: column, gap: 3, flex: 1, width: full },
        rail_extended_destinations: { gap: 1 },
        rail_item: {
            display: flex,
            direction: column,
            align: center,
            gap: 1,
            text: xs,
            font: medium,
            color: muted_foreground,
            cursor: pointer,
            border: 1,
            border_color: transparent,
            radius: md,
            focus: { border_color: ring },
        },
        row_item: {
            display: flex,
            direction: row,
            align: center,
            gap: 3,
            height: 12,
            padding_x: 4,
            radius: full,
            text: sm,
            font: medium,
            color: muted_foreground,
            cursor: pointer,
            border: 1,
            border_color: transparent,
            hover: { background: accent, color: accent_foreground },
            focus: { border_color: ring },
        },
        row_item_selected: {
            background: primary/15,
            color: foreground,
            font: semibold,
            hover: { background: primary/20, color: foreground },
        },
        row_label: { flex: 1, truncate: true },
        row_badge: { text: xs, font: medium },
        drawer: {
            display: flex,
            direction: column,
            gap: 0.5,
            width: full,
            padding: 3,
        },
        drawer_header: { display: flex, direction: column, padding_x: 4, padding_y: 3 },
        drawer_section: {
            display: flex,
            align: center,
            height: 12,
            padding_x: 4,
            text: sm,
            font: medium,
            color: muted_foreground,
        },
        drawer_divider: { height: 0, border_top: 1, border_color: border, margin_y: 2, margin_x: 4 },
        disabled: { opacity: 0.4, cursor: default },
    }
}

/// The icon with its badge, inside the pill-shaped selection indicator.
fn indicator(destination: &NavigationDestination, selected: bool, cx: &App) -> gpui::Div {
    let colors = &cx.theme().colors;
    let icon = match (selected, destination.selected_icon) {
        (true, Some(icon)) => icon,
        _ => destination.icon,
    };
    let color = if selected {
        colors.foreground
    } else {
        colors.muted_foreground
    };
    div()
        .sx((
            &NAVIGATION.indicator,
            selected.then_some(&NAVIGATION.indicator_selected),
        ))
        .child(Icon::new(icon).size(px(20.)).color(color))
        .when_some(destination.badge.clone(), |indicator, badge| {
            indicator.child(render_badge(badge))
        })
}

fn render_badge(badge: SharedString) -> gpui::Div {
    let dot = badge.is_empty();
    div()
        .sx((&NAVIGATION.badge, dot.then_some(&NAVIGATION.badge_dot)))
        .top(px(if dot { 4. } else { -2. }))
        .left(px(if dot { 38. } else { 34. }))
        .when(!dot, |badge_element| badge_element.child(badge))
}

/// Make `item` select `index` on click, Enter or Space, unless it is disabled.
fn selectable(
    item: gpui::Stateful<gpui::Div>,
    index: usize,
    destination: &NavigationDestination,
    on_change: Option<&EventHandler<usize>>,
    after_change: Option<Callback>,
) -> gpui::Stateful<gpui::Div> {
    if destination.disabled {
        return item.sx(&NAVIGATION.disabled);
    }
    let on_change = on_change.cloned();
    on_activate(
        item.tab_index(0),
        Rc::new(move |window, cx| {
            if let Some(handler) = on_change.as_ref() {
                handler(&index, window, cx);
            }
            if let Some(after_change) = after_change.as_ref() {
                after_change(window, cx);
            }
        }),
    )
}

/// A destination as a full-width row with its icon and label, for drawers and
/// extended rails.
fn row_item(
    id: ElementId,
    index: usize,
    destination: &NavigationDestination,
    selected: bool,
    on_change: Option<&EventHandler<usize>>,
    after_change: Option<Callback>,
    cx: &App,
) -> AnyElement {
    let colors = &cx.theme().colors;
    let icon = match (selected, destination.selected_icon) {
        (true, Some(icon)) => icon,
        _ => destination.icon,
    };
    let icon_color = if selected {
        colors.foreground
    } else {
        colors.muted_foreground
    };
    let item = div()
        .id(id)
        .sx((
            &NAVIGATION.row_item,
            selected.then_some(&NAVIGATION.row_item_selected),
        ))
        .child(Icon::new(icon).size(px(20.)).color(icon_color))
        .child(
            div()
                .sx(&NAVIGATION.row_label)
                .child(super::bidi_text::text(destination.label.clone())),
        )
        .when_some(destination.badge.clone(), |item, badge| {
            if badge.is_empty() {
                item.child(render_badge(badge).relative().top_0().left_0())
            } else {
                item.child(
                    div()
                        .sx(&NAVIGATION.row_badge)
                        .child(super::bidi_text::text(badge)),
                )
            }
        });
    selectable(item, index, destination, on_change, after_change).into_any_element()
}

/// A bar of three to five destinations along the bottom of the window, for
/// compact (phone-sized) layouts.
///
/// ```ignore
/// NavigationBar::new("main-nav")
///     .destination(NavigationDestination::new(IconName::Home, "Home"))
///     .destination(NavigationDestination::new(IconName::Inbox, "Inbox").badge("3"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx))
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
                                .child(super::bidi_text::text(destination.label.clone())),
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

/// A narrow column of destinations along the start edge, for medium and
/// expanded layouts. `extended` widens it to show labels beside the icons.
///
/// ```ignore
/// NavigationRail::new("rail")
///     .leading(FloatingActionButton::new("compose", IconName::Pencil))
///     .destination(NavigationDestination::new(IconName::Inbox, "Inbox"))
///     .destination(NavigationDestination::new(IconName::Send, "Sent"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx))
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
                                .child(super::bidi_text::text(destination.label.clone())),
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

        let border_end = super::direction::is_rtl();
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

/// An entry in a [`NavigationDrawer`].
enum DrawerEntry {
    Section(SharedString),
    Divider,
    Destination(NavigationDestination),
}

/// A list of destinations, grouped under section headings, for a side panel:
/// as the `drawer` of a [`super::Scaffold`] (it closes when a destination is
/// picked) or as a permanent panel in wide layouts.
///
/// Destinations are numbered in order across sections, ignoring headings and
/// dividers.
///
/// ```ignore
/// NavigationDrawer::new("drawer")
///     .header(H4::new("Mail"))
///     .destination(NavigationDestination::new(IconName::Inbox, "Inbox").badge("24"))
///     .destination(NavigationDestination::new(IconName::Send, "Outbox"))
///     .divider()
///     .section("Labels")
///     .destination(NavigationDestination::new(IconName::Star, "Starred"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx))
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
                        .child(super::bidi_text::text(label.clone()))
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
