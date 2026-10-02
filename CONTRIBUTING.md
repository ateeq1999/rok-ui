# Contributing to rok-ui

Thanks for helping. Bug reports, fixes, new components and documentation improvements are all
welcome. This guide covers how to set up, what a change needs before it can be merged, and the
conventions the codebase follows.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting bugs and asking for features

- **Bugs:** open an issue with the bug report template. Include your OS, the rok-ui and gpui
  versions, a minimal snippet, and what you expected to see. A screenshot helps with visual bugs.
- **Features and new components:** open an issue first so we can agree on the API before you
  write it. For a shadcn/ui component, link its page on ui.shadcn.com.
- **Security problems:** do not open a public issue. See [SECURITY.md](SECURITY.md).

## Setting up

You need a recent stable Rust (the minimum is the `rust-version` in `Cargo.toml`).

On Linux, GPUI needs the X11 and Wayland development packages. On Debian or Ubuntu:

```sh
sudo apt install libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libvulkan1 libfontconfig-dev
```

Then:

```sh
git clone https://github.com/ateeq1999/rok-ui
cd rok-ui
cargo test
cargo run --example gallery          # every component, with light/dark/RTL toggles
cargo run --example gallery -- --rtl # start in right-to-left mode
```

## Before you open a pull request

Run the same checks CI runs:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
scripts/check-features.sh            # every Cargo feature builds on its own
```

`scripts/check-features.sh` is slow (one build per feature). While iterating, pass only the
features you touched: `scripts/check-features.sh button dialog`.

A pull request should also:

- **Include tests** for bug fixes and new behaviour. `tests/components.rs` renders every component
  under every theme and in RTL; add your component to those views. Layout bugs can be caught by
  measuring bounds with a `canvas` probe (see `button_group_is_as_wide_as_its_items`).
- **Update the docs:** the README component table for new components, and an entry under
  `[Unreleased]` in [CHANGELOG.md](CHANGELOG.md) for anything a user would notice.
- **Check the gallery** in light, dark and RTL for visual changes, and attach before/after
  screenshots to the pull request.

## Conventions

### Components

- **One file per component** in `src/components/`, gated by a Cargo feature of the same name in
  kebab-case. Declare the components it builds on as feature dependencies in `Cargo.toml`.
- **Styles live in a `styles!` table** at the top of the file. Use builder calls only for values
  computed at render time (measured sizes, positions, data-driven widths).
- **Apply styles in one `.sx(..)` call** and merge the caller's `sx` into it last:
  `.sx((&BUTTON.base, BUTTON.variant(v), &self.sx))`. GPUI allows one hover and one focus style
  per element, so a second `.sx(..)` with states on the same element panics.
- **Every visual component** stores `sx: Sx` and `style_overrides: StyleRefinement`, implements
  them with `implement_style_overrides!`, and applies `style_overrides` last.
- **Theme colors come from tokens** (`background`, `muted_foreground`, `primary/90`), never
  hard-coded colors, so every preset and mode works.

### Right-to-left

- Rows built with `display: flex` (or `flex_dir()`) flow in the reading direction. Use logical
  properties such as `padding_start`, `margin_end`, `border_start` and `inset_end` for anything
  that should mirror, and physical ones (`left`, `padding_right`) only when it must not.
- Use `flex_ltr()` for content that never mirrors, such as charts and one-time codes.
- Event handlers run outside the `Direction` scope. Read `is_rtl()` while rendering and capture
  the value in the closure.

### Code style

- Follow `rustfmt` and Clippy. Public items get doc comments with a short usage example.
- Comments explain why, not what. Name things for what they are (`trigger_width`, not `tw`).
- Keep public APIs close to shadcn/ui's names and props so they are easy to find.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): `feat: add Toggle component`,
`fix(select): keep the menu open while scrolling`, `docs: ...`, `refactor: ...`, `test: ...`,
`chore: ...`. Mark breaking changes with `!` (`feat!: rename Sheet::side`) and explain them in
the body.

## Releasing (maintainers)

1. Move the `[Unreleased]` changelog entries under a new version heading with today's date.
2. Bump `version` in `[workspace.package]` and the `rok-ui-macros` dependency version in
   `Cargo.toml`, and update the version in the README install snippets.
3. Check both crates: `cargo publish --workspace --dry-run`.
4. Commit, tag `vX.Y.Z`, and push the tag.
5. Publish the macro crate first, then the main crate:
   `cargo publish -p rok-ui-macros && cargo publish -p rok-ui`.
6. Create a GitHub release from the tag with the changelog section as its notes.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
