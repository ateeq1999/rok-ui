//! Procedural macros for `rok-ui`: entry points only.
//!
//! Parsing and code generation live in `rok-ui-grammar`, where they are unit-tested; each
//! macro here forwards its tokens and turns errors into `compile_error!`.

use proc_macro::TokenStream;
use rok_ui_grammar::{
    children, component, file_route, form_values, procedure, search, store, styles,
};

/// Turn an expansion result into tokens, errors included.
fn expansion(result: syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    result
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Turn a function into a reusable component, React style.
///
/// ```ignore
/// use rok_ui::prelude::*;
///
/// /// A greeting card.
/// #[component]
/// pub fn Greeting(
///     name: SharedString,
///     #[prop(optional)] excited: bool,
///     #[prop(optional)] on_wave: Option<EventHandler<ClickEvent>>,
///     #[children] children: Vec<AnyElement>,
///     window: &mut Window,
///     cx: &mut App,
/// ) -> impl IntoElement {
///     div().child(format!("Hello, {name}{}", if excited { "!" } else { "." })).children(children)
/// }
///
/// // Usage: required props go to `new`, optional props are builder methods,
/// // children use the familiar `.child(..)`.
/// Greeting::new("Ada").excited(true).on_wave(|_, _, _| {}).child("Welcome back");
/// ```
///
/// Parameter rules:
/// - A parameter named `window` or `cx` receives the GPUI window or app context.
///   `cx: &mut Cx` receives both as one [`Cx`](../rok_ui/struct.Cx.html) handle instead
///   (then there is no `window` parameter).
/// - Plain parameters are required props and become arguments of `new(..)` (taking `impl Into<T>`).
/// - `#[default]` parameters start at `Default::default()`, `#[default(expr)]` ones at `expr`,
///   and get a builder method with the same name. For `Option<T>` the method takes
///   `impl Into<T>`. `#[prop(optional)]` is the older spelling of `#[default]`.
/// - Props of type `EventHandler<E>` (or `Option<EventHandler<E>>`) take a closure
///   `Fn(&E, &mut Window, &mut App)` directly.
/// - One `#[children]` parameter of type `Vec<AnyElement>` makes the component a
///   `ParentElement`, so `.child(..)` and `.children(..)` work.
/// - One `#[style]` parameter of type `StyleRefinement` makes the component `Styled`,
///   so callers can chain `.w_full()`, `.mt_4()`, … like a `className`. Apply it in
///   the body with `.apply_style_overrides(&style_overrides)`.
/// - One `#[sx] sx: Sx` parameter makes the component accept `.sx(..)` styles
///   (`StyleX`'s `xstyle`). Apply them last in the body: `.sx((&MY_STYLES.base, &sx))`.
#[proc_macro_attribute]
pub fn component(attribute_arguments: TokenStream, item: TokenStream) -> TokenStream {
    expansion(component::expand(attribute_arguments.into(), item.into()))
}

/// Declare the route in a route file under `src/routes/` (see `rok_ui::routes!`).
///
/// Keys: `params: { id: u64 }` (types of the path's `$` parameters), `search: Type`,
/// `component: Page` or `layout: Layout` (a layout renders its child route as its children),
/// `before_load: |location, cx| ..` (a guard returning `Result<(), RouteControl>`), and on
/// pages `loader: |route: &Route, cx| ..` (starts loading data before the page renders).
#[proc_macro]
pub fn file_route(input: TokenStream) -> TokenStream {
    expansion(file_route::expand_file_route(input.into()))
}

/// Implement `rok_ui::form::FormValues` and add a typed field constant per struct field:
/// `email: String` becomes `pub const EMAIL: Field<Self, String>`.
#[proc_macro_derive(FormValues)]
pub fn derive_form_values(input: TokenStream) -> TokenStream {
    expansion(form_values::expand_form_values(input.into()))
}

/// Generate `NameStore`, a fine-grained store with one signal per field (`state` feature):
/// `store.title()` reads (and tracks) only `title`, `store.set_title(..)` re-renders only its
/// readers, and `store.get()` / `store.set(..)` work on the whole value.
#[proc_macro_derive(Store)]
pub fn derive_store(input: TokenStream) -> TokenStream {
    expansion(store::expand_store(input.into()))
}

/// Implement `rok_ui::router::Search` for a struct of query parameters.
///
/// Field attributes: `#[search(default = expr)]` (the value when missing or invalid; otherwise
/// `Default::default()`), `#[search(rename = "p")]` (the query key). `Option` fields are `None`
/// when missing.
#[proc_macro_derive(Search, attributes(search))]
pub fn derive_search(input: TokenStream) -> TokenStream {
    expansion(search::expand_search(input.into()))
}

/// Declare a typed async command: a [`Procedure`](../rok_ui/query/trait.Procedure.html) with
/// optional queries to invalidate after it succeeds.
///
/// The function is `async`, takes an optional `TaskCx` and one input, and returns
/// `Result<Output, Error>`. The attribute turns it into a unit struct of the same name that
/// implements `Procedure`, for `use_procedure(cx, create_note)` or `create_note.call(cx, input)`.
///
/// ```ignore
/// #[procedure(invalidates = [query_key!["notes"]])]
/// async fn create_note(cx: TaskCx, input: NewNote) -> Result<Note, NoteError> { .. }
/// ```
#[proc_macro_attribute]
pub fn procedure(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expansion(procedure::expand_procedure(arguments.into(), item.into()))
}

/// Memoize an async function: calls with the same arguments share one in-flight future and its
/// result, until `rok_ui::query::memo::invalidate` forgets it. Arguments are owned and `Debug`
/// (they form the cache key); the output is `Clone + Send + Sync`.
///
/// ```ignore
/// #[memoize]
/// async fn current_user(cx: TaskCx) -> Option<User> { session::load(&cx).await }
/// ```
#[proc_macro_attribute]
pub fn memoize(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expansion(procedure::expand_memoize(arguments.into(), item.into()))
}

/// Define StyleX-style style objects once, at module level.
///
/// ```ignore
/// styles! {
///     pub CARD = {
///         base: {
///             display: flex, direction: column, gap: 6, padding: 6,
///             radius: xl, border: 1, border_color: border, background: card,
///             hover: { border_color: ring },
///         },
///         compact: { padding: 3, gap: 3 },
///         variant(ButtonVariant): {
///             Primary: { background: primary, color: primary_foreground },
///             Outline: { border: 1, border_color: input },
///         },
///     }
/// }
///
/// div().sx((&CARD.base, compact.then_some(&CARD.compact), CARD.variant(variant)))
/// ```
///
/// Each object becomes a static (`CARD`) with one `Sx` field per key and one
/// lookup method per variant table. Values:
/// - lengths: numbers are multiples of 4px (`gap: 6` is 24px); also `50%`,
///   `full`, `auto`, or any expression (`px(10.)`, `{width}`);
/// - colors: theme tokens (`primary`, `muted_foreground`), with opacity
///   (`primary/90`), `transparent`, or an expression;
/// - radius: `none`, `sm`, `md`, `lg`, `xl`, `full`, or a length;
/// - `hover`, `focus` and `active` blocks style interaction states.
#[proc_macro]
pub fn styles(input: TokenStream) -> TokenStream {
    expansion(styles::expand_styles(input.into()))
}

/// One inline style object, for one-off or dynamic values:
/// `style! { width: {px(width)}, background: primary/90 }`.
#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    expansion(styles::expand_style(input.into()))
}

/// A `Vec<AnyElement>` from mixed element types, with control flow:
///
/// ```ignore
/// div().children(children![
///     Title::new("Projects"),
///     if loading { Spinner::new() } else { Badge::new("Ready") },
///     for project in &projects => Item::new(project.id).title(project.name.clone()),
///     match status { Status::Ok => "Up to date", Status::Stale => Button::new("refresh") },
///     "plain text",
/// ])
/// ```
#[proc_macro]
pub fn children(input: TokenStream) -> TokenStream {
    expansion(children::expand_children(input.into()))
}

/// JSX-like markup compiled to builder calls.
///
/// ```ignore
/// view! {
///     Card(sx = [CARD.base, compact => CARD.compact]) {
///         CardHeader {
///             CardTitle("Create project")
///             CardDescription("Deploy your new project in one click.")
///         }
///         if let Some(error) = error {
///             Alert("Deploy failed", description = error).destructive()
///         }
///         for project in &projects {
///             Item(project.id, title = project.name.clone())
///         }
///         div(sx = ROW.end) { "Raw text" {some_element} }
///     }
/// }
/// ```
///
/// - `Name(a, b, key = value)` is `Name::new(a, b).key(value)`; lowercase
///   `div(..)`, `img(src)` and `svg()` are GPUI element functions.
/// - `sx = [a, cond => b]` merges styles like `sx![..]`; `sx = expr` passes one.
/// - `.method(..)` after the arguments is passed through unchanged.
/// - `{ .. }` after an element holds its children: elements, `"text"`,
///   `{expr}`, `if` / `if let` / `match` / `for`.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    expansion(children::expand_view(input.into()))
}

/// Define keyframe sequences for [`Motion`](../rok_ui/motion/struct.Motion.html).
///
/// ```ignore
/// keyframes! {
///     pub FADE_UP = {
///         from: { opacity: 0, y: 2 },
///         60%: { opacity: 1 },
///         to: { y: 0, background: card },
///     }
/// }
///
/// div().motion("enter", Motion::new(&FADE_UP).duration_ms(250).easing(Easing::EaseOut))
/// ```
///
/// Offsets are `from`, `to` or percentages. Animatable properties: `opacity`,
/// `x` / `y` (offsets from the laid-out position), `width`, `height`, `radius`,
/// `background`, `color` and `border_color`, with the same values as `styles!`.
#[proc_macro]
pub fn keyframes(input: TokenStream) -> TokenStream {
    expansion(styles::expand_keyframes(input.into()))
}
