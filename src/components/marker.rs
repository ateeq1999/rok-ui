//! Marker: an inline status, a system note, a bordered row or a labeled
//! separator inside a conversation.

use gpui::{div, prelude::*, px, App, Hsla, SharedString, StyleRefinement, Window};

use super::direction::DirectionalStyled;
use super::spinner::Spinner;
use crate::{
    icon::{Icon, IconName},
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

impl RenderOnce for Marker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let muted = colors.muted_foreground;
        let icon = self
            .icon
            .map(|icon| Icon::new(icon).size(px(14.)).color(muted));
        let row = div()
            .flex_dir()
            .items_center()
            .gap(px(8.))
            .text_xs()
            .text_color(muted);

        match self.kind {
            MarkerKind::Status { dot, busy } => row
                .map(|row| {
                    if busy {
                        row.child(Spinner::new().size(px(12.)))
                    } else {
                        row.child(div().size(px(6.)).rounded_full().bg(dot.unwrap_or(muted)))
                    }
                })
                .children(icon)
                .child(self.text),
            MarkerKind::Note => row
                .w_full()
                .justify_center()
                .children(icon)
                .child(self.text),
            MarkerKind::Row => row
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .rounded(theme.radius_medium())
                .border_1()
                .border_color(colors.border)
                .text_color(colors.foreground)
                .children(icon)
                .child(div().flex_1().truncate().child(self.text))
                .when_some(self.detail, |row, detail| {
                    row.child(div().text_color(muted).child(detail))
                }),
            MarkerKind::Separator => row
                .w_full()
                .child(div().flex_1().h(px(1.)).bg(colors.border))
                .children(icon)
                .child(self.text)
                .child(div().flex_1().h(px(1.)).bg(colors.border)),
        }
        .apply_style_overrides(&self.style_overrides)
    }
}
