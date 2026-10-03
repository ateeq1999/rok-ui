//! Spinner: a rotating loader icon.

use std::time::Duration;

use gpui::{
    percentage, prelude::*, px, svg, Animation, AnimationExt, App, Hsla, Pixels, Transformation,
    Window,
};

use crate::{
    icon::IconName,
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

styles! {
    SPINNER = { root: { flex: none, color: muted_foreground } }
}

/// An indeterminate loading indicator. Used by `Button::loading(true)`.
#[derive(IntoElement)]
pub struct Spinner {
    size: Pixels,
    color: Option<Hsla>,
    sx: Sx,
    style_overrides: gpui::StyleRefinement,
}

crate::implement_style_overrides!(Spinner);

impl Spinner {
    /// A spinning loading indicator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            size: px(16.),
            color: None,
            sx: Sx::new(),
            style_overrides: gpui::StyleRefinement::default(),
        }
    }

    /// The diameter. Default: 16px.
    #[must_use]
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// The stroke color. Default: the current text color.
    #[must_use]
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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = self.color.map(|color| Sx::new().text_color(color));
        svg()
            .path(IconName::Loader.asset_path())
            .size(self.size)
            .sx((&SPINNER.root, color, &self.sx))
            .apply_style_overrides(&self.style_overrides)
            .with_animation(
                "rok-ui-spinner-rotation",
                Animation::new(Duration::from_millis(900)).repeat(),
                |spinner, progress| {
                    spinner.with_transformation(Transformation::rotate(percentage(progress)))
                },
            )
    }
}
