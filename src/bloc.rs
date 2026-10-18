#![doc = include_str!("../docs/guide/architecture.md")]

use std::sync::Arc;

pub use rok_ui_bloc::{
    bloc_test, clear_observer, set_observer, test, Bloc, BlocHandle, BlocObserver, BoxFuture,
    Concurrency, Cubit, CubitHandle, Emitter, LogObserver, Observable, Subscription,
};

use crate::scope::find;

#[cfg(feature = "persist")]
mod hydrated;
mod provider;
mod repository;
mod widgets;

#[cfg(feature = "persist")]
pub use hydrated::Hydrated;

pub use provider::{BlocProvider, MultiBlocProvider};
pub use repository::{MultiRepositoryProvider, Repositories, RepositoryProvider};
pub use widgets::{BlocBuilder, BlocConsumer, BlocListener, BlocSelector};

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
