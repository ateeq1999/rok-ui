//! Bloc providers: create blocs and cubits for the elements below, and close them when the
//! provider goes away.

use std::{
    any::{Any, TypeId},
    rc::Rc,
};

use futures::{channel::mpsc, StreamExt as _};
use gpui::{prelude::*, AnyElement, App, ElementId, Task, Window};
use rok_ui_bloc::{Bloc, BlocHandle, Cubit, CubitHandle, Observable, Subscription};

use super::Scope;
use crate::{
    hooks::use_keyed_state,
    scope::{single, Frame, ScopeElement},
};

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
pub(super) fn watch<O: Observable>(source: &O, cx: &mut App) -> (Subscription, Task<()>) {
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
