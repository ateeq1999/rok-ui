# Changelog

All notable changes to rok-ui and rok-ui-macros are recorded here. Both crates share one
version number.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html). While the version is `0.x`, a
minor release (`0.2` → `0.3`) may contain breaking changes.

## [Unreleased]

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

[Unreleased]: https://github.com/ateeq1999/rok-ui/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/ateeq1999/rok-ui/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ateeq1999/rok-ui/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ateeq1999/rok-ui/releases/tag/v0.1.0
