//! Message: one turn in a conversation, with avatar, header and footer.

use gpui::{div, prelude::*, AnyElement, App, SharedString, StyleRefinement, Window};

use super::bubble::BubbleAlign;
use crate::sx::SxStyled;
use crate::{styles, styles::ApplyStyleOverrides};

/// ```ignore
/// Message::new()
///     .avatar(Avatar::new("AL"))
///     .name("Ada Lovelace")
///     .timestamp("10:42")
///     .child(Bubble::new("m1").child("Have you seen the new engine drawings?"))
///     .footer("Read")
///
/// Message::new().align(BubbleAlign::End)
///     .child(Bubble::new("m2").variant(BubbleVariant::Primary).align(BubbleAlign::End).child("Yes!"))
/// ```
#[derive(IntoElement)]
pub struct Message {
    align: BubbleAlign,
    avatar: Option<AnyElement>,
    name: Option<SharedString>,
    timestamp: Option<SharedString>,
    header: Vec<AnyElement>,
    footer: Vec<AnyElement>,
    children: Vec<AnyElement>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Message);

impl Message {
    pub fn new() -> Self {
        Self {
            align: BubbleAlign::Start,
            avatar: None,
            name: None,
            timestamp: None,
            header: Vec::new(),
            footer: Vec::new(),
            children: Vec::new(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// `End` puts the avatar on the right and right-aligns everything (your own turns).
    pub fn align(mut self, align: BubbleAlign) -> Self {
        self.align = align;
        self
    }

    /// Usually an [`super::Avatar`]. Its column stays reserved for grouping.
    pub fn avatar(mut self, avatar: impl IntoElement) -> Self {
        self.avatar = Some(avatar.into_any_element());
        self
    }

    /// Sender name in the header.
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Muted time next to the name.
    pub fn timestamp(mut self, timestamp: impl Into<SharedString>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    /// Extra header content, like a badge.
    pub fn header(mut self, element: impl IntoElement) -> Self {
        self.header.push(element.into_any_element());
        self
    }

    /// Muted line under the content: delivery state, actions.
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.footer.push(element.into_any_element());
        self
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Message {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

styles! {
    MESSAGE = {
        row: { display: flex, align: start, gap: 3, width: full },
        // End-aligned turns mirror the row that already flows in the reading direction.
        flipped: { direction: row_reverse },
        avatar: { flex: none },
        column: { display: flex, direction: column, flex: 1, min_width: 0, gap: 1 },
        // Physical sides: end-aligned turns sit on the right in LTR and the left in RTL.
        column_left: { align: start },
        column_right: { align: end },
        header: { display: flex, align: center, gap: 2 },
        name: { text: sm, font: semibold },
        timestamp: { text: xs, color: muted_foreground },
        footer: { display: flex, align: center, gap: 2, text: xs, color: muted_foreground },
    }
}

impl RenderOnce for Message {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let is_end = self.align == BubbleAlign::End;
        let flipped = is_end.then_some(&MESSAGE.flipped);
        let has_header = self.name.is_some() || self.timestamp.is_some() || !self.header.is_empty();

        let header = has_header.then(|| {
            div()
                .sx((&MESSAGE.header, flipped))
                .when_some(self.name, |header, name| {
                    header.child(div().sx(&MESSAGE.name).child(name))
                })
                .when_some(self.timestamp, |header, timestamp| {
                    header.child(div().sx(&MESSAGE.timestamp).child(timestamp))
                })
                .children(self.header)
        });
        let footer =
            (!self.footer.is_empty()).then(|| div().sx(&MESSAGE.footer).children(self.footer));

        let column_side = if is_end ^ super::direction::is_rtl() {
            &MESSAGE.column_right
        } else {
            &MESSAGE.column_left
        };
        div()
            .sx((&MESSAGE.row, flipped, &self.sx))
            .when_some(self.avatar, |message, avatar| {
                message.child(div().sx(&MESSAGE.avatar).child(avatar))
            })
            .child(
                div()
                    .sx((&MESSAGE.column, column_side))
                    .children(header)
                    .children(self.children)
                    .children(footer),
            )
            .apply_style_overrides(&self.style_overrides)
    }
}
