//! Motion: keyframe animations, transitions and enter / exit presence.
//!
//! ```no_run
//! # use rok_ui::prelude::*;
//! keyframes! {
//!     pub FADE_UP = {
//!         from: { opacity: 0, y: 2 },
//!         to: { opacity: 1, y: 0 },
//!     }
//! }
//!
//! # fn example(window: &mut Window, cx: &mut App, open: bool) {
//! // Keyframes, CSS-animation style.
//! let card = div().motion("card-enter", Motion::new(&FADE_UP).duration_ms(200).easing(Easing::EaseOut));
//! let badge = div().motion("badge-pulse", motion::pulse().infinite());
//!
//! // A value that animates toward its target whenever the target changes.
//! let width = use_transition(
//!     "sidebar-width",
//!     window,
//!     cx,
//!     if open { 256. } else { 48. },
//!     Transition::ease_out(200),
//! );
//! let sidebar = div().w(px(width));
//!
//! // Keep an element mounted while it animates out.
//! let presence = use_presence("panel", window, cx, open, Transition::ease_out(150));
//! let panel = div().when(presence.is_mounted(), |div| {
//!     div.child(Card::new().opacity(presence.progress()))
//! });
//! # }
//! ```
//!
//! GPUI has no transforms for divs, so `x` / `y` move an element by offsetting
//! it from its laid-out position (it becomes `relative`). Animatable properties:
//! `opacity`, `x`, `y`, `width`, `height`, `background`, `color`, `border_color`
//! and `radius`.

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{Animation, AnimationElement, AnimationExt, ElementId, IntoElement, Styled};

mod easing;
mod keyframes;
pub mod presets;
mod transition;

pub use easing::{Easing, Interpolate};
pub use keyframes::{Frame, Keyframes};
pub use transition::{use_presence, use_transition, Presence, Transition};

// ---------------------------------------------------------------------------
// Reduced motion.

thread_local! {
    static REDUCED_MOTION: Cell<bool> = const { Cell::new(false) };
}

/// Skip animations app-wide (accessibility setting): motions jump to their end
/// state and transitions finish immediately.
pub fn set_reduced_motion(reduced: bool) {
    REDUCED_MOTION.with(|flag| flag.set(reduced));
}

/// Whether animations are turned off (see [`set_reduced_motion`]).
pub fn reduced_motion() -> bool {
    REDUCED_MOTION.with(Cell::get)
}

// ---------------------------------------------------------------------------
// Motion: keyframes plus timing.

/// How many times a motion plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Iterations {
    /// Play this many times.
    Count(u32),
    /// Loop forever.
    Infinite,
}

/// Which way each iteration runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MotionDirection {
    /// From the first frame to the last.
    #[default]
    Normal,
    /// From the last frame to the first.
    Reverse,
    /// Forward, then backward, and so on.
    Alternate,
}

/// Keyframes with timing, like a CSS `animation` declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    keyframes: Rc<Keyframes>,
    duration: Duration,
    delay: Duration,
    easing: Easing,
    iterations: Iterations,
    direction: MotionDirection,
}

impl Motion {
    /// 200ms, ease-out, played once.
    #[must_use]
    pub fn new(keyframes: &Keyframes) -> Self {
        Self {
            keyframes: Rc::new(keyframes.clone()),
            duration: Duration::from_millis(200),
            delay: Duration::ZERO,
            easing: Easing::EaseOut,
            iterations: Iterations::Count(1),
            direction: MotionDirection::Normal,
        }
    }

    /// How long one iteration takes.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// `duration` in milliseconds.
    #[must_use]
    pub fn duration_ms(self, milliseconds: u64) -> Self {
        self.duration(Duration::from_millis(milliseconds))
    }

    /// Wait before starting (before every iteration when infinite).
    #[must_use]
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Wait this many milliseconds before starting.
    #[must_use]
    pub fn delay_ms(self, milliseconds: u64) -> Self {
        self.delay(Duration::from_millis(milliseconds))
    }

    /// The timing function.
    #[must_use]
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Play `count` times.
    #[must_use]
    pub fn iterations(mut self, count: u32) -> Self {
        self.iterations = Iterations::Count(count.max(1));
        self
    }

    /// Loop forever.
    #[must_use]
    pub fn infinite(mut self) -> Self {
        self.iterations = Iterations::Infinite;
        self
    }

    /// Which way each iteration runs.
    #[must_use]
    pub fn direction(mut self, direction: MotionDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Shorthand for `.direction(MotionDirection::Alternate)`.
    #[must_use]
    pub fn alternate(self) -> Self {
        self.direction(MotionDirection::Alternate)
    }

    /// Shorthand for `.direction(MotionDirection::Reverse)`.
    #[must_use]
    pub fn reverse(self) -> Self {
        self.direction(MotionDirection::Reverse)
    }

    /// Keyframe progress at `elapsed` (0 to 1) of the whole GPUI animation,
    /// accounting for delay, iterations and direction.
    #[must_use]
    pub fn progress_at(&self, elapsed: f32) -> f32 {
        let iteration_length = self.delay.as_secs_f32() + self.duration.as_secs_f32();
        let iteration_length = iteration_length.max(f32::EPSILON);
        let iterations = match self.iterations {
            Iterations::Count(count) => count as f32,
            Iterations::Infinite => 1.,
        };
        // Position within the current iteration, in seconds.
        let total = iteration_length * iterations;
        let time = elapsed.clamp(0., 1.) * total;
        let iteration = ((time / iteration_length).floor() as u32).min(iterations as u32 - 1);
        let within = time - iteration as f32 * iteration_length - self.delay.as_secs_f32();
        let t = (within / self.duration.as_secs_f32().max(f32::EPSILON)).clamp(0., 1.);
        let t = match self.direction {
            MotionDirection::Normal => t,
            MotionDirection::Reverse => 1. - t,
            MotionDirection::Alternate if iteration % 2 == 1 => 1. - t,
            MotionDirection::Alternate => t,
        };
        self.easing.apply(t)
    }

    fn animation(&self) -> Animation {
        let iteration_length = self.delay + self.duration;
        let total = match self.iterations {
            Iterations::Count(count) => iteration_length * count,
            Iterations::Infinite => match self.direction {
                // A full there-and-back cycle repeats seamlessly.
                MotionDirection::Alternate => iteration_length * 2,
                _ => iteration_length,
            },
        };
        let animation = Animation::new(total.max(Duration::from_millis(1)));
        match self.iterations {
            Iterations::Infinite => animation.repeat(),
            Iterations::Count(_) => animation,
        }
    }
}

/// Play motions on any styled element.
pub trait MotionExt: IntoElement + Styled + 'static {
    /// Animate with `motion`. The animation restarts whenever `id` changes, so
    /// key it on whatever should replay it (an open count, a value).
    fn motion(self, id: impl Into<ElementId>, motion: Motion) -> AnimationElement<Self> {
        let reduced = reduced_motion();
        let mut motion = motion;
        if matches!(motion.iterations, Iterations::Infinite)
            && motion.direction == MotionDirection::Alternate
        {
            // One GPUI cycle covers two iterations.
            motion.iterations = Iterations::Count(2);
            let animation = motion.animation().repeat();
            return self.with_animation(id, animation, move |element, elapsed| {
                let progress = if reduced {
                    1.
                } else {
                    motion.progress_at(elapsed)
                };
                motion.keyframes.apply(element, progress)
            });
        }
        let animation = motion.animation();
        self.with_animation(id, animation, move |element, elapsed| {
            let progress = if reduced {
                1.
            } else {
                motion.progress_at(elapsed)
            };
            motion.keyframes.apply(element, progress)
        })
    }
}

impl<E: IntoElement + Styled + 'static> MotionExt for E {}

/// The edge a [`presets::slide_in`] motion starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MotionSide {
    /// From or to above.
    Top,
    /// From or to below.
    Bottom,
    /// From or to the left.
    Left,
    /// From or to the right.
    Right,
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Eased endpoints are exact.
mod tests {
    use super::*;

    #[test]
    fn motion_timing_handles_delay_iterations_and_alternation() {
        let keyframes = Keyframes::new();
        let motion = Motion::new(&keyframes)
            .duration_ms(100)
            .delay_ms(100)
            .easing(Easing::Linear);
        // First half of the total time is the delay.
        assert_eq!(motion.progress_at(0.25), 0.);
        assert!((motion.progress_at(0.75) - 0.5).abs() < 1e-4);

        let alternating = Motion::new(&keyframes)
            .duration_ms(100)
            .easing(Easing::Linear)
            .iterations(2)
            .alternate();
        assert!((alternating.progress_at(0.25) - 0.5).abs() < 1e-4);
        // The second iteration runs backward.
        assert!((alternating.progress_at(0.75) - 0.5).abs() < 1e-4);
        assert!(alternating.progress_at(0.95) < 0.2);
    }
}
