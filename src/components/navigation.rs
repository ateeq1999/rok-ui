//! Material-style app navigation: a bottom [`NavigationBar`], a side
//! [`NavigationRail`] and a [`NavigationDrawer`], all built from the same
//! [`NavigationDestination`]s. [`super::AdaptiveScaffold`] switches between them
//! by window width.
//!
//! Each one is controlled: pass `selected_index` and update it from `on_change`.

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, ElementId, SharedString};

use super::interaction::{on_activate, Callback};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    sx::SxStyled,
    theme::ActiveTheme,
};

mod bar;
mod drawer;
mod rail;

pub use bar::NavigationBar;
pub use drawer::{DrawerScope, NavigationDrawer};
pub use rail::NavigationRail;

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
