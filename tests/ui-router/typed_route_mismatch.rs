use rok_ui::typed_route;

typed_route! {
    pub struct NoteRoute = "/notes/:id" { pub note_id: u64 }
}

fn main() {}
