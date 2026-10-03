//! A notes app with file-based routes.
//!
//! ```sh
//! cargo run -p rok-ui-example-file-routes
//! ```
//!
//! - `src/routes/` holds one file per route; `build.rs` turns them into the [`routes`] module.
//! - `__root.rs` and `notes.rs` are layouts; `_auth.rs` is a pathless layout whose guard sends
//!   signed-out users to `/login`.
//! - Links are typed: `Link::to(&routes::NotesId { id: 2 })` is checked by the compiler.
//! - Notes load through `rok_ui::query`, with `Suspense` for the loading state.

pub mod features;

rok_ui::routes!();
