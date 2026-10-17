//! Checkbox and switch controls bound to `bool` fields.

use gpui::{prelude::*, App, SharedString, Window};

use super::{anchored, element_id, errors};
use crate::components::{Checkbox, Field, Switch};
use crate::form::{path::FormValues, state::FieldApi};

/// A checkbox bound to a `bool` field, with its errors.
#[must_use = "components do nothing unless rendered as a child"]
pub struct CheckboxField<V> {
    field: FieldApi<V, bool>,
    label: SharedString,
}

impl<V: FormValues> CheckboxField<V> {
    /// A checkbox for `field` labeled `label`.
    pub fn new(field: &FieldApi<V, bool>, label: impl Into<SharedString>) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
        }
    }
}

impl<V: FormValues> RenderOnce for CheckboxField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = element_id(&self.field.form, "checkbox", self.field.name());
        let element = Field::new()
            .invalid(self.field.should_show_errors())
            .child(
                Checkbox::new(id)
                    .checked(self.field.value().copied().unwrap_or_default())
                    .label(self.label)
                    .on_change(self.field.change_handler()),
            )
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

impl<V: FormValues> IntoElement for CheckboxField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A switch bound to a `bool` field, with its errors.
#[must_use = "components do nothing unless rendered as a child"]
pub struct SwitchField<V> {
    field: FieldApi<V, bool>,
    label: SharedString,
}

impl<V: FormValues> SwitchField<V> {
    /// A switch for `field` labeled `label`.
    pub fn new(field: &FieldApi<V, bool>, label: impl Into<SharedString>) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
        }
    }
}

impl<V: FormValues> RenderOnce for SwitchField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = element_id(&self.field.form, "switch", self.field.name());
        let element = Field::new()
            .invalid(self.field.should_show_errors())
            .child(
                Switch::new(id)
                    .checked(self.field.value().copied().unwrap_or_default())
                    .label(self.label)
                    .on_change(self.field.change_handler()),
            )
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

impl<V: FormValues> IntoElement for SwitchField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
