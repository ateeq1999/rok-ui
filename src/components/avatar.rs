//! Avatar: a round user image with an initials fallback.

use gpui::{div, img, prelude::*, px, App, FontWeight, ImageSource, Pixels, SharedString, Window};

use crate::theme::ActiveTheme;

/// `Avatar::new("AT")` shows initials; add `.image(..)` to show a picture,
/// falling back to the initials if it fails to load.
#[derive(IntoElement)]
pub struct Avatar {
    fallback_initials: SharedString,
    image: Option<ImageSource>,
    size: Pixels,
}

impl Avatar {
    pub fn new(fallback_initials: impl Into<SharedString>) -> Self {
        Self {
            fallback_initials: fallback_initials.into(),
            image: None,
            size: px(32.),
        }
    }

    /// A file path, URL or embedded image.
    pub fn image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let size = self.size;
        let initials_text_size = size * 0.4;
        let initials_color = colors.muted_foreground;
        let initials_background = colors.muted;

        let initials = move |initials_text: SharedString| {
            div()
                .size_full()
                // GPUI clips children to a rectangle, so round the children too.
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(initials_background)
                .text_color(initials_color)
                .text_size(initials_text_size)
                .font_weight(FontWeight::MEDIUM)
                .child(initials_text)
        };

        let fallback_initials = self.fallback_initials.clone();
        div()
            .flex_none()
            .size(size)
            .rounded_full()
            .overflow_hidden()
            .child(match self.image {
                Some(image_source) => img(image_source)
                    .size_full()
                    .rounded_full()
                    .with_fallback(move || initials(fallback_initials.clone()).into_any_element())
                    .into_any_element(),
                None => initials(self.fallback_initials).into_any_element(),
            })
    }
}
