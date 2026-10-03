//! Hierarchical query keys.

use std::{fmt, sync::Arc};

use gpui::SharedString;

/// Identifies cached data: a list of parts, such as `["notes", "3"]`.
///
/// Keys are hierarchical: [`invalidate`](super::invalidate) with `["notes"]` affects every key
/// that starts with it (`["notes", "3"]`, `["notes", "list", ..]`). Build keys with
/// [`query_key!`](crate::query_key):
///
/// ```
/// use rok_ui::query_key;
///
/// let list = query_key!["notes", "list", 2];
/// assert!(list.starts_with(&query_key!["notes"]));
/// assert_eq!(list.to_string(), "notes/list/2");
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct QueryKey(Arc<[SharedString]>);

impl QueryKey {
    /// A key from its parts.
    #[must_use]
    pub fn from_parts(parts: Vec<SharedString>) -> Self {
        Self(parts.into())
    }

    /// The empty key. It is a prefix of every key, so invalidating it invalidates everything.
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// The parts, outermost first.
    #[must_use]
    pub fn parts(&self) -> &[SharedString] {
        &self.0
    }

    /// Whether `prefix`'s parts are the first parts of this key.
    #[must_use]
    pub fn starts_with(&self, prefix: &QueryKey) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// This key with one more part.
    #[must_use]
    pub fn child(&self, part: &impl fmt::Debug) -> Self {
        let mut parts = self.0.to_vec();
        parts.push(key_part(part));
        Self(parts.into())
    }
}

impl fmt::Display for QueryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, part) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str("/")?;
            }
            formatter.write_str(part)?;
        }
        Ok(())
    }
}

impl fmt::Debug for QueryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.0.iter()).finish()
    }
}

/// One key part: the value's `Debug` text, without the quotes strings get.
#[doc(hidden)]
pub fn key_part(value: &impl fmt::Debug) -> SharedString {
    let text = format!("{value:?}");
    match text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(inner) => inner.to_string().into(),
        None => text.into(),
    }
}

/// Build a [`QueryKey`](crate::query::QueryKey) from parts. Any `Debug` value is a part:
/// strings, numbers, ids and search structs.
///
/// ```
/// # use rok_ui::query_key;
/// let id = 7_u64;
/// assert_eq!(query_key!["notes", id].to_string(), "notes/7");
/// ```
#[macro_export]
macro_rules! query_key {
    [$($part:expr),* $(,)?] => {
        $crate::query::QueryKey::from_parts(::std::vec![$($crate::query::key::key_part(&$part)),*])
    };
}
