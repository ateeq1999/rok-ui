//! Rebuilding and reacting to state: `BlocBuilder`, `BlocSelector`, `BlocListener` and
//! `BlocConsumer`.

use std::{cell::RefCell, rc::Rc};

use futures::{channel::mpsc, StreamExt as _};
use gpui::{prelude::*, AnyElement, App, ElementId, Task, Window};
use rok_ui_bloc::{Observable, Subscription};

use crate::{hooks::use_keyed_state, scope::single};

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
