//! The app's composition root: repositories are built once here, and blocs are provided
//! above the routes, so every page reads the same instances.
//!
//! `cargo rok-ui generate` adds lines at the `// rok-ui:` markers; keep them.

// Used by the lines `cargo rok-ui generate` adds.
#[allow(unused_imports)]
use std::sync::Arc;

use rok_ui::bloc::{BlocProvider, Repositories, RepositoryProvider};
use rok_ui::http::{HttpClient, Session};
use rok_ui::prelude::*;

use crate::routes;

/// The repositories, built once for the app's lifetime. HTTP providers share `client`.
#[allow(clippy::needless_pass_by_value)]
fn repositories(client: HttpClient, session: Session) -> Repositories {
    Repositories::new()
        // Views watch the session (see `shared::widgets::SessionGuard`).
        .with::<Session>(Arc::new(session))
        // rok-ui:repositories
}

/// Repositories, then blocs, then the routes.
#[component]
pub fn App(client: HttpClient, session: Session, cx: &mut Cx) -> impl IntoElement {
    let repositories = cx.use_state(move || repositories(client, session)).get(cx);
    RepositoryProvider::from(repositories).child(
        BlocProvider::new()
            // rok-ui:blocs
            .child(routes::tree()),
    )
}
