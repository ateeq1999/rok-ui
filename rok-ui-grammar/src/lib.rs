//! Parsing and code generation for rok-ui's macros.
//!
//! Each module parses one macro's input into syntax trees and generates its expansion from
//! them. `rok-ui-macros` only forwards tokens here, so expansions are tested as plain Rust
//! (see the tests in each module). Generated code uses absolute `::rok_ui::..` paths.

pub mod children;
pub mod component;
pub mod file_route;
pub mod form_values;
pub mod procedure;
pub mod search;
pub mod store;
pub mod styles;
