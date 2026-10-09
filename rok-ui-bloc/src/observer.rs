//! [`BlocObserver`]: one place to see every bloc and cubit, for logging, analytics and
//! errors.

use std::{
    any::Any,
    fmt::Debug,
    future::Future,
    panic::{catch_unwind, AssertUnwindSafe},
    pin::Pin,
    sync::{Arc, PoisonError, RwLock},
    task::{Context, Poll},
};

/// Sees what every bloc and cubit does. Set one for the process with [`set_observer`];
/// every method has an empty default, so implement only what you need.
///
/// `bloc` is the bloc's type name (`my_app::features::notes::bloc::notes_bloc::NotesBloc`).
/// Methods run on the thread that caused them (often a runtime worker): keep them quick and
/// never block.
///
/// ```
/// use std::fmt::Debug;
///
/// use rok_ui_bloc::{set_observer, BlocObserver};
///
/// struct Printer;
///
/// impl BlocObserver for Printer {
///     fn on_event(&self, bloc: &'static str, event: &dyn Debug) {
///         println!("{bloc} <- {event:?}");
///     }
///
///     fn on_error(&self, bloc: &'static str, error: &str) {
///         eprintln!("{bloc} failed: {error}");
///     }
/// }
///
/// set_observer(Printer);
/// # rok_ui_bloc::clear_observer();
/// ```
pub trait BlocObserver: Send + Sync + 'static {
    /// A bloc or cubit started.
    fn on_create(&self, bloc: &'static str) {
        let _ = bloc;
    }

    /// An event was added to a bloc (before it is handled, and even if its concurrency mode
    /// drops it).
    fn on_event(&self, bloc: &'static str, event: &dyn Debug) {
        let _ = (bloc, event);
    }

    /// The state changed from `current` to `next` (equal states are never emitted).
    fn on_change(&self, bloc: &'static str, current: &dyn Debug, next: &dyn Debug) {
        let _ = (bloc, current, next);
    }

    /// A handler panicked. The bloc keeps running; the event that panicked is lost.
    fn on_error(&self, bloc: &'static str, error: &str) {
        let _ = (bloc, error);
    }

    /// A bloc or cubit was closed.
    fn on_close(&self, bloc: &'static str) {
        let _ = bloc;
    }
}

static OBSERVER: RwLock<Option<Arc<dyn BlocObserver>>> = RwLock::new(None);

/// Make `observer` see every bloc and cubit in the process, replacing the previous one. Set
/// it once, at startup.
pub fn set_observer(observer: impl BlocObserver) {
    *OBSERVER.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(observer));
}

/// Remove the observer.
pub fn clear_observer() {
    *OBSERVER.write().unwrap_or_else(PoisonError::into_inner) = None;
}

/// The current observer.
pub(crate) fn current() -> Option<Arc<dyn BlocObserver>> {
    OBSERVER
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Prints every event, change and error to standard error: a quick way to see what the app
/// does. `set_observer(LogObserver)`.
#[derive(Clone, Copy, Debug, Default)]
pub struct LogObserver;

impl BlocObserver for LogObserver {
    fn on_event(&self, bloc: &'static str, event: &dyn Debug) {
        eprintln!("[bloc] {} <- {event:?}", short(bloc));
    }

    fn on_change(&self, bloc: &'static str, current: &dyn Debug, next: &dyn Debug) {
        eprintln!("[bloc] {}: {current:?} -> {next:?}", short(bloc));
    }

    fn on_error(&self, bloc: &'static str, error: &str) {
        eprintln!("[bloc] {} panicked: {error}", short(bloc));
    }
}

/// `NotesBloc` for `my_app::features::notes::bloc::notes_bloc::NotesBloc`.
pub(crate) fn short(name: &str) -> &str {
    let without_generics = name.split('<').next().unwrap_or(name);
    without_generics
        .rsplit("::")
        .next()
        .unwrap_or(without_generics)
}

/// Run `work`, reporting a panic to the observer instead of unwinding into the runtime.
pub(crate) async fn guarded(bloc: &'static str, work: impl Future<Output = ()> + Send) {
    if let Err(payload) = (CatchUnwind {
        inner: Box::pin(work),
    })
    .await
    {
        let message = panic_message(payload.as_ref());
        if let Some(observer) = current() {
            observer.on_error(bloc, &message);
        }
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a handler panicked".into())
}

struct CatchUnwind<F> {
    inner: Pin<Box<F>>,
}

impl<F: Future<Output = ()>> Future for CatchUnwind<F> {
    type Output = Result<(), Box<dyn Any + Send>>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.inner.as_mut();
        match catch_unwind(AssertUnwindSafe(|| inner.poll(context))) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(())) => Poll::Ready(Ok(())),
            Err(payload) => Poll::Ready(Err(payload)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::short;

    #[test]
    fn names_are_shortened() {
        assert_eq!(short("app::features::notes::NotesBloc"), "NotesBloc");
        assert_eq!(short("app::Wrapper<app::Inner>"), "Wrapper");
        assert_eq!(short("Plain"), "Plain");
    }
}
