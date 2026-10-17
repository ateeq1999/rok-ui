//! Draft persistence (`FormOptions::persist_draft`, feature `persist`).

#[cfg(feature = "persist")]
use std::rc::Rc;

#[cfg(feature = "persist")]
use gpui::{App, Entity, Subscription};

use super::FormInner;
#[cfg(feature = "persist")]
use super::FormValues;

/// How [`FormOptions::persist_draft`] loads and saves a form's values.
#[cfg(feature = "persist")]
pub(super) struct Draft<V> {
    pub(super) options: crate::persist::PersistOptions,
    pub(super) load: fn(&crate::persist::DraftFile) -> Option<V>,
    pub(super) save: fn(&crate::persist::DraftFile, &V, &App),
}

/// A form's draft file, the revision saved last, and the observer that saves edits.
#[cfg(feature = "persist")]
pub(super) struct DraftState {
    file: Rc<crate::persist::DraftFile>,
    saved: u64,
    _observer: Subscription,
}

#[cfg(feature = "persist")]
impl<V> FormInner<V> {
    /// The values were submitted or reset: delete the draft until the next edit.
    pub(super) fn discard_draft(&mut self) {
        let revision = self.revision;
        if let Some(draft) = &mut self.draft {
            draft.file.clear();
            draft.saved = revision;
        }
    }
}

#[cfg(not(feature = "persist"))]
impl<V> FormInner<V> {
    #[allow(clippy::unused_self)] // Mirrors the `persist` version.
    pub(super) fn discard_draft(&mut self) {}
}

/// Save the form's values to its draft file after every edit.
#[cfg(feature = "persist")]
pub(super) fn watch_draft<V: FormValues>(
    entity: &Entity<FormInner<V>>,
    cx: &mut App,
    file: Rc<crate::persist::DraftFile>,
    save: fn(&crate::persist::DraftFile, &V, &App),
) {
    let observer = {
        let file = file.clone();
        cx.observe(entity, move |entity, cx| {
            let inner = entity.read(cx);
            let Some(draft) = &inner.draft else {
                return;
            };
            if draft.saved == inner.revision {
                return;
            }
            save(&file, &inner.values, cx);
            entity.update(cx, |inner, _| {
                let revision = inner.revision;
                if let Some(draft) = &mut inner.draft {
                    draft.saved = revision;
                }
            });
        })
    };
    entity.update(cx, |inner, _| {
        inner.draft = Some(DraftState {
            file,
            saved: inner.revision,
            _observer: observer,
        });
    });
}
