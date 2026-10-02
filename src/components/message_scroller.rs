//! MessageScroller: the scrolling column of a chat.
//!
//! Built on GPUI's virtualized `list`, anchored to the bottom like a chat log:
//! - it follows new and streamed messages while the reader is at the bottom,
//! - prepending history keeps the reader's place instead of jumping,
//! - a "Jump to latest" button appears when the reader has scrolled up,
//! - [`MessageScrollerState::scroll_to_message`] jumps to any message.

use std::rc::Rc;

use gpui::{
    div, list, prelude::*, px, AnyElement, App, ListAlignment, ListOffset, ListState,
    StyleRefinement, Window,
};

use super::button::{Button, IconPosition};
use crate::{icon::IconName, styles::ApplyStyleOverrides};

/// Scroll position and message count of a [`MessageScroller`]. Keep it in your
/// view and tell it when messages change. Cheap to clone; clones share state.
#[derive(Clone)]
pub struct MessageScrollerState {
    list: ListState,
}

impl MessageScrollerState {
    /// A scroller showing `message_count` messages, scrolled to the latest.
    pub fn new(message_count: usize) -> Self {
        Self {
            list: ListState::new(message_count, ListAlignment::Bottom, px(800.)),
        }
    }

    pub fn message_count(&self) -> usize {
        self.list.item_count()
    }

    /// `count` messages were added at the end.
    pub fn push(&self, count: usize) {
        let end = self.list.item_count();
        self.list.splice(end..end, count);
    }

    /// `count` older messages were added at the start (history loaded). The
    /// reader stays on the message they were looking at.
    pub fn prepend(&self, count: usize) {
        self.list.splice(0..0, count);
    }

    /// The message at `index` changed size (a streamed reply grew); re-measure it.
    pub fn message_changed(&self, index: usize) {
        if index < self.list.item_count() {
            self.list.splice(index..index + 1, 1);
        }
    }

    /// Replace everything (open a different transcript), scrolled to the latest.
    pub fn reset(&self, message_count: usize) {
        self.list.reset(message_count);
    }

    /// Put message `index` at the top of the view.
    pub fn scroll_to_message(&self, index: usize) {
        self.list.scroll_to(ListOffset {
            item_ix: index,
            offset_in_item: px(0.),
        });
    }

    /// Scroll so message `index` is fully visible, moving as little as possible.
    pub fn reveal_message(&self, index: usize) {
        self.list.scroll_to_reveal_item(index);
    }

    pub fn scroll_to_bottom(&self) {
        self.list.scroll_to(ListOffset {
            item_ix: self.list.item_count(),
            offset_in_item: px(0.),
        });
    }

    /// Whether the newest message is in view (the scroller is following).
    pub fn is_at_bottom(&self) -> bool {
        if self.list.logical_scroll_top().item_ix >= self.list.item_count() {
            return true;
        }
        let max_scroll = self.list.max_offset_for_scrollbar().height;
        let scrolled = -self.list.scroll_px_offset_for_scrollbar().y;
        scrolled >= max_scroll - px(2.)
    }
}

type RenderMessage = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement>;

/// ```ignore
/// // In your view: scroller: MessageScrollerState, messages: Vec<ChatMessage>
/// MessageScroller::new(&self.scroller, {
///     let messages = self.messages.clone();
///     move |index, _, _| render_message(&messages[index]).into_any_element()
/// })
/// .on_reach_top(cx.listener(|view, _, _, cx| view.load_older_messages(cx)))
/// .h_full()
/// ```
#[derive(IntoElement)]
pub struct MessageScroller {
    state: MessageScrollerState,
    render_message: RenderMessage,
    on_reach_top: Option<crate::hooks::EventHandler<()>>,
    jump_label: gpui::SharedString,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(MessageScroller);

impl MessageScroller {
    pub fn new(
        state: &MessageScrollerState,
        render_message: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            state: state.clone(),
            render_message: Rc::new(render_message),
            on_reach_top: None,
            jump_label: "Jump to latest".into(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Called when the oldest loaded message scrolls into view; load history here
    /// and call [`MessageScrollerState::prepend`].
    pub fn on_reach_top(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_reach_top = Some(Rc::new(handler));
        self
    }

    /// Text of the button shown when scrolled away from the latest message.
    pub fn jump_label(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.jump_label = label.into();
        self
    }
}

impl RenderOnce for MessageScroller {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let list_state = self.state.list.clone();
        if let Some(on_reach_top) = self.on_reach_top.clone() {
            list_state.set_scroll_handler(move |event, window, cx| {
                if event.is_scrolled && event.visible_range.start == 0 {
                    on_reach_top(&(), window, cx);
                }
            });
        }
        let render_message = self.render_message.clone();
        let show_jump = !self.state.is_at_bottom();
        let jump_state = self.state.clone();

        div()
            .relative()
            .flex()
            .flex_col()
            .child(
                list(list_state, move |index, window, cx| {
                    div()
                        .w_full()
                        .px(px(16.))
                        .py(px(6.))
                        .child(render_message(index, window, cx))
                        .into_any_element()
                })
                .size_full(),
            )
            .when(show_jump, |scroller| {
                scroller.child(
                    div()
                        .absolute()
                        .bottom(px(12.))
                        .left_0()
                        .w_full()
                        .flex()
                        .justify_center()
                        .child(
                            Button::new("message-scroller-jump")
                                .outline()
                                .small()
                                .rounded_full()
                                .shadow_md()
                                .label(self.jump_label)
                                .icon(IconName::ArrowDown)
                                .icon_position(IconPosition::End)
                                .on_click(move |_, window, _| {
                                    jump_state.scroll_to_bottom();
                                    window.refresh();
                                }),
                        ),
                )
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
