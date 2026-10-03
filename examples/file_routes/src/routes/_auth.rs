use rok_ui::prelude::*;
use rok_ui::router::{file_route, Location, RouteControl};

use crate::features::session::signed_in;

file_route! {
    before_load: |location, _cx| {
        if signed_in() {
            Ok(())
        } else {
            Err(RouteControl::redirect(Location::build("/login", &[("next", location.path())])))
        }
    },
    layout: Authenticated,
}

/// Pages for signed-in users. The guard above runs before any of them renders.
#[component]
fn Authenticated(#[children] children: Vec<AnyElement>) -> impl IntoElement {
    div().children(children)
}
