---
name: route
description: Conventions for rok-ui routes and the router.
---

# Routes

The router (feature `router`, `src/router.rs`) is code-based, as of 0.6:

```rust
Router::new()
    .route("/notes", |_, _, _| NoteList::new())
    .route("/notes/:id", |route, _, _| NoteView::new(route.param_as::<u64>("id").unwrap_or(0)))
    .redirect("/n/:id", "/notes/:id")
    .not_found(|_, _, _| Empty::new().title("Not found"))
```

Typed routes, file-based routing, typed search params, guards and loaders are Router v2
(enhance.md Part E, planned for 0.8; see `roadmap.md`). None of them exist yet, so don't use
or document them as if they did.

Conventions for app code today:

- Keep route builders thin: read params and query values, then render a component from
  `features/`. Parse params with `param_as::<T>` and handle `None` (a bad URL is user input).
- Shareable UI state (filters, tabs, pages) goes in the query string (`route.query("tab")`,
  `router::navigate("/notes?tab=archived", cx)`), not in component state.
- Build paths in one place (a small function per route) so links and `navigate` calls stay in
  sync until typed links land.
- `Link::new(id, path)` for navigation in markup; `.exact(true)` when the active style must
  not match child paths.

Checklist for a router change:

- Pattern matching and history are pure functions with `#[test]`s in `src/router.rs`.
- Navigation behavior has a `#[gpui::test]`.
- Update `docs/guide/router.md` and `llms.txt`.
