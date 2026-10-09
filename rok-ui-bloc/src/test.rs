//! Test helpers: events (or cubit calls) in, states out, like `bloc_test`.
//!
//! ```
//! use rok_ui_bloc::{test, Bloc, Emitter};
//!
//! struct Echo;
//!
//! impl Bloc for Echo {
//!     type Event = &'static str;
//!     type State = String;
//!
//!     fn initial_state(&self) -> String {
//!         String::new()
//!     }
//!
//!     async fn on(&self, event: &'static str, emit: &Emitter<String>) {
//!         emit.emit(event.to_string());
//!     }
//! }
//!
//! assert_eq!(test::run(Echo, ["a", "a", "b"]), ["a", "b"], "equal states are not repeated");
//! ```

use std::sync::{Arc, Mutex, PoisonError};

use crate::{Bloc, BlocHandle, Cubit, CubitHandle, Observable};

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a test runtime")
}

/// Start `bloc`, add every event, wait until all handlers finish, and return the states it
/// emitted (not the initial one). Runs on a runtime of its own.
///
/// # Panics
///
/// Panics if the test runtime cannot start.
pub fn run<B: Bloc>(bloc: B, events: impl IntoIterator<Item = B::Event>) -> Vec<B::State> {
    let runtime = runtime();
    let (handle, states, _subscription) = start(bloc, runtime.handle());
    for event in events {
        handle.add(event);
    }
    runtime.block_on(handle.idle());
    handle.close();
    take(&states)
}

/// Like [`run`], but waits for each event's handlers to finish before adding the next, so the
/// states do not depend on timing (`Droppable` and `Restartable` events never overlap).
///
/// # Panics
///
/// Panics if the test runtime cannot start.
pub fn run_settled<B: Bloc>(bloc: B, events: impl IntoIterator<Item = B::Event>) -> Vec<B::State> {
    let runtime = runtime();
    let (handle, states, _subscription) = start(bloc, runtime.handle());
    for event in events {
        handle.add(event);
        runtime.block_on(handle.idle());
    }
    handle.close();
    take(&states)
}

/// Start `cubit`, call its methods in `act`, wait until the work they spawned finishes, and
/// return the states it emitted (not the initial one). Runs on a runtime of its own.
///
/// ```
/// use rok_ui_bloc::{test, Cubit, Emitter};
///
/// struct Counter(Emitter<u32>);
///
/// impl Counter {
///     fn increment(&self) {
///         let emit = self.0.clone();
///         self.0.spawn(async move {
///             emit.update(|count| *count += 1);
///         });
///     }
/// }
///
/// impl Cubit for Counter {
///     type State = u32;
///
///     fn emitter(&self) -> &Emitter<u32> {
///         &self.0
///     }
/// }
///
/// let states = test::run_cubit(Counter(Emitter::new(0)), |counter| {
///     counter.increment();
///     counter.increment();
/// });
/// assert_eq!(states.last(), Some(&2));
/// ```
///
/// # Panics
///
/// Panics if the test runtime cannot start.
pub fn run_cubit<C: Cubit>(cubit: C, act: impl FnOnce(&C)) -> Vec<C::State> {
    let runtime = runtime();
    let handle = CubitHandle::start(cubit, runtime.handle());
    let states: Recorded<C::State> = Arc::default();
    let recorder = states.clone();
    let subscription = handle.subscribe(move |state| {
        recorder
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(state.clone());
    });
    {
        let _context = runtime.enter();
        act(&handle);
    }
    runtime.block_on(handle.emitter().idle());
    handle.close();
    drop(subscription);
    take(&states)
}

type Recorded<S> = Arc<Mutex<Vec<S>>>;

fn start<B: Bloc>(
    bloc: B,
    runtime: &tokio::runtime::Handle,
) -> (BlocHandle<B>, Recorded<B::State>, crate::Subscription) {
    let handle = BlocHandle::start(bloc, runtime);
    let states: Recorded<B::State> = Arc::default();
    let recorder = states.clone();
    let subscription = handle.subscribe(move |state| {
        recorder
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(state.clone());
    });
    (handle, states, subscription)
}

fn take<S>(states: &Recorded<S>) -> Vec<S> {
    std::mem::take(&mut *states.lock().unwrap_or_else(PoisonError::into_inner))
}

/// A test for a bloc: build it, add events, and compare the states it emitted (`bloc_test`).
///
/// ```
/// use rok_ui_bloc::{bloc_test, Bloc, Emitter};
///
/// struct Counter;
///
/// impl Bloc for Counter {
///     type Event = i32;
///     type State = i32;
///
///     fn initial_state(&self) -> i32 {
///         0
///     }
///
///     async fn on(&self, by: i32, emit: &Emitter<i32>) {
///         emit.update(|count| *count += by);
///     }
/// }
///
/// bloc_test! {
///     /// Adding counts up; adding zero emits nothing.
///     adding_counts_up,
///     build: Counter,
///     act: [1, 2, 0],
///     expect: [1, 3],
/// }
///
/// bloc_test! {
///     // `settle: true` waits for each event before adding the next (for timed modes).
///     each_event_settles,
///     build: Counter,
///     settle: true,
///     act: [5],
///     verify: |states: Vec<i32>| assert_eq!(states.last(), Some(&5)),
/// }
/// ```
///
/// It expands to a `#[test]` function named after the first argument: `expect` compares the
/// emitted states with `assert_eq!`, `verify` gets them to check as it likes. Events are added
/// with [`run`] (or [`run_settled`] with `settle: true`).
#[macro_export]
macro_rules! bloc_test {
    (
        $(#[$meta:meta])*
        $name:ident,
        build: $build:expr,
        $(settle: $settle:expr,)?
        act: [$($event:expr),* $(,)?],
        expect: [$($state:expr),* $(,)?] $(,)?
    ) => {
        $(#[$meta])*
        #[test]
        fn $name() {
            let states = $crate::bloc_test!(@run $build, [$($event),*] $(, $settle)?);
            ::std::assert_eq!(states, ::std::vec![$($state),*]);
        }
    };
    (
        $(#[$meta:meta])*
        $name:ident,
        build: $build:expr,
        $(settle: $settle:expr,)?
        act: [$($event:expr),* $(,)?],
        verify: $verify:expr $(,)?
    ) => {
        $(#[$meta])*
        #[test]
        fn $name() {
            let states = $crate::bloc_test!(@run $build, [$($event),*] $(, $settle)?);
            let verify = $verify;
            verify(states);
        }
    };
    (@run $build:expr, [$($event:expr),*]) => {
        $crate::test::run($build, [$($event),*])
    };
    (@run $build:expr, [$($event:expr),*], $settle:expr) => {
        if $settle {
            $crate::test::run_settled($build, [$($event),*])
        } else {
            $crate::test::run($build, [$($event),*])
        }
    };
}
