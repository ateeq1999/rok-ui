//! Integration tests: the `#[component]` macro's generated API, the `State` hook
//! handle, and a full render of every component under every theme.
//!
//! A test crate's root looks for modules beside it, so each one names its file.

#[path = "components/every.rs"]
mod every;
#[path = "components/interaction.rs"]
mod interaction;
#[path = "components/macros.rs"]
mod macros;
#[path = "components/parity.rs"]
mod parity;
