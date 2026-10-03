//! The shared background runtime.
//!
//! GPUI runs components on the UI thread and has no tokio reactor. Async work
//! that needs one (rok-db, HTTP clients, queries, procedures) runs on a single
//! multi-threaded tokio runtime that rok-ui starts on first use. Every async
//! feature (`db`, `query`) shares it, so an app never starts two.
//!
//! ```no_run
//! let answer = rok_ui::runtime::block_on(async { 6 * 7 });
//! assert_eq!(answer, 42);
//! ```

use std::{future::Future, sync::OnceLock};

/// The runtime, started with two worker threads on first use.
///
/// # Panics
///
/// Panics if the operating system refuses to start the worker threads.
pub fn get() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("rok-ui-runtime")
            .enable_all()
            .build()
            .expect("failed to start the rok-ui runtime")
    })
}

/// Run `future` on the shared runtime. The handle resolves on any executor,
/// including GPUI's.
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    get().spawn(future)
}

/// Block the current thread on `future`. For tests and startup code only; never
/// call it from a component or an event handler.
pub fn block_on<F: Future>(future: F) -> F::Output {
    get().block_on(future)
}
