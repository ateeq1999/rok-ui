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
    pub id: u64,
    /// The title.
    pub title: String,
    /// The text.
    pub body: String,
}

fn all() -> Vec<Note> {
    [
        (1, "Groceries", "Milk, eggs, dates and coffee."),
        (
            2,
            "Release checklist",
            "Bump versions, update the changelog, tag and publish.",
        ),
        (3, "Ideas", "File-based routes for desktop apps."),
    ]
    .into_iter()
    .map(|(id, title, body)| Note {
        id,
        title: title.into(),
        body: body.into(),
    })
    .collect()
}

/// Every note.
#[must_use]
pub fn notes_query() -> QueryOptions<Vec<Note>> {
    QueryOptions::new(query_key!["notes", "list"], |_| async {
        Ok::<_, QueryError>(all())
    })
    .stale_time(Duration::from_secs(30))
}

/// One note, or an error when there is none with `id`.
#[must_use]
pub fn note_query(id: u64) -> QueryOptions<Note> {
    QueryOptions::new(query_key!["notes", id], move |_| async move {
        all()
            .into_iter()
            .find(|note| note.id == id)
            .ok_or_else(|| QueryError::msg(format!("There is no note {id}.")))
    })
    .stale_time(Duration::from_secs(30))
}
