//! Attachment: a file or image chip with metadata, upload state and actions.

use std::rc::Rc;

use gpui::{
    div, img, prelude::*, px, relative, App, CursorStyle, ElementId, FontWeight, ImageSource,
    SharedString, StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::{button::Button, spinner::Spinner};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
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
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Attachment);

impl Attachment {
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
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Secondary line: size, type, page count…
    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// Override the icon picked from the file extension.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// Show a thumbnail instead of an icon.
    pub fn image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }

    pub fn state(mut self, state: AttachmentState) -> Self {
        self.state = state;
        self
    }

    /// Clicking the attachment opens it.
    pub fn on_open(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Show a remove button.
    pub fn on_remove(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    /// Show a retry button when the upload failed.
    pub fn on_retry(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

/// A file-type icon from the extension of `name`.
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

impl RenderOnce for Attachment {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let failed = matches!(self.state, AttachmentState::Failed(_));

        let media = div()
            .relative()
            .flex_none()
            .size(px(40.))
            .flex_dir()
            .items_center()
            .justify_center()
            .rounded(theme.radius_medium())
            .overflow_hidden()
            .bg(colors.muted)
            .map(|media| match self.image.clone() {
                Some(image) => {
                    media.child(img(image).size_full().object_fit(gpui::ObjectFit::Cover))
                }
                None => media.child(Icon::new(self.icon).size(px(20.)).color(if failed {
                    colors.destructive_text
                } else {
                    colors.muted_foreground
                })),
            })
            .when(
                matches!(self.state, AttachmentState::Uploading(_)),
                |media| {
                    media.child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .flex_dir()
                            .items_center()
                            .justify_center()
                            .bg(colors.background.opacity(0.6))
                            .child(Spinner::new()),
                    )
                },
            );

        let detail = match &self.state {
            AttachmentState::Ready => self.meta.clone().map(|meta| {
                div()
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .truncate()
                    .child(meta)
                    .into_any_element()
            }),
            AttachmentState::Uploading(progress) => Some(match progress {
                Some(progress) => div()
                    .mt(px(4.))
                    .h(px(4.))
                    .w_full()
                    .rounded_full()
                    .bg(colors.primary.opacity(0.2))
                    .child(
                        div()
                            .h_full()
                            .w(relative((progress / 100.).clamp(0., 1.)))
                            .rounded_full()
                            .bg(colors.primary),
                    )
                    .into_any_element(),
                None => div()
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child("Uploading…")
                    .into_any_element(),
            }),
            AttachmentState::Failed(message) => Some(
                div()
                    .text_xs()
                    .text_color(colors.destructive_text)
                    .truncate()
                    .child(message.clone())
                    .into_any_element(),
            ),
        };

        let retry = (failed).then_some(self.on_retry).flatten().map(|handler| {
            Button::new("attachment-retry")
                .ghost()
                .icon_only(IconName::Refresh)
                .w(px(28.))
                .h(px(28.))
                .tooltip("Retry")
                .on_click(move |_, window, cx| handler(&(), window, cx))
        });
        let remove = self.on_remove.map(|handler| {
            Button::new("attachment-remove")
                .ghost()
                .icon_only(IconName::Close)
                .w(px(28.))
                .h(px(28.))
                .tooltip("Remove")
                .on_click(move |_, window, cx| handler(&(), window, cx))
        });

        div()
            .id(self.id)
            .flex_dir()
            .items_center()
            .gap(px(12.))
            .w(px(280.))
            .p(px(8.))
            .rounded(theme.radius_large())
            .border_1()
            .border_color(if failed {
                colors.destructive.opacity(0.5)
            } else {
                colors.border
            })
            .bg(colors.card)
            .when_some(self.on_open, |attachment, handler| {
                attachment
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(colors.accent.opacity(0.5)))
                    .on_click(move |_, window, cx| handler(&(), window, cx))
            })
            .child(media)
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(self.name),
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
