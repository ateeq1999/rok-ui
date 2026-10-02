# rok-ui-macros

Procedural macros for [`rok-ui`](https://crates.io/crates/rok-ui), the shadcn/ui-style component
system for [GPUI](https://gpui.rs) desktop apps.

You normally do not depend on this crate directly. `rok-ui` re-exports every macro it provides:

| Macro | What it does |
|---|---|
| `#[component]` | Turns a function into a component with a builder API. |
| `styles!` | Defines a table of named styles, with states and enum-keyed variants. |
| `style!` | Defines a single style. |
| `keyframes!` | Defines a keyframe animation for `Motion`. |
| `children!` | Builds a list of children, with `if`, `for` and `match` mixed in. |
| `view!` | JSX-like markup for element trees. |

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

The [rok-ui README](https://github.com/ateeq1999/rok-ui#writing-a-component) covers the
`#[component]` parameter rules, and its
[Styling and markup](https://github.com/ateeq1999/rok-ui#styling-and-markup) section covers
`styles!`, `children!` and `view!`.

## License

Licensed under the [MIT License](LICENSE).
