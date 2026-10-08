//! [`Bloc`]: events in, states out, and [`BlocHandle`], the running bloc.

use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    mem::Discriminant,
    sync::{Arc, Mutex, PoisonError},
};

use tokio::{runtime::Handle, sync::mpsc};

use crate::{Emitter, Observable};

/// How a bloc handles an event that arrives while others run (`bloc_concurrency`'s
/// transformers). Picked per event by [`Bloc::concurrency`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Concurrency {
    /// One at a time, in the order they arrived (one queue per bloc). The default.
    #[default]
    Sequential,
    /// Ignore the event while an event of the same variant is being handled (a submit button
    /// pressed twice).
    Droppable,
    /// Cancel the running event of the same variant and handle the new one (search as you
    /// type).
    Restartable,
    /// Handle it at once, alongside anything else.
    Concurrent,
}

/// Business logic that turns events into states (the BLoC pattern).
///
/// A bloc learns things only from its events and the repositories it was built with. It never
/// holds another bloc: [`BlocHandle`] is not `Send`, and a bloc must be, so storing a handle
/// does not compile. React to another bloc in the view, with a `BlocListener`.
///
/// ```
/// use rok_ui_bloc::{Bloc, Concurrency, Emitter};
///
/// #[derive(Clone, Debug, Default, PartialEq)]
/// struct SearchState {
///     query: String,
///     results: Vec<String>,
/// }
///
/// enum SearchEvent {
///     QueryChanged(String),
/// }
///
/// struct SearchBloc;
///
/// impl Bloc for SearchBloc {
///     type Event = SearchEvent;
///     type State = SearchState;
///
///     fn initial_state(&self) -> SearchState {
///         SearchState::default()
///     }
///
///     // A newer query cancels the search still running for an older one.
///     fn concurrency(&self, _event: &SearchEvent) -> Concurrency {
///         Concurrency::Restartable
///     }
///
///     async fn on(&self, event: SearchEvent, emit: &Emitter<SearchState>) {
///         let SearchEvent::QueryChanged(query) = event;
///         let results = vec![format!("{query} (result)")];
///         emit.emit(SearchState { query, results });
///     }
/// }
/// # let states = rok_ui_bloc::test::run(SearchBloc, [SearchEvent::QueryChanged("rust".into())]);
/// # assert_eq!(states[0].results, ["rust (result)"]);
/// ```
pub trait Bloc: Send + Sync + 'static {
    /// What happened (past tense: `NoteAdded`).
    type Event: Send + 'static;
    /// What the view shows: an immutable value.
    type State: Clone + PartialEq + Send + Sync + 'static;

    /// The state before any event.
    fn initial_state(&self) -> Self::State;

    /// Handle one event, emitting states as it goes. Runs on the runtime the bloc was started
    /// with; awaiting a repository is fine.
    fn on(
        &self,
        event: Self::Event,
        emit: &Emitter<Self::State>,
    ) -> impl Future<Output = ()> + Send;

    /// How to handle `event` relative to others (default: [`Concurrency::Sequential`]).
    fn concurrency(&self, _event: &Self::Event) -> Concurrency {
        Concurrency::Sequential
    }
}

struct Core<B: Bloc> {
    bloc: Arc<B>,
    emitter: Emitter<B::State>,
    queue: mpsc::UnboundedSender<B::Event>,
    /// The running task per event variant, for `Droppable` and `Restartable`.
    running: Mutex<HashMap<Discriminant<B::Event>, u64>>,
}

/// A running bloc: add events, read the state, close it. Cheap to clone; views get one from
/// `cx.bloc::<B>()`.
///
/// Not `Send`, on purpose: blocs must be `Send`, so a bloc cannot store another bloc's handle.
pub struct BlocHandle<B: Bloc> {
    core: Arc<Core<B>>,
    _not_send: PhantomData<*const ()>,
}

impl<B: Bloc> Clone for BlocHandle<B> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            _not_send: PhantomData,
        }
    }
}

impl<B: Bloc> std::fmt::Debug for BlocHandle<B> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BlocHandle")
            .field("bloc", &std::any::type_name::<B>())
            .field("closed", &self.is_closed())
            .finish()
    }
}

impl<B: Bloc> BlocHandle<B> {
    /// Start `bloc` with its handlers running on `runtime`.
    #[must_use]
    pub fn start(bloc: B, runtime: &Handle) -> Self {
        let emitter = Emitter::new(bloc.initial_state());
        emitter.attach(runtime.clone());
        let bloc = Arc::new(bloc);
        let (queue, mut events) = mpsc::unbounded_channel::<B::Event>();
        // The sequential queue: one worker, events in arrival order.
        {
            let (bloc, worker_emitter) = (bloc.clone(), emitter.clone());
            emitter.spawn_task(async move {
                while let Some(event) = events.recv().await {
                    bloc.on(event, &worker_emitter).await;
                    worker_emitter.end();
                }
            });
            // The worker itself is not work in progress; only its queued events are.
            emitter.end();
        }
        Self {
            core: Arc::new(Core {
                bloc,
                emitter,
                queue,
                running: Mutex::new(HashMap::new()),
            }),
            _not_send: PhantomData,
        }
    }

    /// Add an event. Ignored once the bloc is closed.
    pub fn add(&self, event: B::Event) {
        let core = &self.core;
        if core.emitter.is_closed() {
            return;
        }
        let variant = std::mem::discriminant(&event);
        match core.bloc.concurrency(&event) {
            Concurrency::Sequential => {
                core.emitter.begin();
                if core.queue.send(event).is_err() {
                    core.emitter.end();
                }
            }
            Concurrency::Concurrent => {
                self.spawn(event);
            }
            Concurrency::Droppable => {
                let mut running = core.running.lock().unwrap_or_else(PoisonError::into_inner);
                let busy = running
                    .get(&variant)
                    .is_some_and(|id| core.emitter.is_running(*id));
                if !busy {
                    if let Some(id) = self.spawn(event) {
                        running.insert(variant, id);
                    }
                }
            }
            Concurrency::Restartable => {
                let mut running = core.running.lock().unwrap_or_else(PoisonError::into_inner);
                if let Some(previous) = running.remove(&variant) {
                    core.emitter.abort(previous);
                }
                if let Some(id) = self.spawn(event) {
                    running.insert(variant, id);
                }
            }
        }
    }

    fn spawn(&self, event: B::Event) -> Option<u64> {
        let (bloc, emitter) = (self.core.bloc.clone(), self.core.emitter.clone());
        self.core.emitter.spawn_task(async move {
            bloc.on(event, &emitter).await;
        })
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> B::State {
        self.core.emitter.state()
    }

    /// Close the bloc: running handlers are cancelled, later events and emits are ignored.
    /// Providers close their bloc when they leave the tree.
    pub fn close(&self) {
        self.core.emitter.close();
    }

    /// Whether two handles are the same running bloc.
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.core, &other.core)
    }

    /// Whether the bloc was closed.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.core.emitter.is_closed()
    }

    /// Wait until no handler runs and no event is queued (for tests and shutdown).
    pub async fn idle(&self) {
        self.core.emitter.idle().await;
    }
}

impl<B: Bloc> Observable for BlocHandle<B> {
    type State = B::State;

    fn emitter(&self) -> &Emitter<B::State> {
        &self.core.emitter
    }
}
