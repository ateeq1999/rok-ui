//! Ready-made motions, mirroring Tailwind's `animate-*` and common enter effects.

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
