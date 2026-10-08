//! Test helpers: events in, states out (`bloc_test`).
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

use crate::{Bloc, BlocHandle, Observable};

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
