---
name: route
description: Conventions for rok-ui routes and the router.
---

# Routes

Router 0.5 (feature `router`) is code-based: `Router::new().route("/notes/:id", ..)`.
Router v2 (enhance.md Part E) adds typed routes. Until file-based routing lands, prefer typed
route values for new code:

- Define one type per route that implements `rok_ui::router::Route` (or use `typed_route!`), so
  links and navigation are compile-checked: `Link::to(NoteRoute { id: 3 })`.
- Shareable UI state (filters, tabs, pages) goes in search params read with
  `rok_ui::router::Search` types, not in component state.
- Keep route builders thin: read params, then render a component from `features/`.

Checklist for a router change:

- Pattern matching and history are pure functions with `#[test]`s in `src/router.rs`.
- Navigation behavior has a `#[gpui::test]`.
- Update `docs/guide/router.md`.
