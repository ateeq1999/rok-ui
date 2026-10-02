# rok-ui-macros

Procedural macros for [`rok-ui`](https://crates.io/crates/rok-ui), the shadcn/ui-style component
system for [GPUI](https://gpui.rs) desktop apps.

You normally do not depend on this crate directly. `rok-ui` re-exports the one macro it provides:

```rust
use rok_ui::prelude::*;

#[component]
fn Greeting(
    name: SharedString,
    #[prop(optional)] excited: bool,
    #[children] children: Vec<AnyElement>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    div().child(format!("Hello, {name}{}", if excited { "!" } else { "." })).children(children)
}

// Required props become arguments of `new(..)`, optional props are builder methods.
Greeting::new("Ada").excited(true).child("Welcome back");
```

See the [rok-ui README](https://github.com/ateeq1999/rok-ui#writing-a-component) for the full
parameter rules and the `#[prop]` and `#[children]` attributes.

## License

MIT