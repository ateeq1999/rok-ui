//! The app: views, routes and the composition root. The layers below live in their own
//! crates (see the architecture guide):
//!
//! - `data`: the `{{name}}-data` crate (models, providers and repositories).
//! - `features`: one module per feature; its business logic (`bloc`) is the
//!   `{{name}}-<feature>` crate, and its views (`view`) live here.
//! - `routes`: thin route files that render feature pages.
//!
//! `cargo rok-ui g feature <name>` adds a feature crate and wires it into `app.rs`.

pub mod app;
pub use {{crate_name}}_data as data;
pub mod features;
pub mod shared;

rok_ui::routes!();
