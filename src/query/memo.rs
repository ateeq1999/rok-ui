//! Memoized async helpers: concurrent callers share one in-flight future.

use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    future::Future,
    sync::{LazyLock, Mutex},
};

use futures::future::{FutureExt, Shared};

use super::options::BoxFuture;

type Memoized = Box<dyn Any + Send + Sync>;

static MEMO: LazyLock<Mutex<HashMap<String, Memoized>>> = LazyLock::new(Mutex::default);

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
    let mut memo = MEMO
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(future) = memo
        .get(&key)
        .and_then(|cached| cached.downcast_ref::<MemoFuture<T>>())
    {
        return future.clone();
    }
    let boxed: BoxFuture<T> = Box::pin(make());
    let future = boxed.shared();
    memo.insert(key, Box::new(future.clone()));
    future
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
