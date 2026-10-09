//! Opens the main window. The app lives in `src/lib.rs`; its providers in `src/app.rs`.

use rok_ui::prelude::*;

struct Main;

impl Render for Main {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child({{crate_name}}::app::App::new())
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut gpui::App| {
            rok_ui::init(cx);
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Main))
                .expect("the window opens");
        });
}
