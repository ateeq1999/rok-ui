//! Opens the notes window. The app lives in the library: `src/lib.rs`.

use rok_ui::prelude::*;
use rok_ui_example_file_routes::routes;

struct NotesWindow;

impl Render for NotesWindow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(routes::tree())
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut App| {
            rok_ui::init(cx);
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| NotesWindow))
                .expect("the window opens");
        });
}
