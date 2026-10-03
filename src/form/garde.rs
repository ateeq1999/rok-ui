//! Schema validation with [garde](https://docs.rs/garde).

use super::{validators::Schema, FormError};

/// Validates form values with their `garde::Validate` derive, so one set of rules covers the
/// form and the procedure that receives it. Errors land on fields by path (`team[0].email` is
/// the field `team.0.email`).
///
/// ```
/// use rok_ui::form::{FormValidators, FormValues, GardeSchema, Schema};
///
/// #[derive(FormValues, garde::Validate, Clone, Default)]
/// struct NewNote {
///     #[garde(length(min = 1, max = 120))]
///     title: String,
///     #[garde(skip)]
///     body: String,
/// }
///
/// let errors = GardeSchema.validate(&NewNote::default()).expect("an empty title is invalid");
/// # let _ = errors;
/// let validators = FormValidators::<NewNote>::new().schema(GardeSchema);
/// # let _ = validators;
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct GardeSchema;

impl<V> Schema<V> for GardeSchema
where
    V: garde::Validate,
    V::Context: Default,
{
    fn validate(&self, values: &V) -> Option<FormError> {
        let report = values.validate().err()?;
        let mut errors = FormError::default();
        for (path, error) in report.iter() {
            let key = field_key(&path.to_string());
            let message = error.to_string();
            errors = if key.is_empty() {
                errors.and_form(message)
            } else {
                errors.merge(FormError::at_key(key, message))
            };
        }
        Some(errors)
    }
}

/// `team[0].email` -> `team.0.email`.
fn field_key(path: &str) -> String {
    path.replace('[', ".")
        .replace(']', "")
        .trim_start_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn garde_paths_become_field_keys() {
        assert_eq!(super::field_key("team[0].email"), "team.0.email");
        assert_eq!(super::field_key("[1]"), "1");
        assert_eq!(super::field_key("title"), "title");
    }
}
