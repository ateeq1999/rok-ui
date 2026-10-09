//! The `Note` model.

/// Note.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    /// `id`.
    pub id: NoteId,
    /// `title`.
    pub title: String,
    /// `body`.
    pub body: String,
}

/// A `Note`'s id.
pub type NoteId = u64;
