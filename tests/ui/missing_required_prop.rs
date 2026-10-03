use rok_ui::prelude::*;

/// A greeting.
#[component]
fn Greeting(name: SharedString, #[default] excited: bool) -> impl IntoElement {
    div().child(format!("Hello, {name}{}", if excited { "!" } else { "." }))
}

fn main() {
    let _ = Greeting::new().excited(true);
}
