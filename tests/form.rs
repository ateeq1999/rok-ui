//! Forms: validation events, async validation, list fields, submission and focus.
//!
//! A test crate's root looks for modules beside it, so each one names its file.

use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{TestAppContext, VisualTestContext};
use rok_ui::{
    form::{self, FieldApi, Form, FormOptions, FormValues, TextField},
    prelude::*,
};

#[path = "form/controls.rs"]
mod controls;
#[path = "form/submit.rs"]
mod submit;
#[path = "form/validation.rs"]
mod validation;

#[derive(FormValues, Clone, Debug, Default, PartialEq)]
struct Invite {
    email: String,
}

#[derive(FormValues, Clone, Debug, Default, PartialEq)]
struct SignUp {
    name: String,
    email: String,
    password: String,
    confirm: String,
    team: Vec<Invite>,
}

type Slot = Rc<RefCell<Option<Form<SignUp>>>>;

/// Renders a form with text fields and remembers it for the test.
struct SignUpView {
    slot: Slot,
    options: Rc<dyn Fn() -> FormOptions<SignUp>>,
}

impl Render for SignUpView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cx = Cx::new(window, cx);
        let form = form::use_form(&mut cx, (self.options)());
        let name = form.field(&mut cx, SignUp::NAME);
        let email = form.field(&mut cx, SignUp::EMAIL);
        *self.slot.borrow_mut() = Some(form);
        div()
            .child(TextField::new(&name, "Name"))
            .child(TextField::new(&email, "Email"))
    }
}

fn open(
    cx: &mut TestAppContext,
    options: impl Fn() -> FormOptions<SignUp> + 'static,
) -> (Slot, &mut VisualTestContext) {
    cx.update(rok_ui::init);
    let slot: Slot = Rc::new(RefCell::new(None));
    let view_slot = slot.clone();
    let options: Rc<dyn Fn() -> FormOptions<SignUp>> = Rc::new(options);
    let (_, window) = cx.add_window_view(move |_, _| SignUpView {
        slot: view_slot,
        options: options.clone(),
    });
    window.run_until_parked();
    (slot, window)
}

fn form(slot: &Slot) -> Form<SignUp> {
    slot.borrow().clone().expect("rendered")
}

fn field<T: Clone + 'static>(
    window: &mut VisualTestContext,
    slot: &Slot,
    path: impl Into<form::Path<SignUp, T>>,
) -> FieldApi<SignUp, T> {
    let path = path.into();
    window.update(|_, cx| form(slot).field(cx, path))
}

#[cfg(feature = "form-garde")]
mod garde_schema {
    use rok_ui::form::{GardeSchema, Schema};

    use super::*;

    #[derive(FormValues, garde::Validate, Clone, Debug, Default)]
    struct Team {
        #[garde(length(min = 1))]
        name: String,
        #[garde(dive)]
        members: Vec<Member>,
    }

    #[derive(FormValues, garde::Validate, Clone, Debug, Default)]
    struct Member {
        #[garde(length(min = 3))]
        email: String,
    }

    #[test]
    fn garde_errors_land_on_the_right_fields() {
        let team = Team {
            name: String::new(),
            members: vec![
                Member {
                    email: "ok@x".into(),
                },
                Member { email: "x".into() },
            ],
        };
        let errors = GardeSchema.validate(&team).expect("invalid");
        let mut fields: Vec<&str> = errors
            .field_errors()
            .iter()
            .map(|(key, _)| key.as_ref())
            .collect();
        fields.sort_unstable();
        assert_eq!(fields, ["members.1.email", "name"]);
        // The key matches the typed path, so the error shows on that field.
        assert_eq!(
            Team::MEMBERS.at(1).then(Member::EMAIL).key().as_ref(),
            "members.1.email"
        );
    }
}

#[cfg(feature = "persist")]
mod drafts {
    use super::*;
    use rok_ui::persist::PersistOptions;

    #[derive(
        FormValues, Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize,
    )]
    struct Note {
        title: String,
    }

    type NoteSlot = Rc<RefCell<Option<Form<Note>>>>;

    struct NoteView(NoteSlot, PersistOptions);

    impl Render for NoteView {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let mut cx = Cx::new(window, cx);
            let form = form::use_form(
                &mut cx,
                FormOptions::new(Note::default())
                    .persist_draft(self.1.clone())
                    .on_submit(|_, _| gpui::Task::ready(Ok(()))),
            );
            let title = form.field(&mut cx, Note::TITLE);
            *self.0.borrow_mut() = Some(form);
            div().child(TextField::new(&title, "Title"))
        }
    }

    fn open(cx: &mut TestAppContext, options: &PersistOptions) -> NoteSlot {
        let slot: NoteSlot = Rc::new(RefCell::new(None));
        let (view_slot, options) = (slot.clone(), options.clone());
        let (_, window) =
            cx.add_window_view(move |_, _| NoteView(view_slot.clone(), options.clone()));
        window.run_until_parked();
        slot
    }

    fn form(slot: &NoteSlot) -> Form<Note> {
        slot.borrow().clone().expect("rendered")
    }

    #[gpui::test]
    fn drafts_survive_closing_the_form_until_it_is_submitted(cx: &mut TestAppContext) {
        cx.update(rok_ui::init);
        let directory = std::env::temp_dir().join(format!("rok-ui-drafts-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        let file = directory.join("note-draft.json");
        let options = PersistOptions::new("note-draft")
            .directory(&directory)
            .debounce(Duration::from_millis(50));
        let wait = |cx: &mut TestAppContext| {
            cx.executor().advance_clock(Duration::from_millis(60));
            cx.run_until_parked();
        };

        let first = open(cx, &options);
        wait(cx);
        assert!(!file.exists(), "nothing is saved before an edit");
        cx.update(|cx| form(&first).set_value(cx, Note::TITLE, "Groceries".to_string()));
        wait(cx);
        assert!(file.exists(), "edits are saved");

        // The same form opened again starts from the draft.
        let second = open(cx, &options);
        assert_eq!(form(&second).values().title, "Groceries");

        // A successful submit deletes it, and nothing writes it back.
        cx.update(|cx| {
            let window = cx.windows()[1];
            window
                .update(cx, |_, window, cx| form(&second).submit(window, cx))
                .unwrap();
        });
        wait(cx);
        assert!(!file.exists(), "submitting deletes the draft");
        let third = open(cx, &options);
        assert_eq!(form(&third).values().title, "");
        std::fs::remove_dir_all(directory).ok();
    }
}
