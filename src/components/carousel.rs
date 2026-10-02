//! Carousel: slides shown one view at a time, with arrows, dots and a slide animation.

use std::{rc::Rc, time::Duration};

use gpui::{
    div, ease_in_out, prelude::*, relative, Animation, AnimationExt, AnyElement, App, ElementId,
    StyleRefinement, Window,
};

use super::{button::Button, direction::ActiveDirection, overlay::child_id};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler, State},
    icon::IconName,
    styles,
    styles::ApplyStyleOverrides,
};

/// Which way slides move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CarouselOrientation {
    #[default]
    Horizontal,
    Vertical,
}

struct CarouselMemory {
    index: usize,
    /// Where the track was before the last move, for the slide animation.
    previous_index: usize,
    /// Bumped on every move so each move restarts the animation.
    generation: usize,
}

/// Uncontrolled by default (the current slide is kept per id); pass `index` and
/// `on_index_change` to control it. Left / Right move while it has focus.
///
/// ```ignore
/// Carousel::new("gallery")
///     .items_per_view(1)
///     .item(slide(1))
///     .item(slide(2))
///     .item(slide(3))
/// ```
#[derive(IntoElement)]
pub struct Carousel {
    id: ElementId,
    items: Vec<AnyElement>,
    items_per_view: usize,
    orientation: CarouselOrientation,
    wrap_around: bool,
    show_dots: bool,
    index: Option<usize>,
    on_index_change: Option<EventHandler<usize>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Carousel);

impl Carousel {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            items_per_view: 1,
            orientation: CarouselOrientation::Horizontal,
            wrap_around: false,
            show_dots: true,
            index: None,
            on_index_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn item(mut self, item: impl IntoElement) -> Self {
        self.items.push(item.into_any_element());
        self
    }

    /// How many slides are visible at once (shadcn's `basis-1/3` → 3).
    pub fn items_per_view(mut self, count: usize) -> Self {
        self.items_per_view = count.max(1);
        self
    }

    pub fn orientation(mut self, orientation: CarouselOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Going past the last slide returns to the first.
    pub fn wrap_around(mut self, wrap_around: bool) -> Self {
        self.wrap_around = wrap_around;
        self
    }

    /// Show the position dots under the slides. On by default.
    pub fn dots(mut self, show: bool) -> Self {
        self.show_dots = show;
        self
    }

    /// The first visible slide (controlled mode).
    pub fn index(mut self, index: usize) -> Self {
        self.index = Some(index);
        self
    }

    pub fn on_index_change(
        mut self,
        handler: impl Fn(&usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_index_change = Some(Rc::new(handler));
        self
    }
}

fn go_to(
    memory: &State<CarouselMemory>,
    target: usize,
    on_index_change: Option<&EventHandler<usize>>,
    window: &mut Window,
    cx: &mut App,
) {
    memory.update(cx, |memory| {
        if memory.index != target {
            memory.previous_index = memory.index;
            memory.index = target;
            memory.generation += 1;
        }
    });
    if let Some(handler) = on_index_change {
        handler(&target, window, cx);
    }
}

styles! {
    CAROUSEL = {
        root: {
            display: flex,
            direction: column,
            width: full,
            border: 1,
            border_color: transparent,
            focus: { border_color: ring },
        },
        frame: { position: relative, width: full, height: full },
        viewport: { size: full, overflow: hidden },
        track: { position: relative, display: flex, size: full },
        track_direction(CarouselOrientation): {
            Horizontal: {},
            Vertical: { direction: column },
        },
        slide: { flex: none, padding: 1 },
        slide_direction(CarouselOrientation): {
            Horizontal: { height: full },
            Vertical: { width: full },
        },
        arrow: { radius: full, width: 8, height: 8 },
        // Arrows straddle the edges, centered on the cross axis.
        arrow_slot: { position: absolute },
        previous_slot(CarouselOrientation): {
            Horizontal: { inset_start: -4, top: 50%, margin_top: -4 },
            Vertical: { top: -4, left: 50%, margin_left: -4 },
        },
        next_slot(CarouselOrientation): {
            Horizontal: { inset_end: -4, top: 50%, margin_top: -4 },
            Vertical: { bottom: -4, left: 50%, margin_left: -4 },
        },
        dots: { display: flex, justify: center, gap: 1.5, padding_top: 3 },
        dot: {
            height: 2,
            width: 2,
            radius: full,
            background: muted_foreground/30,
            cursor: pointer,
        },
        dot_active: { width: 5, background: primary },
    }
}

impl RenderOnce for Carousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let memory: State<CarouselMemory> =
            use_keyed_state(child_id(&self.id, "carousel"), window, cx, || {
                CarouselMemory {
                    index: 0,
                    previous_index: 0,
                    generation: 0,
                }
            });
        let last_index = self.items.len().saturating_sub(self.items_per_view);
        if let Some(index) = self.index {
            if memory.read(cx).index != index.min(last_index) {
                let index = index.min(last_index);
                memory.update(cx, |memory| {
                    memory.previous_index = memory.index;
                    memory.index = index;
                    memory.generation += 1;
                });
            }
        }
        let (index, previous_index, generation) = {
            let memory = memory.read(cx);
            (
                memory.index.min(last_index),
                memory.previous_index.min(last_index),
                memory.generation,
            )
        };
        let orientation = self.orientation;
        let per_view = self.items_per_view as f32;
        let is_horizontal = self.orientation == CarouselOrientation::Horizontal;
        let is_rtl = cx.direction().is_rtl() && is_horizontal;
        let can_go_back = index > 0 || self.wrap_around;
        let can_go_forward = index < last_index || self.wrap_around;

        let step = |forward: bool| -> usize {
            match (forward, index) {
                (true, index) if index >= last_index => 0,
                (true, index) => index + 1,
                (false, 0) => last_index,
                (false, index) => index - 1,
            }
        };
        let (previous_target, next_target) = (step(false), step(true));

        let items = self.items.into_iter().map(|item| {
            div()
                .sx((&CAROUSEL.slide, CAROUSEL.slide_direction(orientation)))
                .map(|slide| {
                    if is_horizontal {
                        slide.w(relative(1. / per_view))
                    } else {
                        slide.h(relative(1. / per_view))
                    }
                })
                .child(item)
        });
        let offset_for = move |index: usize| -(index as f32) / per_view;
        let (from, to) = (offset_for(previous_index), offset_for(index));
        let track = div()
            .sx((&CAROUSEL.track, CAROUSEL.track_direction(orientation)))
            .children(items)
            .with_animation(
                ElementId::NamedInteger("carousel-slide".into(), generation as u64),
                Animation::new(Duration::from_millis(300)).with_easing(ease_in_out),
                move |track, progress| {
                    let offset = relative(from + (to - from) * progress);
                    match (is_horizontal, is_rtl) {
                        (true, false) => track.left(offset),
                        (true, true) => track.right(offset),
                        (false, _) => track.top(offset),
                    }
                },
            );

        let arrow = |forward: bool| {
            let target = if forward {
                next_target
            } else {
                previous_target
            };
            let enabled = if forward { can_go_forward } else { can_go_back };
            let icon = match (is_horizontal, forward ^ is_rtl) {
                (true, true) => IconName::ArrowRight,
                (true, false) => IconName::ArrowLeft,
                (false, true) => IconName::ArrowDown,
                (false, false) => IconName::ArrowUp,
            };
            let memory = memory.clone();
            let on_index_change = self.on_index_change.clone();
            Button::new(if forward {
                "carousel-next"
            } else {
                "carousel-previous"
            })
            .outline()
            .icon_only(icon)
            .sx(&CAROUSEL.arrow)
            .tooltip(if forward {
                "Next slide"
            } else {
                "Previous slide"
            })
            .disabled(!enabled)
            .on_click(move |_, window, cx| {
                go_to(&memory, target, on_index_change.as_ref(), window, cx)
            })
        };
        let previous_button = div()
            .sx((&CAROUSEL.arrow_slot, CAROUSEL.previous_slot(orientation)))
            .child(arrow(false));
        let next_button = div()
            .sx((&CAROUSEL.arrow_slot, CAROUSEL.next_slot(orientation)))
            .child(arrow(true));

        let dots = (self.show_dots && last_index > 0).then(|| {
            div()
                .sx(&CAROUSEL.dots)
                .children((0..=last_index).map(|dot_index| {
                    let memory = memory.clone();
                    let on_index_change = self.on_index_change.clone();
                    div()
                        .id(("carousel-dot", dot_index))
                        .sx((
                            &CAROUSEL.dot,
                            (dot_index == index).then_some(&CAROUSEL.dot_active),
                        ))
                        .on_click(move |_, window, cx| {
                            go_to(&memory, dot_index, on_index_change.as_ref(), window, cx)
                        })
                }))
        });

        let key_memory = memory.clone();
        let key_on_change = self.on_index_change.clone();
        div()
            .id(self.id)
            .tab_index(0)
            // The caller's `sx` is merged into the same call: GPUI allows a single
            // hover / focus style per element.
            .sx((&CAROUSEL.root, &self.sx))
            .on_key_down(move |event, window, cx| {
                let forward_key = if is_horizontal { "right" } else { "down" };
                let back_key = if is_horizontal { "left" } else { "up" };
                let key = event.keystroke.key.as_str();
                let forward = if key == forward_key {
                    !is_rtl
                } else if key == back_key {
                    is_rtl
                } else {
                    return;
                };
                if (forward && !can_go_forward) || (!forward && !can_go_back) {
                    return;
                }
                cx.stop_propagation();
                let target = if forward {
                    next_target
                } else {
                    previous_target
                };
                go_to(&key_memory, target, key_on_change.as_ref(), window, cx);
            })
            .child(
                div()
                    .sx(&CAROUSEL.frame)
                    .child(div().sx(&CAROUSEL.viewport).child(track))
                    .child(previous_button)
                    .child(next_button),
            )
            .children(dots)
            .apply_style_overrides(&self.style_overrides)
    }
}
