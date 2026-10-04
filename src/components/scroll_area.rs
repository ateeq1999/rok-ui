//! `ScrollArea`: a scrolling box with a thin themed scrollbar.

use std::rc::Rc;

use gpui::{
    div, point, prelude::*, px, AnyElement, App, ElementId, MouseButton, Pixels, ScrollHandle,
    StyleRefinement, Window,
};

use super::{interaction::track_drag, overlay::child_id};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, State},
    styles,
    styles::ApplyStyleOverrides,
};

styles! {
    SCROLL_AREA = {
        root: { position: relative, overflow: hidden },
        content: { display: flex },
        content_column: { direction: column },
        track: { position: absolute, padding: 0.25 },
        track_vertical: { top: 0.5, inset_end: 0.25, width: 2.5 },
        track_horizontal: { left: 0.5, bottom: 0.25, height: 2.5 },
        thumb_frame: { position: relative, size: full },
        thumb: {
            position: absolute,
            radius: full,
            background: border,
            cursor: default,
            hover: { background: muted_foreground/50 },
        },
    }
}

/// Which way a [`ScrollArea`] scrolls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    /// Scroll up and down.
    #[default]
    Vertical,
    /// Scroll left and right.
    Horizontal,
    /// Scroll in both directions.
    Both,
}

struct ScrollMemory {
    handle: ScrollHandle,
    /// Pointer position and scroll offset (along the dragged axis) when a thumb drag started.
    drag: Option<(bool, Pixels, Pixels)>,
    /// The history entry shown at the last render, with `restore_scroll`.
    #[cfg(feature = "router")]
    entry: Option<usize>,
}

/// Thumb length and offset along a track of `track_length`, for a viewport of
/// `viewport_length` scrolled `scrolled` into content longer by `max_scroll`.
#[doc(hidden)]
#[must_use]
pub fn thumb_geometry(
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
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let tag_list = div();
/// let tags = ScrollArea::new("tags").h(px(288.)).w(px(192.)).child(tag_list);
/// # }
/// ```
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    axis: ScrollAxis,
    handle: Option<ScrollHandle>,
    #[cfg(feature = "router")]
    restore: bool,
    children: Vec<AnyElement>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ScrollArea);

impl ScrollArea {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            axis: ScrollAxis::Vertical,
            handle: None,
            #[cfg(feature = "router")]
            restore: false,
            children: Vec::new(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Which directions scroll.
    #[must_use]
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Shorthand for `.axis(ScrollAxis::Horizontal)`.
    #[must_use]
    pub fn horizontal(self) -> Self {
        self.axis(ScrollAxis::Horizontal)
    }

    /// Remember the scroll position per history entry (feature `router`): going back or forward
    /// to a page returns this area to where it was, and a new page starts at the top. Areas
    /// are told apart by their id, so give each scroll area that restores a distinct id.
    #[cfg(feature = "router")]
    #[must_use]
    pub fn restore_scroll(mut self, restore: bool) -> Self {
        self.restore = restore;
        self
    }

    /// Scroll with `handle` (create it once and keep it, like any GPUI scroll handle), so
    /// code outside can read or set the position: scroll to an item, or let a form scroll to
    /// its first invalid field.
    #[must_use]
    pub fn track_scroll(mut self, handle: &ScrollHandle) -> Self {
        self.handle = Some(handle.clone());
        self
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
                #[cfg(feature = "router")]
                entry: None,
            });
        let handle = self
            .handle
            .clone()
            .unwrap_or_else(|| memory.read(cx).handle.clone());
        #[cfg(feature = "router")]
        if self.restore {
            let last = memory.read(cx).entry;
            let entry = crate::router::restore_scroll(
                &self.id.to_string().into(),
                &handle,
                last,
                window,
                cx,
            );
            memory.update(cx, |memory| memory.entry = Some(entry));
        }
        let drag = memory.read(cx).drag;
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
                .sx(&SCROLL_AREA.thumb)
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
                        memory.drag = Some((vertical, pointer, scrolled));
                    });
                });
            Some(
                div()
                    .map(|track| {
                        if vertical {
                            track
                                .sx((&SCROLL_AREA.track, &SCROLL_AREA.track_vertical))
                                .h(track_length)
                        } else {
                            track
                                .sx((&SCROLL_AREA.track, &SCROLL_AREA.track_horizontal))
                                .w(track_length)
                        }
                    })
                    .child(div().sx(&SCROLL_AREA.thumb_frame).child(thumb))
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
                    .sx((
                        &SCROLL_AREA.content,
                        (scrolls_vertically && !scrolls_horizontally)
                            .then_some(&SCROLL_AREA.content_column),
                    ))
                    .children(self.children),
            );

        div()
            .id(self.id)
            .sx((&SCROLL_AREA.root, &self.sx))
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
