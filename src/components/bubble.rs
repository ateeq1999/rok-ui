//! Bubble: conversational content in a chat bubble.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ElementId, Pixels, SharedString, StyleRefinement, Window,
};

use super::overlay::child_id;
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler},
    styles,
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
    /// At the reading-direction start (messages from others).
    #[default]
    Start,
    /// At the reading-direction end (your own messages).
    End,
}

/// Position within a run of bubbles from the same sender. Corners on the tail
/// side tighten so a run reads as one group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BubbleGroupPosition {
    /// A message on its own: every corner rounded.
    #[default]
    Single,
    /// The first of consecutive messages from one sender.
    First,
    /// Between the first and last of a group.
    Middle,
    /// The last of consecutive messages from one sender.
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
    /// An `emoji` reaction with its `count`.
    pub fn new(emoji: impl Into<SharedString>, count: usize) -> Self {
        Self {
            emoji: emoji.into(),
            count,
            reacted: false,
        }
    }

    /// Highlight the chip: the current user added this reaction.
    #[must_use]
    pub fn reacted(mut self, reacted: bool) -> Self {
        self.reacted = reacted;
        self
    }
}

/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let long_text = "A long message.";
/// let question = Bubble::new("m1")
///     .align(BubbleAlign::End)
///     .variant(BubbleVariant::Primary)
///     .group_position(BubbleGroupPosition::First)
///     .child("Are we still on for tomorrow?");
///
/// let reply = Bubble::new("m2")
///     .reaction(BubbleReaction::new("👍", 2))
///     .collapse_after(px(120.))
///     .child(long_text);
/// # }
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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Bubble);

impl Bubble {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
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
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The visual variant.
    #[must_use]
    pub fn variant(mut self, variant: BubbleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Which side of the conversation the bubble sits on.
    #[must_use]
    pub fn align(mut self, align: BubbleAlign) -> Self {
        self.align = align;
        self
    }

    /// Where the bubble sits in a run of messages from one sender.
    #[must_use]
    pub fn group_position(mut self, position: BubbleGroupPosition) -> Self {
        self.group_position = position;
        self
    }

    /// Add a reaction below the bubble.
    #[must_use]
    pub fn reaction(mut self, reaction: BubbleReaction) -> Self {
        self.reactions.push(reaction);
        self
    }

    /// Called with the emoji when a reaction chip is clicked.
    #[must_use]
    pub fn on_reaction(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reaction = Some(Rc::new(handler));
        self
    }

    /// Clip content taller than `height` behind a "Show more" toggle.
    #[must_use]
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

styles! {
    BUBBLE = {
        // Physical sides: end-aligned bubbles sit on the right in LTR and the left in RTL.
        column: { display: flex, direction: column, gap: 1 },
        column_left: { align: start },
        column_right: { align: end },
        bubble: {
            display: flex,
            direction: column,
            max_width: 120,
            padding_x: 3.5,
            padding_y: 2,
            border: 1,
            text: sm,
        },
        variant(BubbleVariant): {
            Default: { background: muted, color: foreground, border_color: muted },
            Primary: { background: primary, color: primary_foreground, border_color: primary },
            Outline: { background: transparent, color: foreground, border_color: border },
            Ghost: {
                padding_x: 0,
                padding_y: 0,
                background: transparent,
                color: foreground,
                border_color: transparent,
            },
        },
        content: { position: relative, display: flex, direction: column, gap: 1 },
        fade: { position: absolute, bottom: 0, left: 0, width: full, height: 6 },
        collapse_toggle: {
            padding_top: 1,
            text: xs,
            font: medium,
            opacity: 0.8,
            cursor: pointer,
            hover: { underline: true },
        },
        reactions: { display: flex, wrap: true, gap: 1 },
        reaction: {
            display: flex,
            align: center,
            gap: 1,
            height: 6,
            padding_x: 2,
            radius: full,
            border: 1,
            border_color: border,
            background: background,
            text: xs,
        },
        reacted: { border_color: ring, background: accent },
        reaction_clickable: { cursor: pointer },
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

        // Collapsed content fades out into the bubble fill (or the page behind a
        // transparent bubble) instead of cutting a line in half.
        let fade_color = match self.variant {
            BubbleVariant::Default => colors.muted,
            BubbleVariant::Primary => colors.primary,
            BubbleVariant::Outline | BubbleVariant::Ghost => colors.background,
        };
        let on_right = (self.align == BubbleAlign::End) ^ super::direction::is_rtl();

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
                .sx(&BUBBLE.collapse_toggle)
                .on_click(move |_, _, cx| expanded.update(cx, |expanded| *expanded = !*expanded))
                .child(if is_expanded {
                    "Show less"
                } else {
                    "Show more"
                })
        });

        let content = div().sx(&BUBBLE.content).children(self.children).when_some(
            self.collapse_after.filter(|_| !is_expanded),
            |content, height| {
                content
                    .max_h(height)
                    .overflow_hidden()
                    .child(div().sx(&BUBBLE.fade).bg(gpui::linear_gradient(
                        180.,
                        gpui::linear_color_stop(fade_color.opacity(0.), 0.),
                        gpui::linear_color_stop(fade_color, 1.),
                    )))
            },
        );

        let bubble = div()
            .sx((&BUBBLE.bubble, BUBBLE.variant(self.variant)))
            // The tail is on the left for start-aligned bubbles in LTR, the right in RTL.
            .map(|bubble| {
                if on_right {
                    bubble
                        .rounded_tl(large)
                        .rounded_bl(large)
                        .rounded_tr(tail_top)
                        .rounded_br(tail_bottom)
                } else {
                    bubble
                        .rounded_tr(large)
                        .rounded_br(large)
                        .rounded_tl(tail_top)
                        .rounded_bl(tail_bottom)
                }
            })
            .child(content)
            .children(collapse_toggle);

        let reactions = (!self.reactions.is_empty()).then(|| {
            let on_reaction = self.on_reaction.clone();
            div()
                .sx(&BUBBLE.reactions)
                .children(
                    self.reactions
                        .into_iter()
                        .enumerate()
                        .map(|(index, reaction)| {
                            let on_reaction = on_reaction.clone();
                            let emoji = reaction.emoji.clone();
                            div()
                                .id(("bubble-reaction", index))
                                .sx((
                                    &BUBBLE.reaction,
                                    reaction.reacted.then_some(&BUBBLE.reacted),
                                    on_reaction.is_some().then_some(&BUBBLE.reaction_clickable),
                                ))
                                .when_some(on_reaction, |chip, handler| {
                                    chip.on_click(move |_, window, cx| handler(&emoji, window, cx))
                                })
                                .child(reaction.emoji)
                                .child(reaction.count.to_string())
                        }),
                )
        });

        div()
            .id(self.id)
            .sx((
                &BUBBLE.column,
                if on_right {
                    &BUBBLE.column_right
                } else {
                    &BUBBLE.column_left
                },
                &self.sx,
            ))
            .child(bubble)
            .children(reactions)
            .apply_style_overrides(&self.style_overrides)
    }
}
