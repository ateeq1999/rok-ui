//! Running validators: per field (sync, then async after a debounce) and over the form.

use std::{rc::Rc, time::Duration};

use gpui::{App, Entity, SharedString};

use super::{
    settle_submit, FieldMeta, FormError, FormInner, FormValues, LocalFuture, Path, ValidationEvent,
    Validators,
};

pub(super) type SyncRunner<V> = Rc<dyn Fn(ValidationEvent, &V) -> Option<SharedString>>;
pub(super) type AsyncRunner<V> = Rc<dyn Fn(ValidationEvent, &V) -> Option<(Duration, AsyncJob)>>;
/// Starts an async validation: called after the debounce, so a pause runs it once.
pub(super) type AsyncJob = Box<dyn FnOnce() -> LocalFuture<Option<SharedString>>>;

/// One field's validators with the types erased.
pub(super) struct ErasedField<V> {
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

pub(super) fn erase<V: 'static, T: Clone + 'static>(
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

impl<V: FormValues> FormInner<V> {
    pub(super) fn meta_mut(&mut self, key: &SharedString) -> &mut FieldMeta {
        if !self.order.contains(key) {
            self.order.push(key.clone());
        }
        self.meta.entry(key.clone()).or_default()
    }

    pub(super) fn is_validating(&self) -> bool {
        !self.async_runs.is_empty()
    }

    pub(super) fn is_valid(&self) -> bool {
        self.meta.values().all(FieldMeta::is_valid) && self.form_errors.values().all(Vec::is_empty)
    }

    /// Run `key`'s synchronous validators for `event`; returns the async job to start, if the
    /// synchronous ones passed and an async one exists.
    pub(super) fn validate_sync(
        &mut self,
        key: &SharedString,
        event: ValidationEvent,
    ) -> Option<(Duration, AsyncJob)> {
        let field = self.fields.get(key)?.clone();
        if !field.events.contains(&event) {
            return None;
        }
        if let Some(error) = (field.sync)(event, &self.values) {
            // The value is invalid already: an async check of it no longer matters.
            self.async_runs.remove(&(key.clone(), event));
            let still_validating = self.async_runs.keys().any(|(other, _)| other == key);
            let meta = self.meta_mut(key);
            meta.error_map.insert(event, error);
            meta.is_validating = still_validating;
            return None;
        }
        self.meta_mut(key).error_map.remove(&event);
        (field.asynchronous)(event, &self.values)
    }

    pub(super) fn run_form_validators(&mut self, event: ValidationEvent) {
        let errors = self
            .form_validators
            .checks
            .iter()
            .filter(|(check_event, _)| *check_event == event)
            .filter_map(|(_, check)| check(&self.values))
            .fold(FormError::default(), FormError::merge);
        self.apply_form_error(event, errors);
    }

    pub(super) fn apply_form_error(&mut self, event: ValidationEvent, errors: FormError) {
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
    pub(super) fn listeners(&self, key: &SharedString) -> Vec<SharedString> {
        self.fields
            .iter()
            .filter(|(_, field)| field.listen_to.contains(key))
            .map(|(listener, _)| listener.clone())
            .collect()
    }
}

/// Start (or restart) an async validation; the previous run for the same field and event is
/// dropped, so only the latest value's result counts.
pub(super) fn start_async<V: FormValues>(
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
            settle_submit(&entity, cx);
        })
        .ok();
    });
    form.update(cx, |inner, _| {
        inner.async_runs.insert((key, event), (generation, task));
    });
}

/// Validate `keys` for `event`, starting async validators where the sync ones pass.
pub(super) fn validate_keys<V: FormValues>(
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
    settle_submit(form, cx);
}
