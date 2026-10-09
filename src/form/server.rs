//! API errors on form fields (features `form` + `http`): [`to_server_errors`],
//! [`Form::apply_server_errors`] and [`use_api_form`].

use std::{future::Future, ops::Deref, rc::Rc};

use gpui::{App, SharedString, Task};

use super::{path::FieldKey, FormError, FormOptions, FormValues};
use crate::{cx::Cx, hooks::State, http::ApiError};

/// The message for a 429.
pub const TOO_MANY_ATTEMPTS: &str = "Too many attempts. Try again in a minute.";
/// The message for an error that is not an [`ApiError`].
pub const SOMETHING_WENT_WRONG: &str = "Something went wrong. Please try again.";

type PickField = Rc<dyn Fn(&ApiError) -> Option<SharedString>>;

/// Where [`to_server_errors`] puts an API error's messages.
///
/// ```
/// use rok_ui::form::{FormValues, ServerErrorOptions};
///
/// #[derive(FormValues, Clone, Default)]
/// struct SignUp {
///     email: String,
///     password: String,
/// }
///
/// let options = ServerErrorOptions::new()
///     .map("emailAddress", &SignUp::EMAIL) // the API's name for the field
///     .conflict_field(&SignUp::EMAIL) // a 409 means the email is taken
///     .unauthorized_field(&SignUp::PASSWORD); // a 401 means a wrong password
/// # let _ = options;
/// ```
#[derive(Clone, Default)]
pub struct ServerErrorOptions {
    field_map: Vec<(String, SharedString)>,
    conflict_field: Option<SharedString>,
    bad_request_field: Option<PickField>,
    unauthorized_field: Option<SharedString>,
}

impl std::fmt::Debug for ServerErrorOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerErrorOptions")
            .field("field_map", &self.field_map)
            .field("conflict_field", &self.conflict_field)
            .field("bad_request_field", &self.bad_request_field.is_some())
            .field("unauthorized_field", &self.unauthorized_field)
            .finish()
    }
}

impl ServerErrorOptions {
    /// Field details go to the form field with the same name; everything else to the form.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Details for `api_field` go to `field` (an API that says `emailAddress` for `email`).
    #[must_use]
    pub fn map(mut self, api_field: impl Into<String>, field: &impl FieldKey) -> Self {
        self.field_map.push((api_field.into(), field.field_key()));
        self
    }

    /// Like [`map`](Self::map), with the form field named by its key (`team.0.email`).
    #[must_use]
    pub fn map_key(mut self, api_field: impl Into<String>, key: impl Into<SharedString>) -> Self {
        self.field_map.push((api_field.into(), key.into()));
        self
    }

    /// A 409 (conflict) shows its message on `field`.
    #[must_use]
    pub fn conflict_field(mut self, field: &impl FieldKey) -> Self {
        self.conflict_field = Some(field.field_key());
        self
    }

    /// A 400 (bad request) shows its message on `field`.
    #[must_use]
    pub fn bad_request_field(self, field: &impl FieldKey) -> Self {
        let key = field.field_key();
        self.bad_request_field_with(move |_| Some(key.clone()))
    }

    /// A 400 shows its message on the field `pick` returns (from the error's `code`, say), or
    /// on the form when it returns `None`.
    #[must_use]
    pub fn bad_request_field_with(
        mut self,
        pick: impl Fn(&ApiError) -> Option<SharedString> + 'static,
    ) -> Self {
        self.bad_request_field = Some(Rc::new(pick));
        self
    }

    /// A 401 shows its message on `field` (sign-in forms: the password).
    #[must_use]
    pub fn unauthorized_field(mut self, field: &impl FieldKey) -> Self {
        self.unauthorized_field = Some(field.field_key());
        self
    }

    fn form_key(&self, api_field: &str) -> SharedString {
        self.field_map
            .iter()
            .find(|(name, _)| name == api_field)
            .map_or_else(
                || SharedString::from(api_field.to_string()),
                |(_, key)| key.clone(),
            )
    }
}

/// An error, sorted for a form: messages on fields, and at most one for the whole form.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerErrors {
    /// (field key, message).
    pub fields: Vec<(SharedString, SharedString)>,
    /// The message for the whole form, if any.
    pub form: Option<SharedString>,
    /// The HTTP status behind [`form`](Self::form) (`0` when the server was not reached).
    pub form_status: Option<u16>,
}

impl ServerErrors {
    /// Whether there is nothing to show (a cancelled request).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.form.is_none()
    }

    /// The field errors only, as a [`FormError`].
    #[must_use]
    pub fn field_errors(&self) -> FormError {
        self.fields
            .iter()
            .fold(FormError::default(), |errors, (key, message)| {
                errors.merge(FormError::at_key(key.clone(), message.clone()))
            })
    }

    fn on_form(error: &ApiError, message: impl Into<SharedString>) -> Self {
        Self {
            fields: Vec::new(),
            form: Some(message.into()),
            form_status: Some(error.status),
        }
    }

    fn on_field(key: SharedString, message: &str) -> Self {
        Self {
            fields: vec![(key, SharedString::from(message.to_string()))],
            form: None,
            form_status: None,
        }
    }
}

impl From<ServerErrors> for FormError {
    fn from(errors: ServerErrors) -> Self {
        let fields = errors.field_errors();
        match errors.form {
            Some(message) => fields.and_form(message),
            None => fields,
        }
    }
}

/// Sort `error` for a form:
///
/// | Error | Goes to |
/// |---|---|
/// | 422 with details | each field (through [`ServerErrorOptions::map`]); the form when there are none |
/// | 409 | the conflict field, else the form |
/// | 400 | the bad-request field, else the form |
/// | 401 | the unauthorized field, else the form |
/// | 429 | the form: "Too many attempts. Try again in a minute." |
/// | other `ApiError` | the form, with its status |
/// | cancelled | nowhere |
/// | not an `ApiError` | the form: "Something went wrong. Please try again." |
#[must_use]
pub fn to_server_errors(
    error: &(dyn std::error::Error + 'static),
    options: &ServerErrorOptions,
) -> ServerErrors {
    let Some(error) = error.downcast_ref::<ApiError>() else {
        return ServerErrors {
            fields: Vec::new(),
            form: Some(SOMETHING_WENT_WRONG.into()),
            form_status: None,
        };
    };
    if error.is_cancelled() {
        return ServerErrors::default();
    }
    match error.status {
        422 => {
            let fields: Vec<(SharedString, SharedString)> = error
                .details
                .iter()
                .flatten()
                .filter_map(|(field, issues)| {
                    let message = issues.iter().find(|issue| !issue.message.is_empty())?;
                    Some((
                        options.form_key(field),
                        SharedString::from(message.message.clone()),
                    ))
                })
                .collect();
            if fields.is_empty() {
                ServerErrors::on_form(error, error.message.clone())
            } else {
                ServerErrors {
                    fields,
                    form: None,
                    form_status: None,
                }
            }
        }
        409 => match &options.conflict_field {
            Some(key) => ServerErrors::on_field(key.clone(), &error.message),
            None => ServerErrors::on_form(error, error.message.clone()),
        },
        400 => match options
            .bad_request_field
            .as_ref()
            .and_then(|pick| pick(error))
        {
            Some(key) => ServerErrors::on_field(key, &error.message),
            None => ServerErrors::on_form(error, error.message.clone()),
        },
        401 => match &options.unauthorized_field {
            Some(key) => ServerErrors::on_field(key.clone(), &error.message),
            None => ServerErrors::on_form(error, error.message.clone()),
        },
        429 => ServerErrors::on_form(error, TOO_MANY_ATTEMPTS),
        _ => ServerErrors::on_form(error, error.message.clone()),
    }
}

impl<V: FormValues> super::Form<V> {
    /// Show `errors` as server errors: on their fields (until the field changes), and the form
    /// message among the form's errors (until the next submit). For forms that submit through a bloc,
    /// apply the error the bloc's state carries.
    pub fn apply_server_errors(&self, cx: &mut App, errors: &ServerErrors) {
        self.apply_errors(cx, errors.clone().into());
    }
}

type Submit<V, T> = Rc<dyn Fn(V) -> std::pin::Pin<Box<dyn Future<Output = Result<T, ApiError>>>>>;
type OnSuccess<T> = Rc<dyn Fn(T, &mut App)>;

/// A form that submits to an API: [`FormOptions`], the call, where its errors go, and what
/// happens after a success.
pub struct ApiFormOptions<V, T> {
    form: FormOptions<V>,
    submit: Submit<V, T>,
    errors: ServerErrorOptions,
    on_success: Option<OnSuccess<T>>,
}

impl<V: FormValues, T: 'static> ApiFormOptions<V, T> {
    /// A form that calls `submit` with its values once validation passes.
    #[must_use]
    pub fn new<Fut>(form: FormOptions<V>, submit: impl Fn(V) -> Fut + 'static) -> Self
    where
        Fut: Future<Output = Result<T, ApiError>> + 'static,
    {
        Self {
            form,
            submit: Rc::new(move |values| Box::pin(submit(values))),
            errors: ServerErrorOptions::default(),
            on_success: None,
        }
    }

    /// Where errors go.
    #[must_use]
    pub fn server_errors(mut self, errors: ServerErrorOptions) -> Self {
        self.errors = errors;
        self
    }

    /// Runs after a successful call only (navigate, toast, reset).
    #[must_use]
    pub fn on_success(mut self, on_success: impl Fn(T, &mut App) + 'static) -> Self {
        self.on_success = Some(Rc::new(on_success));
        self
    }
}

/// A [`Form`](super::Form) whose submit calls an API, from [`use_api_form`]. Derefs to the
/// form; adds the form-level error.
pub struct ApiForm<V> {
    form: super::Form<V>,
    form_error: State<Option<(SharedString, Option<u16>)>>,
    current: Option<(SharedString, Option<u16>)>,
}

impl<V> Clone for ApiForm<V> {
    fn clone(&self) -> Self {
        Self {
            form: self.form.clone(),
            form_error: self.form_error.clone(),
            current: self.current.clone(),
        }
    }
}

impl<V> Deref for ApiForm<V> {
    type Target = super::Form<V>;

    fn deref(&self) -> &super::Form<V> {
        &self.form
    }
}

impl<V> ApiForm<V> {
    /// The message for the whole form from the last submit (field messages are on fields).
    #[must_use]
    pub fn form_error(&self) -> Option<&SharedString> {
        self.current.as_ref().map(|(message, _)| message)
    }

    /// The HTTP status behind [`form_error`](Self::form_error) (`0`: the server was not
    /// reached; `None`: not an API error).
    #[must_use]
    pub fn form_error_status(&self) -> Option<u16> {
        self.current.as_ref().and_then(|(_, status)| *status)
    }

    /// Hide the form-level error (when the user dismisses it).
    pub fn clear_form_error(&self, cx: &mut App) {
        self.form_error.set(None, cx);
    }

    /// The form.
    #[must_use]
    pub fn form(&self) -> &super::Form<V> {
        &self.form
    }
}

/// [`use_form`](super::use_form) for a form that submits to an API.
///
/// Field errors from the response show on their fields (see [`to_server_errors`]); the
/// form-level message is [`ApiForm::form_error`]. `on_success` runs only when the call
/// succeeds; a cancelled call does nothing.
///
/// ```no_run
/// use rok_ui::{
///     form::{self, ApiFormOptions, FormOptions, FormValues, ServerErrorOptions, SubmitButton, TextField},
///     http::{ApiError, HttpClient, Options},
///     prelude::*,
/// };
///
/// #[derive(FormValues, Clone, Default, serde::Serialize)]
/// struct SignIn {
///     email: String,
///     password: String,
/// }
///
/// #[component]
/// fn SignInForm(client: HttpClient, cx: &mut Cx) -> impl IntoElement {
///     let form = form::use_api_form(
///         cx,
///         ApiFormOptions::new(FormOptions::new(SignIn::default()), move |values: SignIn| {
///             let client = client.clone();
///             async move {
///                 client
///                     .request_empty("/sessions", Options::post().json(&values).skip_expire(true))
///                     .await
///             }
///         })
///         .server_errors(ServerErrorOptions::new().unauthorized_field(&SignIn::PASSWORD)),
///     );
///     div()
///         .child(TextField::new(&form.field(cx, SignIn::EMAIL), "Email"))
///         .child(TextField::new(&form.field(cx, SignIn::PASSWORD), "Password").password())
///         .children(form.form_error().cloned())
///         .child(SubmitButton::new(&form, "Sign in"))
/// }
/// ```
pub fn use_api_form<V: FormValues, T: 'static>(
    cx: &mut Cx,
    options: ApiFormOptions<V, T>,
) -> ApiForm<V> {
    let form_error: State<Option<(SharedString, Option<u16>)>> = cx.use_state(|| None);
    let ApiFormOptions {
        form,
        submit,
        errors,
        on_success,
    } = options;
    let state = form_error.clone();
    let form = form.on_submit(move |values, cx| -> Task<Result<(), FormError>> {
        state.set(None, cx);
        let running = submit(values);
        let state = state.clone();
        let errors = errors.clone();
        let on_success = on_success.clone();
        cx.spawn(async move |cx| match running.await {
            Ok(value) => {
                if let Some(on_success) = on_success {
                    cx.update(|cx| on_success(value, cx)).ok();
                }
                Ok(())
            }
            Err(error) => {
                let sorted = to_server_errors(&error, &errors);
                if let Some(message) = sorted.form.clone() {
                    let status = sorted.form_status;
                    cx.update(|cx| state.set(Some((message, status)), cx)).ok();
                }
                Err(sorted.field_errors())
            }
        })
    });
    let form = super::use_form(cx, form);
    let current = form_error.get(cx);
    ApiForm {
        form,
        form_error,
        current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{FieldDetails, FieldIssue};

    struct Key(&'static str);

    impl FieldKey for Key {
        fn field_key(&self) -> SharedString {
            self.0.into()
        }
    }

    fn details(pairs: &[(&str, &str)]) -> FieldDetails {
        pairs
            .iter()
            .map(|(field, message)| {
                (
                    (*field).to_string(),
                    vec![FieldIssue {
                        message: (*message).to_string(),
                        ..FieldIssue::default()
                    }],
                )
            })
            .collect()
    }

    fn sorted(
        error: &(dyn std::error::Error + 'static),
        options: &ServerErrorOptions,
    ) -> (Vec<(String, String)>, Option<String>, Option<u16>) {
        let errors = to_server_errors(error, options);
        (
            errors
                .fields
                .iter()
                .map(|(key, message)| (key.to_string(), message.to_string()))
                .collect(),
            errors.form.map(|message| message.to_string()),
            errors.form_status,
        )
    }

    fn field(key: &str, message: &str) -> (String, String) {
        (key.to_string(), message.to_string())
    }

    #[test]
    fn errors_go_where_the_table_says() {
        let options = ServerErrorOptions::new()
            .map("emailAddress", &Key("email"))
            .conflict_field(&Key("email"))
            .unauthorized_field(&Key("password"))
            .bad_request_field_with(|error| (error.code == "bad_name").then(|| "name".into()));
        let none = ServerErrorOptions::new();
        let api = |status, code: &str, message: &str| ApiError::new(status, code, message);

        let invalid = api(422, "invalid", "Check the form")
            .with_details(details(&[("emailAddress", "Taken"), ("name", "Required")]));
        assert_eq!(
            sorted(&invalid, &options),
            (
                vec![field("email", "Taken"), field("name", "Required")],
                None,
                None
            )
        );
        let invalid_without_details = api(422, "invalid", "Check the form");
        assert_eq!(
            sorted(&invalid_without_details, &options),
            (vec![], Some("Check the form".into()), Some(422))
        );

        let conflict = api(409, "taken", "Already registered");
        assert_eq!(
            sorted(&conflict, &options),
            (vec![field("email", "Already registered")], None, None)
        );
        assert_eq!(
            sorted(&conflict, &none),
            (vec![], Some("Already registered".into()), Some(409))
        );

        let bad_name = api(400, "bad_name", "Pick another name");
        assert_eq!(
            sorted(&bad_name, &options),
            (vec![field("name", "Pick another name")], None, None)
        );
        let bad_other = api(400, "bad", "Bad request");
        assert_eq!(
            sorted(&bad_other, &options),
            (vec![], Some("Bad request".into()), Some(400))
        );

        let unauthorized = api(401, "unauthorized", "Wrong password");
        assert_eq!(
            sorted(&unauthorized, &options),
            (vec![field("password", "Wrong password")], None, None)
        );
        assert_eq!(
            sorted(&unauthorized, &none),
            (vec![], Some("Wrong password".into()), Some(401))
        );

        let throttled = api(429, "slow_down", "Rate limited");
        assert_eq!(
            sorted(&throttled, &options),
            (vec![], Some(TOO_MANY_ATTEMPTS.into()), Some(429))
        );

        let server = api(500, "boom", "Server error");
        assert_eq!(
            sorted(&server, &options),
            (vec![], Some("Server error".into()), Some(500))
        );
        assert_eq!(
            sorted(&ApiError::network(), &options),
            (vec![], Some("Cannot reach the server".into()), Some(0))
        );

        assert!(to_server_errors(&ApiError::cancelled(), &options).is_empty());

        let other = std::io::Error::other("disk on fire");
        assert_eq!(
            sorted(&other, &options),
            (vec![], Some(SOMETHING_WENT_WRONG.into()), None)
        );
    }

    #[test]
    fn server_errors_become_form_errors() {
        let errors = ServerErrors {
            fields: vec![("email".into(), "Taken".into())],
            form: Some("Check the form".into()),
            form_status: Some(422),
        };
        let form_error: FormError = errors.clone().into();
        assert_eq!(
            form_error.field_errors(),
            [("email".into(), "Taken".into())]
        );
        assert_eq!(form_error.form_errors(), ["Check the form"]);
        assert_eq!(errors.field_errors().form_errors(), &[] as &[SharedString]);
    }
}
