//! Widgets shared by features.

use rok_ui::prelude::*;

/// A page's frame: a title over the page's content.
#[component]
pub fn PageFrame(title: SharedString, #[children] children: Vec<AnyElement>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .child(H2::new(title))
        .children(children)
}
