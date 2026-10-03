---
name: quality
description: Review a rok-ui change for code quality before committing or opening a pull request (correctness, API conventions, performance in render paths, docs, tests, dependencies).
---

# Code quality review

Run the `check` skill first; this list covers what the tools don't catch. Read the whole diff
(`git diff origin/main...`), not only the last commit.

## Correctness

- Hooks are called unconditionally and in the same order every render; loops use
  `use_keyed_state` with a stable key, never the index of a reorderable list.
- One `.sx(..)` call per element when styles carry hover or focus states (a second one
  panics). The caller's `sx` and `style_overrides` are applied last.
- Event handlers don't read `is_rtl()` or the theme lazily; they capture values read while
  rendering.
- No `unwrap()` / `expect()` on values the app or the user controls. `expect` is fine for
  invariants, with a message saying which invariant.
- Async work (`query`, `db`): futures are `Send + 'static`, don't capture UI handles, and
  every write invalidates the queries that read what it changed.
- No `unsafe` (the lint denies it); the only exception is the documented DirectWrite lookup
  in `src/fonts.rs`.

## API conventions

- New components follow the `component` skill: `new(required..)`, `#[must_use]` builders that
  document their defaults, shared prop names, controlled first.
- Names match shadcn/ui or TanStack where an equivalent exists, so they are easy to find.
- Public types that users store or compare derive `Clone`, `Debug`, `PartialEq` where it makes
  sense; error types implement `std::error::Error`.
- A breaking change is marked with `!` in the commit and gets a `docs/migration/` note.

## Performance in render paths

- No allocation per frame that can be avoided: borrow `SharedString`, clone `Rc`/`Arc`, keep
  `styles!` tables static.
- No blocking calls on the UI thread (file or network I/O, `block_on`); use
  `crate::runtime` or a query.
- Lists that can grow long use GPUI's `uniform_list` (virtualized) rather than one element per
  row.

## Docs and tests

- Every public item has a doc comment; new examples compile (no `ignore` blocks; `no_run`
  for windows).
- Tests check behavior (what a user sees or what a call returns), not internal counts.
- New components render in every theme and RTL in `tests/components.rs`; macro error messages
  have trybuild cases in `tests/ui/`.
- `CHANGELOG.md` has an `[Unreleased]` entry for anything a user would notice; `llms.txt` and
  the guides are updated with new concepts; `roadmap.md` with plan items.

## Dependencies

- New dependencies are optional behind the feature that needs them, with a version only.
- `cargo deny --all-features check` passes. A new license or an ignored advisory in
  `deny.toml` needs a reason and a maintainer's agreement.
- Prefer crates GPUI already pulls in over new ones that do the same job.

## Report

Summarize findings as: must fix (bugs, broken conventions, failing checks), should fix
(missing tests or docs), and optional (style). Fix the first two groups before opening the
pull request.
