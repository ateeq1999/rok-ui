//! Opens the main window. The app lives in `src/lib.rs`; its providers in `src/app.rs`.
//!
//! The API's address comes from `ROK_API_URL` (default `http://localhost:8080`).

use rok_ui::http::{HttpClient, Session};
use rok_ui::prelude::*;

struct Main {
    client: HttpClient,
    session: Session,
}

impl Render for Main {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child({{crate_name}}::app::App::new(
            self.client.clone(),
            self.session.clone(),
        ))
    }
}

fn main() {
    // The session holds the token; the client sends it, and expires the session on a 401.
    // The token lives in memory: see `Session::persisted` to keep users signed in.
    let session = Session::new();
    let client = HttpClient::builder().session(session.clone()).build();
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(move |cx: &mut gpui::App| {
            rok_ui::init(cx);
            cx.open_window(WindowOptions::default(), |_, cx| {
                cx.new(|_| Main { client, session })
            })
            .expect("the window opens");
        });
}
