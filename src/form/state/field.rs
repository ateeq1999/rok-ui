//! [`FieldApi`]: one field of a form, as of a render.

use gpui::{App, Entity, SharedString, Window};

#[cfg(doc)]
use super::Form;
use super::{blur, set_value, FieldMeta, FormInner};
use crate::form::path::{FormValues, Path};

/// One field of a [`Form`], as of this render: its value, state and handlers.
pub struct FieldApi<V, T> {
    pub(crate) form: Entity<FormInner<V>>,
    pub(crate) path: Path<V, T>,
    value: Option<T>,
    meta: FieldMeta,
    submission_attempts: u32,
}

impl<V, T: Clone> Clone for FieldApi<V, T> {
    fn clone(&self) -> Self {
        Self {
            form: self.form.clone(),
            path: self.path.clone(),
            value: self.value.clone(),
            meta: self.meta.clone(),
            submission_attempts: self.submission_attempts,
        }
    }
}

impl<V: FormValues, T: Clone + 'static> FieldApi<V, T> {
    pub(super) fn read(form: &Entity<FormInner<V>>, path: Path<V, T>, cx: &App) -> Self {
        let inner = form.read(cx);
        Self {
            value: path.get(&inner.values).cloned(),
            meta: inner.meta.get(path.key()).cloned().unwrap_or_default(),
            submission_attempts: inner.attempts,
            form: form.clone(),
            path,
        }
    }

    /// The field's key: `email`, `team.0.email`.
    #[must_use]
    pub fn name(&self) -> &SharedString {
        self.path.key()
    }

    /// The value. `None` only for a list item that no longer exists.
    #[must_use]
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// The interaction and validation state.
    #[must_use]
    pub fn meta(&self) -> &FieldMeta {
        &self.meta
    }

    /// Every error.
    #[must_use]
    pub fn errors(&self) -> Vec<SharedString> {
        self.meta.errors()
    }

    /// Whether to show errors now: after the user touched the field or tried to submit.
    #[must_use]
    pub fn should_show_errors(&self) -> bool {
        (self.meta.is_touched || self.submission_attempts > 0) && !self.meta.is_valid()
    }

    /// Set the value (what a bound control calls on input).
    pub fn handle_change(&self, cx: &mut App, value: T) {
        set_value(&self.form, cx, &self.path, value);
    }

    /// Mark the field blurred and run blur validators.
    pub fn handle_blur(&self, cx: &mut App) {
        blur(&self.form, cx, self.path.key());
    }

    /// A handler for controls whose `on_change` passes the new value (`Checkbox`, `Switch`,
    /// `Slider`).
    pub fn change_handler(&self) -> impl Fn(&T, &mut Window, &mut App) + 'static {
        let (form, path) = (self.form.clone(), self.path.clone());
        move |value, _, cx| set_value(&form, cx, &path, value.clone())
    }
}
