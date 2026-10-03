//! Avatar: a round user image with an initials fallback.

use gpui::{div, img, prelude::*, px, App, ImageSource, Pixels, SharedString, Window};

use crate::{styles, styles::ApplyStyleOverrides, sx::SxStyled};

/// `Avatar::new("AT")` shows initials; add `.image(..)` to show a picture,
/// falling back to the initials if it fails to load.
#[derive(IntoElement)]
pub struct Avatar {
    fallback_initials: SharedString,
    image: Option<ImageSource>,
    size: Pixels,
    sx: crate::sx::Sx,
    style_overrides: gpui::StyleRefinement,
}

crate::implement_style_overrides!(Avatar);

impl Avatar {
    /// An avatar showing `fallback_initials` until an image loads.
    pub fn new(fallback_initials: impl Into<SharedString>) -> Self {
        Self {
            fallback_initials: fallback_initials.into(),
            image: None,
            size: px(32.),
            sx: crate::sx::Sx::new(),
            style_overrides: gpui::StyleRefinement::default(),
        }
    }

    /// A file path, URL or embedded image.
    #[must_use]
    pub fn image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }

    /// The diameter. Default: 32px.
    #[must_use]
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }
}

styles! {
    AVATAR = {
        root: { flex: none, radius: full, overflow: hidden },
        image: { size: full, radius: full },
        // GPUI clips children to a rectangle, so the children are rounded too.
        initials: {
            size: full,
            radius: full,
            display: flex,
            align: center,
            justify: center,
            background: muted,
            color: muted_foreground,
            font: medium,
        },
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let size = self.size;
        let initials = move |initials_text: SharedString| {
            div()
                .sx(&AVATAR.initials)
                .text_size(size * 0.4)
                .child(initials_text)
        };

        let fallback_initials = self.fallback_initials.clone();
        div()
            .size(size)
            .sx((&AVATAR.root, &self.sx))
            .child(match self.image {
                Some(image_source) => img(image_source)
                    .sx(&AVATAR.image)
                    .with_fallback(move || initials(fallback_initials.clone()).into_any_element())
                    .into_any_element(),
                None => initials(self.fallback_initials).into_any_element(),
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
