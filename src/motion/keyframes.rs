//! Keyframes: the styles a motion passes through.

use gpui::{px, Hsla, Pixels, Styled};

use super::Interpolate;
use crate::sx::{current_theme, SxColor, SxLength, SxRadius};

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

#[cfg(test)]
#[allow(clippy::float_cmp)] // Eased endpoints are exact.
mod tests {
    use super::*;

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
}
