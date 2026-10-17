//! What the devtools overlay shows about live forms.

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
