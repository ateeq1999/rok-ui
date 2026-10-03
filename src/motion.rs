//! Motion: keyframe animations, transitions and enter / exit presence.
//!
//! ```ignore
//! keyframes! {
//!     pub FADE_UP = {
//!         from: { opacity: 0, y: 2 },
//!         to: { opacity: 1, y: 0 },
//!     }
//! }
//!
//! // Keyframes, CSS-animation style.
//! div().motion("card-enter", Motion::new(&FADE_UP).duration_ms(200).easing(Easing::EaseOut))
//! div().motion("badge-pulse", motion::pulse().infinite())
//!
//! // A value that animates toward its target whenever the target changes.
//! let width = use_transition("sidebar-width", window, cx, if open { 256. } else { 48. },
//!                            Transition::spring());
//! div().w(px(width))
//!
//! // Keep an element mounted while it animates out.
//! let presence = use_presence("panel", window, cx, open, Transition::ease_out(150));
//! div().when(presence.is_mounted(), |div| div.child(panel.opacity(presence.progress())))
//! ```
//!
//! GPUI has no transforms for divs, so `x` / `y` move an element by offsetting
//! it from its laid-out position (it becomes `relative`). Animatable properties:
//! `opacity`, `x`, `y`, `width`, `height`, `background`, `color`, `border_color`
//! and `radius`.

use std::{
    cell::Cell,
    f32::consts::PI,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    px, Animation, AnimationElement, AnimationExt, App, ElementId, Hsla, IntoElement, Pixels,
    Styled, Window,
};

use crate::sx::{current_theme, SxColor, SxLength, SxRadius};

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
fn spring(damping: f32, t: f32) -> f32 {
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

// ---------------------------------------------------------------------------
// Keyframes.

/// The animatable properties at one keyframe. `None` means "not set here".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// Opacity from 0 to 1.
    pub opacity: Option<f32>,
    /// Horizontal offset.
    pub x: Option<SxLength>,
    /// Vertical offset.
    pub y: Option<SxLength>,
    /// Width.
    pub width: Option<SxLength>,
    /// Height.
    pub height: Option<SxLength>,
    /// Background color.
    pub background: Option<SxColor>,
    /// Text color.
    pub color: Option<SxColor>,
    /// Border color.
    pub border_color: Option<SxColor>,
    /// Corner radius.
    pub radius: Option<SxRadius>,
}

impl Frame {
    /// An empty frame: it animates nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Animate opacity (0 to 1).
    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Horizontal offset from the laid-out position (spacing units by default).
    #[must_use]
    pub fn x(mut self, x: impl Into<SxLength>) -> Self {
        self.x = Some(x.into());
        self
    }

    /// Vertical offset from the laid-out position.
    #[must_use]
    pub fn y(mut self, y: impl Into<SxLength>) -> Self {
        self.y = Some(y.into());
        self
    }

    /// Animate the width.
    #[must_use]
    pub fn width(mut self, width: impl Into<SxLength>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Animate the height.
    #[must_use]
    pub fn height(mut self, height: impl Into<SxLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Animate the background color.
    #[must_use]
    pub fn background(mut self, color: impl Into<SxColor>) -> Self {
        self.background = Some(color.into());
        self
    }

    /// Animate the text color.
    #[must_use]
    pub fn color(mut self, color: impl Into<SxColor>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Animate the border color.
    #[must_use]
    pub fn border_color(mut self, color: impl Into<SxColor>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Animate the corner radius.
    #[must_use]
    pub fn radius(mut self, radius: impl Into<SxRadius>) -> Self {
        self.radius = Some(radius.into());
        self
    }
}

/// Lengths that interpolate: spacing units and pixels become pixels.
fn length_pixels(length: SxLength) -> Option<Pixels> {
    match length {
        SxLength::Units(units) => Some(px(units * 4.)),
        SxLength::Px(pixels) => Some(pixels),
        _ => None,
    }
}

/// A keyframe sequence: frames at offsets from 0 to 1 (`0%` to `100%`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyframes {
    frames: Vec<(f32, Frame)>,
}

impl Keyframes {
    /// No frames yet; add them with `.at(..)`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a frame at `offset` (0 to 1).
    #[must_use]
    pub fn at(mut self, offset: f32, frame: Frame) -> Self {
        self.frames.push((offset.clamp(0., 1.), frame));
        self.frames
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        self
    }

    /// `at(0., frame)`.
    #[must_use]
    pub fn from(self, frame: Frame) -> Self {
        self.at(0., frame)
    }

    /// `at(1., frame)`.
    #[must_use]
    pub fn to(self, frame: Frame) -> Self {
        self.at(1., frame)
    }

    /// The value of one property at `progress`, between the nearest frames
    /// that set it (holding the first / last value outside them).
    fn sample<T: Copy, V: Interpolate>(
        &self,
        progress: f32,
        property: impl Fn(&Frame) -> Option<T>,
        convert: impl Fn(T) -> Option<V>,
    ) -> Option<V> {
        let defined: Vec<(f32, V)> = self
            .frames
            .iter()
            .filter_map(|(offset, frame)| {
                property(frame)
                    .and_then(&convert)
                    .map(|value| (*offset, value))
            })
            .collect();
        let (first, last) = (defined.first()?, defined.last()?);
        if progress <= first.0 {
            return Some(first.1);
        }
        if progress >= last.0 {
            return Some(last.1);
        }
        defined.windows(2).find_map(|pair| {
            let ((start, from), (end, to)) = (pair[0], pair[1]);
            (progress >= start && progress <= end).then(|| {
                let span = (end - start).max(f32::EPSILON);
                from.interpolate(to, (progress - start) / span)
            })
        })
    }

    /// Style `element` as it looks at `progress` (0 to 1, springs may overshoot).
    pub fn apply<E: Styled>(&self, element: E, progress: f32) -> E {
        let theme = current_theme();
        let colors = &theme.colors;
        let color = |value: SxColor| Some(sx_color(value, colors));
        let mut element = element;
        if let Some(opacity) = self.sample(progress, |frame| frame.opacity, Some) {
            element = element.opacity(opacity.clamp(0., 1.));
        }
        let x = self.sample(progress, |frame| frame.x, length_pixels);
        let y = self.sample(progress, |frame| frame.y, length_pixels);
        if x.is_some() || y.is_some() {
            element = element.relative();
            if let Some(x) = x {
                element = element.left(x);
            }
            if let Some(y) = y {
                element = element.top(y);
            }
        }
        if let Some(width) = self.sample(progress, |frame| frame.width, length_pixels) {
            element = element.w(width);
        }
        if let Some(height) = self.sample(progress, |frame| frame.height, length_pixels) {
            element = element.h(height);
        }
        if let Some(background) = self.sample(progress, |frame| frame.background, color) {
            element = element.bg(background);
        }
        if let Some(text) = self.sample(progress, |frame| frame.color, color) {
            element = element.text_color(text);
        }
        if let Some(border) = self.sample(progress, |frame| frame.border_color, color) {
            element = element.border_color(border);
        }
        let radius_pixels = |radius: SxRadius| match radius.resolve(&theme) {
            gpui::AbsoluteLength::Pixels(pixels) => Some(pixels),
            gpui::AbsoluteLength::Rems(rems) => Some(px(rems.0 * 16.)),
        };
        if let Some(radius) = self.sample(progress, |frame| frame.radius, radius_pixels) {
            element = element.rounded(radius);
        }
        element
    }
}

fn sx_color(color: SxColor, colors: &crate::theme::ThemeColors) -> Hsla {
    match color {
        SxColor::Token(token, alpha) => token.resolve(colors).opacity(alpha),
        SxColor::Value(value) => value,
    }
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

// ---------------------------------------------------------------------------
// Presets.

/// Ready-made motions, mirroring Tailwind's `animate-*` and common enter effects.
pub mod presets {
    use std::sync::LazyLock;

    use super::MotionSide;
    use super::{Easing, Frame, Keyframes, Motion};
    use crate::sx::ColorToken;

    static FADE_IN: LazyLock<Keyframes> = LazyLock::new(|| {
        Keyframes::new()
            .from(Frame::new().opacity(0.))
            .to(Frame::new().opacity(1.))
    });

    /// Opacity 0 → 1.
    #[must_use]
    pub fn fade_in() -> Motion {
        Motion::new(&FADE_IN)
    }

    /// Opacity 1 → 0.
    #[must_use]
    pub fn fade_out() -> Motion {
        Motion::new(&FADE_IN).reverse()
    }

    /// Fade in while moving `distance` spacing units from `side`.
    #[must_use]
    pub fn slide_in(side: MotionSide, distance: f32) -> Motion {
        let start = match side {
            MotionSide::Top => Frame::new().y(-distance),
            MotionSide::Bottom => Frame::new().y(distance),
            MotionSide::Left => Frame::new().x(-distance),
            MotionSide::Right => Frame::new().x(distance),
        };
        let end = match side {
            MotionSide::Top | MotionSide::Bottom => Frame::new().y(0.),
            MotionSide::Left | MotionSide::Right => Frame::new().x(0.),
        };
        Motion::new(&Keyframes::new().from(start.opacity(0.)).to(end.opacity(1.)))
    }

    /// Tailwind's `animate-pulse`: opacity dips to 0.5 and back, forever.
    #[must_use]
    pub fn pulse() -> Motion {
        Motion::new(
            &Keyframes::new()
                .at(0., Frame::new().opacity(1.))
                .at(0.5, Frame::new().opacity(0.5))
                .at(1., Frame::new().opacity(1.)),
        )
        .duration_ms(2000)
        .easing(Easing::CubicBezier(0.4, 0., 0.6, 1.))
        .infinite()
    }

    /// Tailwind's `animate-bounce`: hop up a quarter of 1rem and land, forever.
    #[must_use]
    pub fn bounce() -> Motion {
        Motion::new(
            &Keyframes::new()
                .at(0., Frame::new().y(0.))
                .at(0.5, Frame::new().y(-2.))
                .at(1., Frame::new().y(0.)),
        )
        .duration_ms(1000)
        .easing(Easing::EaseInOut)
        .infinite()
    }

    /// A short horizontal shake, for invalid input.
    #[must_use]
    pub fn shake() -> Motion {
        Motion::new(
            &Keyframes::new()
                .at(0., Frame::new().x(0.))
                .at(0.2, Frame::new().x(-2.))
                .at(0.4, Frame::new().x(2.))
                .at(0.6, Frame::new().x(-1.5))
                .at(0.8, Frame::new().x(1.5))
                .at(1., Frame::new().x(0.)),
        )
        .duration_ms(400)
        .easing(Easing::Linear)
    }

    /// Briefly flash the background with the accent color, then fade back.
    #[must_use]
    pub fn highlight() -> Motion {
        Motion::new(
            &Keyframes::new()
                .from(Frame::new().background(ColorToken::Accent))
                .to(Frame::new().background(ColorToken::Accent.alpha(0.))),
        )
        .duration_ms(800)
    }
}

// ---------------------------------------------------------------------------
// Transitions.

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
    fn keyframes_interpolate_between_defined_frames() {
        let keyframes = Keyframes::new()
            .at(0., Frame::new().opacity(0.).x(0.))
            .at(0.5, Frame::new().opacity(1.))
            .at(1., Frame::new().x(10.));
        let opacity = |progress| keyframes.sample(progress, |frame| frame.opacity, Some);
        assert_eq!(opacity(0.25), Some(0.5));
        // Held after the last frame that sets it.
        assert_eq!(opacity(0.9), Some(1.));
        let x = keyframes.sample(0.5, |frame| frame.x, length_pixels);
        assert_eq!(x, Some(px(20.)));
    }

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
