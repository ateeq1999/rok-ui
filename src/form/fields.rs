//! Controls bound to form fields.

use gpui::{prelude::*, AnyElement, App, ElementId, Entity};

use super::{
    path::FormValues,
    state::{FieldApi, FormInner},
};
use crate::components::FieldError;

#[cfg(any(
    feature = "select",
    feature = "radio-group",
    feature = "slider",
    feature = "combobox",
    feature = "date-picker",
    feature = "input-otp"
))]
mod pickers;
mod submit;
mod text;
mod toggles;

#[cfg(feature = "combobox")]
pub use pickers::ComboboxField;
#[cfg(feature = "date-picker")]
pub use pickers::DatePickerField;
#[cfg(feature = "input-otp")]
pub use pickers::InputOtpField;
#[cfg(feature = "radio-group")]
pub use pickers::RadioGroupField;
#[cfg(feature = "select")]
pub use pickers::SelectField;
#[cfg(feature = "slider")]
pub use pickers::SliderField;
pub use submit::{FormErrors, SubmitButton};
pub use text::{BoundInput, TextField, TextareaField};
pub use toggles::{CheckboxField, SwitchField};

fn element_id<V: 'static>(form: &Entity<FormInner<V>>, role: &str, key: &str) -> ElementId {
    ElementId::Name(format!("rok-form-{}-{role}:{key}", form.entity_id()).into())
}

/// `element` with its position recorded, so a failed submit can scroll to it, when the form
/// knows its scroll container ([`FormOptions::scroll_handle`](super::FormOptions::scroll_handle)).
fn anchored<V: FormValues, T: Clone + 'static>(
    field: &FieldApi<V, T>,
    element: impl IntoElement,
    cx: &mut App,
) -> AnyElement {
    let Some(handle) = field.form.read(cx).scroll.clone() else {
        return element.into_any_element();
    };
    let key = field.name().clone();
    let anchor = field
        .form
        .update(cx, |inner, _| inner.anchors.entry(key).or_default().clone());
    // A zero-size marker at the field's top-left corner records where it is.
    let marker = gpui::canvas(
        move |bounds, _, _| anchor.set(Some(bounds.origin - handle.offset())),
        |_, (), _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_0();
    gpui::div()
        .relative()
        .child(marker)
        .child(element)
        .into_any_element()
}

fn errors<V: FormValues, T: Clone + 'static>(field: &FieldApi<V, T>) -> Vec<AnyElement> {
    if !field.should_show_errors() {
        return Vec::new();
    }
    field
        .errors()
        .into_iter()
        .map(|error| FieldError::new(error).into_any_element())
        .collect()
}
