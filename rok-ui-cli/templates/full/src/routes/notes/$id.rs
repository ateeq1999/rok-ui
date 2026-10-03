use rok_ui::prelude::*;
use rok_ui::{
    query::{self, Suspense},
    router::file_route,
};

use crate::features::notes::note_query;

file_route! {
    params: { id: i64 },
    loader: |route, cx| query::prefetch_query(cx, &note_query(route.id)),
    component: NotePage,
}

/// One note.
#[component]
fn NotePage(cx: &mut Cx) -> impl IntoElement {
    let Route { id } = params(cx);
    Suspense::new(move |cx| {
        let note = query::use_suspense_query(cx, note_query(id))?;
        Ok(H2::new(note.title.clone()))
    })
    .fallback(Skeleton::new("note").h(px(80.)))
}
