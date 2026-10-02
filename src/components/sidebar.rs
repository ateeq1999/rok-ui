//! Sidebar: an application side navigation that can collapse to icons.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, FontWeight, SharedString,
    StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::{button::Button, direction::ActiveDirection, overlay::child_id, tooltip::Tooltip};
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Which edge the sidebar sits on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

/// A navigation row. Rows with sub-items expand and collapse.
pub struct SidebarItem {
    label: SharedString,
    icon: Option<IconName>,
    badge: Option<SharedString>,
    active: bool,
    on_click: Option<EventHandler<()>>,
    sub_items: Vec<SidebarItem>,
    default_open: bool,
}

impl SidebarItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            badge: None,
            active: false,
            on_click: None,
            sub_items: Vec::new(),
            default_open: false,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A count or tag on the right, like unread messages.
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Highlight as the current page.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// A nested row, shown when this row is expanded.
    pub fn sub_item(mut self, item: SidebarItem) -> Self {
        self.sub_items.push(item);
        self
    }

    /// Start expanded (rows with sub-items only).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
}

/// A labeled set of [`SidebarItem`]s.
pub struct SidebarGroup {
    label: Option<SharedString>,
    items: Vec<SidebarItem>,
}

impl SidebarGroup {
    pub fn new() -> Self {
        Self {
            label: None,
            items: Vec::new(),
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn item(mut self, item: SidebarItem) -> Self {
        self.items.push(item);
        self
    }
}

impl Default for SidebarGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// Controlled: pass `collapsed`; toggle it from a [`SidebarTrigger`]. Collapsed,
/// it shrinks to icons with tooltips.
///
/// ```ignore
/// div().flex_dir().size_full()
///     .child(Sidebar::new("app-sidebar")
///         .collapsed(collapsed)
///         .header(team_switcher)
///         .group(SidebarGroup::new().label("Application")
///             .item(SidebarItem::new("Home").icon(IconName::Home).active(true))
///             .item(SidebarItem::new("Inbox").icon(IconName::Inbox).badge("24")))
///         .footer(user_menu))
///     .child(div().flex_1().child(SidebarTrigger::new("toggle").on_toggle(..)))
/// ```
#[derive(IntoElement)]
pub struct Sidebar {
    id: ElementId,
    collapsed: bool,
    side: SidebarSide,
    header: Vec<AnyElement>,
    footer: Vec<AnyElement>,
    groups: Vec<SidebarGroup>,
    children: Vec<AnyElement>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Sidebar);

impl Sidebar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            collapsed: false,
            side: SidebarSide::Left,
            header: Vec::new(),
            footer: Vec::new(),
            groups: Vec::new(),
            children: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn side(mut self, side: SidebarSide) -> Self {
        self.side = side;
        self
    }

    /// Pinned to the top (an app or team switcher).
    pub fn header(mut self, element: impl IntoElement) -> Self {
        self.header.push(element.into_any_element());
        self
    }

    /// Pinned to the bottom (the user menu).
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.footer.push(element.into_any_element());
        self
    }

    pub fn group(mut self, group: SidebarGroup) -> Self {
        self.groups.push(group);
        self
    }
}

impl ParentElement for Sidebar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

fn render_item(
    item: SidebarItem,
    item_id: ElementId,
    collapsed: bool,
    depth: usize,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let has_sub_items = !item.sub_items.is_empty();
    let default_open = item.default_open;
    let open_state = use_keyed_state(child_id(&item_id, "open"), window, cx, || default_open);
    let is_open = has_sub_items && open_state.get(cx);
    let theme = cx.theme();
    let colors = theme.colors.clone();
    let ring_color = colors.ring;
    let on_click = item.on_click.clone();
    let label = item.label.clone();

    let row = div()
        .id(item_id.clone())
        .flex_dir()
        .items_center()
        .gap(px(8.))
        .h(if depth > 0 { px(28.) } else { px(32.) })
        .px(px(8.))
        .rounded(theme.radius_medium())
        .text_sm()
        .cursor(CursorStyle::PointingHand)
        .tab_index(0)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.border_color(ring_color))
        .hover(|style| style.bg(colors.accent).text_color(colors.accent_foreground))
        .when(item.active, |row| {
            row.bg(colors.accent)
                .text_color(colors.accent_foreground)
                .font_weight(FontWeight::MEDIUM)
        })
        .when(collapsed, |row| {
            row.w(px(32.))
                .justify_center()
                .px(px(0.))
                .tooltip(Tooltip::text(label.clone()))
        })
        .on_click(move |_, window, cx| {
            if has_sub_items {
                open_state.update(cx, |open| *open = !*open);
            }
            if let Some(handler) = on_click.as_ref() {
                handler(&(), window, cx);
            }
        })
        .when_some(item.icon, |row, icon| {
            row.child(Icon::new(icon).size(px(16.)).color(colors.foreground))
        })
        .when(!collapsed, |row| {
            row.child(div().flex_1().truncate().child(item.label))
                .when_some(item.badge, |row, badge| {
                    row.child(div().text_xs().font_weight(FontWeight::MEDIUM).child(badge))
                })
                .when(has_sub_items, |row| {
                    row.child(
                        Icon::new(if is_open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight.for_direction()
                        })
                        .size(px(14.))
                        .color(colors.muted_foreground),
                    )
                })
        });

    if !is_open || collapsed {
        return row.into_any_element();
    }
    let sub_rows: Vec<AnyElement> = item
        .sub_items
        .into_iter()
        .enumerate()
        .map(|(index, sub_item)| {
            let sub_id = ElementId::NamedChild(Box::new(item_id.clone()), index.to_string().into());
            render_item(sub_item, sub_id, collapsed, depth + 1, window, cx)
        })
        .collect();
    div()
        .flex_dir()
        .flex_col()
        .gap(px(2.))
        .child(row)
        .child(
            div()
                .flex_dir()
                .flex_col()
                .gap(px(2.))
                .ms(px(14.))
                .ps(px(10.))
                .border_s_1()
                .border_color(cx.theme().colors.border)
                .children(sub_rows),
        )
        .into_any_element()
}

impl RenderOnce for Sidebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let collapsed = self.collapsed;
        let side = match (self.side, cx.direction().is_rtl()) {
            (SidebarSide::Left, true) => SidebarSide::Right,
            (SidebarSide::Right, true) => SidebarSide::Left,
            (side, false) => side,
        };
        let groups: Vec<AnyElement> = self
            .groups
            .into_iter()
            .enumerate()
            .map(|(group_index, group)| {
                let muted_foreground = cx.theme().colors.muted_foreground;
                let rows: Vec<AnyElement> = group
                    .items
                    .into_iter()
                    .enumerate()
                    .map(|(item_index, item)| {
                        let item_id = ElementId::NamedChild(
                            Box::new(child_id(&self.id, "item")),
                            format!("{group_index}-{item_index}").into(),
                        );
                        render_item(item, item_id, collapsed, 0, window, cx)
                    })
                    .collect();
                div()
                    .flex_dir()
                    .flex_col()
                    .gap(px(2.))
                    .p(px(8.))
                    .when_some(group.label.filter(|_| !collapsed), |group, label| {
                        group.child(
                            div()
                                .h(px(32.))
                                .px(px(8.))
                                .flex_dir()
                                .items_center()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(muted_foreground)
                                .child(label),
                        )
                    })
                    .children(rows)
                    .into_any_element()
            })
            .collect();

        let colors = cx.theme().colors.clone();
        div()
            .id(self.id)
            .flex_dir()
            .flex_col()
            .flex_none()
            .h_full()
            .w(if collapsed { px(48.) } else { px(256.) })
            .bg(colors.muted.opacity(0.4))
            .border_color(colors.border)
            .map(|sidebar| match side {
                SidebarSide::Left => sidebar.border_r_1(),
                SidebarSide::Right => sidebar.border_l_1(),
            })
            .when(!self.header.is_empty(), |sidebar| {
                sidebar.child(
                    div()
                        .flex_dir()
                        .flex_col()
                        .gap(px(8.))
                        .p(px(8.))
                        .children(self.header),
                )
            })
            .child(
                div()
                    .id("sidebar-content")
                    .flex_dir()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .children(groups)
                    .children(self.children),
            )
            .when(!self.footer.is_empty(), |sidebar| {
                sidebar.child(
                    div()
                        .flex_dir()
                        .flex_col()
                        .gap(px(8.))
                        .p(px(8.))
                        .children(self.footer),
                )
            })
            .apply_style_overrides(&self.style_overrides)
    }
}

/// The ghost button that collapses and expands a [`Sidebar`].
#[derive(IntoElement)]
pub struct SidebarTrigger {
    id: ElementId,
    on_toggle: Option<EventHandler<()>>,
}

impl SidebarTrigger {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            on_toggle: None,
        }
    }

    pub fn on_toggle(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SidebarTrigger {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let on_toggle = self.on_toggle;
        Button::new(self.id)
            .ghost()
            .icon_only(IconName::PanelLeft)
            .w(px(28.))
            .h(px(28.))
            .tooltip("Toggle sidebar")
            .on_click(move |_, window, cx| {
                if let Some(handler) = on_toggle.as_ref() {
                    handler(&(), window, cx);
                }
            })
    }
}
