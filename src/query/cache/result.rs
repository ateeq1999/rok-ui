//! [`QueryResult`]: what a query hook returns.

use std::{rc::Rc, time::Instant};

#[cfg(doc)]
use super::use_query;
use crate::query::QueryError;

/// The state of a query for one reader, returned by [`use_query`].
pub struct QueryResult<T> {
    pub(super) data: Option<Rc<T>>,
    pub(super) error: Option<QueryError>,
    pub(super) is_fetching: bool,
    pub(super) is_placeholder: bool,
    pub(super) is_stale: bool,
    pub(super) updated_at: Option<Instant>,
}

/// A query's status, for `match`ing in a component.
#[derive(Debug)]
pub enum QueryState<'a, T> {
    /// No data yet; the first fetch is running (or the query is disabled).
    Pending,
    /// No data, and the last fetch failed.
    Error(&'a QueryError),
    /// Data, possibly stale or from a placeholder while a fetch runs.
    Success(&'a T),
}

impl<T> QueryResult<T> {
    /// The status: pending, an error, or data.
    #[must_use]
    pub fn state(&self) -> QueryState<'_, T> {
        match (&self.data, &self.error) {
            (Some(data), _) => QueryState::Success(data),
            (None, Some(error)) => QueryState::Error(error),
            (None, None) => QueryState::Pending,
        }
    }

    /// The data, if there is any. Kept while a refetch runs, so lists do not flash empty.
    #[must_use]
    pub fn data(&self) -> Option<&T> {
        self.data.as_deref()
    }

    /// The data as a shared handle.
    #[must_use]
    pub fn data_rc(&self) -> Option<Rc<T>> {
        self.data.clone()
    }

    /// The error of the last fetch, if it failed.
    #[must_use]
    pub fn error(&self) -> Option<&QueryError> {
        self.error.as_ref()
    }

    /// No data and no error yet.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.data.is_none() && self.error.is_none()
    }

    /// Whether data is available.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.data.is_some()
    }

    /// Whether the last fetch failed.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Whether a fetch is running (the first one or a background refetch).
    #[must_use]
    pub fn is_fetching(&self) -> bool {
        self.is_fetching
    }

    /// Pending with a fetch running: show a spinner.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.is_pending() && self.is_fetching
    }

    /// Whether the data is placeholder or previous-key data rather than this key's.
    #[must_use]
    pub fn is_placeholder_data(&self) -> bool {
        self.is_placeholder
    }

    /// Whether the data is older than the query's `stale_time` or was invalidated.
    #[must_use]
    pub fn is_stale(&self) -> bool {
        self.is_stale
    }

    /// When the data or error was last stored.
    #[must_use]
    pub fn updated_at(&self) -> Option<Instant> {
        self.updated_at
    }

    /// The data, or why there is none, for `?` inside [`Suspense`](crate::query::Suspense).
    ///
    /// # Errors
    ///
    /// [`Suspend::Pending`](crate::query::Suspend::Pending) without data and error, and
    /// [`Suspend::Failed`](crate::query::Suspend::Failed) when the fetch failed and there is no data.
    pub fn suspend(self) -> Result<Rc<T>, crate::query::Suspend> {
        match (self.data, self.error) {
            (Some(data), _) => Ok(data),
            (None, Some(error)) => Err(crate::query::Suspend::Failed(error)),
            (None, None) => Err(crate::query::Suspend::Pending),
        }
    }
}

impl<T> Clone for QueryResult<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            error: self.error.clone(),
            is_fetching: self.is_fetching,
            is_placeholder: self.is_placeholder,
            is_stale: self.is_stale,
            updated_at: self.updated_at,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for QueryResult<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QueryResult")
            .field("data", &self.data)
            .field("error", &self.error)
            .field("is_fetching", &self.is_fetching)
            .finish_non_exhaustive()
    }
}
