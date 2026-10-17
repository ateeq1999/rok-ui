//! Transitions: animating a value when it changes ([`use_transition`]) and an element in
//! and out ([`use_presence`]).

use std::time::{Duration, Instant};

use gpui::{App, ElementId, Styled, Window};

use super::{reduced_motion, Easing, Interpolate, Keyframes};

/// Timing for [`use_transition`] and [`use_presence`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    /// How long the transition takes.
    pub duration: Duration,
    /// The timing function.
    pub easing: Easing,
}

impl Transition {
    /// A transition over `duration` with `easing`.
    #[must_use]
    pub fn new(duration: Duration, easing: Easing) -> Self {
        Self { duration, easing }
    }

    /// An ease-out transition over `milliseconds`.
    #[must_use]
    pub fn ease_out(milliseconds: u64) -> Self {
        Self::new(Duration::from_millis(milliseconds), Easing::EaseOut)
    }

    /// An ease-in-out transition over `milliseconds`.
    #[must_use]
    pub fn ease_in_out(milliseconds: u64) -> Self {
        Self::new(Duration::from_millis(milliseconds), Easing::EaseInOut)
    }

    /// A linear transition over `milliseconds`.
    #[must_use]
    pub fn linear(milliseconds: u64) -> Self {
        Self::new(Duration::from_millis(milliseconds), Easing::Linear)
    }

    /// A gentle spring over 500ms.
    #[must_use]
    pub fn spring() -> Self {
        Self::new(Duration::from_millis(500), Easing::Spring { damping: 0.6 })
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::ease_out(200)
    }
}

struct Tween<T> {
    from: T,
    to: T,
    started: Instant,
}

/// `useTransition`-style motion value: returns `target` the first time, then
/// animates toward each new `target` over `transition`, re-rendering every
/// frame until it arrives. Interrupted transitions start from the current value.
pub fn use_transition<T: Interpolate + PartialEq + 'static>(
    key: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    target: T,
    transition: Transition,
) -> T {
    let state = window.use_keyed_state(key, cx, |_, _| Tween {
        from: target,
        to: target,
        started: Instant::now(),
    });
    let now = Instant::now();
    let value_at = |tween: &Tween<T>, now: Instant| -> (T, bool) {
        if reduced_motion() || transition.duration.is_zero() {
            return (tween.to, true);
        }
        let elapsed = now.duration_since(tween.started).as_secs_f32();
        let t = elapsed / transition.duration.as_secs_f32();
        if t >= 1. {
            (tween.to, true)
        } else {
            (
                tween.from.interpolate(tween.to, transition.easing.apply(t)),
                false,
            )
        }
    };
    if state.read(cx).to != target {
        let (current, _) = value_at(state.read(cx), now);
        state.update(cx, |tween, _| {
            tween.from = current;
            tween.to = target;
            tween.started = now;
        });
    }
    let (value, finished) = value_at(state.read(cx), now);
    if !finished {
        window.request_animation_frame();
    }
    value
}

/// Enter / exit state for an element that appears and disappears.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Presence {
    progress: f32,
    present: bool,
}

impl Presence {
    /// Render the element: true while present or still animating out.
    #[must_use]
    pub fn is_mounted(&self) -> bool {
        self.present || self.progress > 0.001
    }

    /// 0 (gone) to 1 (fully in); use it for opacity, offsets, scale-like effects.
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Whether the element is entering or present (false while exiting).
    #[must_use]
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// Style `element` between `keyframes`' first (gone) and last (in) frames.
    pub fn apply<E: Styled>(&self, element: E, keyframes: &Keyframes) -> E {
        keyframes.apply(element, self.progress)
    }
}

/// Keep something mounted while it animates out, like Framer Motion's
/// `AnimatePresence`. Starts fully present when first rendered as present.
pub fn use_presence(
    key: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    present: bool,
    transition: Transition,
) -> Presence {
    let target = if present { 1. } else { 0. };
    let progress = use_transition(key, window, cx, target, transition);
    Presence { progress, present }
}
