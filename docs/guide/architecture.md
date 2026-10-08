# Architecture: BLoC

Every rok-ui app, small or large, is built in three layers (the BLoC pattern, from
[bloclibrary.dev](https://bloclibrary.dev/architecture/)):

| Layer | What | Where |
|---|---|---|
| Data | Providers talk to a database or an API; repositories turn that into domain models | `src/data/` |
| Business logic | Blocs and cubits turn events into immutable states | `src/features/<feature>/bloc/` |
| Presentation | Views and routes show states and add events | `src/features/<feature>/view/`, `src/routes/` |

This module (feature `bloc`) has the pieces: [`Bloc`], [`Cubit`], [`Emitter`],
[`BlocProvider`], [`RepositoryProvider`], [`BlocBuilder`], [`BlocListener`], [`BlocConsumer`],
[`BlocSelector`], and the [`test`] helpers.

## A bloc

```rust,no_run
use std::sync::Arc;

use rok_ui::bloc::{Bloc, Emitter};

/// A note.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub title: String,
}

/// Where notes come from: a trait, so tests can pass a fake.
pub trait NotesRepository: Send + Sync {
    fn list(&self) -> Vec<Note>;
}

/// What happened (past tense).
pub enum NotesEvent {
    NotesRequested,
}

/// What the view shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum NotesState {
    #[default]
    Initial,
    Loaded(Vec<Note>),
}

pub struct NotesBloc {
    repository: Arc<dyn NotesRepository>,
}

impl Bloc for NotesBloc {
    type Event = NotesEvent;
    type State = NotesState;

    fn initial_state(&self) -> NotesState {
        NotesState::Initial
    }

    async fn on(&self, event: NotesEvent, emit: &Emitter<NotesState>) {
        match event {
            NotesEvent::NotesRequested => {
                emit.emit(NotesState::Loaded(self.repository.list()));
            }
        }
    }
}
```

## Providing and reading it

```rust,no_run
# use std::sync::Arc;
# use rok_ui::{prelude::*, bloc::{Bloc, BlocBuilder, BlocProvider, Emitter, RepositoryProvider}};
# #[derive(Clone, PartialEq)] pub struct Note { pub title: String }
# pub trait NotesRepository: Send + Sync { fn list(&self) -> Vec<Note>; }
# struct Fixed; impl NotesRepository for Fixed { fn list(&self) -> Vec<Note> { Vec::new() } }
# pub enum NotesEvent { NotesRequested }
# #[derive(Clone, PartialEq)] pub enum NotesState { Initial, Loaded(Vec<Note>) }
# pub struct NotesBloc { repository: Arc<dyn NotesRepository> }
# impl Bloc for NotesBloc {
#     type Event = NotesEvent; type State = NotesState;
#     fn initial_state(&self) -> NotesState { NotesState::Initial }
#     async fn on(&self, _: NotesEvent, _: &Emitter<NotesState>) {}
# }
#[component]
fn NotesPage(cx: &mut Cx) -> impl IntoElement {
    let notes = cx.bloc::<NotesBloc>();
    let refresh = notes.clone();
    div()
        .child(Button::new("refresh").label("Refresh").on_click(move |_, _, _| {
            refresh.add(NotesEvent::NotesRequested);
        }))
        .child(BlocBuilder::new(&notes, |state, _, _| match state {
            NotesState::Initial => div().child("Nothing yet"),
            NotesState::Loaded(notes) => div().child(format!("{} notes", notes.len())),
        }))
}

fn app() -> impl IntoElement {
    RepositoryProvider::new()
        .provide::<dyn NotesRepository>(Arc::new(Fixed))
        .child(
            BlocProvider::bloc(|scope| NotesBloc { repository: scope.repository::<dyn NotesRepository>() })
                .child(NotesPage::new()),
        )
}
```

The full standard (rules, folder structure, naming, testing, the generator) is in
`docs/guide/architecture.md` in the repository.
