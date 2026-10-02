//! Bubble: conversational content in a chat bubble.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, Pixels, SharedString,
    StyleRefinement, Window,
};

use super::overlay::child_id;
use crate::{
    hooks::{use_keyed_state, EventHandler},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Fill of a [`Bubble`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BubbleVariant {
    /// Muted fill: messages from others.
    #[default]
    Default,
    /// Primary fill: your own messages.
    Primary,
    /// Bordered, no fill.
    Outline,
    /// No fill or padding: assistant replies that read like a document.
    Ghost,
}

/// Which side the bubble's tail is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BubbleAlign {
    #[default]
    Start,
    End,
}

/// Position within a run of bubbles from the same sender. Corners on the tail
/// side tighten so a run reads as one group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BubbleGroupPosition {
    #[default]
    Single,
    First,
    Middle,
    Last,
}

/// A reaction chip under a bubble.
#[derive(Clone)]
pub struct BubbleReaction {
    emoji: SharedString,
    count: usize,
    reacted: bool,
}

impl BubbleReaction {
    pub fn new(emoji: impl Into<SharedString>, count: usize) -> Self {
        Self {
            emoji: emoji.into(),
            count,
            reacted: false,
        }
    }

    /// Highlight the chip: the current user added this reaction.
    pub fn reacted(mut self, reacted: bool) -> Self {
        self.reacted = reacted;
        self
    }
}

/// ```ignore
/// Bubble::new("m1").align(BubbleAlign::End).variant(BubbleVariant::Primary)
///     .group_position(BubbleGroupPosition::First)
///     .child("Are we still on for tomorrow?")
///
/// Bubble::new("m2").reaction(BubbleReaction::new("👍", 2))
///     .collapse_after(px(120.))
///     .child(long_text)
/// ```
#[derive(IntoElement)]
pub struct Bubble {
    id: ElementId,
    variant: BubbleVariant,
    align: BubbleAlign,
    group_position: BubbleGroupPosition,
    reactions: Vec<BubbleReaction>,
    on_reaction: Option<EventHandler<SharedString>>,
    collapse_after: Option<Pixels>,
    children: Vec<AnyElement>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Bubble);

impl Bubble {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: BubbleVariant::Default,
            align: BubbleAlign::Start,
            group_position: BubbleGroupPosition::Single,
            reactions: Vec::new(),
            on_reaction: None,
            collapse_after: None,
            children: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: BubbleVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn align(mut self, align: BubbleAlign) -> Self {
        self.align = align;
        self
    }

    pub fn group_position(mut self, position: BubbleGroupPosition) -> Self {
        self.group_position = position;
        self
    }

    pub fn reaction(mut self, reaction: BubbleReaction) -> Self {
        self.reactions.push(reaction);
        self
    }

    /// Called with the emoji when a reaction chip is clicked.
    pub fn on_reaction(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reaction = Some(Rc::new(handler));
        self
    }

    /// Clip content taller than `height` behind a "Show more" toggle.
    pub fn collapse_after(mut self, height: impl Into<Pixels>) -> Self {
        self.collapse_after = Some(height.into());
        self
    }
}

impl ParentElement for Bubble {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Bubble {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let expanded = use_keyed_state(child_id(&self.id, "expanded"), window, cx, || false);
        let is_expanded = expanded.get(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let large = theme.radius_extra_large().max(px(12.));
        let small = px(4.);

        let (background, text, border) = match self.variant {
            BubbleVariant::Default => (colors.muted, colors.foreground, colors.muted),
            BubbleVariant::Primary => (colors.primary, colors.primary_foreground, colors.primary),
            BubbleVariant::Outline => (gpui::transparent_black(), colors.foreground, colors.border),
            BubbleVariant::Ghost => (
                gpui::transparent_black(),
                colors.foreground,
                gpui::transparent_black(),
            ),
        };
        let is_ghost = self.variant == BubbleVariant::Ghost;

        // Tail-side corners: the bottom one is the tail; the top one tightens when
        // the bubble continues a run from the same sender.
        let tail_top = match self.group_position {
            BubbleGroupPosition::Single | BubbleGroupPosition::First => large,
            BubbleGroupPosition::Middle | BubbleGroupPosition::Last => small,
        };
        let tail_bottom = small;

        let collapse_toggle = self.collapse_after.map(|_| {
            let expanded = expanded.clone();
            div()
                .id("bubble-collapse-toggle")
                .pt(px(4.))
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .opacity(0.8)
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.underline())
                .on_click(move |_, _, cx| expanded.update(cx, |expanded| *expanded = !*expanded))
                .child(if is_expanded {
                    "Show less"
                } else {
                    "Show more"
                })
        });

        // Collapsed content fades out at the bottom instead of cutting a line in half.
        let fade_color = if background.a > 0. {
            background
        } else {
            colors.background
        };
        let content = div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(self.children)
            .when_some(
                self.collapse_after.filter(|_| !is_expanded),
                |content, height| {
                    content.max_h(height).overflow_hidden().child(
                        div().absolute().bottom_0().left_0().w_full().h(px(24.)).bg(
                            gpui::linear_gradient(
                                180.,
                                gpui::linear_color_stop(fade_color.opacity(0.), 0.),
                                gpui::linear_color_stop(fade_color, 1.),
                            ),
                        ),
                    )
                },
            );

        let bubble = div()
            .flex()
            .flex_col()
            .max_w(px(480.))
            .when(!is_ghost, |bubble| bubble.px(px(14.)).py(px(8.)))
            .border_1()
            .border_color(border)
            .bg(background)
            .text_color(text)
            .text_sm()
            .map(|bubble| match self.align {
                BubbleAlign::Start => bubble
                    .rounded_tr(large)
                    .rounded_br(large)
                    .rounded_tl(tail_top)
                    .rounded_bl(tail_bottom),
                BubbleAlign::End => bubble
                    .rounded_tl(large)
                    .rounded_bl(large)
                    .rounded_tr(tail_top)
                    .rounded_br(tail_bottom),
            })
            .child(content)
            .children(collapse_toggle);

        let reactions = (!self.reactions.is_empty()).then(|| {
            let on_reaction = self.on_reaction.clone();
            div().flex().flex_wrap().gap(px(4.)).children(
                self.reactions
                    .into_iter()
                    .enumerate()
                    .map(|(index, reaction)| {
                        let on_reaction = on_reaction.clone();
                        let emoji = reaction.emoji.clone();
                        div()
                            .id(("bubble-reaction", index))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .h(px(24.))
                            .px(px(8.))
                            .rounded_full()
                            .border_1()
                            .border_color(if reaction.reacted {
                                colors.ring
                            } else {
                                colors.border
                            })
                            .bg(if reaction.reacted {
                                colors.accent
                            } else {
                                colors.background
                            })
                            .text_xs()
                            .when_some(on_reaction, |chip, handler| {
                                chip.cursor(CursorStyle::PointingHand)
                                    .on_click(move |_, window, cx| handler(&emoji, window, cx))
                            })
                            .child(reaction.emoji)
                            .child(reaction.count.to_string())
                    }),
            )
        });

        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(4.))
            .map(|column| match self.align {
                BubbleAlign::Start => column.items_start(),
                BubbleAlign::End => column.items_end(),
            })
            .child(bubble)
            .children(reactions)
            .apply_style_overrides(&self.style_overrides)
    }
}
