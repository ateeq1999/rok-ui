//! Values that background work can read: [`TaskCx`] and [`provide`].

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    sync::Arc,
};

use gpui::{App, Global};

use super::QueryError;

type ServiceMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

/// What query fetchers, procedures and memoized helpers receive: the values registered with
/// [`provide`] (a database handle, an HTTP client, the signed-in session), readable from any
/// thread.
///
/// ```
/// # use rok_ui::query::TaskCx;
/// #[derive(Clone)]
/// struct ApiBase(String);
///
/// fn base(cx: &TaskCx) -> String {
///     cx.get::<ApiBase>().map_or_else(|| "http://localhost".into(), |base| base.0.clone())
/// }
/// # assert_eq!(base(&TaskCx::default()), "http://localhost");
/// ```
#[derive(Clone, Default)]
pub struct TaskCx {
    services: Arc<ServiceMap>,
}

impl TaskCx {
    /// The value of type `T`, if one was provided.
    #[must_use]
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.services.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// The value of type `T`, or an error naming the missing type.
    ///
    /// # Errors
    ///
    /// Fails when nothing of type `T` was provided with [`provide`].
    pub fn require<T: Any + Send + Sync>(&self) -> Result<&T, QueryError> {
        self.get::<T>().ok_or_else(|| {
            QueryError::msg(format!(
                "no {} was provided; call rok_ui::query::provide at startup",
                std::any::type_name::<T>()
            ))
        })
    }
}

impl fmt::Debug for TaskCx {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskCx")
            .field("services", &self.services.len())
            .finish()
    }
}

#[derive(Default)]
struct Services(TaskCx);

impl Global for Services {}

/// Make `value` available to every fetcher, procedure and memoized helper through
/// [`TaskCx::get`]. Providing a second value of the same type replaces the first; work already
/// running keeps the value it started with.
pub fn provide<T: Any + Send + Sync>(cx: &mut App, value: T) {
    let services = &mut cx.default_global::<Services>().0.services;
    Arc::make_mut(services).insert(TypeId::of::<T>(), Arc::new(value));
}

/// Stop providing the value of type `T`.
pub fn unprovide<T: Any + Send + Sync>(cx: &mut App) {
    let services = &mut cx.default_global::<Services>().0.services;
    Arc::make_mut(services).remove(&TypeId::of::<T>());
}

/// A snapshot of the provided values, to hand to background work.
#[must_use]
pub fn task_cx(cx: &App) -> TaskCx {
    cx.try_global::<Services>()
        .map(|services| services.0.clone())
        .unwrap_or_default()
}
