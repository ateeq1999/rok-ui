//! Flutter-style layout widgets: `Row`, `Column`, `Expanded`, `Stack`, `Padding` and
//! friends, as typed wrappers over flexbox and grid.
//!
//! ```no_run
//! # use rok_ui::prelude::*;
//! # fn example(window: &mut Window, cx: &mut App) {
//! # let search_field = div();
//! # let results = div();
//! let page = Column::new()
//!     .cross_axis_alignment(CrossAxisAlignment::Stretch)
//!     .spacing(px(12.))
//!     .child(H3::new("Results"))
//!     .child(
//!         Row::new()
//!             .main_axis_alignment(MainAxisAlignment::SpaceBetween)
//!             .child(Expanded::new().child(search_field))
//!             .child(SizedBox::width(px(8.)))
//!             .child(Button::new("filter").label("Filter")),
//!     )
//!     .child(Expanded::new().child(results));
//! # }
//! ```
//!
//! "Start" and "end" follow the reading direction like Flutter's
//! `AlignmentDirectional`: a `Row` flows right to left in RTL, and
//! `CrossAxisAlignment::Start` in a `Column` hugs the right edge.
//!
//! Sizing follows CSS flexbox, which differs from Flutter's constraints in one
//! place: a `Column` inside a `Column` fills the parent's height by default
//! (`MainAxisSize::Max`). Give it `MainAxisSize::Min` or wrap it in `Expanded`.

mod align;
mod flex;
mod grid;
mod responsive;
mod stack;

pub use align::{Aligned, Alignment, Center, EdgeInsets, Padding, SizedBox};
use flex::fill_box;
pub use flex::{
    Column, CrossAxisAlignment, Expanded, Flexible, MainAxisAlignment, MainAxisSize, Row, Spacer,
};
pub use grid::{GridView, Wrap};
pub use responsive::{BoxConstraints, LayoutBuilder, WindowSizeClass};
pub use stack::{Positioned, Stack, StackFit};
