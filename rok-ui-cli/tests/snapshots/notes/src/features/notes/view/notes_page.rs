//! The `notes` page.

use crate::features::notes::bloc::notes_bloc::NotesBloc;
use crate::features::notes::bloc::notes_event::NotesEvent;
use crate::features::notes::bloc::notes_state::NotesStatus;
use rok_ui::bloc::BlocBuilder;
use rok_ui::form::{self, FormOptions, FormValues, SubmitButton};
use rok_ui::prelude::*;

/// What the `NoteAdded` form edits.
#[derive(FormValues, Clone, Default)]
struct NoteAddedForm {
    /// `title`.
    title: String,
    /// `body`.
    body: String,
}

/// Shows `notes` and adds its events.
#[component]
pub fn NotesPage(cx: &mut Cx) -> impl IntoElement {
    let bloc = cx.bloc::<NotesBloc>();
    // Once, when the page opens: a state initializer runs on the first render only.
    let first = bloc.clone();
    cx.use_state(move || first.add(NotesEvent::NotesRequested));
    let submitting = bloc.clone();
    let form = form::use_form(
        cx,
        FormOptions::new(NoteAddedForm::default()).on_submit(move |values: NoteAddedForm, _| {
            submitting.add(NotesEvent::NoteAdded {
                title: values.title,
                body: values.body,
            });
            gpui::Task::ready(Ok(()))
        }),
    );
    let editor = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(form::TextField::new(
            &form.field(cx, NoteAddedForm::TITLE),
            "Title",
        ))
        .child(form::TextField::new(
            &form.field(cx, NoteAddedForm::BODY),
            "Body",
        ))
        .child(SubmitButton::new(&form, "Add"));
    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_6()
        .child(H2::new("Notes"))
        .child(div().flex().gap_2().child({
            let handler = bloc.clone();
            Button::new("notes-requested")
                .outline()
                .label("Refresh")
                .on_click(move |_, _, _| {
                    handler.add(NotesEvent::NotesRequested);
                })
        }))
        .child(editor)
        .child(BlocBuilder::new(&bloc, {
            let bloc = bloc.clone();
            move |state, _, _| match state.status {
                NotesStatus::Loading => div().child("Loading..."),
                NotesStatus::Failure => div().child(
                    state
                        .error
                        .as_ref()
                        .map_or_else(|| "Something went wrong.".to_string(), ToString::to_string),
                ),
                _ => div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(state.notes.iter().enumerate().map(|(index, item)| {
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(item.title.clone())
                            .child({
                                let handler = bloc.clone();
                                let id = item.id;
                                Button::new(("note-deleted", index))
                                    .ghost()
                                    .label("Delete")
                                    .on_click(move |_, _, _| {
                                        handler.add(NotesEvent::NoteDeleted { id });
                                    })
                            })
                    })),
            }
        }))
}
