//! The gallery pages beyond the overview. Each page is a `#[component]` that
//! keeps its demo state in hooks, so the views stay small.

use rok_ui::prelude::*;

// This file is included with `#[path]`, so its modules name their files too.
#[path = "pages/chat.rs"]
mod chat;
#[path = "pages/data.rs"]
mod data;
#[path = "pages/forms.rs"]
mod forms;
#[path = "pages/layout.rs"]
mod layout;
#[path = "pages/overlays.rs"]
mod overlays;

pub use chat::ChatPage;
pub use data::DataPage;
use data::{centered, RepositoryRow};
pub use forms::FormsPage;
pub use layout::LayoutPage;
pub use overlays::OverlaysPage;

/// A titled demo block.
fn demo(title: &'static str, description: &'static str, content: impl IntoElement) -> Card {
    Card::new()
        .child(
            CardHeader::new()
                .child(CardTitle::new(title))
                .child(CardDescription::new(description)),
        )
        .child(CardContent::new().child(content))
}

fn column() -> Div {
    div().flex_dir().flex_col().flex_1().min_w_0().gap(px(16.))
}

fn row() -> Div {
    div().flex_dir().flex_wrap().items_center().gap(px(8.))
}
