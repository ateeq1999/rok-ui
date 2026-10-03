use rok_ui::prelude::*;
use rok_ui::{
    query::{self, Suspense},
    router::{file_route, Search},
};

use crate::features::notes::note_query;

file_route! {
    params: { id: u64 },
    search: NoteSearch,
    // Starts fetching on navigation, and on hover over links with `.preload(true)`.
    loader: |route, cx| query::prefetch_query(cx, &note_query(route.id)),
    component: NotePage,
}

/// `?history=true` shows the note's history.
#[derive(Search, Clone, Debug, PartialEq)]
pub struct NoteSearch {
    #[search(default)]
    history: bool,
}

/// One note.
#[component]
fn NotePage(cx: &mut Cx) -> impl IntoElement {
    let Route { id } = params(cx);
    let search = search(cx);
    Suspense::new(move |cx| {
        let note = query::use_suspense_query(cx, note_query(id))?;
        Ok(div()
            .flex()
            .flex_col()
            .gap_2()
            .child(H2::new(note.title.clone()))
            .child(P::new(note.body.clone()))
            .child(if search.history {
                Link::to(&Route { id }).child("Hide history")
            } else {
                Link::to(&Route { id })
                    .search(&NoteSearch { history: true })
                    .child("Show history")
            }))
    })
    .fallback(Skeleton::new("note").h(px(120.)))
}
