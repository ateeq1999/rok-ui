//! ScrollArea: a scrolling box with a thin themed scrollbar.

use std::rc::Rc;

use gpui::{
    div, point, prelude::*, px, AnyElement, App, CursorStyle, ElementId, MouseButton, Pixels,
    ScrollHandle, StyleRefinement, Window,
};

use super::{interaction::track_drag, overlay::child_id};
use crate::{
    hooks::{use_keyed_state, State},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Which way a [`ScrollArea`] scrolls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

struct ScrollMemory {
    handle: ScrollHandle,
    /// Pointer position and scroll offset (along the dragged axis) when a thumb drag started.
    drag: Option<(bool, Pixels, Pixels)>,
}

/// Thumb length and offset along a track of `track_length`, for a viewport of
/// `viewport_length` scrolled `scrolled` into content longer by `max_scroll`.
pub(crate) fn thumb_geometry(
    track_length: Pixels,
    viewport_length: Pixels,
    max_scroll: Pixels,
    scrolled: Pixels,
) -> (Pixels, Pixels) {
    let content_length = viewport_length + max_scroll;
    if content_length <= px(0.) || max_scroll <= px(0.) {
        return (track_length, px(0.));
    }
    let thumb_length = (track_length * (viewport_length / content_length)).max(px(24.));
    let travel = (track_length - thumb_length).max(px(0.));
    let progress = (scrolled / max_scroll).clamp(0., 1.);
    (thumb_length, travel * progress)
}

/// Give it a size (`.h(px(288.))`); content larger than that scrolls.
///
/// ```ignore
/// ScrollArea::new("tags").h(px(288.)).w(px(192.)).child(tag_list)
/// ```
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    axis: ScrollAxis,
    children: Vec<AnyElement>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ScrollArea);

impl ScrollArea {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            axis: ScrollAxis::Vertical,
            children: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Shorthand for `.axis(ScrollAxis::Horizontal)`.
    pub fn horizontal(self) -> Self {
        self.axis(ScrollAxis::Horizontal)
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let memory: State<ScrollMemory> =
            use_keyed_state(child_id(&self.id, "scroll"), window, cx, || ScrollMemory {
                handle: ScrollHandle::new(),
                drag: None,
            });
        let handle = memory.read(cx).handle.clone();
        let drag = memory.read(cx).drag;
        let colors = cx.theme().colors.clone();
        let thumb_color = colors.border;
        let viewport = handle.bounds().size;
        let max_offset = handle.max_offset();
        let offset = handle.offset();
        let scrolls_vertically = self.axis != ScrollAxis::Horizontal;
        let scrolls_horizontally = self.axis != ScrollAxis::Vertical;

        let scrollbar = |vertical: bool| -> Option<AnyElement> {
            let (viewport_length, max_scroll, scrolled) = if vertical {
                (viewport.height, max_offset.height, -offset.y)
            } else {
                (viewport.width, max_offset.width, -offset.x)
            };
            if max_scroll <= px(0.) {
                return None;
            }
            let track_length = viewport_length - px(4.);
            let (thumb_length, thumb_offset) =
                thumb_geometry(track_length, viewport_length, max_scroll, scrolled);
            let press_memory = memory.clone();
            let thumb = div()
                .id(if vertical {
                    "scroll-thumb-y"
                } else {
                    "scroll-thumb-x"
                })
                .absolute()
                .rounded_full()
                .bg(thumb_color)
                .hover(|style| style.bg(colors.muted_foreground.opacity(0.5)))
                .cursor(CursorStyle::Arrow)
                .map(|thumb| {
                    if vertical {
                        thumb.top(thumb_offset).h(thumb_length).w_full()
                    } else {
                        thumb.left(thumb_offset).w(thumb_length).h_full()
                    }
                })
                .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                    cx.stop_propagation();
                    let pointer = if vertical {
                        event.position.y
                    } else {
                        event.position.x
                    };
                    press_memory.update(cx, |memory| {
                        memory.drag = Some((vertical, pointer, scrolled))
                    });
                });
            Some(
                div()
                    .absolute()
                    .p(px(1.))
                    .map(|track| {
                        if vertical {
                            track.top(px(2.)).right(px(1.)).w(px(10.)).h(track_length)
                        } else {
                            track.left(px(2.)).bottom(px(1.)).h(px(10.)).w(track_length)
                        }
                    })
                    .child(div().relative().size_full().child(thumb))
                    .into_any_element(),
            )
        };
        let vertical_bar = scrolls_vertically.then(|| scrollbar(true)).flatten();
        let horizontal_bar = scrolls_horizontally.then(|| scrollbar(false)).flatten();

        let drag_handle = handle.clone();
        let end_memory = memory.clone();
        let content = div()
            .id(child_id(&self.id, "viewport"))
            .size_full()
            .track_scroll(&handle)
            .map(|content| match self.axis {
                ScrollAxis::Vertical => content.overflow_y_scroll(),
                ScrollAxis::Horizontal => content.overflow_x_scroll(),
                ScrollAxis::Both => content.overflow_scroll(),
            })
            .child(
                div()
                    .flex()
                    .when(scrolls_vertically && !scrolls_horizontally, |inner| {
                        inner.flex_col()
                    })
                    .children(self.children),
            );

        div()
            .id(self.id)
            .relative()
            .overflow_hidden()
            .child(content)
            .children(vertical_bar)
            .children(horizontal_bar)
            .child(track_drag(
                drag.is_some(),
                Rc::new(move |position, window, _| {
                    let Some((vertical, start_pointer, start_scrolled)) = drag else {
                        return;
                    };
                    let viewport = drag_handle.bounds().size;
                    let max_offset = drag_handle.max_offset();
                    let (pointer, viewport_length, max_scroll) = if vertical {
                        (position.y, viewport.height, max_offset.height)
                    } else {
                        (position.x, viewport.width, max_offset.width)
                    };
                    let track_length = viewport_length - px(4.);
                    let (thumb_length, _) =
                        thumb_geometry(track_length, viewport_length, max_scroll, px(0.));
                    let travel = (track_length - thumb_length).max(px(1.));
                    let scrolled = (start_scrolled
                        + (pointer - start_pointer) * (max_scroll / travel))
                        .clamp(px(0.), max_scroll);
                    let current = drag_handle.offset();
                    drag_handle.set_offset(if vertical {
                        point(current.x, -scrolled)
                    } else {
                        point(-scrolled, current.y)
                    });
                    window.refresh();
                }),
                Rc::new(move |_, cx| end_memory.update(cx, |memory| memory.drag = None)),
            ))
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::thumb_geometry;

    #[test]
    fn thumb_tracks_scroll_progress() {
        // Content twice the viewport: half-size thumb.
        assert_eq!(
            thumb_geometry(px(100.), px(100.), px(100.), px(0.)),
            (px(50.), px(0.))
        );
        assert_eq!(
            thumb_geometry(px(100.), px(100.), px(100.), px(100.)),
            (px(50.), px(50.))
        );
        // Very long content: the thumb keeps a minimum size.
        assert_eq!(
            thumb_geometry(px(100.), px(100.), px(10_000.), px(0.)).0,
            px(24.)
        );
    }
}
