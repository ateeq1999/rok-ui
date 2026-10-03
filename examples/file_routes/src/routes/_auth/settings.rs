use rok_ui::prelude::*;
use rok_ui::router::{self, file_route};

use crate::features::session::set_signed_in;

file_route! {
    component: SettingsPage,
}

/// Account settings.
#[component]
fn SettingsPage() -> impl IntoElement {
    div().flex().flex_col().gap_2().child(H2::new("Settings")).child(
        Button::new("sign-out")
            .outline()
            .label("Sign out")
            .on_click(|_, _, cx| {
                set_signed_in(false);
                router::navigate_to(&super::Index, cx);
            }),
    )
}
