//! Timing functions and the values a motion can interpolate.

use std::f32::consts::PI;

use gpui::{px, Hsla, Pixels};

// ---------------------------------------------------------------------------
// Easing.

/// Timing functions, matching CSS where they share a name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// `cubic-bezier(0.42, 0, 1, 1)`.
    EaseIn,
    /// `cubic-bezier(0, 0, 0.58, 1)`.
    EaseOut,
    /// `cubic-bezier(0.42, 0, 0.58, 1)`.
    EaseInOut,
    /// Any CSS `cubic-bezier(x1, y1, x2, y2)`.
    CubicBezier(f32, f32, f32, f32),
    /// A damped spring settling over the animation's duration. Lower damping
    /// overshoots more (0.3 is bouncy, 1.0 settles without overshoot).
    Spring {
        /// 0.3 is bouncy; 1.0 settles without overshoot.
        damping: f32,
    },
    /// Jumps to the end at the given number of steps.
    Steps(u32),
}

impl Easing {
    /// Progress (0 to 1, possibly overshooting for springs) at time `t` (0 to 1).
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0., 1.);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => cubic_bezier(0.42, 0., 1., 1., t),
            Easing::EaseOut => cubic_bezier(0., 0., 0.58, 1., t),
            Easing::EaseInOut => cubic_bezier(0.42, 0., 0.58, 1., t),
            Easing::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2, t),
            Easing::Spring { damping } => spring(damping, t),
            Easing::Steps(steps) => {
                let steps = steps.max(1) as f32;
                ((t * steps).floor() / steps).min(1.)
            }
        }
    }
}

/// CSS `cubic-bezier`: solve x(s) = t for s by Newton's method, return y(s).
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let curve = |a: f32, b: f32, s: f32| {
        3. * a * s * (1. - s).powi(2) + 3. * b * s.powi(2) * (1. - s) + s.powi(3)
    };
    let slope = |a: f32, b: f32, s: f32| {
        3. * a * (1. - s).powi(2) + 6. * (b - a) * s * (1. - s) + 3. * (1. - b) * s.powi(2)
    };
    let mut s = t;
    for _ in 0..8 {
        let error = curve(x1, x2, s) - t;
        let derivative = slope(x1, x2, s);
        if error.abs() < 1e-5 || derivative.abs() < 1e-6 {
            break;
        }
        s = (s - error / derivative).clamp(0., 1.);
    }
    curve(y1, y2, s)
}

/// An underdamped spring response, scaled so it settles at t = 1.
pub(super) fn spring(damping: f32, t: f32) -> f32 {
    if t >= 1. {
        return 1.;
    }
    let damping = damping.clamp(0.05, 1.);
    // Enough oscillation to read as a spring, fewer as damping grows.
    let frequency = 2. * PI * (1.5 - damping);
    let decay = (-6. * t).exp();
    if damping >= 1. {
        return 1. - decay * (1. + 6. * t);
    }
    1. - decay * (frequency * t).cos() * (1. - damping).max(0.) - decay * damping
}

// ---------------------------------------------------------------------------
// Interpolation.

/// Values that can be tweened.
pub trait Interpolate: Copy {
    /// The value `progress` (0 to 1) of the way from `self` to `to`.
    #[must_use]
    fn interpolate(self, to: Self, progress: f32) -> Self;
}

impl Interpolate for f32 {
    fn interpolate(self, to: Self, progress: f32) -> Self {
        self + (to - self) * progress
    }
}

impl Interpolate for Pixels {
    fn interpolate(self, to: Self, progress: f32) -> Self {
        px(f32::from(self).interpolate(f32::from(to), progress))
    }
}

impl Interpolate for Hsla {
    fn interpolate(self, to: Self, progress: f32) -> Self {
        // Take the short way round the hue circle; grey ends keep the other hue.
        let (from_hue, to_hue) = match (self.s < 0.01, to.s < 0.01) {
            (true, false) => (to.h, to.h),
            (false, true) => (self.h, self.h),
            _ => (self.h, to.h),
        };
        let mut hue_delta = to_hue - from_hue;
        if hue_delta > 0.5 {
            hue_delta -= 1.;
        } else if hue_delta < -0.5 {
            hue_delta += 1.;
        }
        Hsla {
            h: (from_hue + hue_delta * progress).rem_euclid(1.),
            s: self.s.interpolate(to.s, progress),
            l: self.l.interpolate(to.l, progress),
            a: self.a.interpolate(to.a, progress),
        }
    }
}

impl<A: Interpolate, B: Interpolate> Interpolate for (A, B) {
    fn interpolate(self, to: Self, progress: f32) -> Self {
        (
            self.0.interpolate(to.0, progress),
            self.1.interpolate(to.1, progress),
        )
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Eased endpoints are exact.
mod tests {
    use super::*;

    #[test]
    fn easings_start_at_zero_and_end_at_one() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::CubicBezier(0.4, 0., 0.2, 1.),
            Easing::Spring { damping: 0.5 },
            Easing::Spring { damping: 1. },
            Easing::Steps(4),
        ] {
            assert!(easing.apply(0.).abs() < 0.05, "{easing:?} at 0");
            assert!((easing.apply(1.) - 1.).abs() < 1e-3, "{easing:?} at 1");
        }
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
        assert_eq!(Easing::Steps(4).apply(0.3), 0.25);
    }

    #[test]
    fn springs_overshoot_when_underdamped() {
        let peak = (1..100)
            .map(|step| Easing::Spring { damping: 0.3 }.apply(step as f32 / 100.))
            .fold(0., f32::max);
        assert!(peak > 1., "peak {peak}");
    }

    #[test]
    fn colors_interpolate_the_short_way_round() {
        let red = Hsla {
            h: 0.95,
            s: 1.,
            l: 0.5,
            a: 1.,
        };
        let orange = Hsla {
            h: 0.05,
            s: 1.,
            l: 0.5,
            a: 1.,
        };
        let middle = red.interpolate(orange, 0.5);
        assert!(middle.h < 0.01 || middle.h > 0.99, "hue {}", middle.h);
    }
}
