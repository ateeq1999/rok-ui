//! Text controls bound to form fields: `BoundInput`, `TextField`, `TextareaField`.

use std::{cell::RefCell, rc::Rc};

use gpui::{prelude::*, App, Entity, SharedString, Window};

use super::{anchored, errors};
use crate::form::{
    path::FormValues,
    state::{blur, set_value, submit, Binding, FieldApi},
};
use crate::{
    components::{
        Field, FieldDescription, FieldLabel, Input, InputEvent, InputState, Spinner, Textarea,
    },
    IconName,
};

/// A text [`Input`] bound to a `String` field: typing changes the value, leaving the field
/// runs blur validators, Enter submits the form, and the first invalid field gets focus when
/// a submit fails.
#[must_use = "components do nothing unless rendered as a child"]
pub struct BoundInput<V> {
    field: FieldApi<V, String>,
    placeholder: SharedString,
    masked: bool,
    leading_icon: Option<IconName>,
    disabled: bool,
    multiline: bool,
}

impl<V: FormValues> BoundInput<V> {
    /// An input for `field`.
    pub fn new(field: &FieldApi<V, String>) -> Self {
        Self {
            field: field.clone(),
            placeholder: SharedString::default(),
            masked: false,
            leading_icon: None,
            disabled: false,
            multiline: false,
        }
    }

    /// Text shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Hide the text, for passwords.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// An icon before the text.
    pub fn leading_icon(mut self, icon: IconName) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    /// Disable it: it ignores input and renders muted.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// The input state bound to `field`, created on first use.
fn bind<V: FormValues>(
    field: &FieldApi<V, String>,
    placeholder: &SharedString,
    masked: bool,
    multiline: bool,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    let form = field.form.clone();
    let key = field.name().clone();
    let existing = form
        .read(cx)
        .bindings
        .get(&key)
        .map(|binding| (binding.input.clone(), binding.path.clone()));
    if let Some((input, path)) = existing {
        *path.borrow_mut() = field.path.clone();
        return input;
    }
    let text = field.value().cloned().unwrap_or_default();
    let placeholder = placeholder.clone();
    let input = cx.new(|cx| {
        InputState::new(cx)
            .with_text(text)
            .with_placeholder(placeholder)
            .with_masked_text(masked)
            .with_multiline(multiline)
    });
    let path = Rc::new(RefCell::new(field.path.clone()));
    let focus_handle = input.read(cx).focus_handle_ref().clone();
    // The form owns these subscriptions, so they hold it weakly: a strong handle would be a
    // cycle that keeps every form alive after its page is gone.
    let changes = {
        let (form, path) = (form.downgrade(), path.clone());
        window.subscribe(&input, cx, move |_, event: &InputEvent, window, cx| {
            let Some(form) = form.upgrade() else {
                return;
            };
            let path = path.borrow().clone();
            match event {
                InputEvent::Changed(text) => {
                    let current = path.get(&form.read(cx).values).cloned();
                    if current.as_deref() != Some(text.as_ref()) {
                        set_value(&form, cx, &path, text.to_string());
                    }
                }
                InputEvent::Submitted(_) => submit(&form, Some(window), cx),
            }
        })
    };
    let blurs = {
        let (form, path) = (form.downgrade(), path.clone());
        window.on_focus_out(&focus_handle, cx, move |_, _, cx| {
            let Some(form) = form.upgrade() else {
                return;
            };
            let key = path.borrow().key().clone();
            blur(&form, cx, &key);
        })
    };
    form.update(cx, |inner, _| {
        inner.focus.insert(key.clone(), focus_handle);
        inner.bindings.insert(
            key,
            Binding {
                input: input.clone(),
                path,
                _subscriptions: vec![changes, blurs],
            },
        );
    });
    input
}

impl<V: FormValues> RenderOnce for BoundInput<V> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = bind(
            &self.field,
            &self.placeholder,
            self.masked,
            self.multiline,
            window,
            cx,
        );
        // Values set from code (reset, set_value) flow into the input.
        let value = self.field.value().cloned().unwrap_or_default();
        if input.read(cx).text().as_ref() != value {
            input.update(cx, |state, cx| state.set_text(value, cx));
        }
        if self.multiline {
            return Textarea::new(&input)
                .invalid(self.field.should_show_errors())
                .disabled(self.disabled)
                .into_any_element();
        }
        let mut element = Input::new(&input)
            .invalid(self.field.should_show_errors())
            .disabled(self.disabled);
        if let Some(icon) = self.leading_icon {
            element = element.leading_icon(icon);
        }
        element.into_any_element()
    }
}

impl<V: FormValues> IntoElement for BoundInput<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A labeled text field with its errors and a spinner while async validation runs.
///
/// ```no_run
/// # use rok_ui::{prelude::*, form::{self, FormOptions, FormValues, TextField}};
/// #[derive(FormValues, Clone, Default)]
/// struct Profile { name: String }
///
/// #[component]
/// fn ProfileForm(cx: &mut Cx) -> impl IntoElement {
///     let form = form::use_form(cx, FormOptions::new(Profile::default()));
///     let name = form.field(cx, Profile::NAME);
///     TextField::new(&name, "Name").placeholder("Ada Lovelace")
/// }
/// ```
#[must_use = "components do nothing unless rendered as a child"]
pub struct TextField<V> {
    input: BoundInput<V>,
    label: SharedString,
    description: Option<SharedString>,
}

impl<V: FormValues> TextField<V> {
    /// A field for `field` labeled `label`.
    pub fn new(field: &FieldApi<V, String>, label: impl Into<SharedString>) -> Self {
        Self {
            input: BoundInput::new(field),
            label: label.into(),
            description: None,
        }
    }

    /// Text shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.input = self.input.placeholder(placeholder);
        self
    }

    /// Hide the text, for passwords.
    pub fn password(mut self) -> Self {
        self.input = self.input.masked(true);
        self
    }

    /// An icon before the text.
    pub fn leading_icon(mut self, icon: IconName) -> Self {
        self.input = self.input.leading_icon(icon);
        self
    }

    /// Help text below the input.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl<V: FormValues> RenderOnce for TextField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let field = self.input.field.clone();
        let invalid = field.should_show_errors();
        let element = Field::new()
            .invalid(invalid)
            .child(FieldLabel::new(self.label))
            .child(self.input)
            .children(self.description.map(FieldDescription::new))
            .children(errors(&field))
            .children(field.meta().is_validating.then(Spinner::new));
        anchored(&field, element, cx)
    }
}

impl<V: FormValues> IntoElement for TextField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A labeled multi-line text field bound to a `String` field. Enter adds a line and
/// Ctrl/Cmd-Enter submits the form; otherwise it behaves like [`TextField`].
#[must_use = "components do nothing unless rendered as a child"]
pub struct TextareaField<V> {
    field: TextField<V>,
}

impl<V: FormValues> TextareaField<V> {
    /// A multi-line field for `field` labeled `label`.
    pub fn new(field: &FieldApi<V, String>, label: impl Into<SharedString>) -> Self {
        let mut text = TextField::new(field, label);
        text.input.multiline = true;
        Self { field: text }
    }

    /// Text shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.field = self.field.placeholder(placeholder);
        self
    }

    /// Help text below the text area.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.field = self.field.description(description);
        self
    }
}

impl<V: FormValues> RenderOnce for TextareaField<V> {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.field
    }
}

impl<V: FormValues> IntoElement for TextareaField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
