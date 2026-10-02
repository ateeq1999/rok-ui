# rok-ui

A [shadcn/ui](https://ui.shadcn.com)-style component system for desktop apps built with
[GPUI](https://gpui.rs), the GPU-accelerated UI framework from the Zed team, with a React-like
developer experience.

- **shadcn/ui's components and tokens.** The full catalog, from Button and Dialog to Data Table,
  Calendar, Chart and the chat components, styled from the same design tokens (`background`, `primary`, `muted-foreground`, `ring`, …).
- **React-like components.** Write a function with `#[component]` and get a builder API. Props
  become arguments or builder methods, `#[children]` gives you `.child(..)`, and `use_state`
  works like `useState`.
- **Overridable styles.** Every visual component implements GPUI's `Styled`, so `.w_full().mt_4()`
  overrides its defaults the way `className` does.
- **Themes.** Light and dark modes, two presets (Rok and Neutral), and every text/surface pair is
  tested to reach at least 4.5:1 contrast.
- **Keyboard support.** Tab and Shift-Tab move focus, Enter and Space activate buttons, and Escape
  closes dialogs. Text fields support IME composition, selection and the clipboard.

| Rok, light | Rok, dark |
|---|---|
| ![Gallery, Rok light](https://raw.githubusercontent.com/ateeq1999/rok-ui/main/docs/screenshots/rok-light.png) | ![Gallery, Rok dark](https://raw.githubusercontent.com/ateeq1999/rok-ui/main/docs/screenshots/rok-dark.png) |
| **Neutral, dark** | **Dialog** |
| ![Gallery, Neutral dark](https://raw.githubusercontent.com/ateeq1999/rok-ui/main/docs/screenshots/neutral-dark.png) | ![Dialog](https://raw.githubusercontent.com/ateeq1999/rok-ui/main/docs/screenshots/rok-dialog.png) |

These are real renders of `cargo run --example gallery` on Linux.

## Install

```toml
[dependencies]
rok-ui = "0.1"
gpui = "0.2.2"   # rok-ui re-exports it as rok_ui::gpui; keep the versions in step
```

rok-ui 0.1 targets **gpui 0.2.2** from crates.io and Rust 1.85 or newer.

On Linux, GPUI needs the X11/Wayland development packages. On Debian/Ubuntu:

```sh
sudo apt install libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libvulkan1
```

## Quick start

```rust
use rok_ui::prelude::*;

struct HelloWindow;

impl Render for HelloWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()                       // applies the theme: background, text, font
            .items_center()
            .justify_center()
            .child(Button::new("hello").icon(IconName::Plus).label("Hello, rok-ui"))
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)         // serves the built-in icons
        .run(|cx: &mut App| {
            rok_ui::init(cx);                // installs the theme and key bindings
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| HelloWindow))
                .unwrap();
        });
}
```

The examples:

```sh
cargo run --example counter                       # a function component with a hook
cargo run --example gallery                       # every component, page by page, with theme switching
cargo run --example gallery -- --page chat        # start on a page: overview, forms, overlays, layout, data, chat
cargo run --example gallery -- --dark --preset neutral
```

## Coming from React and shadcn/ui

| React / shadcn/ui | rok-ui |
|---|---|
| `function Card(props) { … }` | `#[component] fn Card(…) -> impl IntoElement { … }` |
| `<Button variant="outline" size="sm">Save</Button>` | `Button::new("save").outline().small().label("Save")` |
| `props.children` | `#[children] children: Vec<AnyElement>` |
| `className="w-full mt-4"` | `.w_full().mt_4()` |
| `onClick={() => …}` | `.on_click(\|event, window, cx\| …)` |
| `const [count, setCount] = useState(0)` | `let count = use_state(window, cx, \|\| 0)` |
| `setCount(5)` / `setCount(c => c + 1)` | `count.set(5, cx)` / `count.update(cx, \|c\| *c += 1)` |
| `useRef` for an input | `use_input_state("email", window, cx, \|s\| s)` |
| `var(--primary)` | `cx.theme().colors.primary` |
| `<ThemeProvider>` | `AppRoot::new()` plus `Theme::set_global(..)` |

### Writing a component

```rust
use rok_ui::prelude::*;

/// A counter, written the way you would in React.
#[component]
pub fn Counter(
    /// Required props become arguments of `Counter::new(..)`.
    title: SharedString,
    /// Optional props start at `Default::default()` and get a builder method.
    #[prop(optional)] step: Option<i32>,
    /// `EventHandler<E>` props accept a closure directly.
    #[prop(optional)] on_change: Option<EventHandler<i32>>,
    /// `window` and `cx` are passed through; leave them out if you do not need them.
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let count = use_state(window, cx, || 0);
    let step = step.unwrap_or(1);

    Card::new()
        .child(CardHeader::new().child(CardTitle::new(title)))
        .child(CardContent::new().child(count.get(cx).to_string()))
        .child(CardFooter::new().child(
            Button::new("increment").label("Increment").on_click(move |_, window, cx| {
                count.update(cx, |value| *value += step);
                if let Some(on_change) = &on_change {
                    on_change(&count.get(cx), window, cx);
                }
            }),
        ))
}

// Usage
Counter::new("Clicks").step(5).on_change(|value, _, _| println!("{value}"));
```

Parameter rules for `#[component]`:

- `window` and `cx` are the GPUI window and app context.
- Plain parameters are required props. `new(..)` takes them as `impl Into<T>`.
- `#[prop(optional)]` parameters default to `Default::default()` and get a builder method with
  the same name. For `Option<T>`, the method takes `impl Into<T>`.
- `EventHandler<E>` props, optional or not, take `Fn(&E, &mut Window, &mut App)` closures.
- One `#[children]` parameter of type `Vec<AnyElement>` makes the component a `ParentElement`.
- One `#[style]` parameter of type `StyleRefinement` makes the component `Styled`. Apply it last
  in the body with `.apply_style_overrides(&style_overrides)`.

### Hooks

`use_state` builds on GPUI's own `Window::use_state`. State lives as long as the component
keeps rendering at the same position, and updating it re-renders the owning view. Follow
React's rules of hooks: call hooks unconditionally and in the same order every render. Inside
loops, use `use_keyed_state(key, …)` with a stable key per item.

Long-lived or shared state still belongs in a GPUI view (`Entity<T>` with `cx.notify()`). The
gallery's settings card shows this pattern, with `cx.listener(..)` as the change handler.

## Styling and markup

rok-ui has a StyleX-style styling system and two ways to write element trees.
Both compile to ordinary GPUI builder calls, and both mix freely with builder code.

### `styles!`: define styles once

```rust
styles! {
    pub CARD = {
        base: {
            display: flex, direction: column, gap: 6, padding: 6,
            radius: xl, border: 1, border_color: border, background: card,
            hover: { border_color: ring },
        },
        compact: { padding: 3, gap: 3 },
        tone(Tone): {
            Calm: { background: muted },
            Loud: { background: primary/90, color: primary_foreground },
        },
    }
}
```

| StyleX | rok-ui |
|---|---|
| `stylex.create({...})` | `styles! { NAME = { key: {...} } }`: a static with one `Sx` per key |
| `stylex.props(a, cond && b)` | `.sx((&A.base, cond.then_some(&A.extra)))` or `.sx(sx![A.base, cond => A.extra])` |
| `':hover': {...}` | `hover: {...}` (also `focus`, `active`) |
| variants | `key(Enum): { Variant: {...} }`, then `A.key(value)`; a missing variant is a compile error |
| `xstyle` prop | components take `.sx(..)`, applied last so the caller wins |
| `defineVars` / `createTheme` | theme tokens (`primary`, `muted_foreground`, `md`) and presets |

Values follow Tailwind:
- **Lengths:** numbers are multiples of 4px (`gap: 6` is 24px). Also `0.5`, `50%`, `full`,
  `auto`, `px(10.)`, or any expression in braces (`{px(width)}`).
- **Colors:** theme tokens (`primary`, `border`), with opacity (`primary/90`), `transparent`,
  or `{expr}`.
- **Radius:** `none`, `sm`, `md`, `lg`, `xl`, `full`, or a length.
- **Text and font:** `text: xs..3xl`, `font: medium`/`semibold`/`bold`, `font_family: mono`.

Tokens resolve against the active theme when the style is applied, so the same static style
follows light/dark and preset switches. Unknown properties, tokens and keywords are compile
errors pointing at the mistake, with a suggestion when there is a close match.

For one-off or dynamic values, use `style! { width: {px(width)}, background: accent }`.
Your own `#[component]`s accept overrides with an `#[sx] sx: Sx` parameter: apply it last,
`div().sx((&MY.base, &sx))`.

### `children![]`: mixed children with control flow

```rust
div().children(children![
    Title::new("Projects"),
    if loading { Spinner::new() } else { Badge::new("Ready") },
    for project in &projects => Item::new(project.id).title(project.name.clone()),
    match status { Status::Ok => "Up to date", _ => Button::new("refresh").label("Refresh") },
])
```

### `view!`: JSX-like markup

```rust
view! {
    Card(sx = [CARD.base, compact => CARD.compact]) {
        CardHeader {
            CardTitle("Create project")
            CardDescription("Deploy your new project in one click.")
        }
        if let Some(error) = error {
            Alert("Deploy failed", description = error).destructive()
        }
        for project in &projects {
            Item(project.id, title = project.name.clone())
        }
        div(sx = ROW.end) {
            Button("cancel", label = "Cancel").outline()
            Button("deploy", label = "Deploy", on_click = move |_, _, cx| deploy(cx))
        }
    }
}
```

The syntax:
- **Components:** `Name(a, b, key = value)` is `Name::new(a, b).key(value)`. Method calls after
  the arguments pass through unchanged.
- **GPUI elements:** lowercase `div`, `img` and `svg` are GPUI's element functions.
- **Children:** `{ … }` holds children: elements, `"text"`, `{expr}`, `if`, `if let`, `match`
  and `for`.

## Components

Every component in shadcn/ui's catalog has a rok-ui counterpart.

| shadcn/ui | rok-ui | Notes |
|---|---|---|
| Accordion | `Accordion`, `AccordionItem` | Single or `.multiple(true)`; uncontrolled (`default_open`) or controlled (`open_items`). |
| Alert | `Alert` | `Default` and `Destructive`, with icon, title and description. |
| Alert Dialog | `AlertDialog` | Modal that needs an answer; the backdrop does not dismiss it, Escape cancels. |
| Aspect Ratio | `AspectRatio` | `AspectRatio::new(16. / 9.)`; children fill the box. |
| Attachment | `Attachment`, `AttachmentState` | File or image chip: metadata, upload progress, failure, retry and remove. |
| Avatar | `Avatar` | Image with an initials fallback. |
| Badge | `Badge` | `Primary`, `Secondary`, `Destructive`, `Outline`, optional icon. |
| Breadcrumb | `Breadcrumb` | Links, current page, `…` (optionally opening a menu), custom separator. |
| Bubble | `Bubble` | Chat bubble: variants, alignment, run grouping, reactions, "Show more" collapsing. |
| Button | `Button` | Six variants, four sizes, icons, loading, disabled, tooltip. |
| Button Group | `ButtonGroup`, `ButtonGroupText` | Joins any styled controls; horizontal or vertical. |
| Calendar | `Calendar`, `CalendarDate`, `DateRange` | Single or range selection, several months, min / max / disabled dates. |
| Card | `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardContent`, `CardFooter` | Composed exactly like shadcn/ui. |
| Carousel | `Carousel` | Animated slides, several per view, arrows, dots, keyboard, wrap-around. |
| Chart | `Chart`, `ChartSeries`, `ChartKind` | Bar (grouped or stacked), line, area, pie and donut; grid, legend, y-axis, hover tooltip. |
| Checkbox | `Checkbox` | Controlled, optional label. |
| Collapsible | `Collapsible` | Trigger, always-visible peek content and a collapsible panel. |
| Combobox | `Combobox` | Searchable select built from a popover and a command list. |
| Command | `Command`, `CommandItem`, `CommandDialog` | Filtered, grouped actions with Up / Down / Enter; `CommandDialog` is the ⌘K palette. |
| Context Menu | `ContextMenu` | Right-click menu at the pointer. |
| Data Table | `DataTable`, `DataColumn` | Filter, sort, row selection, column visibility, pagination, row actions. |
| Date Picker | `DatePicker` | Calendar in a popover; single or range, with presets. |
| Dialog | `Dialog` | Closes on Escape, backdrop click or the close button. |
| Direction | `Direction`, `TextDirection` | RTL subtree: mirrors breadcrumbs, pagination, carousels and sidebars. See limitations. |
| Drawer | `Drawer` | Bottom sheet with a grab handle. |
| Dropdown Menu | `DropdownMenu`, `Menu`, `MenuItem` | Icons, shortcuts, checkbox and radio items, submenus, labels, keyboard navigation. |
| Empty | `Empty` | Icon or media, title, description and actions; optional dashed border. |
| Field | `Field`, `FieldLabel`, `FieldDescription`, `FieldError`, `FieldGroup`, `FieldSet`, `FieldLegend`, `FieldSeparator`, `FieldContent`, `FieldTitle` | Vertical or horizontal form fields. |
| Hover Card | `HoverCard` | Opens after a delay, stays open while the pointer is on the trigger or card. |
| Input | `Input`, `InputState` | Single-line field: placeholder, masked text, leading icon, invalid state. |
| Input Group | `InputGroup` | Text, icon or element addons inside the border; textarea mode with toolbars. |
| Input OTP | `InputOtp` | One box per character, groups with separators, paste, digits or alphanumeric. |
| Item | `Item`, `ItemGroup` | Media, title, description and actions; default, outline or muted. |
| Kbd | `Kbd` (= `KeyboardShortcut`), `KbdGroup` | Key caps. |
| Label | `Label` | Form label with a disabled state. |
| Marker | `Marker` | Inline status, system note, bordered event row or labeled separator. |
| Menubar | `Menubar` | Desktop menu bar; hover switches menus while one is open, Left / Right too. |
| Message | `Message` | Conversation turn with avatar, name, timestamp and footer; start or end aligned. |
| Message Scroller | `MessageScroller`, `MessageScrollerState` | Virtualized chat log: follows new and streamed messages, keeps place when history loads, jump-to-latest, scroll to any message. |
| Native Select | `NativeSelect` | Full-width select styled like a native control (GPUI has no platform select). |
| Navigation Menu | `NavigationMenu`, `NavigationMenuLink` | Links plus titles that reveal content panels on hover. |
| Pagination | `Pagination` | Previous / next, page numbers and ellipses. |
| Popover | `Popover` | Floating panel; uncontrolled or controlled, four sides, two alignments. |
| Progress | `Progress` | Percentage bar. |
| Questionnaire | `Questionnaire`, `Question` | Steps of single-choice, multiple-choice and freeform questions, any skippable; summary at the end. |
| Radio Group | `RadioGroup` | Options with optional descriptions; arrow keys change the selection. |
| Resizable | `ResizablePanelGroup`, `ResizablePanel` | Draggable (and keyboard-adjustable) handles with min / max sizes. |
| Scroll Area | `ScrollArea` | Thin themed scrollbars that can be dragged; vertical, horizontal or both. |
| Select | `Select` | Groups, separators, disabled options, keyboard navigation. |
| Separator | `Separator` | Horizontal or vertical. |
| Sheet | `Sheet` | Dialog attached to any edge of the window. |
| Sidebar | `Sidebar`, `SidebarGroup`, `SidebarItem`, `SidebarTrigger` | Header, footer, groups, nested items, badges; collapses to icons with tooltips. |
| Skeleton | `Skeleton` | Pulsing placeholder. |
| Slider | `Slider` | Single value or range; step snapping; drag, click or arrow keys. |
| Spinner | `Spinner` | Rotating loader. |
| Switch | `Switch` | Controlled toggle with a label. |
| Table | `Table`, `TableHeader`, `TableBody`, `TableFooter`, `TableRow`, `TableHead`, `TableCell`, `TableCaption` | Flex-based rows; size a column by giving its cells a width. |
| Tabs | `Tabs` | Controlled tab list; you render the panel. |
| Textarea | `Textarea`, `use_textarea_state` | Multi-line field that grows with its text; Enter adds a line, Ctrl/Cmd-Enter submits. |
| Toast | `toast(cx, Toast::…)`, `Toaster` | Sonner-style: variants, loading toasts, actions, update and dismiss. `AppRoot` draws them. |
| Toggle | `Toggle` | Pressed / unpressed, default or outline. |
| Toggle Group | `ToggleGroup` | Single or multiple selection; outline groups join into one strip. |
| Tooltip | `Tooltip` | `Tooltip::text("…")` for any `.tooltip(..)`. |
| Typography | `H1`–`H4`, `P`, `Lead`, `Large`, `Small`, `Muted`, `Blockquote`, `List`, `InlineCode` | shadcn/ui's text styles. |

`Icon` ships 66 stroke icons in the Lucide style (`IconName::ALL`).

Value components are controlled, like React inputs: you pass `checked`, `value` or `open` in and
read changes back through `on_change` / `on_close`. Floating and disclosure components (popovers,
menus, selects, accordions, hover cards, carousels) keep their open state per element id, like
Radix, unless you pass `open` (or `index`, `open_items`) yourself.

## Theming

Tokens mirror shadcn/ui's CSS variables:

| shadcn/ui | rok-ui (`cx.theme().colors.*`) |
|---|---|
| `--background` / `--foreground` | `background` / `foreground` |
| `--card`, `--popover` (+ `-foreground`) | `card`, `popover` (+ `_foreground`) |
| `--primary`, `--secondary`, `--muted`, `--accent` (+ `-foreground`) | same names in snake_case |
| `--destructive` (+ `-foreground`) | `destructive`, `destructive_foreground`, plus `destructive_text` for error text on surfaces |
| `--border`, `--input`, `--ring` | `border`, `input`, `ring` |
| `--radius` | `theme.radius`, with `radius_small()`, `radius_medium()`, `radius_large()`, `radius_extra_large()` |

```rust
Theme::toggle_mode(cx);                                         // light ↔ dark
Theme::change_preset(ThemePreset::Neutral, cx);                 // switch palette
Theme::sync_with_system_appearance(window, cx);                 // follow the OS

// A custom brand: start from a preset and override tokens.
let mut theme = Theme::from_preset(ThemePreset::Neutral, ThemeMode::Light);
theme.colors.primary = gpui::rgb(0x2F5D50).into();
theme.radius = px(10.);
Theme::set_global(theme, cx);
```

**Rok** (the default) uses warm stone neutrals, an ember accent (`#B4400F` light, `#EA7A45` dark)
and 4px corners. **Neutral** is shadcn/ui's neutral palette with 8px corners. Two of its values
are darkened (muted text `#666666`, destructive `#DC2626`) so every pair passes 4.5:1. The
`every_text_pair_reaches_four_point_five_to_one` test enforces this for every preset and mode.

If your app has its own assets, layer rok-ui's icons over them:

```rust
Application::new().with_assets(rok_ui::Assets::with_fallback(MyAssets))
```

## Project layout

```
rok-ui/
├── Cargo.toml              workspace + the rok-ui crate
├── macros/                 rok-ui-macros: the #[component] attribute
├── assets/icons/           built-in SVG icons (embedded at compile time)
├── src/
│   ├── lib.rs              init(), re-exports
│   ├── prelude.rs          use rok_ui::prelude::*
│   ├── theme/              Theme, ThemeColors, presets, ActiveTheme
│   ├── hooks.rs            use_state, use_keyed_state, State, EventHandler
│   ├── styles.rs           ApplyStyleOverrides, ComponentSize
│   ├── icon.rs             Icon, IconName, Assets
│   └── components/         one file per component
├── examples/               counter.rs, gallery.rs (+ gallery/pages.rs)
├── tests/components.rs     macro API, hooks, full render under every theme
└── docs/screenshots/
```

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

To publish, release the macro crate first, then the main crate:

```sh
cargo publish -p rok-ui-macros
cargo publish -p rok-ui
```

## Known limitations (0.1)

- Focus rings show after mouse clicks as well as keyboard focus. GPUI 0.2.2 has no
  `:focus-visible` equivalent.
- Dialogs move focus into the panel, but they do not trap Tab yet. Focus does not jump to the
  first field automatically.
- GPUI shapes text left to right only. `Direction` mirrors layouts and arrows but does not
  reorder bidirectional text.
- `CalendarDate::today()` is the UTC date; rok-ui has no time-zone database.
- Floating surfaces (popovers, menus, selects) dismiss on a click outside themselves, so a popover
  nested inside another popover closes its parent when clicked.
- Font weights depend on the system UI font. Some Linux fonts have no medium or semibold weight,
  so those render as regular.

## License

MIT
