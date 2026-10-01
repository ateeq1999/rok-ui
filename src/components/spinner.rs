//! Spinner: a rotating loader icon.

use std::time::Duration;

use gpui::{
    percentage, prelude::*, px, svg, Animation, AnimationExt, App, Hsla, Pixels, Transformation,
    Window,
};

use crate::{icon::IconName, theme::ActiveTheme};

/// An indeterminate loading indicator. Used by `Button::loading(true)`.
#[derive(IntoElement)]
pub struct Spinner {
    size: Pixels,
    color: Option<Hsla>,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: px(16.),
            color: None,
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().colors.muted_foreground);
        svg()
            .path(IconName::Loader.asset_path())
            .flex_none()
            .size(self.size)
            .text_color(color)
            .with_animation(
                "rok-ui-spinner-rotation",
                Animation::new(Duration::from_millis(900)).repeat(),
                |spinner, progress| {
                    spinner.with_transformation(Transformation::rotate(percentage(progress)))
                },
            )
    }
}
