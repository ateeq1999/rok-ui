use rok_ui::prelude::*;
use rok_ui::router::file_route;

file_route! {
    component: NotFound,
}

/// Nothing lives at this path.
#[component]
fn NotFound(cx: &mut Cx) -> impl IntoElement {
    let path = rok_ui::router::location(cx).path().to_string();
    Empty::new()
        .title("Page not found")
        .description(format!("Nothing lives at {path}."))
}
