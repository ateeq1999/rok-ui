use rok_ui::prelude::*;
use rok_ui::router::file_route;

file_route! {
    component: Home,
}

/// The start page.
#[component]
fn Home() -> impl IntoElement {
    div().child(H1::new("{{name}}"))
}
