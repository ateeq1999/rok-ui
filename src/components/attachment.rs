//! Attachment: a file or image chip with metadata, upload state and actions.

use std::rc::Rc;

use gpui::{
    div, img, prelude::*, px, relative, App, ElementId, ImageSource, SharedString, StyleRefinement,
    Window,
};

use super::{button::Button, spinner::Spinner};
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Where an attachment is in its upload.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum AttachmentState {
    /// Uploaded (or never needed uploading).
    #[default]
    Ready,
    /// Uploading; carries progress from 0 to 100, or `None` when unknown.
    Uploading(Option<f32>),
    /// Failed; carries the message to show.
    Failed(SharedString),
}

/// ```ignore
/// Attachment::new("report", "Q3 report.pdf")
///     .meta("PDF · 2.4 MB")
///     .state(AttachmentState::Uploading(Some(40.)))
///     .on_remove(|_, _, cx| remove_attachment(cx))
///
/// Attachment::new("photo", "beach.jpg").image("assets/beach.jpg")
/// ```
#[derive(IntoElement)]
pub struct Attachment {
    id: ElementId,
    name: SharedString,
    meta: Option<SharedString>,
    icon: IconName,
    image: Option<ImageSource>,
    state: AttachmentState,
    on_open: Option<EventHandler<()>>,
    on_remove: Option<EventHandler<()>>,
    on_retry: Option<EventHandler<()>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Attachment);

impl Attachment {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        let icon = icon_for_file_name(&name);
        Self {
            id: id.into(),
            name,
            meta: None,
            icon,
            image: None,
            state: AttachmentState::Ready,
            on_open: None,
            on_remove: None,
            on_retry: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Secondary line: size, type, page count…
    #[must_use]
    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// Override the icon picked from the file extension.
    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// Show a thumbnail instead of an icon.
    #[must_use]
    pub fn image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Upload state: shows progress, a spinner or an error.
    #[must_use]
    pub fn state(mut self, state: AttachmentState) -> Self {
        self.state = state;
        self
    }

    /// Clicking the attachment opens it.
    #[must_use]
    pub fn on_open(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Show a remove button.
    #[must_use]
    pub fn on_remove(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    /// Show a retry button when the upload failed.
    #[must_use]
    pub fn on_retry(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

/// A file-type icon from the extension of `name`.
#[must_use]
pub fn icon_for_file_name(name: &str) -> IconName {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "avif" => IconName::Image,
        "pdf" | "txt" | "md" | "doc" | "docx" | "rtf" | "csv" => IconName::FileText,
        _ => IconName::File,
    }
}

styles! {
    ATTACHMENT = {
        root: {
            display: flex,
            align: center,
            gap: 3,
            width: 70,
            padding: 2,
            radius: lg,
            border: 1,
            border_color: border,
            background: card,
        },
        failed: { border_color: destructive/50 },
        openable: { cursor: pointer, hover: { background: accent/50 } },
        media: {
            position: relative,
            flex: none,
            size: 10,
            display: flex,
            align: center,
            justify: center,
            radius: md,
            overflow: hidden,
            background: muted,
        },
        image: { size: full },
        uploading_overlay: {
            position: absolute,
            top: 0,
            left: 0,
            size: full,
            display: flex,
            align: center,
            justify: center,
            background: background/60,
        },
        text: { display: flex, direction: column, flex: 1, min_width: 0, gap: 0.5 },
        name: { text: sm, font: medium, truncate: true },
        meta: { text: xs, color: muted_foreground, truncate: true },
        uploading: { text: xs, color: muted_foreground },
        error: { text: xs, color: destructive_text, truncate: true },
        progress_track: {
            margin_top: 1,
            height: 1,
            width: full,
            radius: full,
            background: primary/20,
        },
        progress_bar: { height: full, radius: full, background: primary },
        action: { width: 7, height: 7 },
    }
}

impl RenderOnce for Attachment {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let failed = matches!(self.state, AttachmentState::Failed(_));
        let icon_color = if failed {
            colors.destructive_text
        } else {
            colors.muted_foreground
        };

        let media = div()
            .sx(&ATTACHMENT.media)
            .map(|media| match self.image.clone() {
                Some(image) => media.child(
                    img(image)
                        .sx(&ATTACHMENT.image)
                        .object_fit(gpui::ObjectFit::Cover),
                ),
                None => media.child(Icon::new(self.icon).size(px(20.)).color(icon_color)),
            })
            .when(
                matches!(self.state, AttachmentState::Uploading(_)),
                |media| {
                    media.child(
                        div()
                            .sx(&ATTACHMENT.uploading_overlay)
                            .child(Spinner::new()),
                    )
                },
            );

        let detail = match &self.state {
            AttachmentState::Ready => self.meta.clone().map(|meta| {
                div()
                    .sx(&ATTACHMENT.meta)
                    .child(crate::components::bidi_text::text(meta))
                    .into_any_element()
            }),
            AttachmentState::Uploading(progress) => Some(match progress {
                Some(progress) => div()
                    .sx(&ATTACHMENT.progress_track)
                    .child(
                        div()
                            .sx(&ATTACHMENT.progress_bar)
                            .w(relative((progress / 100.).clamp(0., 1.))),
                    )
                    .into_any_element(),
                None => div()
                    .sx(&ATTACHMENT.uploading)
                    .child("Uploading…")
                    .into_any_element(),
            }),
            AttachmentState::Failed(message) => Some(
                div()
                    .sx(&ATTACHMENT.error)
                    .child(crate::components::bidi_text::text(message.clone()))
                    .into_any_element(),
            ),
        };

        let retry = (failed).then_some(self.on_retry).flatten().map(|handler| {
            Button::new("attachment-retry")
                .ghost()
                .icon_only(IconName::Refresh)
                .sx(&ATTACHMENT.action)
                .tooltip("Retry")
                .on_click(move |_, window, cx| handler(&(), window, cx))
        });
        let remove = self.on_remove.map(|handler| {
            Button::new("attachment-remove")
                .ghost()
                .icon_only(IconName::Close)
                .sx(&ATTACHMENT.action)
                .tooltip("Remove")
                .on_click(move |_, window, cx| handler(&(), window, cx))
        });

        div()
            .id(self.id)
            // The caller's `sx` is merged into the same call: GPUI allows a single
            // hover / focus style per element.
            .sx((
                &ATTACHMENT.root,
                failed.then_some(&ATTACHMENT.failed),
                self.on_open.is_some().then_some(&ATTACHMENT.openable),
                &self.sx,
            ))
            .when_some(self.on_open, |attachment, handler| {
                attachment.on_click(move |_, window, cx| handler(&(), window, cx))
            })
            .child(media)
            .child(
                div()
                    .sx(&ATTACHMENT.text)
                    .child(
                        div()
                            .sx(&ATTACHMENT.name)
                            .child(crate::components::bidi_text::text(self.name)),
                    )
                    .children(detail),
            )
            .children(retry)
            .children(remove)
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_follow_the_extension() {
        assert_eq!(icon_for_file_name("photo.JPG"), IconName::Image);
        assert_eq!(icon_for_file_name("report.pdf"), IconName::FileText);
        assert_eq!(icon_for_file_name("archive.zip"), IconName::File);
        assert_eq!(icon_for_file_name("README"), IconName::File);
    }
}
