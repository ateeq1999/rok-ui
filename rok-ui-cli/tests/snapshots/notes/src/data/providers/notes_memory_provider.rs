//! `NotesMemoryProvider`: where `notes` data comes from, kept in memory.

use crate::data::models::note::Note;
use crate::data::models::note::NoteId;
use std::sync::{Mutex, PoisonError};

/// Notes, kept in memory: the starting point, and what tests use.
#[derive(Debug, Default)]
pub struct NotesMemoryProvider {
    items: Mutex<Vec<Note>>,
    next_id: Mutex<u64>,
}

impl NotesMemoryProvider {
    /// An empty source.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every item.
    #[must_use]
    pub fn all(&self) -> Vec<Note> {
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The item with `id`.
    #[must_use]
    pub fn find(&self, id: &NoteId) -> Option<Note> {
        self.all().into_iter().find(|item| &item.id == id)
    }

    /// Store `item` under a new id and return it.
    pub fn insert(&self, mut item: Note) -> Note {
        let mut next_id = self.next_id.lock().unwrap_or_else(PoisonError::into_inner);
        *next_id += 1;
        let next = *next_id;
        item.id = next;
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(item.clone());
        item
    }

    /// Change the item with `id`, and return it changed.
    pub fn update(&self, id: &NoteId, change: impl FnOnce(&mut Note)) -> Option<Note> {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        let item = items.iter_mut().find(|item| &item.id == id)?;
        change(item);
        Some(item.clone())
    }

    /// Forget the item with `id`, and return it.
    pub fn remove(&self, id: &NoteId) -> Option<Note> {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        let index = items.iter().position(|item| &item.id == id)?;
        Some(items.remove(index))
    }
}
