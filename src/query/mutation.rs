//! Mutations: writes with pending / error / data state, invalidation and optimistic updates.

use std::{fmt, future::Future, rc::Rc, sync::Arc, time::Instant};

use gpui::{App, Entity, Task};

use super::{
    cache::{cache, AnyData},
    invalidate,
    options::BoxFuture,
    services::task_cx,
    QueryKey, TaskCx,
};
use crate::Cx;

type Runner<I, O, E> = Arc<dyn Fn(TaskCx, I) -> BoxFuture<Result<O, E>> + Send + Sync>;
type SuccessHandler<I, O> = Rc<dyn Fn(&O, &I, &mut App)>;
type ErrorHandler<I, E> = Rc<dyn Fn(&E, &I, &mut App)>;
type OptimisticUpdate<I> = Rc<dyn Fn(&I, &mut Optimistic<'_>)>;

/// What a mutation runs and what happens around it.
///
/// ```
/// use rok_ui::{query::MutationOptions, query_key};
///
/// let rename = MutationOptions::new(|_cx, name: String| async move {
///     Ok::<_, std::io::Error>(name.to_uppercase())
/// })
/// .invalidates(query_key!["users"]);
/// # let _ = rename;
/// ```
pub struct MutationOptions<I, O, E> {
    run: Runner<I, O, E>,
    invalidates: Vec<QueryKey>,
    on_success: Vec<SuccessHandler<I, O>>,
    on_error: Vec<ErrorHandler<I, E>>,
    optimistic: Option<OptimisticUpdate<I>>,
}

impl<I, O, E> MutationOptions<I, O, E>
where
    I: Send + 'static,
    O: Send + 'static,
    E: Send + 'static,
{
    /// A mutation that runs `run` on the shared background runtime with the mutation's input.
    pub fn new<F, Fut>(run: F) -> Self
    where
        F: Fn(TaskCx, I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, E>> + Send + 'static,
    {
        Self {
            run: Arc::new(move |cx, input| Box::pin(run(cx, input))),
            invalidates: Vec::new(),
            on_success: Vec::new(),
            on_error: Vec::new(),
            optimistic: None,
        }
    }
}

impl<I, O, E> MutationOptions<I, O, E> {
    /// Invalidate every query whose key starts with `prefix` after a success.
    #[must_use]
    pub fn invalidates(mut self, prefix: QueryKey) -> Self {
        self.invalidates.push(prefix);
        self
    }

    /// Run after a success, on the UI thread, before invalidated queries refetch.
    #[must_use]
    pub fn on_success(mut self, handler: impl Fn(&O, &I, &mut App) + 'static) -> Self {
        self.on_success.push(Rc::new(handler));
        self
    }

    /// Run after a failure, on the UI thread, after optimistic updates are rolled back.
    #[must_use]
    pub fn on_error(mut self, handler: impl Fn(&E, &I, &mut App) + 'static) -> Self {
        self.on_error.push(Rc::new(handler));
        self
    }

    /// Update cached queries before the mutation runs, for instant UI. The changes are rolled
    /// back if it fails.
    #[must_use]
    pub fn optimistic(mut self, update: impl Fn(&I, &mut Optimistic<'_>) + 'static) -> Self {
        self.optimistic = Some(Rc::new(update));
        self
    }
}

impl<I, O, E> Clone for MutationOptions<I, O, E> {
    fn clone(&self) -> Self {
        Self {
            run: self.run.clone(),
            invalidates: self.invalidates.clone(),
            on_success: self.on_success.clone(),
            on_error: self.on_error.clone(),
            optimistic: self.optimistic.clone(),
        }
    }
}

/// Cache access for [`MutationOptions::optimistic`]. Every change is recorded so it can be
/// rolled back.
pub struct Optimistic<'a> {
    cx: &'a mut App,
    snapshots: Vec<Snapshot>,
}

struct Snapshot {
    key: QueryKey,
    data: Option<AnyData>,
}

impl Optimistic<'_> {
    /// Change the cached data for `key` in place, if there is data of type `T`.
    pub fn update<T: Clone + 'static>(&mut self, key: &QueryKey, update: impl FnOnce(&mut T)) {
        self.record(key);
        super::update_query_data(self.cx, key, update);
    }

    /// Replace the cached data for `key`.
    pub fn set<T: 'static>(&mut self, key: &QueryKey, data: T) {
        self.record(key);
        super::set_query_data(self.cx, key, data);
    }

    fn record(&mut self, key: &QueryKey) {
        if self.snapshots.iter().any(|snapshot| &snapshot.key == key) {
            return;
        }
        let data = cache(self.cx)
            .entries
            .get(key)
            .and_then(|entry| entry.data.clone());
        self.snapshots.push(Snapshot {
            key: key.clone(),
            data,
        });
    }
}

fn roll_back(cx: &mut App, snapshots: Vec<Snapshot>) {
    let entries = &mut cache(cx).entries;
    for snapshot in snapshots {
        match snapshot.data {
            Some(data) => {
                if let Some(entry) = entries.get_mut(&snapshot.key) {
                    entry.set_data(data);
                }
            }
            None => {
                entries.remove(&snapshot.key);
            }
        }
    }
}

/// Where a mutation is in its lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MutationStatus {
    /// Not run yet (or reset).
    Idle,
    /// Running.
    Pending,
    /// The last run succeeded.
    Success,
    /// The last run failed.
    Error,
}

struct MutationState<O, E> {
    status: MutationStatus,
    data: Option<Rc<O>>,
    error: Option<Rc<E>>,
    run: u64,
    submitted_at: Option<Instant>,
    task: Option<Task<()>>,
}

impl<O, E> Default for MutationState<O, E> {
    fn default() -> Self {
        Self {
            status: MutationStatus::Idle,
            data: None,
            error: None,
            run: 0,
            submitted_at: None,
            task: None,
        }
    }
}

/// A mutation for one call site, returned by [`use_mutation`] and
/// [`use_procedure`](super::use_procedure). Cheap to clone into event handlers.
pub struct Mutation<I, O, E> {
    state: Entity<MutationState<O, E>>,
    options: MutationOptions<I, O, E>,
    status: MutationStatus,
    data: Option<Rc<O>>,
    error: Option<Rc<E>>,
}

impl<I, O, E> Clone for Mutation<I, O, E> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            options: self.options.clone(),
            status: self.status,
            data: self.data.clone(),
            error: self.error.clone(),
        }
    }
}

impl<I, O, E> fmt::Debug for Mutation<I, O, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mutation")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

impl<I, O, E> Mutation<I, O, E>
where
    I: Clone + Send + 'static,
    O: Send + 'static,
    E: Send + 'static,
{
    /// Run the mutation with `input`. A run started while another is pending supersedes it:
    /// only the latest run's result is kept.
    pub fn mutate(&self, cx: &mut App, input: I) {
        let options = self.options.clone();
        let snapshots = match &options.optimistic {
            Some(update) => {
                let mut optimistic = Optimistic {
                    cx,
                    snapshots: Vec::new(),
                };
                update(&input, &mut optimistic);
                optimistic.snapshots
            }
            None => Vec::new(),
        };

        let run_id = self.state.update(cx, |state, _| {
            state.run += 1;
            state.status = MutationStatus::Pending;
            state.submitted_at = Some(Instant::now());
            state.run
        });
        let services = task_cx(cx);
        let run = options.run.clone();
        let sent_input = input.clone();
        let running = crate::runtime::spawn(async move { run(services, sent_input).await });
        let state = self.state.clone();
        let task = cx.spawn(async move |cx| {
            let outcome = running.await;
            cx.update(|cx| {
                if state.read(cx).run != run_id {
                    return;
                }
                match outcome {
                    Ok(Ok(data)) => {
                        let data = Rc::new(data);
                        state.update(cx, |state, _| {
                            state.status = MutationStatus::Success;
                            state.data = Some(data.clone());
                            state.error = None;
                            state.task = None;
                        });
                        for handler in &options.on_success {
                            handler(&data, &input, cx);
                        }
                        for prefix in &options.invalidates {
                            invalidate(cx, prefix);
                        }
                    }
                    Ok(Err(error)) => {
                        roll_back(cx, snapshots);
                        let error = Rc::new(error);
                        state.update(cx, |state, _| {
                            state.status = MutationStatus::Error;
                            state.error = Some(error.clone());
                            state.task = None;
                        });
                        for handler in &options.on_error {
                            handler(&error, &input, cx);
                        }
                    }
                    Err(_) => {
                        roll_back(cx, snapshots);
                        state.update(cx, |state, _| {
                            state.status = MutationStatus::Idle;
                            state.task = None;
                        });
                    }
                }
                cx.refresh_windows();
            })
            .ok();
        });
        self.state.update(cx, |state, _| state.task = Some(task));
        cx.refresh_windows();
    }

    /// A click handler that runs the mutation with a clone of `input`.
    pub fn mutate_handler<Event>(
        &self,
        input: I,
    ) -> impl Fn(&Event, &mut gpui::Window, &mut App) + 'static {
        let mutation = self.clone();
        move |_, _, cx| mutation.mutate(cx, input.clone())
    }
}

impl<I, O: 'static, E: 'static> Mutation<I, O, E> {
    /// Run `handler` after a success, in addition to the options' handlers.
    #[must_use]
    pub fn on_success(mut self, handler: impl Fn(&O, &I, &mut App) + 'static) -> Self {
        self.options.on_success.push(Rc::new(handler));
        self
    }

    /// Run `handler` after a failure, in addition to the options' handlers.
    #[must_use]
    pub fn on_error(mut self, handler: impl Fn(&E, &I, &mut App) + 'static) -> Self {
        self.options.on_error.push(Rc::new(handler));
        self
    }

    /// Update cached queries before the mutation runs; rolled back if it fails.
    #[must_use]
    pub fn optimistic(mut self, update: impl Fn(&I, &mut Optimistic<'_>) + 'static) -> Self {
        self.options.optimistic = Some(Rc::new(update));
        self
    }

    /// Where the mutation is, as of this render.
    #[must_use]
    pub fn status(&self) -> MutationStatus {
        self.status
    }

    /// Whether a run is in progress.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.status == MutationStatus::Pending
    }

    /// Whether nothing has run since the mutation was created or reset.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.status == MutationStatus::Idle
    }

    /// Whether the last run succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status == MutationStatus::Success
    }

    /// Whether the last run failed.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.status == MutationStatus::Error
    }

    /// The last successful result.
    #[must_use]
    pub fn data(&self) -> Option<&O> {
        self.data.as_deref()
    }

    /// The last error.
    #[must_use]
    pub fn error(&self) -> Option<&E> {
        self.error.as_deref()
    }

    /// Forget the last result and return to idle. A pending run's result is dropped.
    pub fn reset(&self, cx: &mut App) {
        self.state.update(cx, |state, _| {
            *state = MutationState {
                run: state.run + 1,
                ..MutationState::default()
            };
        });
        cx.refresh_windows();
    }
}

/// A mutation for this call site, like TanStack Query's `useMutation`.
///
/// ```no_run
/// # use rok_ui::{prelude::*, query::{self, MutationOptions}, query_key};
/// #[component]
/// fn SaveButton(cx: &mut Cx) -> impl IntoElement {
///     let save = query::use_mutation(
///         cx,
///         MutationOptions::new(|_cx, title: String| async move {
///             Ok::<_, std::io::Error>(title)
///         })
///         .invalidates(query_key!["notes"]),
///     );
///     Button::new("save")
///         .label("Save")
///         .loading(save.is_pending())
///         .on_click(save.mutate_handler("Untitled".to_string()))
/// }
/// ```
#[track_caller]
pub fn use_mutation<I, O, E>(cx: &mut Cx, options: MutationOptions<I, O, E>) -> Mutation<I, O, E>
where
    O: 'static,
    E: 'static,
{
    let state = cx
        .window
        .use_state(cx.app, |_, _| MutationState::<O, E>::default());
    let (status, data, error) = {
        let current = state.read(cx.app);
        (current.status, current.data.clone(), current.error.clone())
    };
    Mutation {
        state,
        options,
        status,
        data,
        error,
    }
}
