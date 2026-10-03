//! A rok-ui app. Run it with `cargo run`.

use rok_ui::prelude::*;

/// A counter: a component with state.
#[component]
fn Counter(cx: &mut Cx) -> impl IntoElement {
    let count = cx.use_state(|| 0);
    let value = count.get(cx);
    Card::new().child(
        CardContent::new().child(
            Button::new("increment")
                .label(format!("Clicked {value} times"))
                .on_click(move |_, _, cx| count.update(cx, |count| *count += 1)),
        ),
    )
}

struct Main;

impl Render for Main {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .items_center()
            .justify_center()
            .child(Counter::new())
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut App| {
            rok_ui::init(cx);
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Main))
                .expect("the window opens");
        });
}
