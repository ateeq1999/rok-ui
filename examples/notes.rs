//! Routing and reactive state: a small notes app.
//!
//! Run with `cargo run --example notes`.
//!
//! - `Router` maps paths to pages: `/`, `/notes/:id`, `/settings`, and a "not
//!   found" page. `Link` and `rok_ui::router::navigate` change the location;
//!   Alt+Left / Alt+Right go back and forward.
//! - The notes live in a rok-ui-hooks `Store`, shared by every page. The window
//!   tracks it with `cx.track`, so any change re-renders.
//! - The settings page keeps a per-element `use_signal`.

use rok_ui::prelude::*;
use rok_ui::router;

#[derive(Clone, Debug)]
struct Note {
    id: usize,
    title: SharedString,
    body: SharedString,
}

fn seed_notes() -> Vec<Note> {
    vec![
        Note {
            id: 1,
            title: "Groceries".into(),
            body: "Milk, eggs, dates and Arabic coffee.".into(),
        },
        Note {
            id: 2,
            title: "Release checklist".into(),
            body: "Bump versions, update the changelog, tag and publish.".into(),
        },
        Note {
            id: 3,
            title: "ملاحظة بالعربية".into(),
            body: "النص العربي يظهر بالترتيب الصحيح في كل مكان.".into(),
        },
    ]
}

/// The list page: every note, and a field to add one.
#[component]
fn NotesPage(notes: Store<Vec<Note>>, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let title = use_input_state("new-note", window, cx, |state| {
        state.with_placeholder("New note title")
    });
    let add = {
        let (notes, title) = (notes.clone(), title.clone());
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            let text = title.read(cx).text().clone();
            if text.trim().is_empty() {
                return;
            }
            notes.update(|notes| {
                let id = notes.iter().map(|note| note.id).max().unwrap_or(0) + 1;
                notes.push(Note {
                    id,
                    title: text,
                    body: "".into(),
                });
            });
            title.update(cx, |state, cx| state.set_text("", cx));
        }
    };
    let rows = notes.get().into_iter().map(|note| {
        Link::new(("note-link", note.id), format!("/notes/{}", note.id))
            .sx(style! { color: foreground })
            .child(
                Item::new(("note", note.id))
                    .title(note.title.clone())
                    .description(if note.body.is_empty() {
                        SharedString::from("Empty note")
                    } else {
                        note.body.clone()
                    }),
            )
    });
    Padding::all(px(24.)).child(
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .spacing(px(12.))
            .child(
                Row::new()
                    .spacing(px(8.))
                    .child(Expanded::new().child(Input::new(&title)))
                    .child(
                        Button::new("add-note")
                            .icon(IconName::Plus)
                            .label("Add")
                            .on_click(add),
                    ),
            )
            .children(rows),
    )
}

/// One note, looked up by the `:id` route parameter.
#[component]
fn NotePage(notes: Store<Vec<Note>>, id: usize) -> impl IntoElement {
    let Some(note) = notes.with(|notes| notes.iter().find(|note| note.id == id).cloned()) else {
        return Padding::all(px(24.))
            .child(
                Empty::new()
                    .icon(IconName::FileText)
                    .title("This note was deleted"),
            )
            .into_any_element();
    };
    let delete = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
        notes.update(|notes| notes.retain(|note| note.id != id));
        router::replace("/", cx);
    };
    Padding::all(px(24.))
        .child(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .spacing(px(16.))
                .child(
                    Button::new("back")
                        .ghost()
                        .icon(IconName::ArrowLeft.for_direction())
                        .label("Back")
                        .on_click(|_, _, cx| router::back(cx)),
                )
                .child(H2::new(note.title))
                .child(P::new(note.body))
                .child(
                    Button::new("delete-note")
                        .destructive()
                        .icon(IconName::Trash)
                        .label("Delete")
                        .on_click(delete),
                ),
        )
        .into_any_element()
}

#[component]
fn SettingsPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let (compact, set_compact) = use_signal(window, cx, || false);
    Padding::all(px(24.)).child(
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(px(16.))
            .child(H3::new("Settings"))
            .child(
                Switch::new("compact")
                    .label("Compact list")
                    .checked(compact.get())
                    .on_change(move |checked, _, _| set_compact.set(*checked)),
            )
            .child(Muted::new(if compact.get() {
                "Compact mode is on (a use_signal owned by this page)."
            } else {
                "Compact mode is off."
            }))
            .child(
                Link::new("missing-link", "/does/not/exist")
                    .child("Open a page that does not exist"),
            ),
    )
}

struct NotesApp {
    notes: Store<Vec<Note>>,
}

impl NotesApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let notes = create_store(seed_notes());
        let watched = notes.clone();
        // Re-render whenever the notes change, wherever the change came from.
        cx.track(move || watched.with(|_| ()));
        Self { notes }
    }
}

impl Render for NotesApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = if router::is_active("/settings", false, cx) {
            1
        } else {
            0
        };
        let title = match router::location(cx).path() {
            "/settings" => "Settings",
            _ => "Notes",
        };
        let count = self.notes.with(|notes| notes.len());
        let (list_notes, detail_notes) = (self.notes.clone(), self.notes.clone());

        AppRoot::new().child(
            AdaptiveScaffold::new("notes-shell")
                .app_bar(
                    AppBar::new()
                        .title(title)
                        .action(
                            Button::new("go-back")
                                .ghost()
                                .icon_only(IconName::ArrowLeft.for_direction())
                                .tooltip("Back (Alt+Left)")
                                .disabled(!router::can_go_back(cx))
                                .on_click(|_, _, cx| router::back(cx)),
                        )
                        .action(
                            Button::new("go-forward")
                                .ghost()
                                .icon_only(IconName::ArrowRight.for_direction())
                                .tooltip("Forward (Alt+Right)")
                                .disabled(!router::can_go_forward(cx))
                                .on_click(|_, _, cx| router::forward(cx)),
                        ),
                )
                .destination(
                    NavigationDestination::new(IconName::FileText, "Notes")
                        .badge(count.to_string()),
                )
                .destination(NavigationDestination::new(IconName::Settings, "Settings"))
                .selected_index(selected)
                .on_change(|index, _, cx| {
                    router::navigate(if *index == 1 { "/settings" } else { "/" }, cx)
                })
                .child(
                    Router::new()
                        .route("/", move |_, _, _| NotesPage::new(list_notes.clone()))
                        .route("/notes/:id", move |route, _, _| {
                            NotePage::new(
                                detail_notes.clone(),
                                route.param_as::<usize>("id").unwrap_or(0),
                            )
                        })
                        .route("/settings", |_, _, _| SettingsPage::new())
                        .redirect("/home", "/")
                        .not_found(|route, _, _| {
                            Padding::all(px(24.)).child(
                                Empty::new()
                                    .icon(IconName::CircleAlert)
                                    .title("Page not found")
                                    .description(format!("Nothing lives at {}.", route.path())),
                            )
                        }),
                ),
        )
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut App| {
            rok_ui::init(cx);
            let bounds = Bounds::centered(None, gpui::size(px(1000.), px(720.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("rok-ui notes".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(NotesApp::new),
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}
