use rok_ui::prelude::*;
use rok_ui::router::file_route;

file_route! {
    component: Home,
}

/// The start page.
#[component]
fn Home() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(H1::new("File-based routes"))
        .child(P::new("Every file in src/routes is a route."))
        .child(Link::to(&super::NotesId { id: 2 }).child("Open the release checklist"))
}
