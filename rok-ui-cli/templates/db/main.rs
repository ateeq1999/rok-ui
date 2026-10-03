//! Opens the main window after connecting to `DATABASE_URL`. The app lives in `src/lib.rs`.

use rok_ui::{db, prelude::*};
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
            let url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/{{name}}".into());
            let connecting = db::connect(url, cx);
            cx.spawn(async move |cx| {
                if connecting.await.is_ok() {
                    cx.update(|cx| {
                        db::run(cx, |db| async move {
                            db.execute(include_str!("../migrations/0001_create_notes.sql")).await
                        })
                        .detach();
                    })
                    .ok();
                }
            })
            .detach();
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Main))
                .expect("the window opens");
        });
}
