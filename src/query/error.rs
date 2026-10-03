//! Errors from queries and mutations.

use std::{error::Error, fmt, sync::Arc};

use gpui::SharedString;

/// Why a query produced no data. Cheap to clone; keeps the original error for
/// [`QueryError::downcast_ref`].
///
/// Any `std::error::Error` converts with `?`. For plain messages use [`QueryError::msg`].
#[derive(Clone)]
pub struct QueryError {
    message: SharedString,
    source: Option<Arc<dyn Error + Send + Sync>>,
}

impl QueryError {
    /// An error with only a message.
    #[must_use]
    pub fn msg(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// The task was cancelled, or panicked, before it finished.
    #[must_use]
    pub fn cancelled() -> Self {
        Self::msg("the task was cancelled")
    }

    /// The message, as shown to users.
    #[must_use]
    pub fn message(&self) -> &SharedString {
        &self.message
    }

    /// The original error, if this was converted from one.
    #[must_use]
    pub fn source_error(&self) -> Option<&(dyn Error + Send + Sync + 'static)> {
        self.source.as_deref()
    }

    /// The original error as `E`, if it was one.
    #[must_use]
    pub fn downcast_ref<E: Error + 'static>(&self) -> Option<&E> {
        self.source.as_deref()?.downcast_ref::<E>()
    }
}

impl<E: Error + Send + Sync + 'static> From<E> for QueryError {
    fn from(error: E) -> Self {
        Self {
            message: error.to_string().into(),
            source: Some(Arc::new(error)),
        }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl fmt::Debug for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueryError")
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}

impl PartialEq for QueryError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}
