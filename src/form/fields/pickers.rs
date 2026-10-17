//! Picker controls bound to form fields: select, radio group, slider, combobox, date
//! picker and one-time code. Picking a value counts as leaving the field.

use gpui::{prelude::*, App, SharedString, Window};

use super::{anchored, element_id, errors};
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
use crate::components::{Field, FieldDescription, FieldLabel};
#[cfg(feature = "input-otp")]
use crate::components::{InputOtp, OtpPattern};
use crate::form::{
    path::FormValues,
    state::{blur, set_value, FieldApi},
};

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
