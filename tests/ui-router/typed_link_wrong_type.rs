use rok_ui::{prelude::*, typed_route};

typed_route! {
    pub struct NoteRoute = "/notes/:id" { pub id: u64 }
}

fn main() {
    let _link = Link::to(&NoteRoute { id: "three" });
}
