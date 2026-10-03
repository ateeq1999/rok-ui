# Routing

The `router` feature adds client-side routing to rok-ui apps: paths mapped to pages,
parameters, query strings, redirects, back and forward history, and links. It is opt-in:

```toml
rok-ui = { version = "0.6", features = ["router"] }
```

`rok_ui::init` registers the router's key bindings: Alt+Left goes back and Alt+Right goes
forward.

## A first router

```rust,ignore
use rok_ui::prelude::*;
use rok_ui::router;

impl Render for App {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(
            Router::new()
                .route("/", |_, _, _| HomePage::new())
                .route("/inbox", |_, _, _| InboxPage::new())
                .route("/inbox/:id", |route, _, _| {
                    MessagePage::new(route.param_as::<u64>("id").unwrap_or_default())
                })
                .route("/settings/*section", |route, _, _| {
                    SettingsPage::new(route.param("section").unwrap_or_default())
                })
                .redirect("/home", "/")
                .not_found(|route, _, _| NotFoundPage::new(route.path())),
        )
    }
}
```

The app starts at `/`. Each route builder receives the `RouteMatch` plus `&mut Window` and
`&mut App`, and returns any element. Builders run on every render of the router, like a
component's render function, so keep them cheap and put state in hooks or stores.

## Patterns

Patterns are matched segment by segment against the path (the query string is separate):

| Segment | Matches | Example |
|---|---|---|
| `users` | Exactly that text | `/users` |
| `:id` | Any one segment, captured as `id` | `/users/:id` matches `/users/42` |
| `*rest` | The rest of the path, possibly empty, captured as `rest` (last segment only) | `/files/*path` matches `/files/a/b.txt` with `path = "a/b.txt"` |

When several patterns match, **the most specific wins, whatever the declaration order**:
static segments beat parameters, which beat wildcards, compared from left to right. If two
patterns are equally specific, the one declared first wins.

| Path | Patterns | Winner |
|---|---|---|
| `/users/new` | `/users/:id`, `/users/new` | `/users/new` |
| `/users/42` | `/users/:id`, `/users/*rest` | `/users/:id` |
| `/users/42/posts` | `/users/:id`, `/users/*rest` | `/users/*rest` |

Trailing slashes are ignored (`/users/` is `/users`), and parameters are percent-decoded.

## Reading the match

```rust,ignore
.route("/products/:id", |route, window, cx| {
    let id: Option<u64> = route.param_as("id");        // parsed with FromStr
    let raw: Option<SharedString> = route.param("id");  // as text
    let tab = route.query("tab").unwrap_or("overview");  // ?tab=reviews
    let path = route.path();                            // "/products/7"
    let pattern = route.pattern();                      // "/products/:id"
    ProductPage::new(id.unwrap_or_default(), tab.to_string())
})
```

Query strings are parsed into decoded pairs (`+` and `%20` become spaces).
`route.location().query_pairs()` returns all of them, repeated keys included. `Location::parse`
works on its own too:

```rust
use rok_ui::router::Location;

let location = Location::parse("/search?q=rust+gui&page=2");
assert_eq!(location.path(), "/search");
assert_eq!(location.query("q"), Some("rust gui"));
assert_eq!(location.query("page"), Some("2"));
```

## Navigating

From anywhere you have `&mut App`, such as click handlers, menu actions, or after saving:

```rust,ignore
use rok_ui::router;

router::navigate("/inbox/42", cx);       // add an entry
router::replace("/login", cx);           // swap the current entry (no back step)
router::back(cx);                        // like the browser's back button
router::forward(cx);
router::can_go_back(cx);                 // for enabling back buttons
router::location(cx).path();             // where the app is now
```

`navigate` to the current location does nothing. Navigating after going back drops the forward
entries, as in a browser. Every navigation re-renders all windows.

### Links

`Link` is a focusable element that navigates on click, Enter or Space:

```rust,ignore
Link::new("docs-link", "/docs").child("Read the docs")              // a text link
Link::new("login", "/login").replace(true).child("Sign in")         // replaces the entry
Link::new(("row", user.id), format!("/users/{}", user.id))          // wrap anything
    .sx(style! { color: foreground })
    .child(Item::new(("user", user.id)).title(user.name.clone()))
```

Links are styled like shadcn's: primary color, underline on hover, and a focus ring. A link to
the current location (or below it, unless `.exact(true)`) is drawn in semibold.

### Marking the active page in navigation

`router::is_active(path, exact, cx)` returns whether the current path is `path` or below it.
Use it to select the right item in a sidebar or an `AdaptiveScaffold`:

```rust,ignore
const PAGES: [(&str, &str, IconName); 3] = [
    ("/", "Home", IconName::Home),
    ("/inbox", "Inbox", IconName::Inbox),
    ("/settings", "Settings", IconName::Settings),
];

let selected = PAGES
    .iter()
    .rposition(|(path, _, _)| router::is_active(path, *path == "/", cx))
    .unwrap_or(0);

AdaptiveScaffold::new("shell")
    .destinations(PAGES.map(|(_, label, icon)| NavigationDestination::new(icon, label)))
    .selected_index(selected)
    .on_change(|index, _, cx| router::navigate(PAGES[*index].0, cx))
    .child(Router::new() /* routes */)
```

## Redirects and not found

```rust,ignore
Router::new()
    .redirect("/", "/inbox")                    // a default page
    .redirect("/u/:id", "/users/:id")           // parameters carry over
    .redirect("/old-docs/*path", "/docs/*path")
    .not_found(|route, _, _| {
        Empty::new()
            .icon(IconName::CircleAlert)
            .title("Page not found")
            .description(format!("Nothing lives at {}.", route.path()))
    })
```

Redirects replace the history entry, so Back skips them. They are followed up to eight hops,
so a redirect cycle can't hang the app. Without `not_found`, an unmatched path renders
nothing.

## Layouts and nested routers

A route can render a layout that contains another `Router`. Both match against the full path,
so the inner router lists full patterns:

```rust,ignore
Router::new()
    .route("/settings/*rest", |_, _, _| {
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(SettingsMenu::new())                           // shared by every settings page
            .child(Expanded::new().child(
                Router::new()
                    .route("/settings/profile", |_, _, _| ProfileSettings::new())
                    .route("/settings/billing", |_, _, _| BillingSettings::new())
                    .redirect("/settings", "/settings/profile"),
            ))
    })
```

Note that `/settings/*rest` also matches `/settings` itself, with an empty `rest`.

## Guards

Route builders are plain closures, so a guard is an `if` in the builder:

```rust,ignore
.route("/admin/*rest", move |_, _, cx| {
    if session.with(|session| session.is_admin()) {
        AdminPage::new().into_any_element()
    } else {
        router::replace("/login?next=/admin", cx);
        div().into_any_element()
    }
})
```

## Listening to navigation

`router::on_navigate` runs after every change, for analytics, window titles, or restoring the
last page on the next launch:

```rust,ignore
router::on_navigate(|location, cx| {
    std::fs::write(last_page_file(), location.path()).ok();
}, cx);

// At startup, before opening windows:
if let Ok(path) = std::fs::read_to_string(last_page_file()) {
    router::replace(path, cx);
}
```

## How it works, and limits

- **One history per app**, like a single browser tab. Every `Router` in every window shows the
  current location. For separate navigation per window, keep a page enum in each view and
  skip the router.
- Builders run during render, inside the router's element, so they can use hooks
  (`use_state`, `use_signal`, `db::use_query`) like any component.
- Nothing is persisted. The history starts at `/` on every launch; restore it yourself with
  `on_navigate` as shown above.

## Testing

Navigation functions only need `&mut App`, so `#[gpui::test]` covers them:

```rust,ignore
#[gpui::test]
fn opening_a_message(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        rok_ui::init(cx);
        router::navigate("/inbox/7", cx);
        assert_eq!(router::location(cx).path(), "/inbox/7");
        router::back(cx);
        assert_eq!(router::location(cx).path(), "/");
    });
}
```

`cargo run --example notes --features router` puts the router, a store and `use_signal`
together in an `AdaptiveScaffold`.

## Reference

| Item | What it does |
|---|---|
| `Router::new().route(pattern, builder)` | Renders the best match for the current location |
| `.redirect(from, to)`, `.not_found(builder)` | Redirects and the fallback page |
| `RouteMatch` | `param`, `param_as`, `query`, `path`, `pattern`, `location` |
| `Location::parse(path)` | `path()`, `query(key)`, `query_pairs()` |
| `navigate`, `replace`, `back`, `forward` | Change the location |
| `can_go_back`, `can_go_forward`, `location`, `is_active` | Read the history |
| `on_navigate(listener, cx)` | Run code after each navigation |
| `Link::new(id, to)` | Navigates on click, Enter or Space; `.replace(..)`, `.exact(..)` |
| `GoBack`, `GoForward` | Actions bound to Alt+Left and Alt+Right |
