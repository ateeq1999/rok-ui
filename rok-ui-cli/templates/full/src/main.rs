//! Opens the main window. The app lives in `src/lib.rs`.

use rok_ui::prelude::*;
use {{crate_name}}::routes;

struct Main;

impl Render for Main {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(routes::tree())
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
