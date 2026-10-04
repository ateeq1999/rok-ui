//! Controls bound to form fields.

use std::{cell::RefCell, rc::Rc};

use gpui::{prelude::*, AnyElement, App, ElementId, Entity, SharedString, Window};

use super::{
    path::FormValues,
    state::{blur, set_value, submit, Binding, FieldApi, Form, FormInner},
};
#[cfg(feature = "combobox")]
use crate::components::Combobox;
#[cfg(feature = "radio-group")]
use crate::components::RadioGroup;
#[cfg(feature = "select")]
use crate::components::Select;
#[cfg(feature = "slider")]
use crate::components::Slider;
#[cfg(feature = "date-picker")]
use crate::components::{CalendarDate, DatePicker};
#[cfg(feature = "input-otp")]
use crate::components::{InputOtp, OtpPattern};
use crate::{
    components::{
        Button, Checkbox, Field, FieldDescription, FieldError, FieldLabel, Input, InputEvent,
        InputState, Spinner, Switch, Textarea,
    },
    IconName,
};

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
    let changes = {
        let (form, path) = (form.clone(), path.clone());
        window.subscribe(&input, cx, move |_, event: &InputEvent, window, cx| {
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
        let (form, path) = (form.clone(), path.clone());
        window.on_focus_out(&focus_handle, cx, move |_, _, cx| {
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

#[cfg(any(
    feature = "select",
    feature = "radio-group",
    feature = "slider",
    feature = "combobox",
    feature = "date-picker",
    feature = "input-otp"
))]
/// Set a field from a picker and count the pick as leaving the field, so blur validators run
/// and errors show (pickers have no text to leave half-typed).
fn pick<V: FormValues, T: Clone + 'static>(
    field: &FieldApi<V, T>,
) -> impl Fn(T, &mut App) + 'static {
    let (form, path) = (field.form.clone(), field.path.clone());
    move |value, cx| {
        set_value(&form, cx, &path, value);
        blur(&form, cx, path.key());
    }
}

#[cfg(feature = "select")]
/// A labeled [`Select`] bound to a `String` field holding the chosen option's value, with
/// its errors. Choosing an option counts as leaving the field.
#[must_use = "components do nothing unless rendered as a child"]
pub struct SelectField<V> {
    field: FieldApi<V, String>,
    label: SharedString,
    options: Vec<(SharedString, SharedString)>,
    placeholder: Option<SharedString>,
    description: Option<SharedString>,
}

#[cfg(feature = "select")]
impl<V: FormValues> SelectField<V> {
    /// A select for `field` labeled `label`, with no options yet.
    pub fn new(field: &FieldApi<V, String>, label: impl Into<SharedString>) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
            options: Vec::new(),
            placeholder: None,
            description: None,
        }
    }

    /// Add an option: the value stored in the field and the label shown.
    pub fn option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.options.push((value.into(), label.into()));
        self
    }

    /// Text shown while nothing is chosen (the field is empty).
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Help text below the select.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[cfg(feature = "select")]
impl<V: FormValues> RenderOnce for SelectField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let invalid = self.field.should_show_errors();
        let value = self
            .field
            .value()
            .filter(|value| !value.is_empty())
            .map(|value| SharedString::from(value.clone()));
        let pick = pick(&self.field);
        let mut select = Select::new(element_id(&self.field.form, "select", self.field.name()))
            .value(value)
            .invalid(invalid)
            .on_change(move |value, _, cx| pick(value.to_string(), cx));
        if let Some(placeholder) = self.placeholder {
            select = select.placeholder(placeholder);
        }
        for (value, label) in self.options {
            select = select.option(value, label);
        }
        let element = Field::new()
            .invalid(invalid)
            .child(FieldLabel::new(self.label))
            .child(select)
            .children(self.description.map(FieldDescription::new))
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

#[cfg(feature = "select")]
impl<V: FormValues> IntoElement for SelectField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(feature = "radio-group")]
/// A labeled [`RadioGroup`] bound to a `String` field holding the chosen option's value, with
/// its errors. Choosing an option counts as leaving the field.
#[must_use = "components do nothing unless rendered as a child"]
pub struct RadioGroupField<V> {
    field: FieldApi<V, String>,
    label: SharedString,
    options: Vec<(SharedString, SharedString)>,
    horizontal: bool,
}

#[cfg(feature = "radio-group")]
impl<V: FormValues> RadioGroupField<V> {
    /// A radio group for `field` labeled `label`, with no options yet.
    pub fn new(field: &FieldApi<V, String>, label: impl Into<SharedString>) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
            options: Vec::new(),
            horizontal: false,
        }
    }

    /// Add an option: the value stored in the field and the label shown.
    pub fn option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.options.push((value.into(), label.into()));
        self
    }

    /// Lay the options out in a row instead of a column.
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }
}

#[cfg(feature = "radio-group")]
impl<V: FormValues> RenderOnce for RadioGroupField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let value = self
            .field
            .value()
            .filter(|value| !value.is_empty())
            .map(|value| SharedString::from(value.clone()));
        let pick = pick(&self.field);
        let mut group = RadioGroup::new(element_id(&self.field.form, "radio", self.field.name()))
            .value(value)
            .horizontal(self.horizontal)
            .on_change(move |value, _, cx| pick(value.to_string(), cx));
        for (value, label) in self.options {
            group = group.option(value, label);
        }
        let element = Field::new()
            .invalid(self.field.should_show_errors())
            .child(FieldLabel::new(self.label))
            .child(group)
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

#[cfg(feature = "radio-group")]
impl<V: FormValues> IntoElement for RadioGroupField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(feature = "slider")]
/// A labeled [`Slider`] bound to an `f32` field, with its errors. Releasing the thumb at a new
/// value counts as leaving the field.
#[must_use = "components do nothing unless rendered as a child"]
pub struct SliderField<V> {
    field: FieldApi<V, f32>,
    label: SharedString,
    range: (f32, f32),
    step: Option<f32>,
}

#[cfg(feature = "slider")]
impl<V: FormValues> SliderField<V> {
    /// A slider for `field` labeled `label`, from 0 to 100.
    pub fn new(field: &FieldApi<V, f32>, label: impl Into<SharedString>) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
            range: (0., 100.),
            step: None,
        }
    }

    /// The lowest and highest values (default 0 to 100).
    pub fn range(mut self, start: f32, end: f32) -> Self {
        self.range = (start, end);
        self
    }

    /// Snap to multiples of `step` (default: the slider's own step).
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }
}

#[cfg(feature = "slider")]
impl<V: FormValues> RenderOnce for SliderField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let pick = pick(&self.field);
        let mut slider = Slider::new(element_id(&self.field.form, "slider", self.field.name()))
            .range(self.range.0, self.range.1)
            .value(self.field.value().copied().unwrap_or(self.range.0))
            .on_change(move |values, _, cx| {
                if let Some(value) = values.first() {
                    pick(*value, cx);
                }
            });
        if let Some(step) = self.step {
            slider = slider.step(step);
        }
        let element = Field::new()
            .invalid(self.field.should_show_errors())
            .child(FieldLabel::new(self.label))
            .child(slider)
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

#[cfg(feature = "slider")]
impl<V: FormValues> IntoElement for SliderField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A labeled searchable [`Combobox`] bound to a `String` field holding the chosen option's
/// value (empty when nothing is chosen), with its errors. Picking the chosen option again
/// clears it; either counts as leaving the field.
#[cfg(feature = "combobox")]
#[must_use = "components do nothing unless rendered as a child"]
pub struct ComboboxField<V> {
    field: FieldApi<V, String>,
    label: SharedString,
    options: Vec<(SharedString, SharedString)>,
    placeholder: Option<SharedString>,
    description: Option<SharedString>,
}

#[cfg(feature = "combobox")]
impl<V: FormValues> ComboboxField<V> {
    /// A combobox for `field` labeled `label`, with no options yet.
    pub fn new(field: &FieldApi<V, String>, label: impl Into<SharedString>) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
            options: Vec::new(),
            placeholder: None,
            description: None,
        }
    }

    /// Add an option: the value stored in the field and the label shown.
    pub fn option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.options.push((value.into(), label.into()));
        self
    }

    /// Text shown while nothing is chosen.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Help text below the combobox.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[cfg(feature = "combobox")]
impl<V: FormValues> RenderOnce for ComboboxField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let value = self
            .field
            .value()
            .filter(|value| !value.is_empty())
            .map(|value| SharedString::from(value.clone()));
        let pick = pick(&self.field);
        let mut combobox =
            Combobox::new(element_id(&self.field.form, "combobox", self.field.name()))
                .value(value)
                .on_change(move |value, _, cx| {
                    pick(
                        value.as_ref().map(ToString::to_string).unwrap_or_default(),
                        cx,
                    );
                });
        if let Some(placeholder) = self.placeholder {
            combobox = combobox.placeholder(placeholder);
        }
        for (value, label) in self.options {
            combobox = combobox.option(value, label);
        }
        let element = Field::new()
            .invalid(self.field.should_show_errors())
            .child(FieldLabel::new(self.label))
            .child(combobox)
            .children(self.description.map(FieldDescription::new))
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

#[cfg(feature = "combobox")]
impl<V: FormValues> IntoElement for ComboboxField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A labeled [`DatePicker`] bound to an `Option<CalendarDate>` field, with its errors.
/// Picking a date counts as leaving the field.
#[cfg(feature = "date-picker")]
#[must_use = "components do nothing unless rendered as a child"]
pub struct DatePickerField<V> {
    field: FieldApi<V, Option<CalendarDate>>,
    label: SharedString,
    placeholder: Option<SharedString>,
    description: Option<SharedString>,
}

#[cfg(feature = "date-picker")]
impl<V: FormValues> DatePickerField<V> {
    /// A date picker for `field` labeled `label`.
    pub fn new(field: &FieldApi<V, Option<CalendarDate>>, label: impl Into<SharedString>) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
            placeholder: None,
            description: None,
        }
    }

    /// Text shown while no date is picked.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Help text below the picker.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[cfg(feature = "date-picker")]
impl<V: FormValues> RenderOnce for DatePickerField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let pick = pick(&self.field);
        let mut picker = DatePicker::new(element_id(&self.field.form, "date", self.field.name()))
            .date(self.field.value().copied().flatten())
            .on_change(move |date, _, cx| pick(Some(*date), cx));
        if let Some(placeholder) = self.placeholder {
            picker = picker.placeholder(placeholder);
        }
        let element = Field::new()
            .invalid(self.field.should_show_errors())
            .child(FieldLabel::new(self.label))
            .child(picker)
            .children(self.description.map(FieldDescription::new))
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

#[cfg(feature = "date-picker")]
impl<V: FormValues> IntoElement for DatePickerField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A labeled [`InputOtp`] bound to a `String` field, with its errors. Every edit changes the
/// value; filling the last box counts as leaving the field.
#[cfg(feature = "input-otp")]
#[must_use = "components do nothing unless rendered as a child"]
pub struct InputOtpField<V> {
    field: FieldApi<V, String>,
    label: SharedString,
    length: usize,
    groups: Option<Vec<usize>>,
    pattern: Option<OtpPattern>,
}

#[cfg(feature = "input-otp")]
impl<V: FormValues> InputOtpField<V> {
    /// A code input with `length` boxes for `field`, labeled `label`.
    pub fn new(field: &FieldApi<V, String>, label: impl Into<SharedString>, length: usize) -> Self {
        Self {
            field: field.clone(),
            label: label.into(),
            length,
            groups: None,
            pattern: None,
        }
    }

    /// Split the boxes into groups of these sizes.
    pub fn groups(mut self, groups: impl IntoIterator<Item = usize>) -> Self {
        self.groups = Some(groups.into_iter().collect());
        self
    }

    /// Which characters the boxes accept.
    pub fn pattern(mut self, pattern: OtpPattern) -> Self {
        self.pattern = Some(pattern);
        self
    }
}

#[cfg(feature = "input-otp")]
impl<V: FormValues> RenderOnce for InputOtpField<V> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let invalid = self.field.should_show_errors();
        let (form, path) = (self.field.form.clone(), self.field.path.clone());
        let complete = pick(&self.field);
        let mut input = InputOtp::new(
            element_id(&self.field.form, "otp", self.field.name()),
            self.length,
        )
        .value(self.field.value().cloned().unwrap_or_default())
        .invalid(invalid)
        .on_change(move |code, _, cx| set_value(&form, cx, &path, code.to_string()))
        .on_complete(move |code, _, cx| complete(code.to_string(), cx));
        if let Some(groups) = self.groups {
            input = input.groups(groups);
        }
        if let Some(pattern) = self.pattern {
            input = input.pattern(pattern);
        }
        let element = Field::new()
            .invalid(invalid)
            .child(FieldLabel::new(self.label))
            .child(input)
            .children(errors(&self.field));
        anchored(&self.field, element, cx)
    }
}

#[cfg(feature = "input-otp")]
impl<V: FormValues> IntoElement for InputOtpField<V> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

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
