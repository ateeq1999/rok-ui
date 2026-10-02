//! Marker: an inline status, a system note, a bordered row or a labeled
//! separator inside a conversation.

use gpui::{div, prelude::*, px, App, Hsla, SharedString, StyleRefinement, Window};

use super::spinner::Spinner;
use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

#[derive(Clone)]
enum MarkerKind {
    Status { dot: Option<Hsla>, busy: bool },
    Note,
    Row,
    Separator,
}

/// ```ignore
/// Marker::status("Ada is typing…").busy(true)
/// Marker::note("Conversation renamed to “Launch plan”")
/// Marker::row("Searched the web").icon(IconName::Globe).detail("4 results")
/// Marker::separator("Today")
/// ```
#[derive(IntoElement)]
pub struct Marker {
    kind: MarkerKind,
    text: SharedString,
    detail: Option<SharedString>,
    icon: Option<IconName>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Marker);

impl Marker {
    fn new(kind: MarkerKind, text: impl Into<SharedString>) -> Self {
        Self {
            kind,
            text: text.into(),
            detail: None,
            icon: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Muted inline status with a leading dot (or spinner when busy).
    pub fn status(text: impl Into<SharedString>) -> Self {
        Self::new(
            MarkerKind::Status {
                dot: None,
                busy: false,
            },
            text,
        )
    }

    /// A centered system note.
    pub fn note(text: impl Into<SharedString>) -> Self {
        Self::new(MarkerKind::Note, text)
    }

    /// A bordered row, for tool calls and other events.
    pub fn row(text: impl Into<SharedString>) -> Self {
        Self::new(MarkerKind::Row, text)
    }

    /// A line with a centered label, for date breaks.
    pub fn separator(text: impl Into<SharedString>) -> Self {
        Self::new(MarkerKind::Separator, text)
    }

    /// Show a spinner instead of the dot (status markers).
    pub fn busy(mut self, busy: bool) -> Self {
        if let MarkerKind::Status { dot, .. } = self.kind {
            self.kind = MarkerKind::Status { dot, busy };
        }
        self
    }

    /// Color of the status dot. Defaults to the muted text color.
    pub fn dot_color(mut self, color: Hsla) -> Self {
        if let MarkerKind::Status { busy, .. } = self.kind {
            self.kind = MarkerKind::Status {
                dot: Some(color),
                busy,
            };
        }
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Trailing muted text (row markers).
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

styles! {
    MARKER = {
        base: { display: flex, align: center, gap: 2, text: xs, color: muted_foreground },
        dot: { size: 1.5, radius: full, background: muted_foreground },
        note: { width: full, justify: center },
        row: {
            width: full,
            padding_x: 3,
            padding_y: 2,
            radius: md,
            border: 1,
            border_color: border,
            color: foreground,
        },
        row_text: { flex: 1, truncate: true },
        row_detail: { color: muted_foreground },
        separator: { width: full },
        rule: { flex: 1, height: 0.25, background: border },
    }
}

impl RenderOnce for Marker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().colors.muted_foreground;
        let icon = self
            .icon
            .map(|icon| Icon::new(icon).size(px(14.)).color(muted));
        let kind_style = match self.kind {
            MarkerKind::Status { .. } => None,
            MarkerKind::Note => Some(&MARKER.note),
            MarkerKind::Row => Some(&MARKER.row),
            MarkerKind::Separator => Some(&MARKER.separator),
        };
        let row = div().sx((&MARKER.base, kind_style, &self.sx));

        match self.kind {
            MarkerKind::Status { dot, busy } => row
                .map(|row| {
                    if busy {
                        row.child(Spinner::new().size(px(12.)))
                    } else {
                        row.child(
                            div()
                                .sx(&MARKER.dot)
                                .when_some(dot, |dot, color| dot.bg(color)),
                        )
                    }
                })
                .children(icon)
                .child(crate::components::bidi_text::text(self.text)),
            MarkerKind::Note => row
                .children(icon)
                .child(crate::components::bidi_text::text(self.text)),
            MarkerKind::Row => row
                .children(icon)
                .child(
                    div()
                        .sx(&MARKER.row_text)
                        .child(crate::components::bidi_text::text(self.text)),
                )
                .when_some(self.detail, |row, detail| {
                    row.child(
                        div()
                            .sx(&MARKER.row_detail)
                            .child(crate::components::bidi_text::text(detail)),
                    )
                }),
            MarkerKind::Separator => row
                .child(div().sx(&MARKER.rule))
                .children(icon)
                .child(crate::components::bidi_text::text(self.text))
                .child(div().sx(&MARKER.rule)),
        }
        .apply_style_overrides(&self.style_overrides)
    }
}
