//! Form state: [`use_form`], [`Form`], [`FieldApi`] and [`ArrayFieldApi`].

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use gpui::{
    App, Entity, FocusHandle, Pixels, Point, ScrollHandle, SharedString, Subscription, Task, Window,
};

use super::{
    path::{FieldKey, FormValues, Path},
    validators::{FormError, FormValidators, LocalFuture, ValidationEvent, Validators},
};
use crate::Cx;

type SyncRunner<V> = Rc<dyn Fn(ValidationEvent, &V) -> Option<SharedString>>;
type AsyncRunner<V> = Rc<dyn Fn(ValidationEvent, &V) -> Option<(Duration, AsyncJob)>>;
/// Starts an async validation: called after the debounce, so a pause runs it once.
type AsyncJob = Box<dyn FnOnce() -> LocalFuture<Option<SharedString>>>;
type SubmitHandler<V> = Rc<dyn Fn(V, &mut App) -> Task<Result<(), FormError>>>;

/// One field's validators with the types erased.
struct ErasedField<V> {
    sync: SyncRunner<V>,
    asynchronous: AsyncRunner<V>,
    events: Vec<ValidationEvent>,
    listen_to: Vec<SharedString>,
}

impl<V> Clone for ErasedField<V> {
    fn clone(&self) -> Self {
        Self {
            sync: self.sync.clone(),
            asynchronous: self.asynchronous.clone(),
            events: self.events.clone(),
            listen_to: self.listen_to.clone(),
        }
    }
}

fn erase<V: 'static, T: Clone + 'static>(
    path: Path<V, T>,
    validators: Validators<T>,
) -> ErasedField<V> {
    let events = validators
        .sync
        .iter()
        .map(|(event, _)| *event)
        .chain(validators.asynchronous.iter().map(|(event, _, _)| *event))
        .collect();
    let listen_to = validators.listen_to.clone();
    let sync_checks = validators.sync;
    let sync_path = path.clone();
    let async_checks = validators.asynchronous;
    ErasedField {
        sync: Rc::new(move |event, values| {
            let value = sync_path.get(values)?;
            sync_checks
                .iter()
                .filter(|(check_event, _)| *check_event == event)
                .find_map(|(_, check)| check(value))
        }),
        asynchronous: Rc::new(move |event, values| {
            let value = path.get(values)?.clone();
            let (_, debounce, check) = async_checks
                .iter()
                .find(|(check_event, _, _)| *check_event == event)?;
            let check = check.clone();
            Some((*debounce, Box::new(move || check(value)) as AsyncJob))
        }),
        events,
        listen_to,
    }
}

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

/// Scroll `handle` so the field at `position` is at the top of the visible area.
fn scroll_to_field(handle: &ScrollHandle, position: Point<Pixels>) {
    let top = handle.bounds().origin.y - position.y;
    let lowest = -handle.max_offset().height;
    let offset = handle.offset();
    handle.set_offset(gpui::point(offset.x, top.clamp(lowest, gpui::px(0.))));
}

/// How [`FormOptions::persist_draft`] loads and saves a form's values.
#[cfg(feature = "persist")]
struct Draft<V> {
    options: crate::persist::PersistOptions,
    load: fn(&crate::persist::DraftFile) -> Option<V>,
    save: fn(&crate::persist::DraftFile, &V, &App),
}

/// A form's draft file, the revision saved last, and the observer that saves edits.
#[cfg(feature = "persist")]
struct DraftState {
    file: Rc<crate::persist::DraftFile>,
    saved: u64,
    _observer: Subscription,
}

#[cfg(feature = "persist")]
impl<V> FormInner<V> {
    /// The values were submitted or reset: delete the draft until the next edit.
    fn discard_draft(&mut self) {
        let revision = self.revision;
        if let Some(draft) = &mut self.draft {
            draft.file.clear();
            draft.saved = revision;
        }
    }
}

#[cfg(not(feature = "persist"))]
impl<V> FormInner<V> {
    #[allow(clippy::unused_self)] // Mirrors the `persist` version.
    fn discard_draft(&mut self) {}
}

impl<V: FormValues> FormInner<V> {
    fn meta_mut(&mut self, key: &SharedString) -> &mut FieldMeta {
        if !self.order.contains(key) {
            self.order.push(key.clone());
        }
        self.meta.entry(key.clone()).or_default()
    }

    fn is_validating(&self) -> bool {
        !self.async_runs.is_empty()
    }

    fn is_valid(&self) -> bool {
        self.meta.values().all(FieldMeta::is_valid) && self.form_errors.values().all(Vec::is_empty)
    }

    /// Run `key`'s synchronous validators for `event`; returns the async job to start, if the
    /// synchronous ones passed and an async one exists.
    fn validate_sync(
        &mut self,
        key: &SharedString,
        event: ValidationEvent,
    ) -> Option<(Duration, AsyncJob)> {
        let field = self.fields.get(key)?.clone();
        if !field.events.contains(&event) {
            return None;
        }
        let error = (field.sync)(event, &self.values);
        let failed = error.is_some();
        let meta = self.meta_mut(key);
        match error {
            Some(error) => {
                meta.error_map.insert(event, error);
            }
            None => {
                meta.error_map.remove(&event);
            }
        }
        if failed {
            self.async_runs.remove(&(key.clone(), event));
            return None;
        }
        (field.asynchronous)(event, &self.values)
    }

    fn run_form_validators(&mut self, event: ValidationEvent) {
        let errors = self
            .form_validators
            .checks
            .iter()
            .filter(|(check_event, _)| *check_event == event)
            .filter_map(|(_, check)| check(&self.values))
            .fold(FormError::default(), FormError::merge);
        self.apply_form_error(event, errors);
    }

    fn apply_form_error(&mut self, event: ValidationEvent, errors: FormError) {
        for meta in self.meta.values_mut() {
            meta.form_error_map.remove(&event);
        }
        for (key, message) in errors.fields {
            self.meta_mut(&key)
                .form_error_map
                .entry(event)
                .or_insert(message);
        }
        if errors.form.is_empty() {
            self.form_errors.remove(&event);
        } else {
            self.form_errors.insert(event, errors.form);
        }
    }

    /// Keys of fields that listen to `key`.
    fn listeners(&self, key: &SharedString) -> Vec<SharedString> {
        self.fields
            .iter()
            .filter(|(_, field)| field.listen_to.contains(key))
            .map(|(listener, _)| listener.clone())
            .collect()
    }
}

/// Start (or restart) an async validation; the previous run for the same field and event is
/// dropped, so only the latest value's result counts.
fn start_async<V: FormValues>(
    form: &Entity<FormInner<V>>,
    cx: &mut App,
    key: SharedString,
    event: ValidationEvent,
    (debounce, job): (Duration, AsyncJob),
) {
    let generation = form.update(cx, |inner, _| {
        inner.async_generation += 1;
        inner.meta_mut(&key).is_validating = true;
        inner.async_generation
    });
    let entity = form.clone();
    let run_key = key.clone();
    let task = cx.spawn(async move |cx| {
        if !debounce.is_zero() {
            cx.background_executor().timer(debounce).await;
        }
        let error = job().await;
        cx.update(|cx| {
            entity.update(cx, |inner, cx| {
                let current = inner
                    .async_runs
                    .get(&(run_key.clone(), event))
                    .is_some_and(|(run, _)| *run == generation);
                if !current {
                    return;
                }
                inner.async_runs.remove(&(run_key.clone(), event));
                let still_validating = inner.async_runs.keys().any(|(other, _)| other == &run_key);
                let meta = inner.meta_mut(&run_key);
                meta.is_validating = still_validating;
                match error {
                    Some(error) => {
                        meta.error_map.insert(event, error);
                    }
                    None => {
                        meta.error_map.remove(&event);
                    }
                }
                cx.notify();
            });
        })
        .ok();
    });
    form.update(cx, |inner, _| {
        inner.async_runs.insert((key, event), (generation, task));
    });
}

/// Validate `keys` for `event`, starting async validators where the sync ones pass.
fn validate_keys<V: FormValues>(
    form: &Entity<FormInner<V>>,
    cx: &mut App,
    keys: &[SharedString],
    event: ValidationEvent,
) {
    let jobs: Vec<(SharedString, (Duration, AsyncJob))> = form.update(cx, |inner, _| {
        let jobs = keys
            .iter()
            .filter_map(|key| {
                inner
                    .validate_sync(key, event)
                    .map(|job| (key.clone(), job))
            })
            .collect();
        inner.run_form_validators(event);
        jobs
    });
    for (key, job) in jobs {
        start_async(form, cx, key, event, job);
    }
    form.update(cx, |_, cx| cx.notify());
}

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
    /// The submit handler is running.
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

/// Save the form's values to its draft file after every edit.
#[cfg(feature = "persist")]
fn watch_draft<V: FormValues>(
    entity: &Entity<FormInner<V>>,
    cx: &mut App,
    file: Rc<crate::persist::DraftFile>,
    save: fn(&crate::persist::DraftFile, &V, &App),
) {
    let observer = {
        let file = file.clone();
        cx.observe(entity, move |entity, cx| {
            let inner = entity.read(cx);
            let Some(draft) = &inner.draft else {
                return;
            };
            if draft.saved == inner.revision {
                return;
            }
            save(&file, &inner.values, cx);
            entity.update(cx, |inner, _| {
                let revision = inner.revision;
                if let Some(draft) = &mut inner.draft {
                    draft.saved = revision;
                }
            });
        })
    };
    entity.update(cx, |inner, _| {
        inner.draft = Some(DraftState {
            file,
            saved: inner.revision,
            _observer: observer,
        });
    });
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
            cx.notify();
        });
    }

    /// Validate everything and, if it is valid, run the submit handler. On failure the first
    /// invalid bound field gets focus. Enter in a bound single-line input submits too.
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

pub(crate) fn submit<V: FormValues>(
    form: &Entity<FormInner<V>>,
    window: Option<&mut Window>,
    cx: &mut App,
) {
    if form.read(cx).is_submitting {
        return;
    }
    let keys = form.update(cx, |inner, _| {
        inner.attempts += 1;
        inner.is_submitted = false;
        // A server error about the whole form ("try again later") must not block the retry;
        // field errors stay until their field changes.
        inner.form_errors.remove(&ValidationEvent::Server);
        for meta in inner.meta.values_mut() {
            meta.is_touched = true;
        }
        inner.order.clone()
    });
    for event in [
        ValidationEvent::Change,
        ValidationEvent::Blur,
        ValidationEvent::Submit,
    ] {
        validate_keys(form, cx, &keys, event);
    }
    let inner = form.read(cx);
    if !inner.is_valid() || inner.is_validating() {
        let first_invalid = inner
            .order
            .iter()
            .find(|key| inner.meta.get(*key).is_some_and(|meta| !meta.is_valid()));
        let focus = first_invalid.and_then(|key| inner.focus.get(key)).cloned();
        let position = first_invalid
            .and_then(|key| inner.anchors.get(key))
            .and_then(|anchor| anchor.get());
        if let (Some(handle), Some(position)) = (&inner.scroll, position) {
            scroll_to_field(handle, position);
        }
        if let Some(window) = window {
            if let Some(focus) = focus {
                window.focus(&focus);
            }
            window.refresh();
        }
        return;
    }
    let Some(handler) = inner.on_submit.clone() else {
        form.update(cx, |inner, cx| {
            inner.is_submitted = true;
            cx.notify();
        });
        return;
    };
    let values = inner.values.clone();
    form.update(cx, |inner, cx| {
        inner.is_submitting = true;
        cx.notify();
    });
    let running = handler(values, cx);
    let entity = form.clone();
    cx.spawn(async move |cx| {
        let result = running.await;
        cx.update(|cx| {
            entity.update(cx, |inner, cx| {
                inner.is_submitting = false;
                match result {
                    Ok(()) => {
                        inner.is_submitted = true;
                        inner.discard_draft();
                        inner.apply_form_error(ValidationEvent::Server, FormError::default());
                    }
                    Err(errors) => inner.apply_form_error(ValidationEvent::Server, errors),
                }
                cx.notify();
            });
        })
        .ok();
    })
    .detach();
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

/// A list field of a [`Form`], with operations that keep each row's field state with its row.
pub struct ArrayFieldApi<V, E> {
    form: Entity<FormInner<V>>,
    path: Path<V, Vec<E>>,
    items: Vec<E>,
}

impl<V, E: Clone> Clone for ArrayFieldApi<V, E> {
    fn clone(&self) -> Self {
        Self {
            form: self.form.clone(),
            path: self.path.clone(),
            items: self.items.clone(),
        }
    }
}

impl<V: FormValues, E: Clone + 'static> ArrayFieldApi<V, E> {
    /// The number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items, as of this render.
    #[must_use]
    pub fn items(&self) -> &[E] {
        &self.items
    }

    /// The path of item `index`, for `form.field(cx, list.at(index).then(..))`.
    #[must_use]
    pub fn at(&self, index: usize) -> Path<V, E> {
        self.path.at(index)
    }

    fn edit(&self, cx: &mut App, change: impl FnOnce(&mut Vec<E>) -> Option<Rekey>) {
        let key = self.path.key().clone();
        let path = self.path.clone();
        form_rekey_and_set(&self.form, cx, &path, &key, change);
    }

    /// Add `value` at the end.
    pub fn push_value(&self, cx: &mut App, value: E) {
        self.edit(cx, |list| {
            list.push(value);
            Some(Rekey::None)
        });
    }

    /// Insert `value` at `index`, moving later rows down.
    pub fn insert_value(&self, cx: &mut App, index: usize, value: E) {
        self.edit(cx, |list| {
            (index <= list.len()).then(|| {
                list.insert(index, value);
                Rekey::Insert(index)
            })
        });
    }

    /// Remove the item at `index`, moving later rows up.
    pub fn remove_value(&self, cx: &mut App, index: usize) {
        self.edit(cx, |list| {
            (index < list.len()).then(|| {
                list.remove(index);
                Rekey::Remove(index)
            })
        });
    }

    /// Swap two items.
    pub fn swap_values(&self, cx: &mut App, a: usize, b: usize) {
        self.edit(cx, |list| {
            (a < list.len() && b < list.len()).then(|| {
                list.swap(a, b);
                Rekey::Swap(a, b)
            })
        });
    }

    /// Move the item at `from` to `to`.
    pub fn move_value(&self, cx: &mut App, from: usize, to: usize) {
        self.edit(cx, |list| {
            (from < list.len() && to < list.len()).then(|| {
                let item = list.remove(from);
                list.insert(to, item);
                Rekey::Move(from, to)
            })
        });
    }

    /// Replace the item at `index`.
    pub fn replace_value(&self, cx: &mut App, index: usize, value: E) {
        self.edit(cx, |list| {
            list.get_mut(index).map(|item| {
                *item = value;
                Rekey::None
            })
        });
    }

    /// A click handler that adds a clone of `value`.
    pub fn push_handler<Event>(
        &self,
        value: E,
    ) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
        let list = self.clone();
        move |_, _, cx| list.push_value(cx, value.clone())
    }

    /// A click handler that removes the item at `index`.
    pub fn remove_handler<Event>(
        &self,
        index: usize,
    ) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
        let list = self.clone();
        move |_, _, cx| list.remove_value(cx, index)
    }
}

/// How row indices move after a list operation.
#[derive(Clone, Copy)]
enum Rekey {
    None,
    Insert(usize),
    Remove(usize),
    Swap(usize, usize),
    Move(usize, usize),
}

impl Rekey {
    /// Where row `index` is now, or `None` if it was removed.
    fn map(self, index: usize) -> Option<usize> {
        match self {
            Self::None => Some(index),
            Self::Insert(at) => Some(if index >= at { index + 1 } else { index }),
            Self::Remove(at) => match index.cmp(&at) {
                std::cmp::Ordering::Less => Some(index),
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => Some(index - 1),
            },
            Self::Swap(a, b) => Some(if index == a {
                b
            } else if index == b {
                a
            } else {
                index
            }),
            Self::Move(from, to) => {
                if index == from {
                    Some(to)
                } else if from < to && index > from && index <= to {
                    Some(index - 1)
                } else if to < from && index >= to && index < from {
                    Some(index + 1)
                } else {
                    Some(index)
                }
            }
        }
    }
}

/// `list.3.email` -> `(3, ".email")` for list key `list`.
fn row_of<'a>(list_key: &str, key: &'a str) -> Option<(usize, &'a str)> {
    let rest = key.strip_prefix(list_key)?.strip_prefix('.')?;
    let end = rest.find('.').unwrap_or(rest.len());
    Some((rest[..end].parse().ok()?, &rest[end..]))
}

fn rekey_map<T>(map: &mut HashMap<SharedString, T>, list_key: &str, rekey: Rekey) {
    let moved: Vec<(SharedString, T)> = map
        .keys()
        .filter(|key| row_of(list_key, key).is_some())
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|key| map.remove(&key).map(|value| (key, value)))
        .collect();
    for (key, value) in moved {
        let Some((row, suffix)) = row_of(list_key, &key) else {
            continue;
        };
        if let Some(row) = rekey.map(row) {
            map.insert(format!("{list_key}.{row}{suffix}").into(), value);
        }
    }
}

fn form_rekey_and_set<V: FormValues, E: 'static>(
    form: &Entity<FormInner<V>>,
    cx: &mut App,
    path: &Path<V, Vec<E>>,
    key: &SharedString,
    change: impl FnOnce(&mut Vec<E>) -> Option<Rekey>,
) {
    let changed = form.update(cx, |inner, _| {
        let list = path.get_mut(&mut inner.values)?;
        let rekey = change(list)?;
        inner.revision += 1;
        rekey_map(&mut inner.meta, key, rekey);
        rekey_map(&mut inner.bindings, key, rekey);
        rekey_map(&mut inner.focus, key, rekey);
        rekey_map(&mut inner.anchors, key, rekey);
        // Row validators are bound to row indices; they register again on the next render.
        inner
            .fields
            .retain(|field_key, _| row_of(key, field_key).is_none());
        inner
            .mounted
            .retain(|field_key| row_of(key, field_key).is_none());
        inner
            .async_runs
            .retain(|(field_key, _), _| row_of(key, field_key).is_none());
        let order: Vec<SharedString> = inner
            .order
            .iter()
            .filter_map(|field_key| match row_of(key, field_key) {
                Some((row, suffix)) => rekey
                    .map(row)
                    .map(|row| SharedString::from(format!("{key}.{row}{suffix}"))),
                None => Some(field_key.clone()),
            })
            .collect();
        inner.order = order;
        let meta = inner.meta_mut(key);
        meta.is_dirty = true;
        meta.is_touched = true;
        Some(())
    });
    if changed.is_some() {
        let mut keys = vec![key.clone()];
        keys.extend(form.read(cx).listeners(key));
        validate_keys(form, cx, &keys, ValidationEvent::Change);
    }
}

impl<F, T> FieldKey for FieldApi<F, T> {
    fn field_key(&self) -> SharedString {
        self.path.key().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{row_of, Rekey};

    #[test]
    fn rows_follow_list_operations() {
        assert_eq!(row_of("team", "team.3.email"), Some((3, ".email")));
        assert_eq!(row_of("team", "team.3"), Some((3, "")));
        assert_eq!(row_of("team", "teams.3"), None);
        assert_eq!(Rekey::Remove(1).map(0), Some(0));
        assert_eq!(Rekey::Remove(1).map(1), None);
        assert_eq!(Rekey::Remove(1).map(2), Some(1));
        assert_eq!(Rekey::Insert(1).map(1), Some(2));
        assert_eq!(Rekey::Swap(0, 2).map(2), Some(0));
        assert_eq!(Rekey::Move(0, 2).map(0), Some(2));
        assert_eq!(Rekey::Move(0, 2).map(1), Some(0));
        assert_eq!(Rekey::Move(2, 0).map(0), Some(1));
    }
}

/// What the devtools overlay shows about live forms.
#[cfg(feature = "devtools")]
pub(crate) mod devtools {
    use std::any::type_name;

    use gpui::{App, Entity, EntityId, Global, SharedString, WeakEntity};

    use super::{FormInner, FormValues};

    /// A live form, as the devtools overlay shows it.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct FormSummary {
        /// The values type's name: `SignUp`.
        pub name: SharedString,
        /// Whether any field changed.
        pub is_dirty: bool,
        /// Whether every validator passes.
        pub is_valid: bool,
        /// Whether a submit is running.
        pub is_submitting: bool,
        /// Whether the last submit succeeded.
        pub is_submitted: bool,
        /// How many times the user tried to submit.
        pub attempts: u32,
        /// Field errors, as (field key, message), in field order.
        pub field_errors: Vec<(SharedString, SharedString)>,
        /// Errors about the whole form.
        pub form_errors: Vec<SharedString>,
    }

    type Summarize = Box<dyn Fn(&App) -> Option<FormSummary>>;

    #[derive(Default)]
    struct Registry {
        forms: Vec<(EntityId, Summarize)>,
    }

    impl Global for Registry {}

    /// Remember `form` (once) so the overlay can list it; forget dropped forms.
    pub(crate) fn register<V: FormValues>(form: &Entity<FormInner<V>>, cx: &mut App) {
        let id = form.entity_id();
        if cx
            .try_global::<Registry>()
            .is_some_and(|registry| registry.forms.iter().any(|(known, _)| *known == id))
        {
            return;
        }
        prune(cx);
        let weak: WeakEntity<FormInner<V>> = form.downgrade();
        let name = type_name::<V>().rsplit("::").next().unwrap_or("form");
        let name = SharedString::from(name.to_string());
        let summarize: Summarize = Box::new(move |cx| {
            let form = weak.upgrade()?;
            let inner = form.read(cx);
            let field_errors = inner
                .order
                .iter()
                .filter_map(|key| {
                    let meta = inner.meta.get(key)?;
                    meta.errors()
                        .first()
                        .map(|message| (key.clone(), message.clone()))
                })
                .collect();
            Some(FormSummary {
                name: name.clone(),
                is_dirty: inner.meta.values().any(|meta| meta.is_dirty),
                is_valid: inner.is_valid(),
                is_submitting: inner.is_submitting,
                is_submitted: inner.is_submitted,
                attempts: inner.attempts,
                field_errors,
                form_errors: inner.form_errors.values().flatten().cloned().collect(),
            })
        });
        let registry = cx.default_global::<Registry>();
        registry.forms.push((id, summarize));
    }

    /// The live forms, oldest first.
    pub fn forms(cx: &App) -> Vec<FormSummary> {
        cx.try_global::<Registry>()
            .map(|registry| {
                registry
                    .forms
                    .iter()
                    .filter_map(|(_, summarize)| summarize(cx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Forget forms that were dropped.
    fn prune(cx: &mut App) {
        if !cx.has_global::<Registry>() {
            return;
        }
        let alive: Vec<bool> = {
            let registry = cx.global::<Registry>();
            registry
                .forms
                .iter()
                .map(|(_, summarize)| summarize(cx).is_some())
                .collect()
        };
        let registry = cx.global_mut::<Registry>();
        let mut alive = alive.into_iter();
        registry.forms.retain(|_| alive.next().unwrap_or(false));
    }
}
