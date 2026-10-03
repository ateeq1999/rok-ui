---
name: style
description: rok-ui code style, documentation and test philosophy.
---

# Code style

- Lints come from `[workspace.lints]` (clippy pedantic, `missing_docs`). Fix warnings; do not
  silence them without a comment explaining why.
- Barrel files, never `mod.rs`.
- In a file: the main struct, its inherent impl, then trait impls, then private helpers, then
  `#[cfg(test)] mod tests`.
- Builders take `mut self`, return `Self`, and are `#[must_use]`.
- No needless allocation in render paths: borrow `SharedString`s, clone `Rc`s.
- Names say what things are (`trigger_width`, not `tw`). Comments explain why, not what.
- New dependencies go in `[dependencies]` with a version only; keep them optional behind the
  feature that needs them.

# Tests

- Test behavior, not hardcoded values: "two components reading one key trigger one fetch", not
  "the cache has 3 entries".
- UI behavior uses `#[gpui::test]` with `TestAppContext`; pure logic uses `#[test]`.
- Every component renders in every theme and in RTL in `tests/components.rs`.
- Macro errors are trybuild cases in `tests/ui/`.
