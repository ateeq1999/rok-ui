//! The smallest rok-ui app: a function component with a hook.
//!
//! Run with `cargo run --example counter`.

use rok_ui::prelude::*;

/// A counter written the way you would write it in React.
#[component]
fn Counter(
    /// Shown above the count.
    title: SharedString,
    /// How much each click adds.
    #[prop(optional)]
    step: Option<i32>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let count = use_state(window, cx, || 0);
    let step = step.unwrap_or(1);
    let current_count = count.get(cx);

    Card::new()
        .w(px(320.))
        .child(
            CardHeader::new()
                .child(CardTitle::new(title))
                .child(CardDescription::new(format!("Each click adds {step}."))),
        )
        .child(
            CardContent::new().child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(current_count.to_string()),
            ),
        )
        .child(
            CardFooter::new()
                .child(
                    Button::new("increment")
                        .icon(IconName::Plus)
                        .label("Increment")
                        .on_click({
                            let count = count.clone();
                            move |_, _, cx| count.update(cx, |value| *value += step)
                        }),
                )
                .child(
                    Button::new("reset")
                        .outline()
                        .label("Reset")
                        .disabled(current_count == 0)
                        .on_click(move |_, _, cx| count.set(0, cx)),
                ),
        )
}

struct CounterWindow;

impl Render for CounterWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .items_center()
            .justify_center()
            .child(Counter::new("Counter").step(5))
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut App| {
            rok_ui::init(cx);
            let bounds = Bounds::centered(None, gpui::size(px(480.), px(360.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("rok-ui counter".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| CounterWindow),
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}
