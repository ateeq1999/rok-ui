#![doc = include_str!("../docs/guide/query.md")]

mod cache;
mod error;
#[doc(hidden)]
pub mod key;
pub mod memo;
mod mutation;
mod options;
mod procedure;
mod services;
mod suspense;

pub use cache::{
    cancel_queries, ensure_query_data, fetch_query, fetching_count, get_query_data, invalidate,
    prefetch_query, queries, reset_queries, set_query_data, update_query_data, use_query,
    use_suspense_query, QueryInfo, QueryResult, QueryState,
};
pub use error::QueryError;
pub use key::QueryKey;
pub use mutation::{use_mutation, Mutation, MutationOptions, MutationStatus, Optimistic};
pub use options::{BoxFuture, QueryOptions};
pub use procedure::{use_procedure, Procedure};
pub use services::{provide, task_cx, unprovide, TaskCx};
pub use suspense::{ErrorBoundary, Suspend, Suspense};
