//! Message: one turn in a conversation, with avatar, header and footer.

use gpui::{
    div, prelude::*, px, AnyElement, App, FontWeight, SharedString, StyleRefinement, Window,
};

use super::bubble::BubbleAlign;
use super::direction::DirectionalStyled;
use crate::sx::SxStyled;
use crate::{styles::ApplyStyleOverrides, theme::ActiveTheme};

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

impl RenderOnce for Message {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let is_end = self.align == BubbleAlign::End;
        let has_header = self.name.is_some() || self.timestamp.is_some() || !self.header.is_empty();

        let header = has_header.then(|| {
            div()
                .flex_dir()
                .items_center()
                .gap(px(8.))
                .when(is_end, flip_row)
                .when_some(self.name, |header, name| {
                    header.child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(name),
                    )
                })
                .when_some(self.timestamp, |header, timestamp| {
                    header.child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(timestamp),
                    )
                })
                .children(self.header)
        });
        let footer = (!self.footer.is_empty()).then(|| {
            div()
                .flex_dir()
                .items_center()
                .gap(px(8.))
                .text_xs()
                .text_color(colors.muted_foreground)
                .children(self.footer)
        });

        div()
            .flex_dir()
            .items_start()
            .gap(px(12.))
            .w_full()
            .when(is_end, flip_row)
            .when_some(self.avatar, |message, avatar| {
                message.child(div().flex_none().child(avatar))
            })
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(4.))
                    .map(|column| {
                        // End-aligned turns sit on the right in LTR and the left in RTL.
                        if is_end ^ super::direction::is_rtl() {
                            column.items_end()
                        } else {
                            column.items_start()
                        }
                    })
                    .children(header)
                    .children(self.children)
                    .children(footer),
            )
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Reverse a row that already flows in the reading direction.
fn flip_row(row: gpui::Div) -> gpui::Div {
    if super::direction::is_rtl() {
        row.flex_row()
    } else {
        row.flex_row_reverse()
    }
}
