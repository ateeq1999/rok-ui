use rok_ui::prelude::*;
use rok_ui::router::file_route;

file_route! {
    component: PickANote,
}

/// Shown at `/notes` before a note is picked.
#[component]
fn PickANote() -> impl IntoElement {
    Empty::new().title("Pick a note")
}
