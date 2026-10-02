//! Pagination: previous / next links around a window of page numbers.

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, ElementId, StyleRefinement, Window};

use super::direction::DirectionalStyled;
use super::{
    button::{Button, IconPosition},
    direction::ActiveDirection,
};
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// One slot in the page list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageSlot {
    Page(usize),
    Ellipsis,
}

/// Pages to show for `current_page` (1-based) out of `page_count`: always the
/// first and last page, `siblings` pages either side of the current one, and an
/// ellipsis wherever two or more pages are skipped (a single gap shows the page).
pub fn page_slots(current_page: usize, page_count: usize, siblings: usize) -> Vec<PageSlot> {
    if page_count == 0 {
        return Vec::new();
    }
    let current_page = current_page.clamp(1, page_count);
    let window_start = current_page.saturating_sub(siblings).max(1);
    let window_end = (current_page + siblings).min(page_count);
    let mut slots = Vec::new();
    if window_start > 1 {
        slots.push(PageSlot::Page(1));
        match window_start {
            3 => slots.push(PageSlot::Page(2)),
            start if start > 3 => slots.push(PageSlot::Ellipsis),
            _ => {}
        }
    }
    slots.extend((window_start..=window_end).map(PageSlot::Page));
    if window_end < page_count {
        match page_count - window_end {
            2 => slots.push(PageSlot::Page(page_count - 1)),
            gap if gap > 2 => slots.push(PageSlot::Ellipsis),
            _ => {}
        }
        slots.push(PageSlot::Page(page_count));
    }
    slots
}

/// Controlled: pass `current_page` (1-based), update it in `on_change`.
///
/// ```ignore
/// Pagination::new("results", 10)
///     .current_page(page)
///     .on_change(move |page, _, cx| set_page(*page, cx))
/// ```
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    page_count: usize,
    current_page: usize,
    siblings: usize,
    on_change: Option<EventHandler<usize>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Pagination);

impl Pagination {
    pub fn new(id: impl Into<ElementId>, page_count: usize) -> Self {
        Self {
            id: id.into(),
            page_count,
            current_page: 1,
            siblings: 1,
            on_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn current_page(mut self, current_page: usize) -> Self {
        self.current_page = current_page;
        self
    }

    /// How many pages to show either side of the current one. Defaults to 1.
    pub fn siblings(mut self, siblings: usize) -> Self {
        self.siblings = siblings;
        self
    }

    /// Receives the page the user picked (1-based).
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Pagination {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().colors.muted_foreground;
        let is_rtl = cx.direction().is_rtl();
        let current_page = self.current_page.clamp(1, self.page_count.max(1));
        let on_change = self.on_change;
        let go_to = move |page: usize| {
            let on_change = on_change.clone();
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                if let Some(handler) = on_change.as_ref() {
                    handler(&page, window, cx);
                }
            }
        };

        let (previous_icon, next_icon) = if is_rtl {
            (IconName::ChevronRight, IconName::ChevronLeft)
        } else {
            (IconName::ChevronLeft, IconName::ChevronRight)
        };
        // Arrows sit on the outer side of each label; the row itself mirrors in RTL.
        let (previous_icon_position, next_icon_position) = (IconPosition::Start, IconPosition::End);
        let previous = Button::new("pagination-previous")
            .ghost()
            .icon(previous_icon)
            .icon_position(previous_icon_position)
            .label("Previous")
            .disabled(current_page <= 1)
            .on_click(go_to(current_page.saturating_sub(1).max(1)));
        let next = Button::new("pagination-next")
            .ghost()
            .label("Next")
            .icon(next_icon)
            .icon_position(next_icon_position)
            .disabled(current_page >= self.page_count)
            .on_click(go_to((current_page + 1).min(self.page_count)));

        let pages: Vec<AnyElement> = page_slots(current_page, self.page_count, self.siblings)
            .into_iter()
            .enumerate()
            .map(|(slot_index, slot)| match slot {
                PageSlot::Page(page) => {
                    let button = Button::new(("pagination-page", page))
                        .label(page.to_string())
                        .w(px(36.))
                        .px(px(0.))
                        .on_click(go_to(page));
                    if page == current_page {
                        button.outline().into_any_element()
                    } else {
                        button.ghost().into_any_element()
                    }
                }
                PageSlot::Ellipsis => div()
                    .id(("pagination-ellipsis", slot_index))
                    .size(px(36.))
                    .flex_dir()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(IconName::Ellipsis)
                            .size(px(16.))
                            .color(muted_foreground),
                    )
                    .into_any_element(),
            })
            .collect();

        div()
            .id(self.id)
            .flex_dir()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .child(previous)
            .children(pages)
            .child(next)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::{page_slots, PageSlot::*};

    #[test]
    fn page_slots_collapse_far_pages() {
        assert_eq!(page_slots(1, 3, 1), vec![Page(1), Page(2), Page(3)]);
        assert_eq!(
            page_slots(5, 10, 1),
            vec![
                Page(1),
                Ellipsis,
                Page(4),
                Page(5),
                Page(6),
                Ellipsis,
                Page(10)
            ]
        );
        assert_eq!(
            page_slots(1, 10, 1),
            vec![Page(1), Page(2), Ellipsis, Page(10)]
        );
        // A single skipped page shows as its number, not an ellipsis.
        assert_eq!(
            page_slots(3, 5, 0),
            vec![Page(1), Page(2), Page(3), Page(4), Page(5)]
        );
        assert_eq!(
            page_slots(4, 7, 0),
            vec![Page(1), Ellipsis, Page(4), Ellipsis, Page(7)]
        );
        assert_eq!(page_slots(2, 3, 0), vec![Page(1), Page(2), Page(3)]);
        assert!(page_slots(1, 0, 1).is_empty());
    }
}
