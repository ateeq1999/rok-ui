//! The chat page: messages, bubbles and attachments.

use rok_ui::prelude::*;

use super::{column, demo, row};

/// One message in the chat demo.
#[derive(Clone)]
pub(super) struct ChatEntry {
    from_me: bool,
    text: &'static str,
    kind: ChatKind,
}

#[derive(Clone, Copy, PartialEq)]
enum ChatKind {
    Text,
    Separator,
    Note,
    ToolCall,
    Attachments,
}

pub(super) fn chat_entries() -> Vec<ChatEntry> {
    let entry = |from_me, text, kind| ChatEntry {
        from_me,
        text,
        kind,
    };
    let mut entries = vec![entry(false, "Yesterday", ChatKind::Separator)];
    for index in 0..12 {
        entries.push(entry(
            index % 2 == 0,
            if index % 2 == 0 {
                "Older message — scroll up from the latest to load more history."
            } else {
                "Loaded from history without moving your place."
            },
            ChatKind::Text,
        ));
    }
    entries.extend([
        entry(false, "Today", ChatKind::Separator),
        entry(false, "Ada joined the conversation", ChatKind::Note),
        entry(false, "Hi! Can you summarize the engine drawings?", ChatKind::Text),
        entry(true, "Sure, attaching them now.", ChatKind::Text),
        entry(true, "", ChatKind::Attachments),
        entry(false, "Searched the archive", ChatKind::ToolCall),
        entry(
            false,
            "The Analytical Engine has a store, a mill, and a reader for punched cards. The store holds numbers, the mill operates on them, and the cards describe the operations — which is what makes it programmable rather than merely a calculator. Expand this bubble to read the rest of this deliberately long reply about the design and its consequences.",
            ChatKind::Text,
        ),
    ]);
    entries
}

pub(super) fn render_chat_entry(index: usize, entry: &ChatEntry) -> AnyElement {
    match entry.kind {
        ChatKind::Separator => Marker::separator(entry.text).into_any_element(),
        ChatKind::Note => Marker::note(entry.text)
            .icon(IconName::User)
            .into_any_element(),
        ChatKind::ToolCall => Marker::row(entry.text)
            .icon(IconName::Search)
            .detail("3 results")
            .into_any_element(),
        ChatKind::Attachments => Message::new()
            .align(BubbleAlign::End)
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        Attachment::new(("chat-attachment", index), "engine-drawings.pdf")
                            .meta("PDF, 12 pages, 4.1 MB"),
                    )
                    .child(
                        Attachment::new(("chat-upload", index), "mill-detail.png")
                            .state(AttachmentState::Uploading(Some(64.)))
                            .on_remove(|(), _, _| {}),
                    )
                    .child(
                        Attachment::new(("chat-failed", index), "store-notes.txt")
                            .state(AttachmentState::Failed("Upload failed".into()))
                            .on_retry(|(), _, _| {})
                            .on_remove(|(), _, _| {}),
                    ),
            )
            .into_any_element(),
        ChatKind::Text if entry.from_me => Message::new()
            .align(BubbleAlign::End)
            .child(
                Bubble::new(("chat-bubble", index))
                    .variant(BubbleVariant::Primary)
                    .align(BubbleAlign::End)
                    .child(entry.text),
            )
            .footer("Read")
            .into_any_element(),
        ChatKind::Text => Message::new()
            .avatar(Avatar::new("AL"))
            .name("Ada Lovelace")
            .timestamp("10:42")
            .child(
                Bubble::new(("chat-bubble", index))
                    .reaction(BubbleReaction::new("+1", 2).reacted(true))
                    .reaction(BubbleReaction::new("Wow", 1))
                    .collapse_after(px(60.))
                    .child(entry.text),
            )
            .into_any_element(),
    }
}

/// Conversation components and the questionnaire.
#[component]
pub fn ChatPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let entries = chat_entries();
    // The first 13 entries are "history", loaded when the reader reaches the top.
    let history_count = 13;
    let loaded_from = use_state(window, cx, move || history_count);
    let scroller = use_state(window, cx, move || {
        MessageScrollerState::new(chat_entries().len() - history_count)
    })
    .get(cx);
    let first_loaded = loaded_from.get(cx);
    let visible: Vec<ChatEntry> = entries[first_loaded..].to_vec();
    let visible_count = visible.len();
    let typing = use_state(window, cx, || true);
    let answers = use_state(window, cx, Vec::<QuestionnaireAnswer>::new);

    let load_history = {
        let loaded_from = loaded_from.clone();
        let scroller = scroller.clone();
        move |(): &(), _: &mut Window, cx: &mut App| {
            let first = loaded_from.get(cx);
            if first > 0 {
                scroller.prepend(first);
                loaded_from.set(0, cx);
            }
        }
    };
    let jump_scroller = scroller.clone();

    div()
        .flex_dir()
        .items_start()
        .gap(px(24.))
        .child(
            column().child(demo(
                "Message Scroller",
                "Messages, bubbles, markers and attachments. Scroll up to load history.",
                div()
                    .flex_dir()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        MessageScroller::new(&scroller, move |index, _, _| {
                            visible.get(index).map_or_else(
                                || div().into_any_element(),
                                |entry| render_chat_entry(index, entry),
                            )
                        })
                        .on_reach_top(load_history)
                        .h(px(460.))
                        .rounded(cx.theme().radius_large())
                        .border_1()
                        .border_color(cx.theme().colors.border),
                    )
                    .child(
                        row()
                            .child(if typing.get(cx) {
                                Marker::status("Ada is typing…")
                                    .busy(true)
                                    .into_any_element()
                            } else {
                                Marker::status("Ada is online")
                                    .dot_color(gpui::rgb(0x16A34A).into())
                                    .into_any_element()
                            })
                            .child(div().flex_1())
                            .child(
                                Button::new("toggle-typing")
                                    .ghost()
                                    .small()
                                    .label("Toggle status")
                                    .on_click({
                                        let typing = typing.clone();
                                        move |_, _, cx| {
                                            typing.update(cx, |typing| *typing = !*typing);
                                        }
                                    }),
                            )
                            .child(
                                Button::new("jump-first")
                                    .outline()
                                    .small()
                                    .label("Jump to first")
                                    .on_click(move |_, window, _| {
                                        jump_scroller.scroll_to_message(0);
                                        window.refresh();
                                    }),
                            ),
                    )
                    .child(Muted::new(format!("{visible_count} messages loaded"))),
            )),
        )
        .child(
            column()
                .child(
                    Questionnaire::new("onboarding")
                        .question(
                            Question::single(
                                "role",
                                "What best describes you?",
                                ["Engineer", "Designer", "Product manager", "Founder"],
                            )
                            .description("We'll tailor the examples to you."),
                        )
                        .question(Question::multiple(
                            "tools",
                            "Which tools do you use every day?",
                            ["Zed", "VS Code", "Figma", "Linear", "GitHub"],
                        ))
                        .question(
                            Question::freeform("goal", "What do you want to build with rok-ui?")
                                .placeholder("A desktop client for…")
                                .skippable(),
                        )
                        .on_complete({
                            let answers = answers.clone();
                            move |result, _, cx| {
                                answers.set(result.clone(), cx);
                                toast(cx, Toast::success("Thanks for your answers"));
                            }
                        }),
                )
                .when(!answers.read(cx).is_empty(), |column| {
                    column.child(Muted::new(format!(
                        "on_complete received {} answers",
                        answers.read(cx).len()
                    )))
                }),
        )
}
