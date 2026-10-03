//! The generated route tree: typed routes, layouts, guards and "not found".

use rok_ui::{
    prelude::*,
    router::{self, Route},
};
use rok_ui_example_file_routes::{features::session, routes};

struct Shell;

impl Render for Shell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(routes::tree())
    }
}

#[test]
fn route_files_become_typed_routes() {
    assert_eq!(routes::Index.href(), "/");
    assert_eq!(routes::Notes.href(), "/notes");
    assert_eq!(routes::NotesId { id: 2 }.href(), "/notes/2");
    assert_eq!(
        routes::NotesId::parse("/notes/7"),
        Some(routes::NotesId { id: 7 })
    );
    // `_auth/settings.rs`: the pathless layout adds no segment.
    assert_eq!(routes::Settings.href(), "/settings");
    assert_eq!(routes::Login.href(), "/login");
}

#[gpui::test]
fn the_auth_guard_redirects_signed_out_users(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (_, window) = cx.add_window_view(|_, _| Shell);
    session::set_signed_in(false);
    window.update(|_, cx| router::navigate_to(&routes::Settings, cx));
    window.run_until_parked();
    window.update(|_, cx| {
        let location = router::location(cx);
        assert_eq!(location.path(), "/login");
        assert_eq!(location.query("next"), Some("/settings"));
    });

    session::set_signed_in(true);
    window.update(|_, cx| router::navigate_to(&routes::Settings, cx));
    window.run_until_parked();
    window.update(|_, cx| assert_eq!(router::location(cx).path(), "/settings"));
}
