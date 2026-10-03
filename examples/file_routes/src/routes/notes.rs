use rok_ui::prelude::*;
use rok_ui::{query, router::file_route};

use crate::features::notes::notes_query;

file_route! {
    layout: NotesLayout,
}

/// The note list beside the selected note.
#[component]
fn NotesLayout(#[children] children: Vec<AnyElement>, cx: &mut Cx) -> impl IntoElement {
    let notes = query::use_query(cx, notes_query());
    let links = notes.data().cloned().unwrap_or_default().into_iter().map(|note| {
        Link::to(&super::NotesId { id: note.id }).child(note.title)
    });
    div()
        .flex()
        .gap_6()
        .child(div().flex().flex_col().gap_2().w(px(200.)).children(links))
        .child(div().flex_1().children(children))
}
