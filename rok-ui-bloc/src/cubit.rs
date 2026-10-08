//! [`Cubit`]: methods in, states out, and [`CubitHandle`], the running cubit.

use std::{marker::PhantomData, ops::Deref, sync::Arc};

use tokio::runtime::Handle;

use crate::{Emitter, Observable};

/// A bloc without events: methods change the state through the cubit's [`Emitter`]. Use it
/// when the logic is a few direct actions (a counter, a toggle, a filter); use a
/// [`Bloc`](crate::Bloc) when events need concurrency control or a log.
///
/// ```
/// use rok_ui_bloc::{Cubit, Emitter};
///
/// struct CounterCubit {
///     state: Emitter<i32>,
/// }
///
/// impl CounterCubit {
///     fn new() -> Self {
///         Self { state: Emitter::new(0) }
///     }
///
///     fn increment(&self) {
///         self.state.update(|count| *count += 1);
///     }
/// }
///
/// impl Cubit for CounterCubit {
///     type State = i32;
///
///     fn emitter(&self) -> &Emitter<i32> {
///         &self.state
///     }
/// }
///
/// let counter = CounterCubit::new();
/// counter.increment();
/// assert_eq!(counter.emitter().state(), 1);
/// ```
pub trait Cubit: Send + Sync + 'static {
    /// What the view shows: an immutable value.
    type State: Clone + PartialEq + Send + Sync + 'static;

    /// The cubit's emitter, created in its constructor with the initial state.
    fn emitter(&self) -> &Emitter<Self::State>;
}

/// A running cubit. Derefs to the cubit, so views call its methods directly. Not `Send`, like
/// [`BlocHandle`](crate::BlocHandle).
pub struct CubitHandle<C: Cubit> {
    cubit: Arc<C>,
    _not_send: PhantomData<*const ()>,
}

impl<C: Cubit> Clone for CubitHandle<C> {
    fn clone(&self) -> Self {
        Self {
            cubit: self.cubit.clone(),
            _not_send: PhantomData,
        }
    }
}

impl<C: Cubit> std::fmt::Debug for CubitHandle<C> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CubitHandle")
            .field("cubit", &std::any::type_name::<C>())
            .finish()
    }
}

impl<C: Cubit> CubitHandle<C> {
    /// Start `cubit`; work it spawns through its emitter runs on `runtime`.
    #[must_use]
    pub fn start(cubit: C, runtime: &Handle) -> Self {
        cubit.emitter().attach(runtime.clone());
        Self {
            cubit: Arc::new(cubit),
            _not_send: PhantomData,
        }
    }

    /// Close the cubit: spawned work is cancelled and later emits are ignored.
    pub fn close(&self) {
        self.cubit.emitter().close();
    }

    /// The cubit, shared (to move into a background task).
    #[must_use]
    pub fn shared(&self) -> Arc<C> {
        self.cubit.clone()
    }
}

impl<C: Cubit> Deref for CubitHandle<C> {
    type Target = C;

    fn deref(&self) -> &C {
        &self.cubit
    }
}

impl<C: Cubit> Observable for CubitHandle<C> {
    type State = C::State;

    fn emitter(&self) -> &Emitter<C::State> {
        self.cubit.emitter()
    }
}
