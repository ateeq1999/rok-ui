//! Slider: pick a number, or a range, by dragging along a track.

use std::{cell::Cell, rc::Rc};

use gpui::{
    div, prelude::*, px, relative, App, Bounds, CursorStyle, ElementId, MouseButton, Pixels, Point,
    StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::{
    focus_ring_shadow,
    interaction::{measure_bounds, track_drag},
    overlay::child_id,
};
use crate::{
    hooks::{use_keyed_state, EventHandler, State},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Controlled: pass the value(s), update them in `on_change`. One value makes a
/// single slider; two (`.range(..)`) make a range with two thumbs.
///
/// ```ignore
/// Slider::new("volume").value(volume).max(100.).step(1.)
///     .on_change(move |values, _, cx| set_volume(values[0], cx))
/// Slider::new("price").range(25., 75.).on_change(..)
/// ```
#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    values: Vec<f32>,
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
    on_change: Option<EventHandler<Vec<f32>>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Slider);

impl Slider {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            values: vec![0.],
            min: 0.,
            max: 100.,
            step: 1.,
            disabled: false,
            on_change: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A single value.
    pub fn value(mut self, value: f32) -> Self {
        self.values = vec![value];
        self
    }

    /// Two thumbs selecting `start..=end`.
    pub fn range(mut self, start: f32, end: f32) -> Self {
        self.values = vec![start.min(end), start.max(end)];
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    /// Values snap to multiples of `step` from `min`. Arrow keys move by one step.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Receives every value (one for a single slider, two for a range).
    pub fn on_change(
        mut self,
        handler: impl Fn(&Vec<f32>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// Snap `value` to the step grid and clamp it to `min..=max`.
pub(crate) fn snap(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let snapped = if step > 0. {
        min + ((value - min) / step).round() * step
    } else {
        value
    };
    snapped.clamp(min, max.max(min))
}

#[derive(Clone)]
struct SliderModel {
    values: Vec<f32>,
    /// Captured while rendering: handlers run outside the `Direction` scope.
    rtl: bool,
    min: f32,
    max: f32,
    step: f32,
    on_change: Option<EventHandler<Vec<f32>>>,
}

impl SliderModel {
    fn fraction(&self, value: f32) -> f32 {
        let span = self.max - self.min;
        if span <= 0. {
            0.
        } else {
            ((value - self.min) / span).clamp(0., 1.)
        }
    }

    fn value_at(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> f32 {
        let width = bounds.size.width.max(px(1.));
        let from_start = if self.rtl {
            bounds.right() - position.x
        } else {
            position.x - bounds.left()
        };
        let fraction = (from_start / width).clamp(0., 1.);
        snap(
            self.min + fraction * (self.max - self.min),
            self.min,
            self.max,
            self.step,
        )
    }

    fn nearest_thumb(&self, value: f32) -> usize {
        self.values
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (*a - value)
                    .abs()
                    .partial_cmp(&(*b - value).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    /// Move thumb `index` to `value`, keeping thumbs in order, and report it.
    fn set(&self, index: usize, value: f32, window: &mut Window, cx: &mut App) {
        let lower = if index > 0 {
            self.values[index - 1]
        } else {
            self.min
        };
        let upper = self.values.get(index + 1).copied().unwrap_or(self.max);
        let value = value.clamp(lower, upper);
        if (self.values[index] - value).abs() < f32::EPSILON {
            return;
        }
        let mut next = self.values.clone();
        next[index] = value;
        if let Some(handler) = self.on_change.as_ref() {
            handler(&next, window, cx);
        }
    }
}

struct SliderMemory {
    bounds: Rc<Cell<Bounds<Pixels>>>,
    dragging_thumb: Option<usize>,
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let memory: State<SliderMemory> =
            use_keyed_state(child_id(&self.id, "slider"), window, cx, || SliderMemory {
                bounds: Rc::new(Cell::new(Bounds::default())),
                dragging_thumb: None,
            });
        let bounds = memory.read(cx).bounds.clone();
        let dragging_thumb = memory.read(cx).dragging_thumb;
        let model = SliderModel {
            rtl: super::direction::is_rtl(),
            values: self
                .values
                .iter()
                .map(|value| value.clamp(self.min, self.max.max(self.min)))
                .collect(),
            min: self.min,
            max: self.max,
            step: self.step,
            on_change: self.on_change,
        };
        let colors = cx.theme().colors.clone();
        let ring_color = colors.ring;
        let is_interactive = !self.disabled;

        let (fill_start, fill_end) = match model.values.as_slice() {
            [single] => (0., model.fraction(*single)),
            [first, .., last] => (model.fraction(*first), model.fraction(*last)),
            [] => (0., 0.),
        };

        let thumbs = model.values.iter().enumerate().map(|(index, value)| {
            let key_model = model.clone();
            let value = *value;
            div()
                .id(("slider-thumb", index))
                .absolute()
                .inset_start(relative(model.fraction(value)))
                .ms(px(-8.))
                .size(px(16.))
                .rounded_full()
                .border_1()
                .border_color(colors.primary)
                .bg(colors.background)
                .shadow_sm()
                .when(is_interactive, |thumb| {
                    thumb
                        .tab_index(0)
                        .cursor(CursorStyle::PointingHand)
                        .focus(move |style| style.shadow(focus_ring_shadow(ring_color)))
                        .on_key_down(move |event, window, cx| {
                            let target = match event.keystroke.key.as_str() {
                                "up" => value + key_model.step,
                                "down" => value - key_model.step,
                                // The track runs right to left in RTL.
                                "right" if key_model.rtl => value - key_model.step,
                                "left" if key_model.rtl => value + key_model.step,
                                "right" => value + key_model.step,
                                "left" => value - key_model.step,
                                "pageup" => value + key_model.step * 10.,
                                "pagedown" => value - key_model.step * 10.,
                                "home" => key_model.min,
                                "end" => key_model.max,
                                _ => return,
                            };
                            cx.stop_propagation();
                            let target = snap(target, key_model.min, key_model.max, key_model.step);
                            key_model.set(index, target, window, cx);
                        })
                })
        });

        let press_model = model.clone();
        let press_bounds = bounds.clone();
        let press_memory = memory.clone();
        let move_model = model.clone();
        let move_bounds = bounds.clone();
        let end_memory = memory.clone();

        div()
            .id(self.id)
            .relative()
            .flex_dir()
            .items_center()
            .w_full()
            .h(px(16.))
            .when(!is_interactive, |slider| slider.opacity(0.5))
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(6.))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(colors.muted)
                    .child(measure_bounds(bounds))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .h_full()
                            .inset_start(relative(fill_start))
                            .w(relative(fill_end - fill_start))
                            .bg(colors.primary),
                    ),
            )
            .children(thumbs)
            .when(is_interactive, |slider| {
                slider
                    .cursor(CursorStyle::PointingHand)
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        let value = press_model.value_at(event.position, press_bounds.get());
                        let thumb = press_model.nearest_thumb(value);
                        press_model.set(thumb, value, window, cx);
                        press_memory.update(cx, |memory| memory.dragging_thumb = Some(thumb));
                    })
                    .child(track_drag(
                        dragging_thumb.is_some(),
                        Rc::new(move |position, window, cx| {
                            if let Some(thumb) = dragging_thumb {
                                let value = move_model.value_at(position, move_bounds.get());
                                move_model.set(thumb, value, window, cx);
                            }
                        }),
                        Rc::new(move |_, cx| {
                            end_memory.update(cx, |memory| memory.dragging_thumb = None)
                        }),
                    ))
            })
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::snap;

    #[test]
    fn snap_rounds_to_step_and_clamps() {
        assert_eq!(snap(42.4, 0., 100., 5.), 40.);
        assert_eq!(snap(42.6, 0., 100., 5.), 45.);
        assert_eq!(snap(-3., 0., 100., 1.), 0.);
        assert_eq!(snap(130., 0., 100., 1.), 100.);
        assert_eq!(snap(0.33, 0., 1., 0.), 0.33);
        assert_eq!(snap(7., 2., 12., 5.), 7.);
    }
}
