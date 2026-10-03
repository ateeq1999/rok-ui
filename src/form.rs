#![doc = include_str!("../docs/guide/forms.md")]

mod fields;
#[cfg(feature = "form-garde")]
mod garde;
mod path;
mod state;
mod validators;

pub use fields::{BoundInput, CheckboxField, FormErrors, SubmitButton, SwitchField, TextField};
#[cfg(feature = "form-garde")]
pub use garde::GardeSchema;
pub use path::{Field, FieldKey, FormValues, Path};
pub use state::{use_form, ArrayFieldApi, FieldApi, FieldMeta, Form, FormOptions, FormState};
pub use validators::{FormError, FormValidators, Schema, ValidationEvent, Validators};

/// Derive [`FormValues`] and a [`Field`] constant per field (`email` becomes `EMAIL`).
pub use rok_ui_macros::FormValues;
