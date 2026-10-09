//! `NotesRepository`: what the `notes` business logic reads and writes.

use crate::data::error::DataError;
use crate::data::models::note::Note;
use crate::data::models::note::NoteId;
use crate::data::providers::notes_memory_provider::NotesMemoryProvider;
use rok_ui::bloc::BoxFuture;

/// What the `notes` business logic reads and writes. A trait, so tests and other data sources can stand in.
pub trait NotesRepository: Send + Sync {
    /// List.
    fn list(&self) -> BoxFuture<'_, Result<Vec<Note>, DataError>>;
    /// Add.
    fn add(&self, title: String, body: String) -> BoxFuture<'_, Result<Note, DataError>>;
    /// Delete.
    fn delete(&self, id: NoteId) -> BoxFuture<'_, Result<(), DataError>>;
}

/// [`NotesRepository`] over a [`NotesMemoryProvider`].
#[derive(Debug)]
pub struct NotesRepositoryImpl {
    provider: NotesMemoryProvider,
}

impl NotesRepositoryImpl {
    /// A repository reading and writing through `provider`.
    #[must_use]
    pub fn new(provider: NotesMemoryProvider) -> Self {
        Self { provider }
    }

    /// The data source, for the methods you add.
    #[must_use]
    pub fn provider(&self) -> &NotesMemoryProvider {
        &self.provider
    }
}

impl NotesRepository for NotesRepositoryImpl {
    fn list(&self) -> BoxFuture<'_, Result<Vec<Note>, DataError>> {
        Box::pin(async move { Ok(self.provider.all()) })
    }

    fn add(&self, title: String, body: String) -> BoxFuture<'_, Result<Note, DataError>> {
        Box::pin(async move {
            Ok(self.provider.insert(Note {
                title,
                body,
                ..Note::default()
            }))
        })
    }

    fn delete(&self, id: NoteId) -> BoxFuture<'_, Result<(), DataError>> {
        Box::pin(async move {
            self.provider
                .remove(&id)
                .map(|_| ())
                .ok_or(DataError::NotFound)
        })
    }
}
