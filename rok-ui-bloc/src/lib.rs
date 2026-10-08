//! The BLoC primitives behind `rok_ui::bloc`, without GPUI.
//!
//! A **bloc** turns events into states; a **cubit** exposes methods that emit states. Both keep
//! their state in an [`Emitter`], which notifies subscribers only when the state changes.
//! Handlers run on a tokio runtime (in rok-ui apps, `rok_ui::runtime`), and every handler is
//! cancelled when its bloc closes.
//!
//! ```
//! use rok_ui_bloc::{Bloc, BlocHandle, Emitter};
//!
//! #[derive(Clone, Debug, PartialEq)]
//! struct Count(u32);
//!
//! enum CounterEvent {
//!     Incremented,
//! }
//!
//! struct CounterBloc;
//!
//! impl Bloc for CounterBloc {
//!     type Event = CounterEvent;
//!     type State = Count;
//!
//!     fn initial_state(&self) -> Count {
//!         Count(0)
//!     }
//!
//!     async fn on(&self, event: CounterEvent, emit: &Emitter<Count>) {
//!         match event {
//!             CounterEvent::Incremented => {
//!                 emit.update(|count| count.0 += 1);
//!             }
//!         }
//!     }
//! }
//!
//! let states = rok_ui_bloc::test::run(CounterBloc, [CounterEvent::Incremented, CounterEvent::Incremented]);
//! assert_eq!(states, [Count(1), Count(2)]);
//! ```

mod bloc;
mod cubit;
mod emitter;
pub mod test;

pub use bloc::{Bloc, BlocHandle, Concurrency};
pub use cubit::{Cubit, CubitHandle};
pub use emitter::{Emitter, Observable, Subscription};
