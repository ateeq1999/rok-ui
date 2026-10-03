//! Notes: the model and its queries.

use std::time::Duration;

use rok_ui::{
    query::{QueryError, QueryOptions},
    query_key,
};

/// A note.
#[derive(Clone, Debug)]
pub struct Note {
    /// Its id, used in `/notes/:id`.
    pub id: i64,
    /// The title.
    pub title: String,
}

fn all() -> Vec<Note> {
    vec![
        Note { id: 1, title: "Welcome".into() },
        Note { id: 2, title: "File-based routes".into() },
    ]
}

/// Every note.
#[must_use]
pub fn notes_query() -> QueryOptions<Vec<Note>> {
    QueryOptions::new(query_key!["notes"], |_| async { Ok::<_, QueryError>(all()) })
        .stale_time(Duration::from_secs(30))
}

/// One note.
#[must_use]
pub fn note_query(id: i64) -> QueryOptions<Note> {
    QueryOptions::new(query_key!["notes", id], move |_| async move {
        all()
            .into_iter()
            .find(|note| note.id == id)
            .ok_or_else(|| QueryError::msg(format!("There is no note {id}.")))
    })
}
