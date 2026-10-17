//! Repository providers: values (API clients, stores) that blocs below can read.

use std::{any::TypeId, rc::Rc, sync::Arc};

use gpui::{prelude::*, AnyElement, App, Window};

use crate::scope::{single, Frame, ScopeElement};

/// Makes repositories available to everything below it (`flutter_bloc`'s
/// `RepositoryProvider` / `MultiRepositoryProvider`). Register a repository under its trait
/// (`dyn NotesRepository`) so tests can provide a fake.
///
/// ```no_run
/// # use std::sync::Arc;
/// # use rok_ui::{prelude::*, bloc::RepositoryProvider};
/// trait NotesRepository: Send + Sync {
///     fn count(&self) -> usize;
/// }
/// struct InMemoryNotes;
/// impl NotesRepository for InMemoryNotes {
///     fn count(&self) -> usize { 0 }
/// }
///
/// # fn app(page: AnyElement) -> impl IntoElement {
/// RepositoryProvider::new()
///     .provide::<dyn NotesRepository>(Arc::new(InMemoryNotes))
///     .child(page)
/// # }
/// ```
#[derive(IntoElement, Default)]
pub struct RepositoryProvider {
    frame: Frame,
    children: Vec<AnyElement>,
}

/// [`RepositoryProvider`] with several repositories: the same type.
pub type MultiRepositoryProvider = RepositoryProvider;

/// A set of repositories built once, at startup, and provided on every render with
/// [`RepositoryProvider::from`]. Cheap to clone.
///
/// ```no_run
/// # use std::sync::Arc;
/// # use rok_ui::{prelude::*, bloc::{RepositoryProvider, Repositories}};
/// # trait NotesRepository: Send + Sync {}
/// # struct InMemoryNotes;
/// # impl NotesRepository for InMemoryNotes {}
/// struct App {
///     repositories: Repositories,
/// }
///
/// impl Render for App {
///     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
///         RepositoryProvider::from(self.repositories.clone()).child(div())
///     }
/// }
///
/// let app = App {
///     repositories: Repositories::new().with::<dyn NotesRepository>(Arc::new(InMemoryNotes)),
/// };
/// # let _ = app;
/// ```
#[derive(Clone, Default)]
pub struct Repositories {
    frame: Frame,
}

impl Repositories {
    /// No repositories yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add `repository` as `R` (often `dyn SomeRepository`).
    #[must_use]
    pub fn with<R: ?Sized + Send + Sync + 'static>(mut self, repository: Arc<R>) -> Self {
        self.frame
            .insert(TypeId::of::<Arc<R>>(), Rc::new(repository));
        self
    }
}

impl std::fmt::Debug for Repositories {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Repositories")
            .field("count", &self.frame.len())
            .finish()
    }
}

impl From<Repositories> for RepositoryProvider {
    fn from(repositories: Repositories) -> Self {
        Self {
            frame: repositories.frame,
            children: Vec::new(),
        }
    }
}

impl RepositoryProvider {
    /// A provider with no repositories yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Provide `repository` as `R` (often `dyn SomeRepository`).
    #[must_use]
    pub fn provide<R: ?Sized + Send + Sync + 'static>(mut self, repository: Arc<R>) -> Self {
        self.frame
            .insert(TypeId::of::<Arc<R>>(), Rc::new(repository));
        self
    }
}

impl ParentElement for RepositoryProvider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for RepositoryProvider {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        ScopeElement {
            frame: Rc::new(self.frame),
            child: single(self.children),
        }
    }
}
