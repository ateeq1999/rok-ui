use rok_ui::prelude::*;
use rok_ui::router::file_route;

file_route! {
    layout: AppShell,
}

/// The window frame: navigation links around every page.
#[component]
fn AppShell(#[children] children: Vec<AnyElement>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(
            div()
                .flex()
                .gap_4()
                .p_4()
                .border_b_1()
                .child(Link::to(&super::Index).exact(true).child("Home"))
                .child(Link::to(&super::Notes).child("Notes"))
                .child(Link::to(&super::Settings).child("Settings")),
        )
        .child(div().flex_1().p_4().children(children))
}
