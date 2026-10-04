//! Memoized async helpers: concurrent callers share one in-flight future.

use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    future::Future,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use futures::future::{FutureExt, Shared};

use super::options::BoxFuture;

/// A cached future, how long it is kept, and when it was made.
struct Memoized {
    future: Box<dyn Any + Send + Sync>,
    scope: MemoScope,
    made: Instant,
}

impl Memoized {
    fn is_expired(&self) -> bool {
        matches!(self.scope, MemoScope::For(ttl) if self.made.elapsed() >= ttl)
    }
}

static MEMO: LazyLock<Mutex<HashMap<String, Memoized>>> = LazyLock::new(Mutex::default);

/// How long a memoized result is kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoScope {
    /// Until [`invalidate`] or [`clear`] removes it (the default).
    #[default]
    App,
    /// Until the next navigation (with the `router` feature; otherwise like `App`): for
    /// lookups that loaders and guards of one page share.
    Navigation,
    /// For this long after the work started.
    For(Duration),
}

/// A memoized future: cloning it, or awaiting it from several places, runs the work once.
pub type MemoFuture<T> = Shared<BoxFuture<T>>;

/// The future cached under `key`, or a new one from `make`. The first call runs the work;
/// later and concurrent calls share its result until [`invalidate`] removes it.
///
/// [`memoize`](crate::memoize) (the attribute) writes this call for you.
pub fn memoize<T, Fut>(key: String, make: impl FnOnce() -> Fut) -> MemoFuture<T>
where
    T: Clone + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
{
    memoize_in(key, MemoScope::App, make)
}

/// [`memoize`] with a [`MemoScope`]: `#[memoize(scope = navigation)]` and
/// `#[memoize(ttl_ms = 5000)]` write this call.
pub fn memoize_in<T, Fut>(
    key: String,
    scope: MemoScope,
    make: impl FnOnce() -> Fut,
) -> MemoFuture<T>
where
    T: Clone + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
{
    let mut memo = MEMO
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(future) = memo
        .get(&key)
        .filter(|cached| !cached.is_expired())
        .and_then(|cached| cached.future.downcast_ref::<MemoFuture<T>>())
    {
        return future.clone();
    }
    let boxed: BoxFuture<T> = Box::pin(make());
    let future = boxed.shared();
    memo.insert(
        key,
        Memoized {
            future: Box::new(future.clone()),
            scope,
            made: Instant::now(),
        },
    );
    future
}

/// Forget results memoized with [`MemoScope::Navigation`]. The router calls this after every
/// navigation.
pub fn end_navigation() {
    MEMO.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .retain(|_, cached| cached.scope != MemoScope::Navigation);
}

/// The cache key for a memoized function called with `arguments`.
#[doc(hidden)]
pub fn key(path: &str, name: &str, arguments: &impl Debug) -> String {
    format!("{path}::{name}{arguments:?}")
}

/// Forget memoized results whose key starts with `prefix`: a module path, or
/// `"my_app::session::current_user"` for one function.
pub fn invalidate(prefix: &str) {
    MEMO.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .retain(|key, _| !key.starts_with(prefix));
}

/// Forget every memoized result.
pub fn clear() {
    MEMO.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}
