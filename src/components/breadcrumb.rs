//! Breadcrumb: the path to the current page as a row of links.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, SharedString, StyleRefinement,
    Window,
};

use super::{
    direction::ActiveDirection,
    menu::{DropdownMenu, Menu},
};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

enum BreadcrumbEntry {
    Link {
        label: SharedString,
        on_click: Option<EventHandler<()>>,
    },
    Page(SharedString),
    Ellipsis(Option<Menu>),
}

/// ```ignore
/// Breadcrumb::new("path")
///     .link("Home", |_, _, cx| navigate("/", cx))
///     .ellipsis_menu(Menu::new().item(MenuItem::new("Documentation")))
///     .link("Components", |_, _, cx| navigate("/components", cx))
///     .page("Breadcrumb")
/// ```
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    entries: Vec<BreadcrumbEntry>,
    separator: Option<SharedString>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Breadcrumb);

impl Breadcrumb {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            separator: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A clickable ancestor page.
    pub fn link(
        mut self,
        label: impl Into<SharedString>,
        on_click: impl Fn(&(), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.entries.push(BreadcrumbEntry::Link {
            label: label.into(),
            on_click: Some(Rc::new(on_click)),
        });
        self
    }

    /// An ancestor shown as plain text.
    pub fn text(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(BreadcrumbEntry::Link {
            label: label.into(),
            on_click: None,
        });
        self
    }

    /// The current page; not clickable.
    pub fn page(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(BreadcrumbEntry::Page(label.into()));
        self
    }

    /// `…` standing in for collapsed levels.
    pub fn ellipsis(mut self) -> Self {
        self.entries.push(BreadcrumbEntry::Ellipsis(None));
        self
    }

    /// `…` that opens a menu of the collapsed levels.
    pub fn ellipsis_menu(mut self, menu: Menu) -> Self {
        self.entries.push(BreadcrumbEntry::Ellipsis(Some(menu)));
        self
    }

    /// Replace the chevron between items with custom text, like `"/"`.
    pub fn separator(mut self, separator: impl Into<SharedString>) -> Self {
        self.separator = Some(separator.into());
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let separator_icon = if cx.direction().is_rtl() {
            IconName::ChevronLeft
        } else {
            IconName::ChevronRight
        };
        let entry_count = self.entries.len();
        let mut children: Vec<AnyElement> = Vec::new();
        for (index, entry) in self.entries.into_iter().enumerate() {
            children.push(match entry {
                BreadcrumbEntry::Link { label, on_click } => div()
                    .id(index)
                    .when_some(on_click, |link, handler| {
                        link.cursor(CursorStyle::PointingHand)
                            .hover(|style| style.text_color(colors.foreground))
                            .on_click(move |_, window, cx| handler(&(), window, cx))
                    })
                    .child(label)
                    .into_any_element(),
                BreadcrumbEntry::Page(label) => div()
                    .text_color(colors.foreground)
                    .child(label)
                    .into_any_element(),
                BreadcrumbEntry::Ellipsis(menu) => {
                    let dots = div()
                        .id(("breadcrumb-ellipsis", index))
                        .size(px(36.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(IconName::Ellipsis)
                                .size(px(16.))
                                .color(colors.muted_foreground),
                        );
                    match menu {
                        Some(menu) => DropdownMenu::new(("breadcrumb-menu", index))
                            .trigger(dots.cursor(CursorStyle::PointingHand))
                            .menu(menu)
                            .into_any_element(),
                        None => dots.into_any_element(),
                    }
                }
            });
            if index + 1 < entry_count {
                children.push(match self.separator.clone() {
                    Some(separator) => div().child(separator).into_any_element(),
                    None => Icon::new(separator_icon)
                        .size(px(14.))
                        .color(colors.muted_foreground)
                        .into_any_element(),
                });
            }
        }

        div()
            .id(self.id)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.))
            .text_sm()
            .text_color(colors.muted_foreground)
            .when(cx.direction().is_rtl(), |row| row.flex_row_reverse())
            .children(children)
            .apply_style_overrides(&self.style_overrides)
    }
}
