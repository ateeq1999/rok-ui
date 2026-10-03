use rok_ui::prelude::*;
use rok_ui::router::{self, file_route};

use crate::features::session::set_signed_in;

file_route! {
    component: LoginPage,
}

/// Sign in, then continue to `?next=`.
#[component]
fn LoginPage(cx: &mut Cx) -> impl IntoElement {
    let next = router::location(cx).query("next").unwrap_or("/").to_string();
    div().flex().flex_col().gap_2().child(H2::new("Sign in")).child(
        Button::new("sign-in").label("Sign in").on_click(move |_, _, cx| {
            set_signed_in(true);
            router::replace(next.clone(), cx);
        }),
    )
}
