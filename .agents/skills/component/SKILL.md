---
name: component
description: Checklist for adding or changing a rok-ui component (the component contract).
---

# Component contract

Files and features:

- One file in `src/components/<name>.rs`, gated by a kebab-case Cargo feature of the same name.
  Add the components it builds on as feature dependencies and add the feature to its group
  (`forms`, `overlays`, `layout`, `data`, `chat`, `shell`).
- Re-export it from `src/components.rs`.

API:

- Struct, inherent impl, trait impls, tests, in that order.
- `new(..)` takes what the component cannot exist without (an `id` for stateful components).
  Everything else is a `#[must_use]` builder that documents its default.
- Controlled by default (`value` + `on_change`, `open` + `on_open_change`); uncontrolled forms
  use `default_value` / `default_open` and keep state by `id`.
- Use the shared names: `checked`, `selected_index`, `open`, `disabled`, `invalid`, `size`,
  `variant`.
- Implement `Styled` through `implement_style_overrides!`, store `sx: Sx`, and apply
  `.sx((&STYLES.base, .., &self.sx))` once, then `apply_style_overrides` last.

Behavior:

- Styles in a `styles!` table, theme tokens only.
- Text through `BidiText`; logical properties (`padding_start`) for anything that mirrors in RTL.
- Keyboard: `tab_index(0)`, activation through `interaction::on_activate`, a visible focus ring.

Done means:

- A doc comment with an example, a gallery entry, and a README table row.
- Rendered in every theme and RTL in `tests/components.rs`, plus a behavior test per
  interactive path.
- `scripts/check-features.sh <feature>` passes.
