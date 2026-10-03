# Changelog

All notable changes to rok-ui and rok-ui-macros are recorded here. Both crates share one
version number.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html). While the version is `0.x`, a
minor release (`0.2` → `0.3`) may contain breaking changes.

## [Unreleased]

## [0.6.0] - 2026-10-03

### Added

- **Data layer** (`query` feature, opt-in; enabled by `db`): `rok_ui::query` with
  `QueryOptions`, hierarchical `query_key!` keys, `use_query` / `use_suspense_query`, one
  cache per app with shared in-flight fetches, stale and gc times, retries, placeholder and
  previous data, refetch intervals, prefix `invalidate`, and `fetch_query`,
  `ensure_query_data`, `prefetch_query`, `set_query_data`, `update_query_data`,
  `cancel_queries`, `reset_queries`.
- **Mutations and procedures:** `use_mutation` with pending / success / error state,
  invalidation and optimistic updates that roll back on failure; `#[procedure]` typed commands
  with `use_procedure` and `.call(cx, input)`; `query::provide` and `TaskCx` for values that
  background work reads.
- **`Suspense` and `ErrorBoundary`**, and `#[memoize]` for async helpers whose concurrent
  callers share one future.
- **`Cx`**: one context handle (window and app). `#[component]` functions can take
  `cx: &mut Cx` instead of `window` and `cx`.
- **`rok_ui::runtime`**: one shared tokio runtime for every async feature.
- `db::db_query` and `db::db_mutation` build query and mutation options on the app's
  connection; `db::invalidate` also invalidates matching queries.
- `AGENTS.md` and `.agents/skills` for coding agents.

### Changed

- The workspace enables clippy pedantic and `missing_docs`; every public item is documented
  and builders are `#[must_use]`.
- CI runs `cargo hack` per feature, `cargo udeps`, docs with `--cfg docsrs`, a PostgreSQL
  service for the database tests, and checks pull request titles. Releases go through
  release-plz.
- `src/components/mod.rs` and `src/theme/mod.rs` became `src/components.rs` and
  `src/theme.rs` (no API change).

### Deprecated

- `db::use_query`: use `query::use_query(cx, db::db_query(..))`. It will be removed in 0.8.

## [0.5.0] - 2026-10-03

### Added

- **Reactive state** (`state` feature, part of `full`): rok-ui-hooks' signals, memos, effects
  and stores, re-exported as `rok_ui::state`, with GPUI glue: `cx.track(..)` re-renders a view
  when the signals it reads change, `use_signal` gives a component its own signal, and
  `use_tracked` re-renders a window for shared signals. `rok_ui::init` pumps rok-ui-hooks' timers
  and async tasks.
- **Router** (`router` feature, opt-in): `Router` with `:param` and `*wildcard`
  patterns ranked by specificity, redirects and a not-found route; app-wide back / forward
  history (`navigate`, `replace`, `back`, `forward`, Alt+Left / Alt+Right); query strings;
  `Link`; `is_active` and `on_navigate`.
- **Database** (`db` feature, opt-in): PostgreSQL through rok-db on a background tokio runtime.
  `db::connect`, `db::run` for any database work, and `db::use_query` with loading and error
  states, caching and `db::invalidate`.
  `db-chrono`, `db-uuid`, `db-json` and `db-migrate` forward rok-db's column types and
  migrations.
- **Guides** in `docs/guide` (also the module docs on docs.rs): app shells and layouts,
  reactive state, routing and the database, with examples throughout.
- The `notes` (router and state) and `db_users` (rok-db) examples.

## [0.4.0] - 2026-10-03

### Added

- **Flutter-style app shells** (`scaffold` and `navigation` features): `Scaffold` with an
  `AppBar`, start and end drawers, permanent side navigation, bottom sheet, footer buttons,
  bottom navigation bar and `FloatingActionButton`; `NavigationBar`, `NavigationRail` and
  `NavigationDrawer` built from shared `NavigationDestination`s; and `AdaptiveScaffold`, which
  switches between a bottom bar, a rail and an extended rail by width (Material 3 breakpoints).
  A `NavigationDrawer` inside a scaffold's drawer closes it when a destination is picked.
- **Flutter-style layout widgets** (`layout-widgets` feature): `Row`, `Column`, `Expanded`,
  `Flexible`, `Spacer`, `Center`, `Aligned`, `Padding` with `EdgeInsets`, `SizedBox`, `Stack`
  with `Positioned`, `Wrap`, `GridView` (`count` and `extent`), `LayoutBuilder` and
  `WindowSizeClass`. Start and end follow the reading direction.
- The `shell` feature group (part of `full`) and the `app_shell` example.

### Changed

- String expressions in `view!` and `children!` markup (`{name}` where `name` is a `String` or
  `SharedString`) become `BidiText`, like string literals, so runtime Arabic text displays
  correctly on Windows.

### Fixed

- Isolated Arabic letters in installed fonts that lack the isolated presentation forms stay in
  that font on Windows: rok-ui now reads installed fonts' coverage through DirectWrite. This adds
  the `windows` crate on Windows, the version GPUI already uses.

## [0.3.1] - 2026-10-03

### Added

- **Noto Sans Arabic** behind the `font-noto-sans-arabic` feature (`rok_ui::fonts::NOTO_SANS_ARABIC`),
  an alternative to Cairo that maps every Arabic presentation form. The `fonts` group now
  includes it. The `arabic` example gets a font switch when the feature is on.
- `BidiText` truncates with an ellipsis under `.truncate()` on Windows.
- String literals with right-to-left letters in `view!` and `children!` markup become `BidiText`
  automatically.

### Fixed

- **Typing Arabic in a `Textarea` on Windows.** Wrapped rows are reordered, and the caret,
  selection and clicks follow the visual text.
- **Isolated Arabic letters in Cairo** (and other fonts without isolated presentation forms) on
  Windows were drawn in a fallback font. rok-ui now reads each registered font's coverage and keeps
  those letters in the chosen font. This adds the `ttf-parser` dependency.

## [0.3.0] - 2026-10-03

### Added

- **Arabic, Persian and Hebrew text on Windows.** GPUI's Windows backend draws right-to-left runs
  mirrored. rok-ui now reorders text there itself (Unicode Bidirectional Algorithm plus Arabic
  contextual forms and lam-alef ligatures), in component text automatically and in your own text
  through the new `BidiText` element, which also wraps paragraphs correctly. Single-line inputs
  keep the caret, selection and clicks right while typing Arabic. See `rok_ui::bidi`.
- **Fonts.** `rok_ui::fonts` registers font files, and the `font-cairo` and `font-inter` features
  bundle those Google Fonts (SIL Open Font License). `Theme::set_font_family` sets the UI font and
  survives preset and mode changes. `scripts/fetch-google-font.sh` downloads any Google Font.
- **Arabic examples:** `arabic` (a settings screen) and `arabic_chat`, in the Cairo font.
- **Keyboard-only focus rings**, like CSS `:focus-visible`: clicking a button no longer shows its
  ring. `sx::set_focus_ring_mode(FocusRingMode::Always)` restores the old behaviour.
- **Dialog focus.** Dialogs, alert dialogs, sheets, drawers and the command palette move focus to
  their first field when they open and keep Tab and Shift-Tab inside.
- `styles!`: `direction: row_ltr` for rows that never mirror, `border_style: dashed`, and
  `text: theme` for the theme's base font size.
- `CalendarDate::today_utc()`.

### Changed

- `CalendarDate::today()` returns the local date instead of the UTC date.
- Theme preset and mode switches keep a custom font family and size.

### Fixed

- Clicking inside a popover that sits inside another popover no longer closes the outer one.

## [0.2.0] - 2026-10-03

### Added

- **Cargo features per component.** Depend on only what you use:
  `default-features = false, features = ["button", "dialog"]`. Features pull in the components
  they are built from. shadcn/ui names work as aliases (`dropdown-menu`, `drawer`, `textarea`,
  `kbd`), and `forms`, `overlays`, `layout`, `data` and `chat` enable whole groups. `full`, the
  default, enables everything.
- **StyleX-style styling.** `styles!` and `style!` define typed style tables on a 4px spacing
  scale, with theme color tokens (`primary/90`), hover/focus/active states, variant tables keyed
  by an enum, and logical properties (`padding_start`, `border_end`, `inset_start`). Typos get a
  compile error with a "did you mean" suggestion.
- **`.sx(..)` on every component**, merged with the component's own styles in one call. `sx![..]`
  adds styles conditionally.
- **`children!` and `view!` markup** for building element trees without long builder chains.
- **Motion.** `keyframes!`, `Motion` with enter presets, `use_transition`, `use_presence` for exit
  animations, spring and cubic-bezier easing, and a reduced-motion switch.
- **Right-to-left layouts.** `Direction` and `set_text_direction` mirror rows, icons, floating
  surfaces, panels, sliders and arrow keys the way CSS `dir="rtl"` does. Single-line inputs and
  textareas right-align their text.
- Compile-error tests (trybuild) for the macros.

### Changed

- Components define their styling with `styles!` tables instead of builder chains. LTR rendering
  is unchanged.
- In RTL, the checked Switch thumb, dialog footers and end-aligned Data Table cells now sit at the
  reading end (the left), as they do with CSS `dir="rtl"`.

### Fixed

- `ButtonGroup` was laid out narrower than its items, so the next element overlapped it. Joined
  items now share a border instead of overlapping by 1px.

## [0.1.0] - 2026-10-02

### Added

- First release: shadcn/ui's component catalog for GPUI, from Button and Dialog to Data Table,
  Calendar, Chart and the chat components.
- `#[component]` for React-like function components with builder APIs, `use_state` and keyed
  state hooks.
- Light and dark themes with the Rok and Neutral presets.
- Keyboard focus navigation and text inputs with IME, selection and clipboard support.

[Unreleased]: https://github.com/ateeq1999/rok-ui/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/ateeq1999/rok-ui/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/ateeq1999/rok-ui/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ateeq1999/rok-ui/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/ateeq1999/rok-ui/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/ateeq1999/rok-ui/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ateeq1999/rok-ui/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ateeq1999/rok-ui/releases/tag/v0.1.0
