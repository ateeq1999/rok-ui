//! Validators: field validators per event, form validators, and their errors.

use std::{future::Future, pin::Pin, rc::Rc, time::Duration};

use gpui::SharedString;

use super::path::FieldKey;

/// When a validator runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValidationEvent {
    /// When the field is first shown.
    Mount,
    /// After every change.
    Change,
    /// When the field loses focus.
    Blur,
    /// When the form is submitted.
    Submit,
    /// Errors returned by the submit handler (a procedure or server).
    Server,
}

/// A future that produces a validation message.
pub(crate) type LocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

type SyncCheck<T> = Rc<dyn Fn(&T) -> Option<SharedString>>;
type AsyncCheck<T> = Rc<dyn Fn(T) -> LocalFuture<Option<SharedString>>>;

/// The validators of one field, per event (TanStack Form's `validators` prop).
///
/// Each returns `None` when the value is valid, or a message. Async validators run after the
/// synchronous one for their event passes, after a debounce, and only the latest run counts.
///
/// ```
/// use std::time::Duration;
/// use rok_ui::form::Validators;
///
/// let email = Validators::new()
///     .on_change(|email: &String| email.is_empty().then_some("Required"))
///     .on_blur(|email: &String| (!email.contains('@')).then_some("Enter a valid email"))
///     .on_change_async_debounce(Duration::from_millis(400), |email: String| async move {
///         (email == "taken@example.com").then_some("This email is already registered")
///     });
/// # let _ = email;
/// ```
pub struct Validators<T> {
    pub(crate) sync: Vec<(ValidationEvent, SyncCheck<T>)>,
    pub(crate) asynchronous: Vec<(ValidationEvent, Duration, AsyncCheck<T>)>,
    pub(crate) listen_to: Vec<SharedString>,
}

impl<T: 'static> Validators<T> {
    /// No validators.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sync: Vec::new(),
            asynchronous: Vec::new(),
            listen_to: Vec::new(),
        }
    }

    fn sync<M: Into<SharedString>>(
        mut self,
        event: ValidationEvent,
        check: impl Fn(&T) -> Option<M> + 'static,
    ) -> Self {
        self.sync
            .push((event, Rc::new(move |value| check(value).map(Into::into))));
        self
    }

    fn asynchronous<M, Fut>(
        mut self,
        event: ValidationEvent,
        debounce: Duration,
        check: impl Fn(T) -> Fut + 'static,
    ) -> Self
    where
        M: Into<SharedString>,
        Fut: Future<Output = Option<M>> + 'static,
    {
        self.asynchronous.push((
            event,
            debounce,
            Rc::new(move |value| {
                let future = check(value);
                Box::pin(async move { future.await.map(Into::into) })
            }),
        ));
        self
    }

    /// Validate when the field is first shown.
    #[must_use]
    pub fn on_mount<M: Into<SharedString>>(
        self,
        check: impl Fn(&T) -> Option<M> + 'static,
    ) -> Self {
        self.sync(ValidationEvent::Mount, check)
    }

    /// Validate after every change.
    #[must_use]
    pub fn on_change<M: Into<SharedString>>(
        self,
        check: impl Fn(&T) -> Option<M> + 'static,
    ) -> Self {
        self.sync(ValidationEvent::Change, check)
    }

    /// Validate when the field loses focus.
    #[must_use]
    pub fn on_blur<M: Into<SharedString>>(self, check: impl Fn(&T) -> Option<M> + 'static) -> Self {
        self.sync(ValidationEvent::Blur, check)
    }

    /// Validate when the form is submitted.
    #[must_use]
    pub fn on_submit<M: Into<SharedString>>(
        self,
        check: impl Fn(&T) -> Option<M> + 'static,
    ) -> Self {
        self.sync(ValidationEvent::Submit, check)
    }

    /// Validate asynchronously after changes, once the value has been still for `debounce`.
    #[must_use]
    pub fn on_change_async_debounce<M, Fut>(
        self,
        debounce: Duration,
        check: impl Fn(T) -> Fut + 'static,
    ) -> Self
    where
        M: Into<SharedString>,
        Fut: Future<Output = Option<M>> + 'static,
    {
        self.asynchronous(ValidationEvent::Change, debounce, check)
    }

    /// Validate asynchronously when the field loses focus.
    #[must_use]
    pub fn on_blur_async<M, Fut>(self, check: impl Fn(T) -> Fut + 'static) -> Self
    where
        M: Into<SharedString>,
        Fut: Future<Output = Option<M>> + 'static,
    {
        self.asynchronous(ValidationEvent::Blur, Duration::ZERO, check)
    }

    /// Validate asynchronously when the form is submitted.
    #[must_use]
    pub fn on_submit_async<M, Fut>(self, check: impl Fn(T) -> Fut + 'static) -> Self
    where
        M: Into<SharedString>,
        Fut: Future<Output = Option<M>> + 'static,
    {
        self.asynchronous(ValidationEvent::Submit, Duration::ZERO, check)
    }

    /// Also run this field's change validators when `other` changes (TanStack's
    /// `onChangeListenTo`): a "confirm password" field listens to the password.
    #[must_use]
    pub fn listen_to(mut self, other: &impl FieldKey) -> Self {
        self.listen_to.push(other.field_key());
        self
    }
}

impl<T: 'static> Default for Validators<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Validators<T> {
    fn clone(&self) -> Self {
        Self {
            sync: self.sync.clone(),
            asynchronous: self.asynchronous.clone(),
            listen_to: self.listen_to.clone(),
        }
    }
}

/// Errors from a form validator or a submit handler: messages for the whole form and for
/// fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormError {
    pub(crate) form: Vec<SharedString>,
    pub(crate) fields: Vec<(SharedString, SharedString)>,
}

impl FormError {
    /// An error about the whole form.
    #[must_use]
    pub fn form(message: impl Into<SharedString>) -> Self {
        Self {
            form: vec![message.into()],
            fields: Vec::new(),
        }
    }

    /// An error shown on one field.
    #[must_use]
    pub fn field(field: &impl FieldKey, message: impl Into<SharedString>) -> Self {
        Self {
            form: Vec::new(),
            fields: vec![(field.field_key(), message.into())],
        }
    }

    /// An error on a field named by its key (`team.0.email`), for errors from schema
    /// validators and servers.
    #[must_use]
    pub fn at_key(key: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self {
            form: Vec::new(),
            fields: vec![(key.into(), message.into())],
        }
    }

    /// Add another field error.
    #[must_use]
    pub fn and_field(mut self, field: &impl FieldKey, message: impl Into<SharedString>) -> Self {
        self.fields.push((field.field_key(), message.into()));
        self
    }

    /// Add another form error.
    #[must_use]
    pub fn and_form(mut self, message: impl Into<SharedString>) -> Self {
        self.form.push(message.into());
        self
    }

    /// Merge `other` into this error.
    #[must_use]
    pub fn merge(mut self, other: FormError) -> Self {
        self.form.extend(other.form);
        self.fields.extend(other.fields);
        self
    }

    /// Whether there are no errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.form.is_empty() && self.fields.is_empty()
    }

    /// The errors about the whole form.
    #[must_use]
    pub fn form_errors(&self) -> &[SharedString] {
        &self.form
    }

    /// The field errors, as (field key, message).
    #[must_use]
    pub fn field_errors(&self) -> &[(SharedString, SharedString)] {
        &self.fields
    }
}

type FormCheck<V> = Rc<dyn Fn(&V) -> Option<FormError>>;

/// Validators over the whole form's values, per event. They can report errors for any field.
///
/// ```
/// use rok_ui::form::{FormError, FormValidators, FormValues};
///
/// #[derive(FormValues, Clone, Default)]
/// struct SignUp { password: String, confirm: String }
///
/// let validators = FormValidators::new().on_submit(|values: &SignUp| {
///     (values.password != values.confirm)
///         .then(|| FormError::field(&SignUp::CONFIRM, "Passwords do not match"))
/// });
/// # let _ = validators;
/// ```
pub struct FormValidators<V> {
    pub(crate) checks: Vec<(ValidationEvent, FormCheck<V>)>,
}

impl<V: 'static> FormValidators<V> {
    /// No validators.
    #[must_use]
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    fn check(
        mut self,
        event: ValidationEvent,
        check: impl Fn(&V) -> Option<FormError> + 'static,
    ) -> Self {
        self.checks.push((event, Rc::new(check)));
        self
    }

    /// Validate after every change to any field.
    #[must_use]
    pub fn on_change(self, check: impl Fn(&V) -> Option<FormError> + 'static) -> Self {
        self.check(ValidationEvent::Change, check)
    }

    /// Validate when any field loses focus.
    #[must_use]
    pub fn on_blur(self, check: impl Fn(&V) -> Option<FormError> + 'static) -> Self {
        self.check(ValidationEvent::Blur, check)
    }

    /// Validate when the form is submitted.
    #[must_use]
    pub fn on_submit(self, check: impl Fn(&V) -> Option<FormError> + 'static) -> Self {
        self.check(ValidationEvent::Submit, check)
    }

    /// Validate with a schema on change and submit: anything implementing [`Schema`] (see
    /// the `garde` adapter behind the `form-garde` feature).
    #[must_use]
    pub fn schema(self, schema: impl Schema<V> + 'static) -> Self {
        let schema = Rc::new(schema);
        let on_submit = schema.clone();
        self.on_change(move |values| schema.validate(values))
            .on_submit(move |values| on_submit.validate(values))
    }
}

impl<V: 'static> Default for FormValidators<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> Clone for FormValidators<V> {
    fn clone(&self) -> Self {
        Self {
            checks: self.checks.clone(),
        }
    }
}

/// A validation schema over form values (TanStack Form's Standard Schema support). Errors use
/// field keys such as `team.0.email`.
pub trait Schema<V> {
    /// The errors in `values`, if any.
    fn validate(&self, values: &V) -> Option<FormError>;
}

impl<V, F: Fn(&V) -> Option<FormError>> Schema<V> for F {
    fn validate(&self, values: &V) -> Option<FormError> {
        self(values)
    }
}
