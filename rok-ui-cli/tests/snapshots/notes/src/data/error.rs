//! Errors the data layer reports to the business layer.

use std::fmt;

/// Why a repository call failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataError {
    /// Nothing has that id.
    NotFound,
    /// The data source failed; the message says why.
    Failed(String),
}

impl fmt::Display for DataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("not found"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DataError {}
