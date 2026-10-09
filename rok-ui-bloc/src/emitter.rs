//! [`Emitter`]: a bloc's or cubit's current state, its subscribers and its running tasks.

use std::{
    collections::HashMap,
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, MutexGuard, PoisonError,
    },
};

use tokio::{runtime::Handle, sync::Notify, task::AbortHandle};

type Listener<S> = Arc<dyn Fn(&S) + Send + Sync>;
type ChangeHook<S> = Arc<dyn Fn(&S, &S) + Send + Sync>;

struct Shared<S> {
    state: Mutex<S>,
    listeners: Mutex<Vec<(u64, Listener<S>)>>,
    next_id: AtomicU64,
    closed: AtomicBool,
    runtime: Mutex<Option<Handle>>,
    tasks: Mutex<HashMap<u64, AbortHandle>>,
    /// Work in progress: running tasks plus queued sequential events.
    active: Mutex<usize>,
    idle: Notify,
    /// The bloc's type name and the observer hook, once a handle started it.
    observed: Mutex<Option<(&'static str, ChangeHook<S>)>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The current state of a bloc or cubit. Handlers and cubit methods call [`Emitter::emit`];
/// subscribers (views, listeners, tests) hear about each state that differs from the last.
///
/// Cheap to clone: clones share the state.
///
/// ```
/// use rok_ui_bloc::Emitter;
///
/// let emitter = Emitter::new(0);
/// assert!(emitter.emit(1));
/// assert!(!emitter.emit(1), "an equal state is not a change");
/// assert_eq!(emitter.state(), 1);
/// ```
pub struct Emitter<S> {
    shared: Arc<Shared<S>>,
}

impl<S> Clone for Emitter<S> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<S: std::fmt::Debug> std::fmt::Debug for Emitter<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Emitter")
            .field("state", &*lock(&self.shared.state))
            .field("closed", &self.shared.closed.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl<S: Clone + PartialEq + Send + Sync + 'static> Emitter<S> {
    /// An emitter starting at `initial`.
    #[must_use]
    pub fn new(initial: S) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(initial),
                listeners: Mutex::new(Vec::new()),
                next_id: AtomicU64::new(0),
                closed: AtomicBool::new(false),
                runtime: Mutex::new(None),
                tasks: Mutex::new(HashMap::new()),
                active: Mutex::new(0),
                idle: Notify::new(),
                observed: Mutex::new(None),
            }),
        }
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> S {
        lock(&self.shared.state).clone()
    }

    /// Replace the state. Subscribers hear about it only when it differs from the current one
    /// (`PartialEq`), and not at all once the bloc is closed. Returns whether it changed.
    pub fn emit(&self, state: S) -> bool {
        if self.is_closed() {
            return false;
        }
        let previous = {
            let mut current = lock(&self.shared.state);
            if *current == state {
                return false;
            }
            std::mem::replace(&mut *current, state.clone())
        };
        let hook = lock(&self.shared.observed)
            .as_ref()
            .map(|(_, hook)| hook.clone());
        if let Some(hook) = hook {
            hook(&previous, &state);
        }
        let listeners: Vec<Listener<S>> = lock(&self.shared.listeners)
            .iter()
            .map(|(_, listener)| listener.clone())
            .collect();
        for listener in listeners {
            listener(&state);
        }
        true
    }

    /// Change the state in place, then [`emit`](Emitter::emit) it.
    pub fn update(&self, change: impl FnOnce(&mut S)) -> bool {
        let mut next = self.state();
        change(&mut next);
        self.emit(next)
    }

    /// Whether the bloc or cubit was closed: emits are ignored and no new work starts.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::SeqCst)
    }

    /// Hear about every state change until the returned [`Subscription`] is dropped.
    pub fn subscribe(&self, listener: impl Fn(&S) + Send + Sync + 'static) -> Subscription {
        let id = self.shared.next_id.fetch_add(1, Ordering::SeqCst);
        lock(&self.shared.listeners).push((id, Arc::new(listener)));
        let shared = Arc::downgrade(&self.shared);
        Subscription {
            unsubscribe: Some(Box::new(move || {
                if let Some(shared) = shared.upgrade() {
                    lock(&shared.listeners).retain(|(listener, _)| *listener != id);
                }
            })),
        }
    }

    /// Run `work` on the bloc's runtime (for a cubit's async methods). It is cancelled when the
    /// cubit closes. Does nothing once closed.
    ///
    /// # Panics
    ///
    /// Panics outside a tokio runtime when the emitter was not started by a handle.
    pub fn spawn(&self, work: impl Future<Output = ()> + Send + 'static) {
        let name = self.name();
        self.spawn_task(crate::observer::guarded(name, work));
    }

    /// Spawn `work` as a tracked task and return its id, or `None` once closed.
    pub(crate) fn spawn_task(
        &self,
        work: impl Future<Output = ()> + Send + 'static,
    ) -> Option<u64> {
        if self.is_closed() {
            return None;
        }
        let runtime = lock(&self.shared.runtime)
            .clone()
            .or_else(|| Handle::try_current().ok())
            .expect("rok-ui-bloc: spawn outside a tokio runtime (start the bloc with a handle)");
        let id = self.shared.next_id.fetch_add(1, Ordering::SeqCst);
        self.begin();
        // Holding the task map while spawning makes the task's own removal wait for the insert.
        let mut tasks = lock(&self.shared.tasks);
        let shared = self.shared.clone();
        let task = runtime.spawn(async move {
            work.await;
            let finished = lock(&shared.tasks).remove(&id).is_some();
            if finished {
                Self::end_shared(&shared);
            }
        });
        tasks.insert(id, task.abort_handle());
        Some(id)
    }

    /// Abort the tracked task `id`, if it still runs.
    pub(crate) fn abort(&self, id: u64) {
        if let Some(task) = lock(&self.shared.tasks).remove(&id) {
            task.abort();
            self.end();
        }
    }

    /// Whether the tracked task `id` still runs.
    pub(crate) fn is_running(&self, id: u64) -> bool {
        lock(&self.shared.tasks).contains_key(&id)
    }

    /// Use `runtime` for this emitter's tasks.
    pub(crate) fn attach(&self, runtime: Handle) {
        *lock(&self.shared.runtime) = Some(runtime);
    }

    /// Count a unit of work (a queued event).
    pub(crate) fn begin(&self) {
        *lock(&self.shared.active) += 1;
    }

    /// Finish a unit of work.
    pub(crate) fn end(&self) {
        Self::end_shared(&self.shared);
    }

    fn end_shared(shared: &Shared<S>) {
        let mut active = lock(&shared.active);
        *active = active.saturating_sub(1);
        if *active == 0 {
            shared.idle.notify_waiters();
        }
    }

    /// Wait until no handler runs and no event is queued.
    pub async fn idle(&self) {
        loop {
            let notified = self.shared.idle.notified();
            let mut notified = std::pin::pin!(notified);
            notified.as_mut().enable();
            if *lock(&self.shared.active) == 0 || self.is_closed() {
                return;
            }
            notified.await;
        }
    }

    /// Close: abort every running task, ignore later emits, and drop the subscribers.
    /// Report this emitter's changes and closing to the observer, under `name`.
    pub(crate) fn observe(&self, name: &'static str)
    where
        S: std::fmt::Debug,
    {
        let hook: ChangeHook<S> = Arc::new(move |current: &S, next: &S| {
            if let Some(observer) = crate::observer::current() {
                observer.on_change(name, current, next);
            }
        });
        *lock(&self.shared.observed) = Some((name, hook));
        if let Some(observer) = crate::observer::current() {
            observer.on_create(name);
        }
    }

    /// The name given to [`observe`](Self::observe).
    pub(crate) fn name(&self) -> &'static str {
        lock(&self.shared.observed)
            .as_ref()
            .map_or("bloc", |(name, _)| *name)
    }

    pub(crate) fn close(&self) {
        if self.shared.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        let name = lock(&self.shared.observed).as_ref().map(|(name, _)| *name);
        if let (Some(name), Some(observer)) = (name, crate::observer::current()) {
            observer.on_close(name);
        }
        for (_, task) in lock(&self.shared.tasks).drain() {
            task.abort();
        }
        *lock(&self.shared.active) = 0;
        self.shared.idle.notify_waiters();
        lock(&self.shared.listeners).clear();
    }
}

/// Something views can show and listen to: a [`BlocHandle`](crate::BlocHandle) or a
/// [`CubitHandle`](crate::CubitHandle).
pub trait Observable: Clone + 'static {
    /// The state it holds.
    type State: Clone + PartialEq + Send + Sync + 'static;

    /// Its emitter.
    fn emitter(&self) -> &Emitter<Self::State>;

    /// The current state.
    fn state(&self) -> Self::State {
        self.emitter().state()
    }

    /// Hear about every state change until the [`Subscription`] is dropped.
    fn subscribe(&self, listener: impl Fn(&Self::State) + Send + Sync + 'static) -> Subscription {
        self.emitter().subscribe(listener)
    }
}

/// A subscription to state changes; dropping it unsubscribes.
#[must_use = "dropping a subscription unsubscribes"]
pub struct Subscription {
    unsubscribe: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Subscription")
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(unsubscribe) = self.unsubscribe.take() {
            unsubscribe();
        }
    }
}
