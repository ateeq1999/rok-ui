//! Procedures: typed commands declared once with `#[procedure]`.

use gpui::{App, Task};

use super::{
    invalidate, mutation::MutationOptions, options::BoxFuture, services::task_cx, use_mutation,
    Mutation, QueryKey, TaskCx,
};
use crate::Cx;

/// A typed async command (TanStack Start's server functions, topcoat's procedures).
///
/// Implement it with the [`procedure`](crate::procedure) attribute:
///
/// ```no_run
/// use rok_ui::{procedure, query::TaskCx, query_key};
///
/// #[derive(Clone)]
/// pub struct NewNote { pub title: String }
///
/// #[derive(Debug)]
/// pub enum NoteError { EmptyTitle }
///
/// #[procedure(invalidates = [query_key!["notes"]])]
/// pub async fn create_note(_cx: TaskCx, input: NewNote) -> Result<u64, NoteError> {
///     if input.title.is_empty() {
///         return Err(NoteError::EmptyTitle);
///     }
///     Ok(1)
/// }
/// ```
///
/// Then `use_procedure(cx, create_note)` in a component, or `create_note.call(cx, input)`
/// anywhere.
pub trait Procedure: Copy + Send + Sync + 'static {
    /// What the procedure takes.
    type Input: Clone + Send + 'static;
    /// What it returns on success.
    type Output: Send + 'static;
    /// What it returns on failure.
    type Error: Send + 'static;

    /// Run the procedure on the shared runtime.
    fn run(&self, cx: TaskCx, input: Self::Input) -> BoxFuture<Result<Self::Output, Self::Error>>;

    /// The queries to invalidate after a success.
    fn invalidates(&self) -> Vec<QueryKey> {
        Vec::new()
    }

    /// The procedure as [`MutationOptions`].
    fn mutation_options(&self) -> MutationOptions<Self::Input, Self::Output, Self::Error> {
        let procedure = *self;
        self.invalidates().into_iter().fold(
            MutationOptions::new(move |cx, input| procedure.run(cx, input)),
            MutationOptions::invalidates,
        )
    }

    /// Run the procedure outside a component. Invalidates its queries after a success.
    /// `Err(None)` means the task was cancelled before it finished.
    fn call(
        &self,
        cx: &mut App,
        input: Self::Input,
    ) -> Task<Result<Self::Output, Option<Self::Error>>> {
        let running = crate::runtime::spawn(self.run(task_cx(cx), input));
        let invalidates = self.invalidates();
        cx.spawn(async move |cx| {
            let result = running.await.map_err(|_| None)?;
            if result.is_ok() {
                cx.update(|cx| {
                    for prefix in &invalidates {
                        invalidate(cx, prefix);
                    }
                })
                .ok();
            }
            result.map_err(Some)
        })
    }
}

/// A [`Mutation`] for a procedure at this call site.
#[track_caller]
pub fn use_procedure<P: Procedure>(
    cx: &mut Cx,
    procedure: P,
) -> Mutation<P::Input, P::Output, P::Error> {
    use_mutation(cx, procedure.mutation_options())
}
