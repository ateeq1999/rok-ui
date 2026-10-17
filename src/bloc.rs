#![doc = include_str!("../docs/guide/architecture.md")]

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
    sync::Arc,
};

use futures::{channel::mpsc, StreamExt as _};
use gpui::{prelude::*, AnyElement, App, ElementId, Task, Window};
pub use rok_ui_bloc::{
    bloc_test, clear_observer, set_observer, test, Bloc, BlocHandle, BlocObserver, BoxFuture,
    Concurrency, Cubit, CubitHandle, Emitter, LogObserver, Observable, Subscription,
};

use crate::{
    hooks::use_keyed_state,
    scope::{find, single, Frame, ScopeElement},
};

// ---------------------------------------------------------------------------
// Lookups.

/// The bloc of type `B` provided above the element being rendered.
#[must_use]
pub fn read<B: Bloc>() -> Option<BlocHandle<B>> {
    find::<BlocHandle<B>>()
}

/// The cubit of type `C` provided above the element being rendered.
#[must_use]
pub fn read_cubit<C: Cubit>() -> Option<CubitHandle<C>> {
    find::<CubitHandle<C>>()
}

/// The repository registered as `R` (often `dyn SomeRepository`) above the element being
/// rendered.
#[must_use]
pub fn repository<R: ?Sized + Send + Sync + 'static>() -> Option<Arc<R>> {
    find::<Arc<R>>()
}

fn missing(kind: &str, name: &str) -> ! {
    panic!(
        "no {kind} `{name}` is provided here: wrap this part of the tree in a provider, and read \
         it in a component rendered below the provider (not in the function that builds it)"
    )
}

/// What a bloc's `create` closure can read: the repositories provided above it. Blocs never
/// read other blocs.
#[derive(Clone, Copy, Debug)]
pub struct Scope(());

impl Scope {
    /// The repository registered as `R`.
    ///
    /// # Panics
    ///
    /// Panics when no `RepositoryProvider` above provides `R`.
    #[must_use]
    pub fn repository<R: ?Sized + Send + Sync + 'static>(&self) -> Arc<R> {
        repository::<R>().unwrap_or_else(|| missing("repository", std::any::type_name::<R>()))
    }
}

/// `Cx` methods for blocs, cubits and repositories (feature `bloc`).
impl crate::Cx<'_> {
    /// The bloc of type `B` provided above (`flutter_bloc`'s `context.read`). Read it while
    /// rendering and move the handle into event handlers.
    ///
    /// # Panics
    ///
    /// Panics when no provider above provides `B`.
    #[must_use]
    pub fn bloc<B: Bloc>(&self) -> BlocHandle<B> {
        read::<B>().unwrap_or_else(|| missing("bloc", std::any::type_name::<B>()))
    }

    /// The current state of the bloc `B` provided above (`flutter_bloc`'s `context.watch`).
    /// Windows re-render whenever a bloc's state changes.
    ///
    /// # Panics
    ///
    /// Panics when no provider above provides `B`.
    #[must_use]
    pub fn watch_bloc<B: Bloc>(&self) -> B::State {
        self.bloc::<B>().state()
    }

    /// The cubit of type `C` provided above.
    ///
    /// # Panics
    ///
    /// Panics when no provider above provides `C`.
    #[must_use]
    pub fn cubit<C: Cubit>(&self) -> CubitHandle<C> {
        read_cubit::<C>().unwrap_or_else(|| missing("cubit", std::any::type_name::<C>()))
    }

    /// The current state of the cubit `C` provided above.
    ///
    /// # Panics
    ///
    /// Panics when no provider above provides `C`.
    #[must_use]
    pub fn watch_cubit<C: Cubit>(&self) -> C::State {
        self.cubit::<C>().state()
    }

    /// The repository registered as `R` above (often `dyn SomeRepository`).
    ///
    /// # Panics
    ///
    /// Panics when no `RepositoryProvider` above provides `R`.
    #[must_use]
    pub fn repository<R: ?Sized + Send + Sync + 'static>(&self) -> Arc<R> {
        Scope(()).repository::<R>()
    }
}

// ---------------------------------------------------------------------------
// Repository providers.

/// Makes repositories available to everything below it (`flutter_bloc`'s
/// `RepositoryProvider` / `MultiRepositoryProvider`). Register a repository under its trait
/// (`dyn NotesRepository`) so tests can provide a fake.
///
/// ```no_run
/// # use std::sync::Arc;
/// # use rok_ui::{prelude::*, bloc::RepositoryProvider};
/// trait NotesRepository: Send + Sync {
///     fn count(&self) -> usize;
/// }
/// struct InMemoryNotes;
/// impl NotesRepository for InMemoryNotes {
///     fn count(&self) -> usize { 0 }
/// }
///
/// # fn app(page: AnyElement) -> impl IntoElement {
/// RepositoryProvider::new()
///     .provide::<dyn NotesRepository>(Arc::new(InMemoryNotes))
///     .child(page)
/// # }
/// ```
#[derive(IntoElement, Default)]
pub struct RepositoryProvider {
    frame: Frame,
    children: Vec<AnyElement>,
}

/// [`RepositoryProvider`] with several repositories: the same type.
pub type MultiRepositoryProvider = RepositoryProvider;

/// A set of repositories built once, at startup, and provided on every render with
/// [`RepositoryProvider::from`]. Cheap to clone.
///
/// ```no_run
/// # use std::sync::Arc;
/// # use rok_ui::{prelude::*, bloc::{RepositoryProvider, Repositories}};
/// # trait NotesRepository: Send + Sync {}
/// # struct InMemoryNotes;
/// # impl NotesRepository for InMemoryNotes {}
/// struct App {
///     repositories: Repositories,
/// }
///
/// impl Render for App {
///     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
///         RepositoryProvider::from(self.repositories.clone()).child(div())
///     }
/// }
///
/// let app = App {
///     repositories: Repositories::new().with::<dyn NotesRepository>(Arc::new(InMemoryNotes)),
/// };
/// # let _ = app;
/// ```
#[derive(Clone, Default)]
pub struct Repositories {
    frame: Frame,
}

impl Repositories {
    /// No repositories yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add `repository` as `R` (often `dyn SomeRepository`).
    #[must_use]
    pub fn with<R: ?Sized + Send + Sync + 'static>(mut self, repository: Arc<R>) -> Self {
        self.frame
            .insert(TypeId::of::<Arc<R>>(), Rc::new(repository));
        self
    }
}

impl std::fmt::Debug for Repositories {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Repositories")
            .field("count", &self.frame.len())
            .finish()
    }
}

impl From<Repositories> for RepositoryProvider {
    fn from(repositories: Repositories) -> Self {
        Self {
            frame: repositories.frame,
            children: Vec::new(),
        }
    }
}

impl RepositoryProvider {
    /// A provider with no repositories yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Provide `repository` as `R` (often `dyn SomeRepository`).
    #[must_use]
    pub fn provide<R: ?Sized + Send + Sync + 'static>(mut self, repository: Arc<R>) -> Self {
        self.frame
            .insert(TypeId::of::<Arc<R>>(), Rc::new(repository));
        self
    }
}

impl ParentElement for RepositoryProvider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for RepositoryProvider {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        ScopeElement {
            frame: Rc::new(self.frame),
            child: single(self.children),
        }
    }
}

// ---------------------------------------------------------------------------
// Bloc providers.

/// Keeps a provided bloc or cubit alive while its provider renders, re-renders windows when its
/// state changes, and closes it (cancelling its handlers) when the provider goes away.
struct Owner {
    close: Box<dyn Fn()>,
    _subscription: Subscription,
    _refresh: Task<()>,
}

impl Drop for Owner {
    fn drop(&mut self) {
        (self.close)();
    }
}

/// Re-render every window whenever `source` changes.
fn watch<O: Observable>(source: &O, cx: &mut App) -> (Subscription, Task<()>) {
    let (sender, mut changes) = mpsc::unbounded::<()>();
    let subscription = source.subscribe(move |_| {
        sender.unbounded_send(()).ok();
    });
    let task = cx.spawn(async move |cx| {
        while changes.next().await.is_some() {
            // Several changes that arrived together re-render once.
            while changes.try_recv().is_ok() {}
            if cx.update(App::refresh_windows).is_err() {
                return;
            }
        }
    });
    (subscription, task)
}

type MakeEntry = Box<dyn FnOnce(&mut Window, &mut App) -> (TypeId, Rc<dyn Any>)>;

/// Creates blocs and cubits once and makes them available below it (`flutter_bloc`'s
/// `BlocProvider` / `MultiBlocProvider`). The provider owns them: when it stops rendering, they
/// close and their running handlers are cancelled.
///
/// Read them in components rendered below the provider with `cx.bloc::<B>()` /
/// `cx.cubit::<C>()`, not in the function that builds the provider.
///
/// ```no_run
/// # use std::sync::Arc;
/// # use rok_ui::{prelude::*, bloc::{Bloc, BlocProvider, Emitter}};
/// # trait NotesRepository: Send + Sync {}
/// # struct NotesBloc { repository: Arc<dyn NotesRepository> }
/// # impl Bloc for NotesBloc {
/// #     type Event = ();
/// #     type State = usize;
/// #     fn initial_state(&self) -> usize { 0 }
/// #     async fn on(&self, _: (), _: &Emitter<usize>) {}
/// # }
/// # fn page(notes_page: AnyElement) -> impl IntoElement {
/// BlocProvider::bloc(|scope| NotesBloc { repository: scope.repository::<dyn NotesRepository>() })
///     .child(notes_page)
/// # }
/// ```
#[derive(IntoElement)]
pub struct BlocProvider {
    entries: Vec<MakeEntry>,
    children: Vec<AnyElement>,
}

/// [`BlocProvider`] with several blocs or cubits: the same type, built with
/// `MultiBlocProvider::new().bloc(..).cubit(..)`.
pub type MultiBlocProvider = BlocProvider;

impl BlocProvider {
    /// A provider with nothing to provide yet (add with [`BlocProvider::with_bloc`] and
    /// [`BlocProvider::with_cubit`]).
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Provide the bloc `create` builds, once, on the provider's first render.
    #[track_caller]
    #[must_use]
    pub fn bloc<B: Bloc>(create: impl FnOnce(&Scope) -> B + 'static) -> Self {
        Self::new().with_bloc(create)
    }

    /// Provide the cubit `create` builds, once, on the provider's first render.
    #[track_caller]
    #[must_use]
    pub fn cubit<C: Cubit>(create: impl FnOnce(&Scope) -> C + 'static) -> Self {
        Self::new().with_cubit(create)
    }

    /// Also provide the bloc `create` builds.
    #[track_caller]
    #[must_use]
    pub fn with_bloc<B: Bloc>(mut self, create: impl FnOnce(&Scope) -> B + 'static) -> Self {
        let key = entry_key::<B>(std::panic::Location::caller());
        self.entries.push(Box::new(move |window, cx| {
            let state = use_keyed_state(key, window, cx, || None::<(BlocHandle<B>, Owner)>);
            let existing = state.read(cx).as_ref().map(|(handle, _)| handle.clone());
            let handle = existing.unwrap_or_else(|| {
                let handle = BlocHandle::start(create(&Scope(())), crate::runtime::get().handle());
                let (subscription, refresh) = watch(&handle, cx);
                let closing = handle.clone();
                let owner = Owner {
                    close: Box::new(move || closing.close()),
                    _subscription: subscription,
                    _refresh: refresh,
                };
                state.update(cx, |slot| *slot = Some((handle.clone(), owner)));
                handle
            });
            (
                TypeId::of::<BlocHandle<B>>(),
                Rc::new(handle) as Rc<dyn Any>,
            )
        }));
        self
    }

    /// Also provide the cubit `create` builds.
    #[track_caller]
    #[must_use]
    pub fn with_cubit<C: Cubit>(mut self, create: impl FnOnce(&Scope) -> C + 'static) -> Self {
        let key = entry_key::<C>(std::panic::Location::caller());
        self.entries.push(Box::new(move |window, cx| {
            let state = use_keyed_state(key, window, cx, || None::<(CubitHandle<C>, Owner)>);
            let existing = state.read(cx).as_ref().map(|(handle, _)| handle.clone());
            let handle = existing.unwrap_or_else(|| {
                let handle = CubitHandle::start(create(&Scope(())), crate::runtime::get().handle());
                let (subscription, refresh) = watch(&handle, cx);
                let closing = handle.clone();
                let owner = Owner {
                    close: Box::new(move || closing.close()),
                    _subscription: subscription,
                    _refresh: refresh,
                };
                state.update(cx, |slot| *slot = Some((handle.clone(), owner)));
                handle
            });
            (
                TypeId::of::<CubitHandle<C>>(),
                Rc::new(handle) as Rc<dyn Any>,
            )
        }));
        self
    }
}

impl Default for BlocProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// The element-state key of one provided bloc: where the provider was written, and its type.
fn entry_key<T: 'static>(location: &'static std::panic::Location<'static>) -> ElementId {
    ElementId::NamedChild(
        Box::new(ElementId::CodeLocation(*location)),
        std::any::type_name::<T>().into(),
    )
}

impl ParentElement for BlocProvider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for BlocProvider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut frame = Frame::new();
        for make in self.entries {
            // Later entries see the earlier ones' repositories, never their blocs.
            let (type_id, value) = make(window, cx);
            frame.insert(type_id, value);
        }
        ScopeElement {
            frame: Rc::new(frame),
            child: single(self.children),
        }
    }
}

// ---------------------------------------------------------------------------
// Builders and listeners.

type Build<S> = Box<dyn FnOnce(&S, &mut Window, &mut App) -> AnyElement>;
type When<S> = Rc<dyn Fn(&S, &S) -> bool>;

/// Builds UI from a bloc's or cubit's state (`flutter_bloc`'s `BlocBuilder`). Windows re-render
/// when the state changes; with [`BlocBuilder::build_when`], the builder keeps showing the last
/// state it accepted.
///
/// ```no_run
/// # use rok_ui::{prelude::*, bloc::{Bloc, BlocBuilder, Emitter}};
/// # struct CounterBloc;
/// # impl Bloc for CounterBloc {
/// #     type Event = ();
/// #     type State = u32;
/// #     fn initial_state(&self) -> u32 { 0 }
/// #     async fn on(&self, _: (), _: &Emitter<u32>) {}
/// # }
/// #[component]
/// fn Count(cx: &mut Cx) -> impl IntoElement {
///     let counter = cx.bloc::<CounterBloc>();
///     BlocBuilder::new(&counter, |count, _, _| div().child(count.to_string()))
///         .build_when(|previous, current| current % 2 == 0 || previous == current)
/// }
/// ```
#[derive(IntoElement)]
pub struct BlocBuilder<O: Observable> {
    source: O,
    build: Build<O::State>,
    build_when: Option<When<O::State>>,
    key: ElementId,
}

impl<O: Observable> BlocBuilder<O> {
    /// Build with `build` from `source`'s state.
    #[track_caller]
    #[must_use]
    pub fn new<E: IntoElement>(
        source: &O,
        build: impl FnOnce(&O::State, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            source: source.clone(),
            build: Box::new(move |state, window, cx| build(state, window, cx).into_any_element()),
            build_when: None,
            key: ElementId::CodeLocation(*std::panic::Location::caller()),
        }
    }

    /// Rebuild only for states where `when(previous_shown, current)` is true (`flutter_bloc`'s
    /// `buildWhen`); otherwise keep showing the last accepted state.
    #[must_use]
    pub fn build_when(mut self, when: impl Fn(&O::State, &O::State) -> bool + 'static) -> Self {
        self.build_when = Some(Rc::new(when));
        self
    }

    /// Key its state, for builders created in a loop.
    #[must_use]
    pub fn key(mut self, key: impl Into<ElementId>) -> Self {
        self.key = key.into();
        self
    }
}

impl<O: Observable> RenderOnce for BlocBuilder<O> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let current = self.source.state();
        let shown = match self.build_when {
            None => current,
            Some(when) => {
                let shown = use_keyed_state(self.key, window, cx, || current.clone());
                let previous = shown.read(cx).clone();
                if previous != current && when(&previous, &current) {
                    shown.update(cx, |shown| *shown = current.clone());
                }
                shown.read(cx).clone()
            }
        };
        (self.build)(&shown, window, cx)
    }
}

/// Builds UI from part of a state (`flutter_bloc`'s `BlocSelector`): `select` picks the part,
/// `build` gets only that.
///
/// ```no_run
/// # use rok_ui::{prelude::*, bloc::{Bloc, BlocSelector, Emitter}};
/// # #[derive(Clone, Debug, PartialEq)] struct NotesState { notes: Vec<String> }
/// # struct NotesBloc;
/// # impl Bloc for NotesBloc {
/// #     type Event = ();
/// #     type State = NotesState;
/// #     fn initial_state(&self) -> NotesState { NotesState { notes: Vec::new() } }
/// #     async fn on(&self, _: (), _: &Emitter<NotesState>) {}
/// # }
/// #[component]
/// fn NoteCount(cx: &mut Cx) -> impl IntoElement {
///     let notes = cx.bloc::<NotesBloc>();
///     BlocSelector::new(&notes, |state| state.notes.len(), |count, _, _| div().child(format!("{count} notes")))
/// }
/// ```
#[derive(IntoElement)]
pub struct BlocSelector<O: Observable> {
    build: Build<O::State>,
    source: O,
}

impl<O: Observable> BlocSelector<O> {
    /// Show `build(select(state))`.
    #[must_use]
    pub fn new<T: 'static, E: IntoElement>(
        source: &O,
        select: impl Fn(&O::State) -> T + 'static,
        build: impl FnOnce(&T, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            build: Box::new(move |state, window, cx| {
                build(&select(state), window, cx).into_any_element()
            }),
            source: source.clone(),
        }
    }
}

impl<O: Observable> RenderOnce for BlocSelector<O> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        (self.build)(&self.source.state(), window, cx)
    }
}

type Listen<S> = Rc<dyn Fn(&S, &mut Window, &mut App)>;

/// The callbacks a listener runs, replaced on every render so they see fresh captures.
struct Callbacks<S> {
    listen: Listen<S>,
    when: Option<When<S>>,
}

/// One listener's subscription and the task that delivers its states.
struct Listening<S> {
    callbacks: Rc<RefCell<Callbacks<S>>>,
    _subscription: Subscription,
    _task: Task<()>,
}

/// Runs a callback once per state change, for things that happen once: navigating, a toast,
/// a dialog, or telling another bloc (`flutter_bloc`'s `BlocListener`). The callback runs on the
/// UI thread with the window. Its child renders as is.
///
/// This is how one bloc reacts to another: the view listens to one and adds an event to the
/// other.
///
/// ```no_run
/// # use rok_ui::{prelude::*, bloc::{Bloc, BlocListener, Emitter}};
/// # #[derive(Clone, Debug, PartialEq)] enum AuthState { SignedIn, SignedOut }
/// # struct AuthBloc;
/// # impl Bloc for AuthBloc {
/// #     type Event = ();
/// #     type State = AuthState;
/// #     fn initial_state(&self) -> AuthState { AuthState::SignedOut }
/// #     async fn on(&self, _: (), _: &Emitter<AuthState>) {}
/// # }
/// # #[derive(Debug)] enum CartEvent { CartCleared }
/// # struct CartBloc;
/// # impl Bloc for CartBloc {
/// #     type Event = CartEvent;
/// #     type State = u32;
/// #     fn initial_state(&self) -> u32 { 0 }
/// #     async fn on(&self, _: CartEvent, _: &Emitter<u32>) {}
/// # }
/// // Signing out empties the cart: the view connects the two blocs.
/// #[component]
/// fn Shell(#[children] children: Vec<AnyElement>, cx: &mut Cx) -> impl IntoElement {
///     let auth = cx.bloc::<AuthBloc>();
///     let cart = cx.bloc::<CartBloc>();
///     BlocListener::new(&auth, move |_, _, _| cart.add(CartEvent::CartCleared))
///         .listen_when(|_, current| *current == AuthState::SignedOut)
///         .children(children)
/// }
/// ```
#[derive(IntoElement)]
pub struct BlocListener<O: Observable> {
    source: O,
    listen: Listen<O::State>,
    when: Option<When<O::State>>,
    key: ElementId,
    children: Vec<AnyElement>,
}

impl<O: Observable> BlocListener<O> {
    /// Call `listen` for every state change of `source`.
    #[track_caller]
    #[must_use]
    pub fn new(source: &O, listen: impl Fn(&O::State, &mut Window, &mut App) + 'static) -> Self {
        Self {
            source: source.clone(),
            listen: Rc::new(listen),
            when: None,
            key: ElementId::CodeLocation(*std::panic::Location::caller()),
            children: Vec::new(),
        }
    }

    /// Only for changes where `when(previous, current)` is true (`flutter_bloc`'s
    /// `listenWhen`).
    #[must_use]
    pub fn listen_when(mut self, when: impl Fn(&O::State, &O::State) -> bool + 'static) -> Self {
        self.when = Some(Rc::new(when));
        self
    }

    /// Key its subscription, for listeners created in a loop.
    #[must_use]
    pub fn key(mut self, key: impl Into<ElementId>) -> Self {
        self.key = key.into();
        self
    }
}

impl<O: Observable> ParentElement for BlocListener<O> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl<O: Observable> RenderOnce for BlocListener<O> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let callbacks = Callbacks {
            listen: self.listen,
            when: self.when,
        };
        let source = self.source;
        let state = use_keyed_state(self.key, window, cx, || None::<Listening<O::State>>);
        let existing = state
            .read(cx)
            .as_ref()
            .map(|listening| listening.callbacks.clone());
        if let Some(existing) = existing {
            *existing.borrow_mut() = callbacks;
        } else {
            let callbacks = Rc::new(RefCell::new(callbacks));
            let (sender, mut states) = mpsc::unbounded::<O::State>();
            let subscription = source.subscribe(move |state| {
                sender.unbounded_send(state.clone()).ok();
            });
            let mut previous = source.state();
            let delivered = callbacks.clone();
            let task = window.spawn(cx, async move |cx| {
                while let Some(current) = states.next().await {
                    let callbacks = delivered.clone();
                    let accepted = callbacks
                        .borrow()
                        .when
                        .as_ref()
                        .is_none_or(|when| when(&previous, &current));
                    if accepted {
                        let listen = callbacks.borrow().listen.clone();
                        if cx
                            .update(|window, cx| listen(&current, window, cx))
                            .is_err()
                        {
                            return;
                        }
                    }
                    previous = current;
                }
            });
            state.update(cx, |slot| {
                *slot = Some(Listening {
                    callbacks,
                    _subscription: subscription,
                    _task: task,
                });
            });
        }
        single(self.children)
    }
}

/// A [`BlocListener`] and a [`BlocBuilder`] on the same bloc (`flutter_bloc`'s
/// `BlocConsumer`).
///
/// ```no_run
/// # use rok_ui::{prelude::*, bloc::{Bloc, BlocConsumer, Emitter}};
/// # #[derive(Clone, Debug, PartialEq)] enum SaveState { Idle, Saved }
/// # struct SaveBloc;
/// # impl Bloc for SaveBloc {
/// #     type Event = ();
/// #     type State = SaveState;
/// #     fn initial_state(&self) -> SaveState { SaveState::Idle }
/// #     async fn on(&self, _: (), _: &Emitter<SaveState>) {}
/// # }
/// #[component]
/// fn SaveStatus(cx: &mut Cx) -> impl IntoElement {
///     let save = cx.bloc::<SaveBloc>();
///     BlocConsumer::new(
///         &save,
///         |state, _, cx| {
///             if *state == SaveState::Saved {
///                 toast(cx, Toast::success("Saved"));
///             }
///         },
///         |state, _, _| div().child(if *state == SaveState::Saved { "Saved" } else { "Editing" }),
///     )
/// }
/// ```
#[derive(IntoElement)]
pub struct BlocConsumer<O: Observable> {
    listener: BlocListener<O>,
    builder: BlocBuilder<O>,
}

impl<O: Observable> BlocConsumer<O> {
    /// Listen with `listen` and build with `build`.
    #[track_caller]
    #[must_use]
    pub fn new<E: IntoElement>(
        source: &O,
        listen: impl Fn(&O::State, &mut Window, &mut App) + 'static,
        build: impl FnOnce(&O::State, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        let location = *std::panic::Location::caller();
        let mut listener = BlocListener::new(source, listen);
        listener.key = ElementId::NamedChild(
            Box::new(ElementId::CodeLocation(location)),
            "listener".into(),
        );
        let mut builder = BlocBuilder::new(source, build);
        builder.key = ElementId::NamedChild(
            Box::new(ElementId::CodeLocation(location)),
            "builder".into(),
        );
        Self { listener, builder }
    }

    /// See [`BlocListener::listen_when`].
    #[must_use]
    pub fn listen_when(mut self, when: impl Fn(&O::State, &O::State) -> bool + 'static) -> Self {
        self.listener = self.listener.listen_when(when);
        self
    }

    /// See [`BlocBuilder::build_when`].
    #[must_use]
    pub fn build_when(mut self, when: impl Fn(&O::State, &O::State) -> bool + 'static) -> Self {
        self.builder = self.builder.build_when(when);
        self
    }
}

impl<O: Observable> RenderOnce for BlocConsumer<O> {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.listener.child(self.builder)
    }
}
