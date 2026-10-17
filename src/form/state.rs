//! Form state: [`use_form`], [`Form`], [`FieldApi`] and [`ArrayFieldApi`].

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
};

use gpui::{
    App, Entity, FocusHandle, Pixels, Point, ScrollHandle, SharedString, Subscription, Task, Window,
};

use super::{
    path::{FieldKey, FormValues, Path},
    validators::{FormError, FormValidators, LocalFuture, ValidationEvent, Validators},
};
use crate::Cx;

mod arrays;
#[cfg(feature = "devtools")]
pub(crate) mod devtools;
mod draft;
mod submit;
mod validation;

pub use arrays::ArrayFieldApi;
#[cfg(feature = "persist")]
use draft::{watch_draft, Draft, DraftState};
pub(crate) use submit::submit;
use submit::{settle_submit, WaitingSubmit};
use validation::{erase, validate_keys, ErasedField};

type SubmitHandler<V> = Rc<dyn Fn(V, &mut App) -> Task<Result<(), FormError>>>;

/// A field's interaction and validation state (TanStack Form's field `meta`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldMeta {
    /// The user changed or left the field.
    pub is_touched: bool,
    /// The field lost focus at least once.
    pub is_blurred: bool,
    /// The value was changed since the form was created or reset.
    pub is_dirty: bool,
    /// An async validator is running.
    pub is_validating: bool,
    /// Errors from the field's own validators, by the event that produced them.
    pub error_map: BTreeMap<ValidationEvent, SharedString>,
    /// Errors that form validators and the submit handler put on this field, by event.
    pub form_error_map: BTreeMap<ValidationEvent, SharedString>,
}

impl FieldMeta {
    /// Every error, field validators first, without duplicates.
    #[must_use]
    pub fn errors(&self) -> Vec<SharedString> {
        let mut errors: Vec<SharedString> = Vec::new();
        for error in self.error_map.values().chain(self.form_error_map.values()) {
            if !errors.contains(error) {
                errors.push(error.clone());
            }
        }
        errors
    }

    /// Whether there are no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.error_map.is_empty() && self.form_error_map.is_empty()
    }

    /// Whether the value is unchanged.
    #[must_use]
    pub fn is_pristine(&self) -> bool {
        !self.is_dirty
    }
}

/// A bound text input: its state, and the path it writes to (updated every render, so list
/// items keep their input when rows move).
pub(crate) struct Binding<V> {
    pub(crate) input: Entity<crate::components::InputState>,
    pub(crate) path: Rc<RefCell<Path<V, String>>>,
    pub(crate) _subscriptions: Vec<Subscription>,
}

/// Everything a form remembers. Owned by the component that called [`use_form`].
pub(crate) struct FormInner<V> {
    pub(crate) values: V,
    defaults: V,
    meta: HashMap<SharedString, FieldMeta>,
    order: Vec<SharedString>,
    fields: HashMap<SharedString, ErasedField<V>>,
    form_validators: FormValidators<V>,
    form_errors: BTreeMap<ValidationEvent, Vec<SharedString>>,
    on_submit: Option<SubmitHandler<V>>,
    is_submitting: bool,
    is_submitted: bool,
    /// A submit waiting for async validators to finish.
    waiting_submit: Option<WaitingSubmit>,
    attempts: u32,
    async_runs: HashMap<(SharedString, ValidationEvent), (u64, Task<()>)>,
    async_generation: u64,
    mounted: HashSet<SharedString>,
    pub(crate) bindings: HashMap<SharedString, Binding<V>>,
    pub(crate) focus: HashMap<SharedString, FocusHandle>,
    /// The scroll container the form lives in, from [`FormOptions::scroll_handle`].
    pub(crate) scroll: Option<ScrollHandle>,
    /// Where each bound field sits in that container's content, recorded as it lays out.
    pub(crate) anchors: HashMap<SharedString, FieldAnchor>,
    /// Bumped whenever the values change, so a draft is saved only after real edits.
    revision: u64,
    #[cfg(feature = "persist")]
    draft: Option<DraftState>,
}

/// A field's position in its scroll container's content (unscrolled, window coordinates),
/// written by the field every frame.
pub(crate) type FieldAnchor = Rc<std::cell::Cell<Option<Point<Pixels>>>>;

/// What a form starts from and how it validates and submits.
///
/// ```
/// use rok_ui::{
///     form::{FormError, FormOptions, FormValidators, FormValues, Validators},
///     gpui::Task,
/// };
///
/// #[derive(FormValues, Clone, Default)]
/// pub struct SignUp {
///     pub email: String,
///     pub password: String,
///     pub confirm: String,
/// }
///
/// pub fn sign_up_form() -> FormOptions<SignUp> {
///     FormOptions::new(SignUp::default())
///         .field(
///             SignUp::EMAIL,
///             Validators::new().on_blur(|email: &String| (!email.contains('@')).then_some("Enter a valid email")),
///         )
///         .validators(FormValidators::new().on_submit(|values: &SignUp| {
///             (values.password != values.confirm)
///                 .then(|| FormError::field(&SignUp::CONFIRM, "Passwords do not match"))
///         }))
///         .on_submit(|_values, _cx| Task::ready(Ok(())))
/// }
/// # let _ = sign_up_form();
/// ```
pub struct FormOptions<V> {
    defaults: V,
    validators: FormValidators<V>,
    fields: Vec<(SharedString, ErasedField<V>)>,
    on_submit: Option<SubmitHandler<V>>,
    scroll: Option<ScrollHandle>,
    #[cfg(feature = "persist")]
    draft: Option<Draft<V>>,
}

impl<V: FormValues> FormOptions<V> {
    /// A form starting at `defaults`.
    #[must_use]
    pub fn new(defaults: V) -> Self {
        Self {
            defaults,
            validators: FormValidators::new(),
            fields: Vec::new(),
            on_submit: None,
            scroll: None,
            #[cfg(feature = "persist")]
            draft: None,
        }
    }

    /// The scroll container the form is in: the handle given to
    /// [`ScrollArea::track_scroll`](crate::components::ScrollArea::track_scroll) (or a div's
    /// `track_scroll`). Pass the same handle on every render. A failed submit then scrolls the
    /// first invalid field to the top of the container, as well as focusing it. Fields
    /// rendered with the bound controls (`TextField`, `CheckboxField`, `SelectField`, ...) are
    /// found; a bare `BoundInput` is not.
    #[must_use]
    pub fn scroll_handle(mut self, handle: ScrollHandle) -> Self {
        self.scroll = Some(handle);
        self
    }

    /// Validators over the whole form.
    #[must_use]
    pub fn validators(mut self, validators: FormValidators<V>) -> Self {
        self.validators = validators;
        self
    }

    /// Validators for one field.
    #[must_use]
    pub fn field<T: Clone + 'static>(
        mut self,
        path: impl Into<Path<V, T>>,
        validators: Validators<T>,
    ) -> Self {
        let path = path.into();
        self.fields
            .push((path.key().clone(), erase(path, validators)));
        self
    }

    /// What submitting does once every validator passes. The form is submitting until the task
    /// resolves; errors it returns show on their fields (as [`ValidationEvent::Server`]).
    #[must_use]
    pub fn on_submit(
        mut self,
        submit: impl Fn(V, &mut App) -> Task<Result<(), FormError>> + 'static,
    ) -> Self {
        self.on_submit = Some(Rc::new(submit));
        self
    }

    /// Save the values as a draft while the user edits (feature `persist`), so a form closed
    /// half-filled opens where it was left. The form starts from the saved draft when there is
    /// one; edits are written after `options`' debounce, and a successful submit or a `reset`
    /// deletes the draft. `options` name the file and its version, like a persisted store.
    #[cfg(feature = "persist")]
    #[must_use]
    pub fn persist_draft(mut self, options: crate::persist::PersistOptions) -> Self
    where
        V: serde::Serialize + serde::de::DeserializeOwned,
    {
        self.draft = Some(Draft {
            options,
            load: |file| file.load::<V>(),
            save: |file, values, cx| file.save(values.clone(), cx),
        });
        self
    }
}

/// The form-wide state, as of this render (TanStack Form's form `state`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormState {
    /// Every validator passes and the form is not submitting.
    pub can_submit: bool,
    /// Submitting: waiting for async validators, or the submit handler is running.
    pub is_submitting: bool,
    /// The last submit went through.
    pub is_submitted: bool,
    /// How many times submit was attempted.
    pub submission_attempts: u32,
    /// No field or form errors.
    pub is_valid: bool,
    /// Any field changed.
    pub is_dirty: bool,
    /// Any field was touched.
    pub is_touched: bool,
    /// An async validator is running.
    pub is_validating: bool,
    /// Errors about the whole form (not tied to a field).
    pub errors: Vec<SharedString>,
}

/// A form at one call site, returned by [`use_form`]. Cheap to clone into handlers.
pub struct Form<V> {
    pub(crate) entity: Entity<FormInner<V>>,
    values: Rc<V>,
    state: FormState,
}

impl<V> Clone for Form<V> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity.clone(),
            values: self.values.clone(),
            state: self.state.clone(),
        }
    }
}

/// A headless, type-safe form for this call site (TanStack Form's `useForm`). Field state,
/// validation and submission live here; render the fields with [`Form::field`] and the bound
/// components (`TextField`, `CheckboxField`, `SubmitButton`).
///
/// `options` are read on every render (validators and the submit handler can change), but the
/// default values only when the form is created.
#[track_caller]
pub fn use_form<V: FormValues>(cx: &mut Cx, options: FormOptions<V>) -> Form<V> {
    let FormOptions {
        defaults,
        validators,
        fields,
        on_submit,
        scroll,
        #[cfg(feature = "persist")]
        draft,
    } = options;
    let entity = cx.window.use_state(cx.app, |_, cx| {
        // Only the draft reads the app.
        #[cfg(not(feature = "persist"))]
        let _ = cx;
        FormInner {
            // Read the draft once, when the form is created.
            #[cfg(feature = "persist")]
            values: draft
                .as_ref()
                .and_then(|draft| {
                    (draft.load)(&crate::persist::DraftFile::new(cx, draft.options.clone()))
                })
                .unwrap_or_else(|| defaults.clone()),
            #[cfg(not(feature = "persist"))]
            values: defaults.clone(),
            defaults,
            meta: HashMap::new(),
            order: Vec::new(),
            fields: HashMap::new(),
            form_validators: FormValidators::new(),
            form_errors: BTreeMap::new(),
            on_submit: None,
            is_submitting: false,
            is_submitted: false,
            waiting_submit: None,
            attempts: 0,
            async_runs: HashMap::new(),
            async_generation: 0,
            mounted: HashSet::new(),
            bindings: HashMap::new(),
            focus: HashMap::new(),
            scroll: None,
            anchors: HashMap::new(),
            revision: 0,
            #[cfg(feature = "persist")]
            draft: None,
        }
    });
    #[cfg(feature = "devtools")]
    devtools::register(&entity, cx.app);
    #[cfg(feature = "persist")]
    if let Some(draft) = draft {
        if entity.read(cx.app).draft.is_none() {
            let file = Rc::new(crate::persist::DraftFile::new(cx.app, draft.options));
            watch_draft(&entity, cx.app, file, draft.save);
        }
    }
    let mut new_fields = Vec::new();
    entity.update(cx.app, |inner, _| {
        inner.form_validators = validators;
        inner.on_submit = on_submit;
        inner.scroll = scroll;
        for (key, field) in fields {
            if !inner.fields.contains_key(&key) {
                new_fields.push(key.clone());
            }
            inner.fields.insert(key, field);
        }
    });
    mount(&entity, cx.app, &new_fields);
    Form::snapshot(entity, cx.app)
}

/// Run mount validators for fields seen for the first time.
fn mount<V: FormValues>(form: &Entity<FormInner<V>>, cx: &mut App, keys: &[SharedString]) {
    let unmounted: Vec<SharedString> = form.update(cx, |inner, _| {
        keys.iter()
            .filter(|key| inner.mounted.insert((*key).clone()))
            .cloned()
            .collect()
    });
    if !unmounted.is_empty() {
        validate_keys(form, cx, &unmounted, ValidationEvent::Mount);
    }
}

impl<V: FormValues> Form<V> {
    fn snapshot(entity: Entity<FormInner<V>>, cx: &App) -> Self {
        let inner = entity.read(cx);
        let is_valid = inner.is_valid();
        let state = FormState {
            can_submit: is_valid && !inner.is_submitting,
            is_submitting: inner.is_submitting,
            is_submitted: inner.is_submitted,
            submission_attempts: inner.attempts,
            is_valid,
            is_dirty: inner.meta.values().any(|meta| meta.is_dirty),
            is_touched: inner.meta.values().any(|meta| meta.is_touched),
            is_validating: inner.is_validating(),
            errors: inner.form_errors.values().flatten().cloned().collect(),
        };
        let values = Rc::new(inner.values.clone());
        Self {
            entity,
            values,
            state,
        }
    }

    /// The values, as of this render.
    #[must_use]
    pub fn values(&self) -> &V {
        &self.values
    }

    /// The form-wide state, as of this render.
    #[must_use]
    pub fn state(&self) -> &FormState {
        &self.state
    }

    /// Whether any field changed: pass it to `router::use_blocker` to guard unsaved work.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.state.is_dirty
    }

    /// The field at `path`: its value, state and handlers.
    pub fn field<T: Clone + 'static>(
        &self,
        cx: &mut App,
        path: impl Into<Path<V, T>>,
    ) -> FieldApi<V, T> {
        let path = path.into();
        let key = path.key().clone();
        self.entity.update(cx, |inner, _| {
            inner.meta_mut(&key);
        });
        mount(&self.entity, cx, std::slice::from_ref(&key));
        FieldApi::read(&self.entity, path, cx)
    }

    /// The field at `path`, with validators declared where it renders.
    pub fn field_with<T: Clone + 'static>(
        &self,
        cx: &mut App,
        path: impl Into<Path<V, T>>,
        validators: Validators<T>,
    ) -> FieldApi<V, T> {
        let path = path.into();
        let key = path.key().clone();
        let field = erase(path.clone(), validators);
        self.entity.update(cx, |inner, _| {
            inner.fields.insert(key, field);
        });
        self.field(cx, path)
    }

    /// The list field at `path`, with operations that keep each row's field state.
    pub fn array_field<E: Clone + 'static>(
        &self,
        cx: &mut App,
        path: impl Into<Path<V, Vec<E>>>,
    ) -> ArrayFieldApi<V, E> {
        let path = path.into();
        let items = path.get(&self.values).cloned().unwrap_or_default();
        let _ = cx;
        ArrayFieldApi {
            form: self.entity.clone(),
            path,
            items,
        }
    }

    /// Set the value at `path`, as if the user changed it.
    pub fn set_value<T: 'static>(&self, cx: &mut App, path: impl Into<Path<V, T>>, value: T) {
        set_value(&self.entity, cx, &path.into(), value);
    }

    /// Show `errors` as server errors (as [`ValidationEvent::Server`]), as if a submit had
    /// returned them. A field's error stays until the field changes; errors about the whole
    /// form clear at the next submit.
    pub fn apply_errors(&self, cx: &mut App, errors: FormError) {
        self.entity.update(cx, |inner, cx| {
            inner.apply_form_error(ValidationEvent::Server, errors);
            cx.notify();
        });
    }

    /// Run every validator for `event` now.
    pub fn validate(&self, cx: &mut App, event: ValidationEvent) {
        let keys = self.entity.read(cx).order.clone();
        validate_keys(&self.entity, cx, &keys, event);
    }

    /// Back to the default values, with fresh field state.
    pub fn reset(&self, cx: &mut App) {
        self.entity.update(cx, |inner, cx| {
            inner.values = inner.defaults.clone();
            inner.revision += 1;
            inner.discard_draft();
            inner.meta.clear();
            inner.form_errors.clear();
            inner.async_runs.clear();
            if inner.waiting_submit.take().is_some() {
                inner.is_submitting = false;
            }
            inner.attempts = 0;
            inner.is_submitted = false;
            cx.notify();
        });
    }

    /// Reset one field to its default value and fresh state.
    pub fn reset_field<T: Clone + 'static>(&self, cx: &mut App, path: impl Into<Path<V, T>>) {
        let path = path.into();
        self.entity.update(cx, |inner, cx| {
            if let Some(default) = path.get(&inner.defaults).cloned() {
                inner.revision += 1;
                if let Some(value) = path.get_mut(&mut inner.values) {
                    *value = default;
                }
            }
            inner.meta.insert(path.key().clone(), FieldMeta::default());
            inner.async_runs.retain(|(key, _), _| key != path.key());
            cx.notify();
        });
        settle_submit(&self.entity, cx);
    }

    /// Validate everything and, if it is valid, run the submit handler. Async validators are
    /// waited for (the form is submitting meanwhile). On failure the first invalid bound field
    /// gets focus. Enter in a bound single-line input submits too.
    pub fn submit(&self, window: &mut Window, cx: &mut App) {
        submit(&self.entity, Some(window), cx);
    }

    /// A click handler that submits the form.
    pub fn submit_handler<Event>(&self) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
        let form = self.entity.clone();
        move |_, window, cx| submit(&form, Some(window), cx)
    }
}

pub(crate) fn set_value<V: FormValues, T: 'static>(
    form: &Entity<FormInner<V>>,
    cx: &mut App,
    path: &Path<V, T>,
    value: T,
) {
    let key = path.key().clone();
    let changed = form.update(cx, |inner, _| {
        let Some(slot) = path.get_mut(&mut inner.values) else {
            return false;
        };
        *slot = value;
        inner.revision += 1;
        let meta = inner.meta_mut(&key);
        meta.is_dirty = true;
        meta.is_touched = true;
        // The user is fixing the field: earlier submit and server errors no longer apply.
        meta.error_map.remove(&ValidationEvent::Submit);
        meta.error_map.remove(&ValidationEvent::Server);
        meta.form_error_map.remove(&ValidationEvent::Submit);
        meta.form_error_map.remove(&ValidationEvent::Server);
        true
    });
    if !changed {
        return;
    }
    let mut keys = vec![key.clone()];
    keys.extend(form.read(cx).listeners(&key));
    validate_keys(form, cx, &keys, ValidationEvent::Change);
}

pub(crate) fn blur<V: FormValues>(form: &Entity<FormInner<V>>, cx: &mut App, key: &SharedString) {
    form.update(cx, |inner, _| {
        let meta = inner.meta_mut(key);
        meta.is_touched = true;
        meta.is_blurred = true;
    });
    validate_keys(form, cx, std::slice::from_ref(key), ValidationEvent::Blur);
}

/// One field of a [`Form`], as of this render: its value, state and handlers.
pub struct FieldApi<V, T> {
    pub(crate) form: Entity<FormInner<V>>,
    pub(crate) path: Path<V, T>,
    value: Option<T>,
    meta: FieldMeta,
    submission_attempts: u32,
}

impl<V, T: Clone> Clone for FieldApi<V, T> {
    fn clone(&self) -> Self {
        Self {
            form: self.form.clone(),
            path: self.path.clone(),
            value: self.value.clone(),
            meta: self.meta.clone(),
            submission_attempts: self.submission_attempts,
        }
    }
}

impl<V: FormValues, T: Clone + 'static> FieldApi<V, T> {
    fn read(form: &Entity<FormInner<V>>, path: Path<V, T>, cx: &App) -> Self {
        let inner = form.read(cx);
        Self {
            value: path.get(&inner.values).cloned(),
            meta: inner.meta.get(path.key()).cloned().unwrap_or_default(),
            submission_attempts: inner.attempts,
            form: form.clone(),
            path,
        }
    }

    /// The field's key: `email`, `team.0.email`.
    #[must_use]
    pub fn name(&self) -> &SharedString {
        self.path.key()
    }

    /// The value. `None` only for a list item that no longer exists.
    #[must_use]
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// The interaction and validation state.
    #[must_use]
    pub fn meta(&self) -> &FieldMeta {
        &self.meta
    }

    /// Every error.
    #[must_use]
    pub fn errors(&self) -> Vec<SharedString> {
        self.meta.errors()
    }

    /// Whether to show errors now: after the user touched the field or tried to submit.
    #[must_use]
    pub fn should_show_errors(&self) -> bool {
        (self.meta.is_touched || self.submission_attempts > 0) && !self.meta.is_valid()
    }

    /// Set the value (what a bound control calls on input).
    pub fn handle_change(&self, cx: &mut App, value: T) {
        set_value(&self.form, cx, &self.path, value);
    }

    /// Mark the field blurred and run blur validators.
    pub fn handle_blur(&self, cx: &mut App) {
        blur(&self.form, cx, self.path.key());
    }

    /// A handler for controls whose `on_change` passes the new value (`Checkbox`, `Switch`,
    /// `Slider`).
    pub fn change_handler(&self) -> impl Fn(&T, &mut Window, &mut App) + 'static {
        let (form, path) = (self.form.clone(), self.path.clone());
        move |value, _, cx| set_value(&form, cx, &path, value.clone())
    }
}

impl<F, T> FieldKey for FieldApi<F, T> {
    fn field_key(&self) -> SharedString {
        self.path.key().clone()
    }
}
