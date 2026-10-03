//! An Arabic chat: messages, bubbles, markers and a composer, right to left, in
//! the Cairo font.
//!
//! Run with `cargo run --example arabic_chat --features font-cairo`, then type a
//! message and press Enter.

use rok_ui::prelude::*;

#[derive(Clone)]
enum Entry {
    Separator(&'static str),
    Note(&'static str),
    Incoming(SharedString),
    Outgoing(SharedString),
}

struct ChatWindow {
    entries: Vec<Entry>,
    composer: Entity<InputState>,
    scroller: MessageScrollerState,
    _subscription: gpui::Subscription,
}

impl ChatWindow {
    fn new(cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| InputState::new(cx).with_placeholder("اكتب رسالة…"));
        let subscription = cx.subscribe(&composer, |this, composer, event: &InputEvent, cx| {
            if let InputEvent::Submitted(text) = event {
                if !text.trim().is_empty() {
                    this.send(text.clone(), cx);
                    composer.update(cx, |state, cx| state.set_text("", cx));
                }
            }
        });
        let entries = vec![
            Entry::Separator("اليوم"),
            Entry::Note("انضمت سارة إلى المحادثة"),
            Entry::Incoming("مرحباً! هل راجعت تصميم الصفحة الرئيسية؟".into()),
            Entry::Outgoing("نعم، يبدو رائعاً. لدي ملاحظتان صغيرتان فقط.".into()),
            Entry::Incoming(
                "ممتاز. أرسل لي الملاحظات وسأحدّث النسخة قبل اجتماع الساعة 3:30 \
                 مع فريق Rust. نريد أيضاً تجربة الوضع الداكن على شاشات 4K."
                    .into(),
            ),
            Entry::Outgoing("تم! أرسلتها الآن 👍".into()),
        ];
        let scroller = MessageScrollerState::new(entries.len());
        Self {
            entries,
            composer,
            scroller,
            _subscription: subscription,
        }
    }

    fn send(&mut self, text: SharedString, cx: &mut Context<Self>) {
        self.entries.push(Entry::Outgoing(text));
        self.scroller.push(1);
        self.scroller.scroll_to_bottom();
        cx.notify();
    }
}

fn render_entry(index: usize, entry: &Entry) -> AnyElement {
    match entry {
        Entry::Separator(text) => Marker::separator(*text).into_any_element(),
        Entry::Note(text) => Marker::note(*text).icon(IconName::User).into_any_element(),
        Entry::Incoming(text) => Message::new()
            .avatar(Avatar::new("س"))
            .name("سارة")
            .timestamp("١٠:٤٢")
            .child(
                Bubble::new(("bubble", index))
                    .reaction(BubbleReaction::new("👍", 2).reacted(true))
                    .child(BidiText::new(text.clone())),
            )
            .into_any_element(),
        Entry::Outgoing(text) => Message::new()
            .align(BubbleAlign::End)
            .child(
                Bubble::new(("bubble", index))
                    .variant(BubbleVariant::Primary)
                    .align(BubbleAlign::End)
                    .child(BidiText::new(text.clone())),
            )
            .footer(BidiText::new("تمت القراءة"))
            .into_any_element(),
    }
}

impl Render for ChatWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.entries.clone();
        let send = cx.listener(|this, _: &ClickEvent, _, cx| {
            let text = this.composer.read(cx).text().clone();
            if !text.trim().is_empty() {
                this.send(text, cx);
                this.composer.update(cx, |state, cx| state.set_text("", cx));
            }
        });

        AppRoot::new().child(
            div()
                .size_full()
                .flex_dir()
                .flex_col()
                .child(
                    div()
                        .flex_dir()
                        .items_center()
                        .gap(px(12.))
                        .px(px(20.))
                        .py(px(14.))
                        .border_b_1()
                        .border_color(cx.theme().colors.border)
                        .child(Avatar::new("س"))
                        .child(
                            div()
                                .flex_1()
                                .flex_dir()
                                .flex_col()
                                .child(Large::new("سارة الأحمد"))
                                .child(
                                    Marker::status("متصلة الآن")
                                        .dot_color(gpui::rgb(0x16A34A).into()),
                                ),
                        )
                        .child(
                            Button::new("theme")
                                .ghost()
                                .icon_only(IconName::Moon)
                                .tooltip("الوضع الداكن")
                                .on_click(|_, _, cx| Theme::toggle_mode(cx)),
                        ),
                )
                .child(
                    MessageScroller::new(&self.scroller, move |index, _, _| {
                        entries.get(index).map_or_else(
                            || div().into_any_element(),
                            |entry| render_entry(index, entry),
                        )
                    })
                    .flex_1(),
                )
                .child(
                    div()
                        .p(px(16.))
                        .border_t_1()
                        .border_color(cx.theme().colors.border)
                        .child(
                            InputGroup::new(&self.composer)
                                .leading_icon(IconName::Smile)
                                .trailing(
                                    Button::new("send")
                                        .small()
                                        .icon_only(IconName::Send.for_direction())
                                        .tooltip("إرسال")
                                        .on_click(send),
                                ),
                        ),
                ),
        )
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut App| {
            rok_ui::init(cx);
            rok_ui::fonts::CAIRO
                .register(cx)
                .expect("the Cairo font is bundled");
            Theme::set_font_family(rok_ui::fonts::CAIRO.family(), cx);
            set_text_direction(TextDirection::Rtl, cx);

            let bounds = Bounds::centered(None, gpui::size(px(560.), px(760.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("محادثة".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(ChatWindow::new),
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}
