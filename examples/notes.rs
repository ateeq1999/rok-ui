//! A small notes app built on the BLoC architecture (`docs/guide/architecture.md`).
//!
//! Run with `cargo run --example notes --features bloc`; `cargo test --example notes
//! --features bloc` runs its bloc tests.
//!
//! The three layers, as modules here (as folders in an app made with `cargo rok-ui new
//! --template bloc`):
//!
//! - `data`: the `Note` model and `NotesRepository`, a trait with an in-memory implementation.
//! - `features::notes::bloc`: `NotesBloc` turns `NotesEvent`s into `NotesState`s, using only
//!   the repository.
//! - `features::notes::view` and `main`: pages read the bloc with `cx.bloc`, show its state
//!   with `BlocBuilder` and add events from handlers. Local UI state (a settings switch) stays
//!   in `use_signal`.

/// The data layer.
mod data {
    use std::sync::{Mutex, PoisonError};

    use rok_ui::prelude::SharedString;

    /// A note.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Note {
        pub id: usize,
        pub title: SharedString,
        pub body: SharedString,
    }

    /// Where notes live. A trait, so tests and other backends can stand in.
    pub trait NotesRepository: Send + Sync {
        /// Every note.
        fn list(&self) -> Vec<Note>;
        /// Store a new note and return it.
        fn add(&self, title: SharedString) -> Note;
        /// Forget a note.
        fn delete(&self, id: usize);
    }

    /// Notes in memory, seeded with a few.
    pub struct InMemoryNotes(Mutex<Vec<Note>>);

    impl InMemoryNotes {
        pub fn seeded() -> Self {
            Self(Mutex::new(vec![
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
            ]))
        }
    }

    impl NotesRepository for InMemoryNotes {
        fn list(&self) -> Vec<Note> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        fn add(&self, title: SharedString) -> Note {
            let mut notes = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let id = notes.iter().map(|note| note.id).max().unwrap_or(0) + 1;
            let note = Note {
                id,
                title,
                body: SharedString::default(),
            };
            notes.push(note.clone());
            note
        }

        fn delete(&self, id: usize) {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .retain(|note| note.id != id);
        }
    }
}

/// Features: business logic and views, one module per feature.
mod features {
    pub mod notes {
        /// Business logic: no GPUI, no components.
        pub mod bloc {
            use std::sync::Arc;

            use rok_ui::{
                bloc::{Bloc, Emitter},
                prelude::SharedString,
            };

            use crate::data::{Note, NotesRepository};

            /// What happened (past tense).
            #[derive(Debug)]
            pub enum NotesEvent {
                NotesRequested,
                NoteAdded { title: SharedString },
                NoteDeleted { id: usize },
            }

            /// How far loading got.
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
            pub enum NotesStatus {
                #[default]
                Initial,
                Success,
            }

            /// What the views show.
            #[derive(Clone, Debug, Default, PartialEq, Eq)]
            pub struct NotesState {
                pub status: NotesStatus,
                pub notes: Vec<Note>,
            }

            /// Notes: listing, adding and deleting through the repository.
            pub struct NotesBloc {
                repository: Arc<dyn NotesRepository>,
            }

            impl NotesBloc {
                pub fn new(repository: Arc<dyn NotesRepository>) -> Self {
                    Self { repository }
                }
            }

            impl Bloc for NotesBloc {
                type Event = NotesEvent;
                type State = NotesState;

                fn initial_state(&self) -> NotesState {
                    NotesState::default()
                }

                async fn on(&self, event: NotesEvent, emit: &Emitter<NotesState>) {
                    match event {
                        NotesEvent::NotesRequested => {}
                        NotesEvent::NoteAdded { title } => {
                            if title.trim().is_empty() {
                                return;
                            }
                            self.repository.add(title);
                        }
                        NotesEvent::NoteDeleted { id } => self.repository.delete(id),
                    }
                    emit.emit(NotesState {
                        status: NotesStatus::Success,
                        notes: self.repository.list(),
                    });
                }
            }
        }

        /// The pages.
        pub mod view {
            use rok_ui::{bloc::BlocBuilder, prelude::*, router};

            use super::bloc::{NotesBloc, NotesEvent};

            /// The list page: every note, and a field to add one.
            #[component]
            pub fn NotesPage(cx: &mut Cx) -> impl IntoElement {
                let notes = cx.bloc::<NotesBloc>();
                // Once, when the page opens: a state initializer runs on the first render only.
                let requested = notes.clone();
                cx.use_state(move || requested.add(NotesEvent::NotesRequested));
                let title = use_input_state("new-note", cx.window, cx.app, |state| {
                    state.with_placeholder("New note title")
                });
                let add = {
                    let (notes, title) = (notes.clone(), title.clone());
                    move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                        let text = title.read(cx).text().clone();
                        notes.add(NotesEvent::NoteAdded { title: text });
                        title.update(cx, |state, cx| state.set_text("", cx));
                    }
                };
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
                        .child(BlocBuilder::new(&notes, |state, _, _| {
                            Column::new()
                                .main_axis_size(MainAxisSize::Min)
                                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                .spacing(px(8.))
                                .children(state.notes.iter().map(|note| {
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
                                }))
                        })),
                )
            }

            /// One note, looked up by the `:id` route parameter.
            #[component]
            pub fn NotePage(id: usize, cx: &mut Cx) -> impl IntoElement {
                let notes = cx.bloc::<NotesBloc>();
                let state = notes.state();
                let Some(note) = state.notes.iter().find(|note| note.id == id).cloned() else {
                    return Padding::all(px(24.))
                        .child(
                            Empty::new()
                                .icon(IconName::FileText)
                                .title("This note was deleted"),
                        )
                        .into_any_element();
                };
                let delete = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                    notes.add(NotesEvent::NoteDeleted { id });
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

            /// Settings: purely local UI state, so a `use_signal`, not a bloc.
            #[component]
            pub fn SettingsPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
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
        }
    }
}

use std::sync::Arc;

use rok_ui::{
    bloc::{BlocProvider, RepositoryProvider},
    prelude::*,
    router,
};

use data::{InMemoryNotes, NotesRepository};
use features::notes::{
    bloc::NotesBloc,
    view::{NotePage, NotesPage, SettingsPage},
};

/// The shell, rendered below the providers so it can read the bloc.
#[component]
fn Shell(cx: &mut Cx) -> impl IntoElement {
    let selected = usize::from(router::is_active("/settings", false, cx));
    let title = match router::location(cx).path() {
        "/settings" => "Settings",
        _ => "Notes",
    };
    let count = cx.watch_bloc::<NotesBloc>().notes.len();
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
            NavigationDestination::new(IconName::FileText, "Notes").badge(count.to_string()),
        )
        .destination(NavigationDestination::new(IconName::Settings, "Settings"))
        .selected_index(selected)
        .on_change(|index, _, cx| {
            router::navigate(if *index == 1 { "/settings" } else { "/" }, cx);
        })
        .child(
            Router::new()
                .route("/", |_, _, _| NotesPage::new())
                .route("/notes/:id", |route, _, _| {
                    NotePage::new(route.param_as::<usize>("id").unwrap_or(0))
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
        )
}

struct NotesApp;

impl Render for NotesApp {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // Repositories, then blocs built from them, then the UI.
        AppRoot::new().child(
            RepositoryProvider::new()
                .provide::<dyn NotesRepository>(Arc::new(InMemoryNotes::seeded()))
                .child(
                    BlocProvider::bloc(|scope| {
                        NotesBloc::new(scope.repository::<dyn NotesRepository>())
                    })
                    .child(Shell::new()),
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
                |_, cx| cx.new(|_| NotesApp),
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}

/// Events in, states out, with a fake repository.
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use rok_ui::{bloc::test, prelude::SharedString};

    use crate::{
        data::{Note, NotesRepository},
        features::notes::bloc::{NotesBloc, NotesEvent, NotesStatus},
    };

    /// Remembers what it was asked to do.
    #[derive(Default)]
    struct FakeNotes(Mutex<Vec<Note>>);

    impl NotesRepository for FakeNotes {
        fn list(&self) -> Vec<Note> {
            self.0.lock().unwrap().clone()
        }

        fn add(&self, title: SharedString) -> Note {
            let note = Note {
                id: self.0.lock().unwrap().len() + 1,
                title,
                body: SharedString::default(),
            };
            self.0.lock().unwrap().push(note.clone());
            note
        }

        fn delete(&self, id: usize) {
            self.0.lock().unwrap().retain(|note| note.id != id);
        }
    }

    fn titles(state: &crate::features::notes::bloc::NotesState) -> Vec<&str> {
        state.notes.iter().map(|note| note.title.as_ref()).collect()
    }

    #[test]
    fn adding_and_deleting_notes() {
        let states = test::run(
            NotesBloc::new(Arc::new(FakeNotes::default())),
            [
                NotesEvent::NotesRequested,
                NotesEvent::NoteAdded {
                    title: "Groceries".into(),
                },
                NotesEvent::NoteAdded {
                    title: "Taxes".into(),
                },
                NotesEvent::NoteDeleted { id: 1 },
            ],
        );
        assert_eq!(states[0].status, NotesStatus::Success);
        let titles: Vec<Vec<&str>> = states.iter().map(titles).collect();
        assert_eq!(
            titles,
            [
                vec![],
                vec!["Groceries"],
                vec!["Groceries", "Taxes"],
                vec!["Taxes"]
            ]
        );
    }

    #[test]
    fn blank_titles_are_ignored() {
        let states = test::run(
            NotesBloc::new(Arc::new(FakeNotes::default())),
            [NotesEvent::NoteAdded { title: "  ".into() }],
        );
        assert!(states.is_empty(), "nothing changed, so nothing was emitted");
    }
}
