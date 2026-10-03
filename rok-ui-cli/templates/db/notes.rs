//! Notes: the model and its queries, stored in PostgreSQL.

use rok_ui::{
    db::{self, rok_db::prelude::*},
    query::{MutationOptions, QueryOptions},
    query_key,
};

/// A note.
#[derive(Clone, Debug, Model)]
#[rok(crate = "rok_ui::db::rok_db", table = "notes")]
pub struct Note {
    /// Its id, used in `/notes/:id`.
    #[rok(primary_key, generated)]
    pub id: i64,
    /// The title.
    pub title: String,
}

/// Every note.
#[must_use]
pub fn notes_query() -> QueryOptions<Vec<Note>> {
    db::db_query(query_key!["notes"], |db| async move {
        Note::query().order_by(Note::ID.asc()).all(&db).await
    })
}

/// One note.
#[must_use]
pub fn note_query(id: i64) -> QueryOptions<Note> {
    db::db_query(query_key!["notes", id], move |db| async move {
        Note::filter(Note::ID.eq(id)).one(&db).await
    })
}

/// Add a note; refreshes every notes query.
#[must_use]
pub fn create_note() -> MutationOptions<String, Note, db::DbError> {
    db::db_mutation(|db, title: String| async move { Note { id: 0, title }.insert(&db).await })
        .invalidates(query_key!["notes"])
}
