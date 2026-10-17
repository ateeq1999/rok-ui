//! The submit button and the form-wide errors.

use gpui::{prelude::*, App, SharedString, Window};

use super::element_id;
use crate::components::{Button, FieldError};
use crate::form::{path::FormValues, state::Form};

/// The form's submit button: disabled while the form cannot submit, spinning while it
/// submits.
#[must_use = "components do nothing unless rendered as a child"]
pub struct SubmitButton<V> {
    form: Form<V>,
    label: SharedString,
}

impl<V: FormValues> SubmitButton<V> {
    /// A submit button for `form` labeled `label`.
    pub fn new(form: &Form<V>, label: impl Into<SharedString>) -> Self {
        Self {
            form: form.clone(),
            label: label.into(),
        }
    }
}

impl<V: FormValues> RenderOnce for SubmitButton<V> {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let state = self.form.state();
        Button::new(element_id(&self.form.entity, "submit", ""))
            .label(self.label)
            .loading(state.is_submitting)
            .disabled(!state.can_submit && state.submission_attempts > 0)
            .on_click(self.form.submit_handler())
    }
}

impl<V: FormValues> IntoElement for SubmitButton<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// Errors about the whole form (from form validators and the submit handler), one per line.
#[must_use = "components do nothing unless rendered as a child"]
pub struct FormErrors {
    errors: Vec<SharedString>,
}

impl FormErrors {
    /// The form-level errors of `form`.
    pub fn new<V: FormValues>(form: &Form<V>) -> Self {
        Self {
            errors: form.state().errors.clone(),
        }
    }
}

impl RenderOnce for FormErrors {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        gpui::div()
            .flex()
            .flex_col()
            .gap_1()
            .children(self.errors.into_iter().map(FieldError::new))
    }
}

impl IntoElement for FormErrors {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
