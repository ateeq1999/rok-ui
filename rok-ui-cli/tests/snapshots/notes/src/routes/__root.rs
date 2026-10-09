use rok_ui::prelude::*;
use rok_ui::router::file_route;

file_route! {
    layout: Shell,
}

/// The frame around every page: navigation, then the page.
#[component]
fn Shell(#[children] children: Vec<AnyElement>) -> impl IntoElement {
    let links = [
        Link::to(&super::Index).exact(true).child("Home"),
        Link::new("nav-notes", "/notes").child("Notes"),
        // rok-ui:nav
    ];
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(div().flex().gap_4().p_4().border_b_1().children(links))
        .child(div().flex_1().children(children))
}
