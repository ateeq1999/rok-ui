#![doc = include_str!("../docs/guide/forms.md")]

mod fields;
#[cfg(feature = "form-garde")]
mod garde;
mod path;
#[cfg(feature = "http")]
mod server;
mod state;
mod validators;

#[cfg(feature = "combobox")]
pub use fields::ComboboxField;
#[cfg(feature = "date-picker")]
pub use fields::DatePickerField;
#[cfg(feature = "input-otp")]
pub use fields::InputOtpField;
#[cfg(feature = "radio-group")]
pub use fields::RadioGroupField;
#[cfg(feature = "select")]
pub use fields::SelectField;
#[cfg(feature = "slider")]
pub use fields::SliderField;
pub use fields::TextareaField;
pub use fields::{BoundInput, CheckboxField, FormErrors, SubmitButton, SwitchField, TextField};
#[cfg(feature = "form-garde")]
pub use garde::GardeSchema;
pub use path::{Field, FieldKey, FormValues, Path};
#[cfg(feature = "http")]
pub use server::{
    to_server_errors, use_api_form, ApiForm, ApiFormOptions, ServerErrorOptions, ServerErrors,
    SOMETHING_WENT_WRONG, TOO_MANY_ATTEMPTS,
};
pub use state::{use_form, ArrayFieldApi, FieldApi, FieldMeta, Form, FormOptions, FormState};
pub use validators::{FormError, FormValidators, Schema, ValidationEvent, Validators};

/// Derive [`FormValues`] and a [`Field`] constant per field (`email` becomes `EMAIL`).
pub use rok_ui_macros::FormValues;
