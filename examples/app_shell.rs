//! Flutter-style app shells and layout widgets.
//!
//! Run with `cargo run --example app_shell`. Resize the window: the navigation
//! moves from a bottom bar (under 600 px) to a rail, and to a rail with labels
//! from 1200 px. Options:
//!
//! - `--drawer`: a `Scaffold` with a modal `NavigationDrawer` and an end drawer
//!   instead of the adaptive shell.
//! - `--width <px>`: the starting window width.
//! - `--rtl`: right to left.

use rok_ui::prelude::*;

const PHOTOS: [(&str, f32); 9] = [
    ("Dunes", 0.08),
    ("Harbor", 0.58),
    ("Forest", 0.33),
    ("Canyon", 0.03),
    ("Glacier", 0.52),
    ("Meadow", 0.25),
    ("Lagoon", 0.48),
    ("Sunset", 0.95),
    ("Valley", 0.38),
];

fn destinations() -> Vec<NavigationDestination> {
    vec![
        NavigationDestination::new(IconName::Home, "Home"),
        NavigationDestination::new(IconName::Image, "Albums"),
        NavigationDestination::new(IconName::Heart, "Favorites").badge("3"),
        NavigationDestination::new(IconName::Settings, "Settings"),
    ]
}

/// A photo tile: a colored box with its name and a heart pinned on top.
fn photo(name: &'static str, hue: f32) -> impl IntoElement {
    Stack::new()
        .child(
            SizedBox::height(px(140.)).w_full().child(
                div()
                    .rounded_lg()
                    .bg(gpui::hsla(hue, 0.55, 0.55, 1.))
                    .size_full(),
            ),
        )
        .positioned(
            Positioned::new().bottom(px(10.)).start(px(12.)).child(
                div()
                    .text_color(gpui::white())
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(name),
            ),
        )
        .positioned(
            Positioned::new().top(px(10.)).end(px(10.)).child(
                Icon::new(IconName::Heart)
                    .size(px(18.))
                    .color(gpui::white()),
            ),
        )
}

fn home_page() -> impl IntoElement {
    Padding::all(px(24.)).child(
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .spacing(px(16.))
            .child(
                Row::new()
                    .child(H3::new("Recent photos"))
                    .child(Spacer::new())
                    .child(Button::new("see-all").ghost().label("See all")),
            )
            .child(
                Wrap::new().spacing(px(8.)).run_spacing(px(8.)).children(
                    [
                        "All",
                        "Landscapes",
                        "Water",
                        "Mountains",
                        "Cities",
                        "People",
                    ]
                    .map(|tag| Badge::new(tag).variant(BadgeVariant::Secondary)),
                ),
            )
            .child(
                GridView::extent("photos", px(240.))
                    .spacing(px(12.))
                    .children(PHOTOS.map(|(name, hue)| photo(name, hue))),
            ),
    )
}

fn albums_page() -> impl IntoElement {
    Padding::all(px(24.)).child(
        GridView::count(2).spacing(px(16.)).children(
            [
                ("Travel", "128 photos"),
                ("Family", "64 photos"),
                ("Work", "12 photos"),
            ]
            .map(|(title, count)| {
                Card::new().child(
                    CardHeader::new()
                        .child(CardTitle::new(title))
                        .child(CardDescription::new(count)),
                )
            }),
        ),
    )
}

fn favorites_page() -> impl IntoElement {
    SizedBox::height(px(420.)).child(
        Center::new().child(
            Empty::new()
                .icon(IconName::Heart)
                .title("No favorites yet")
                .description("Tap the heart on a photo to keep it here."),
        ),
    )
}

fn settings_page(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let backup = use_state(window, cx, || true);
    let originals = use_state(window, cx, || false);
    let row = |label: &'static str, description: &'static str, control: AnyElement| {
        Row::new()
            .spacing(px(16.))
            .child(
                Expanded::new().child(
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .spacing(px(2.))
                        .child(div().font_weight(FontWeight::MEDIUM).child(label))
                        .child(Muted::new(description)),
                ),
            )
            .child(control)
    };
    Padding::all(px(24.)).child(
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .spacing(px(20.))
            .child(row(
                "Back up photos",
                "Upload new photos when you are on Wi-Fi.",
                Switch::new("backup")
                    .checked(backup.get(cx))
                    .on_change(move |checked, _, cx| backup.set(*checked, cx))
                    .into_any_element(),
            ))
            .child(row(
                "Keep originals",
                "Store full-resolution copies. Uses more space.",
                Switch::new("originals")
                    .checked(originals.get(cx))
                    .on_change(move |checked, _, cx| originals.set(*checked, cx))
                    .into_any_element(),
            )),
    )
}

#[component]
fn PageBody(page: usize, window: &mut Window, cx: &mut App) -> impl IntoElement {
    match page {
        0 => home_page().into_any_element(),
        1 => albums_page().into_any_element(),
        2 => favorites_page().into_any_element(),
        _ => settings_page(window, cx).into_any_element(),
    }
}

fn app_bar(title: &'static str) -> AppBar {
    AppBar::new()
        .title(title)
        .action(
            Button::new("search")
                .ghost()
                .icon_only(IconName::Search)
                .tooltip("Search"),
        )
        .action(
            Button::new("theme")
                .ghost()
                .icon_only(IconName::Moon)
                .tooltip("Toggle dark mode")
                .on_click(|_, _, cx| Theme::toggle_mode(cx)),
        )
}

struct Shell {
    page: usize,
    drawer_mode: bool,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let select = move |index: &usize, _: &mut Window, cx: &mut App| {
            entity.update(cx, |shell, cx| {
                shell.page = *index;
                cx.notify();
            });
        };
        let titles = ["Photos", "Albums", "Favorites", "Settings"];
        let title = titles[self.page.min(titles.len() - 1)];
        let fab = FloatingActionButton::new("upload", IconName::Upload).label("Upload");

        let shell = if self.drawer_mode {
            Scaffold::new("drawer-shell")
                .app_bar(
                    app_bar(title).action(
                        Button::new("filters")
                            .ghost()
                            .icon_only(IconName::Filter)
                            .tooltip("Filters"),
                    ),
                )
                .drawer(
                    NavigationDrawer::new("drawer")
                        .header(H4::new("Photos"))
                        .destinations(destinations().into_iter().take(3))
                        .divider()
                        .section("Account")
                        .destinations(destinations().into_iter().skip(3))
                        .selected_index(self.page)
                        .on_change(select),
                )
                .end_drawer(
                    Padding::all(px(16.)).child(
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .spacing(px(12.))
                            .child(H4::new("Filters"))
                            .child(Muted::new("Narrow the photos you see.")),
                    ),
                )
                .floating_action_button(fab)
                .child(PageBody::new(self.page))
                .into_any_element()
        } else {
            AdaptiveScaffold::new("adaptive-shell")
                .app_bar(app_bar(title))
                .destinations(destinations())
                .selected_index(self.page)
                .on_change(select)
                .floating_action_button(fab)
                .child(PageBody::new(self.page))
                .into_any_element()
        };
        AppRoot::new().child(shell)
    }
}

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let drawer_mode = arguments.iter().any(|argument| argument == "--drawer");
    let rtl = arguments.iter().any(|argument| argument == "--rtl");
    let width = arguments
        .iter()
        .position(|argument| argument == "--width")
        .and_then(|index| arguments.get(index + 1))
        .and_then(|width| width.parse::<f32>().ok())
        .unwrap_or(1000.);

    Application::new()
        .with_assets(rok_ui::Assets)
        .run(move |cx: &mut App| {
            rok_ui::init(cx);
            if rtl {
                set_text_direction(TextDirection::Rtl, cx);
            }
            let bounds = Bounds::centered(None, gpui::size(px(width), px(760.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("rok-ui app shell".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| Shell {
                        page: 0,
                        drawer_mode,
                    })
                },
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}
