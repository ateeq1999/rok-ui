# App shells and layouts

rok-ui's `shell` feature group (part of `full`) gives desktop apps the structure Flutter
developers know from Material: a `Scaffold` with an app bar, drawers and navigation, an
`AdaptiveScaffold` that follows the window width, and layout widgets (`Row`, `Column`,
`Expanded`, `Stack`, `GridView`, `LayoutBuilder` and the rest).

| Feature | Contents |
|---|---|
| `layout-widgets` | `Row`, `Column`, `Expanded`, `Flexible`, `Spacer`, `Center`, `Aligned`, `Padding`, `SizedBox`, `Stack`, `Positioned`, `Wrap`, `GridView`, `LayoutBuilder`, `WindowSizeClass` |
| `navigation` | `NavigationBar`, `NavigationRail`, `NavigationDrawer`, `NavigationDestination` |
| `scaffold` | `Scaffold`, `AppBar`, `FloatingActionButton`, `AdaptiveScaffold` (enables the two above) |

Run `cargo run --example app_shell` and resize the window to see them adapt.

## Layout widgets

### Row and Column

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
# let search = use_input_state("search", window, cx, |state| state);
let recent = Column::new()
    .main_axis_alignment(MainAxisAlignment::Start)        // along the column
    .cross_axis_alignment(CrossAxisAlignment::Stretch)    // across it
    .main_axis_size(MainAxisSize::Min)                    // only as tall as the children
    .spacing(px(12.))                                     // gap between children
    .child(H3::new("Recent"))
    .child(
        Row::new()
            .spacing(px(8.))
            .child(Expanded::new().child(Input::new(&search)))   // takes the free width
            .child(Button::new("go").label("Search")),
    );
# }
```

| Setting | Values | Default |
|---|---|---|
| `main_axis_alignment` | `Start`, `End`, `Center`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly` | `Start` |
| `cross_axis_alignment` | `Start`, `End`, `Center`, `Stretch`, `Baseline` | `Center` |
| `main_axis_size` | `Max` (fill the parent along the main axis), `Min` | `Max` |
| `spacing` | `Pixels` | `0` |

`Start` and `End` follow the reading direction, like Flutter's `AlignmentDirectional`. A `Row`
flows right to left in RTL, and `CrossAxisAlignment::Start` in a `Column` hugs the right edge.

### Expanded, Flexible and Spacer

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
# let (main_panel, side_panel, logo, user_menu, long_label) = (div(), div(), div(), div(), div());
let split = Row::new()
    .child(Expanded::new().flex(2.).child(main_panel))     // two thirds of the free space
    .child(Expanded::new().child(side_panel));             // one third
let header = Row::new()
    .child(logo)
    .child(Spacer::new())                                  // pushes what follows to the end
    .child(user_menu);
let label = Row::new()
    .child(Flexible::new().child(long_label));             // may grow, keeps its natural size
# }
```

`Expanded` makes its children fill the space it gets. `Flexible` lets them stay smaller
(Flutter's `FlexFit.loose`).

### Padding, SizedBox, Center, Aligned

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
# let content = || div();
# let (chart, canvas) = (div(), div());
let padded = Padding::all(px(16.)).child(content());
let wide = Padding::symmetric(px(24.), px(8.)).child(content());        // horizontal, vertical
let custom = Padding::new(EdgeInsets::zero().top(px(8.)).start(px(16.))).child(content());

let gap = SizedBox::height(px(24.));                                   // a vertical gap
let frame = SizedBox::new(px(320.), px(200.)).child(chart);            // a fixed frame
let square = SizedBox::square(px(48.)).child(Avatar::new("LA"));
let fill = SizedBox::expand().child(canvas);                           // fill the parent

let centered = Center::new().child(Spinner::new());                    // fills, centers
let corner = Aligned::new(Alignment::BottomEnd).child(Button::new("next").label("Next"));
# }
```

`EdgeInsets` uses `start` and `end`, which follow the reading direction.

### Stack and Positioned

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
let avatar = Stack::new()
    .alignment(Alignment::Center)                        // where `.child(..)` children sit
    .child(SizedBox::square(px(64.)).child(Avatar::new("LA")))
    .positioned(
        Positioned::new()
            .bottom(px(0.))
            .end(px(0.))
            .child(div().size(px(14.)).rounded_full().bg(gpui::green())),
    );
# }
```

`.child(..)` children are layered and size the stack. `.positioned(..)` children are pinned to
its edges (`top`, `bottom`, `start`, `end`, `width`, `height`, or `Positioned::fill()`).
`StackFit::Expand` makes the stack fill its parent.

### Wrap and GridView

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
# let tags = ["rust", "gpui"].into_iter();
# let cards: Vec<AnyElement> = Vec::new();
# let photos: Vec<AnyElement> = Vec::new();
let tag_cloud = Wrap::new().spacing(px(8.)).run_spacing(px(8.)).children(tags.map(Badge::new));

let grid = GridView::count(3).spacing(px(12.)).children(cards);   // always 3 columns
let gallery = GridView::extent("photos", px(240.))                // as many 240 px columns as fit
    .spacing(px(12.))
    .row_height(px(180.))
    .children(photos);
# }
```

Grids fill in reading order, right to left in RTL. `extent` measures its own width, so it
needs an id. Put long grids inside a scrolling container. `Scaffold` bodies scroll by
default.

### LayoutBuilder and size classes

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
# fn list() -> Div { div() }
# fn detail() -> Div { div() }
let inbox = LayoutBuilder::new("inbox", |constraints, _, _| {
    if constraints.max_width >= px(900.) {
        Row::new()
            .child(SizedBox::width(px(320.)).child(list()))
            .child(Expanded::new().child(detail()))
            .into_any_element()
    } else {
        list().into_any_element()
    }
});
# }
```

`LayoutBuilder` fills its parent's width, measures itself after layout, and rebuilds when its
size changes. The first frame uses the window size. `constraints.size_class()` and
`WindowSizeClass::of(window)` give Material 3's breakpoints:

| Class | Width |
|---|---|
| `Compact` | under 600 px |
| `Medium` | 600–839 px |
| `Expanded` | 840–1199 px |
| `Large` | 1200–1599 px |
| `ExtraLarge` | 1600 px and up |

Size classes are ordered, so you can compare them: `if class >= WindowSizeClass::Medium { … }`.

### Differences from Flutter

Sizing is CSS flexbox, not Flutter's box constraints. In practice:

- **A `Column` inside a `Column` fills the parent's height** with the default
  `MainAxisSize::Max`. Use `MainAxisSize::Min`, or wrap it in `Expanded` to share the height.
- **Boxes without content have no width.** A colored `SizedBox::height(..)` inside a `Stack`
  or a `Row` needs `.w_full()` (or `Expanded`) to be visible. In Flutter it would expand to the
  incoming constraints.
- Every widget implements `Styled` and takes `.sx(..)`, so `.w_full()`, `.bg(..)` or a
  `style! { .. }` table work on all of them.

## Navigation

`NavigationDestination` describes one place in the app. The navigation components take a
list of them, a `selected_index` and an `on_change` callback. Like every rok-ui control, they
are controlled: you store the selection.

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
let destinations = vec![
    NavigationDestination::new(IconName::Inbox, "Inbox").badge("24"),     // a count
    NavigationDestination::new(IconName::Star, "Starred").badge(""),      // a dot
    NavigationDestination::new(IconName::Send, "Sent").selected_icon(IconName::Check),
    NavigationDestination::new(IconName::Trash, "Trash").disabled(true),
];
# }
```

| Component | Use it for |
|---|---|
| `NavigationBar` | 3–5 destinations along the bottom, compact windows. `label_behavior`: `AlwaysShow`, `OnlyShowSelected`, `AlwaysHide` (labels become tooltips). |
| `NavigationRail` | A column of destinations along the start edge. `.extended(true)` shows labels beside icons; `.leading(..)` and `.trailing(..)` hold a FAB, menu button or avatar; `.alignment(Top \| Center \| Bottom)`. |
| `NavigationDrawer` | A panel of destinations with `.header(..)`, `.section("Labels")` and `.divider()`. Indices count destinations only. |

All of them support the keyboard (Tab, then Enter or Space) and mirror in RTL.

## Scaffold

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
# let destinations: Vec<NavigationDestination> = Vec::new();
# let page = 0;
# fn set_page(_: usize, _: &mut App) {}
# let (filters_panel, message_list) = (div(), div());
let mail = Scaffold::new("mail")
    .app_bar(
        AppBar::new()
            .title("Inbox")
            .action(Button::new("search").ghost().icon_only(IconName::Search))
            .bottom(Tabs::new("folders").tab("Primary").tab("Updates")),
    )
    .drawer(
        NavigationDrawer::new("mail-drawer")
            .header(H4::new("Mail"))
            .destinations(destinations)
            .selected_index(page)
            .on_change(move |index, _, cx| set_page(*index, cx)),
    )
    .end_drawer(filters_panel)
    .floating_action_button(FloatingActionButton::new("compose", IconName::Pencil).label("Compose"))
    .fab_location(FabLocation::EndFloat)
    .child(message_list);
# }
```

| Slot | Notes |
|---|---|
| `app_bar(AppBar)` | `leading`, `title` (text) or `title_element`, `action` (repeatable), `center_title`, `bottom` |
| body (`.child`) | Scrolls vertically; `.body_scrollable(false)` when the body manages its own scrolling |
| `drawer(..)` | Slides in from the start edge. The app bar gets a menu button that opens it when it has no `leading` |
| `end_drawer(..)` | Slides in from the end edge; the app bar gets a button when it has no actions |
| `drawer_open(bool)` + `on_drawer_change` | Control the start drawer yourself; otherwise it keeps its state per scaffold id |
| `navigation(..)` | Permanent side navigation, usually a `NavigationRail` |
| `bottom_navigation_bar(..)` | Usually a `NavigationBar` |
| `bottom_sheet(..)`, `footer_button(..)` | A panel under the body, and a row of buttons |
| `floating_action_button(..)` | `fab_location`: `EndFloat`, `CenterFloat`, `StartFloat` |

A `NavigationDrawer` inside a scaffold's drawer closes it when a destination is picked.
Escape and clicking the backdrop close it too.

### FloatingActionButton

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
let primary = FloatingActionButton::new("add", IconName::Plus);                         // 56 px, primary
let extended = FloatingActionButton::new("add", IconName::Plus).label("New note");    // extended
let small = FloatingActionButton::new("edit", IconName::Pencil)
    .size(FabSize::Small)
    .variant(FabVariant::Surface);
# }
```

Sizes are `Small` (40 px), `Regular` (56 px) and `Large` (96 px). Variants are `Primary`,
`Secondary` and `Surface`. A hidden label (`.show_label(false)`) becomes the tooltip.

## AdaptiveScaffold

One description of your app's destinations, three layouts:

```rust,no_run
# use rok_ui::prelude::*;
# fn example(window: &mut Window, cx: &mut App) {
# let destinations: Vec<NavigationDestination> = Vec::new();
# let page = 0;
# fn set_page(_: usize, _: &mut App) {}
# let page_content = div();
let app = AdaptiveScaffold::new("app")
    .app_bar(AppBar::new().title("Photos"))
    .destinations(destinations)
    .selected_index(page)
    .on_change(move |index, _, cx| set_page(*index, cx))
    .floating_action_button(FloatingActionButton::new("upload", IconName::Upload).label("Upload"))
    .child(page_content);
# }
```

| Width | Navigation | FAB |
|---|---|---|
| Below `rail_from` (default `Medium`, 600 px) | Bottom `NavigationBar` | Floating, bottom end |
| From `rail_from` | `NavigationRail` | Icon-only, at the top of the rail |
| From `extended_from` (default `Large`, 1200 px) | Extended rail with labels | Extended, at the top of the rail |

`.rail_from(..)` and `.extended_from(..)` take any `WindowSizeClass`. The scaffold measures
its own width with a `LayoutBuilder`, so it adapts correctly inside a split view too.

With the `router` feature, derive `selected_index` from the location and navigate in
`on_change`. The routing guide shows how.
