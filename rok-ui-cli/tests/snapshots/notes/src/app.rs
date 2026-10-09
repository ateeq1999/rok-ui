//! The app's composition root: repositories are built once here, and blocs are provided
//! above the routes, so every page reads the same instances.
//!
//! `cargo rok-ui generate` adds lines at the `// rok-ui:` markers; keep them.

// Used by the lines `cargo rok-ui generate` adds.
#[allow(unused_imports)]
use std::sync::Arc;

use rok_ui::bloc::{BlocProvider, Repositories, RepositoryProvider};
use rok_ui::prelude::*;

use crate::routes;

/// The repositories, built once for the app's lifetime.
fn repositories() -> Repositories {
    Repositories::new()
        .with::<dyn crate::data::repositories::notes_repository::NotesRepository>(Arc::new(crate::data::repositories::notes_repository::NotesRepositoryImpl::new(crate::data::providers::notes_memory_provider::NotesMemoryProvider::new())))
        // rok-ui:repositories
}

/// Repositories, then blocs, then the routes.
#[component]
pub fn App(cx: &mut Cx) -> impl IntoElement {
    let repositories = cx.use_state(repositories).get(cx);
    RepositoryProvider::from(repositories).child(
        BlocProvider::new()
            .with_bloc(|scope| crate::features::notes::bloc::notes_bloc::NotesBloc::new(scope.repository::<dyn crate::data::repositories::notes_repository::NotesRepository>()))
            // rok-ui:blocs
            .child(routes::tree()),
    )
}
