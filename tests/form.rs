//! Forms: validation events, async validation, list fields, submission and focus.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use gpui::{TestAppContext, VisualTestContext};
use rok_ui::{
    form::{
        self, FieldApi, Form, FormError, FormOptions, FormValidators, FormValues, TextField,
        ValidationEvent, Validators,
    },
    prelude::*,
};

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

#[gpui::test]
fn blur_and_submit_errors_are_kept_apart(cx: &mut TestAppContext) {
    let (slot, window) = open(cx, || {
        FormOptions::new(SignUp::default()).field(
            SignUp::EMAIL,
            Validators::new()
                .on_blur(|email: &String| (!email.contains('@')).then_some("Enter a valid email"))
                .on_submit(|email: &String| {
                    email
                        .ends_with("example.test")
                        .then_some("Test addresses are not allowed")
                }),
        )
    });
    let email = field(window, &slot, SignUp::EMAIL);
    window.update(|_, cx| {
        email.handle_change(cx, "ada.example.test".into());
        email.handle_blur(cx);
    });
    window.update(|window, cx| form(&slot).submit(window, cx));
    window.run_until_parked();
    let email = field(window, &slot, SignUp::EMAIL);
    let meta = email.meta();
    assert_eq!(
        meta.error_map
            .get(&ValidationEvent::Blur)
            .map(AsRef::as_ref),
        Some("Enter a valid email")
    );
    assert_eq!(
        meta.error_map
            .get(&ValidationEvent::Submit)
            .map(AsRef::as_ref),
        Some("Test addresses are not allowed")
    );
    assert!(!form(&slot).state().can_submit);

    // Fixing the value clears the submit error but keeps the blur error until the next blur.
    window.update(|_, cx| email.handle_change(cx, "ada@example.com".into()));
    let email = field(window, &slot, SignUp::EMAIL);
    assert!(!email
        .meta()
        .error_map
        .contains_key(&ValidationEvent::Submit));
    assert!(email.meta().error_map.contains_key(&ValidationEvent::Blur));
    window.update(|_, cx| email.handle_blur(cx));
    assert!(field(window, &slot, SignUp::EMAIL).meta().is_valid());
}

#[gpui::test]
fn a_debounced_async_validator_runs_once_per_pause(cx: &mut TestAppContext) {
    let runs = Rc::new(Cell::new(0));
    let counted = runs.clone();
    let (slot, window) = open(cx, move || {
        let counted = counted.clone();
        FormOptions::new(SignUp::default()).field(
            SignUp::NAME,
            Validators::new().on_change_async_debounce(
                Duration::from_millis(300),
                move |name: String| {
                    counted.set(counted.get() + 1);
                    async move { (name == "taken").then_some("That name is taken") }
                },
            ),
        )
    });
    let name = field(window, &slot, SignUp::NAME);
    for text in ["t", "ta", "tak", "taken"] {
        window.update(|_, cx| name.handle_change(cx, text.into()));
        window.executor().advance_clock(Duration::from_millis(100));
        window.run_until_parked();
    }
    assert_eq!(runs.get(), 0, "still typing");
    assert!(field(window, &slot, SignUp::NAME).meta().is_validating);
    window.executor().advance_clock(Duration::from_millis(300));
    window.run_until_parked();
    assert_eq!(runs.get(), 1);
    let name = field(window, &slot, SignUp::NAME);
    assert!(!name.meta().is_validating);
    assert_eq!(name.errors(), ["That name is taken"]);
}

#[gpui::test]
fn list_operations_keep_each_rows_state(cx: &mut TestAppContext) {
    let (slot, window) = open(cx, || FormOptions::new(SignUp::default()));
    window.update(|_, cx| {
        let team = form(&slot).array_field(cx, SignUp::TEAM);
        for email in ["a@x", "b@x", "c@x"] {
            team.push_value(
                cx,
                Invite {
                    email: email.into(),
                },
            );
        }
    });
    window.run_until_parked();
    // Touch and break the second row, then remove the first.
    let second = SignUp::TEAM.at(1).then(Invite::EMAIL);
    window.update(|_, cx| {
        let field = form(&slot).field_with(
            cx,
            second.clone(),
            Validators::new()
                .on_change(|email: &String| (!email.contains('.')).then_some("Needs a domain")),
        );
        field.handle_change(cx, "b@y".into());
    });
    window.run_until_parked();
    assert_eq!(
        field(window, &slot, second.clone()).errors(),
        ["Needs a domain"]
    );

    window.update(|_, cx| {
        form(&slot)
            .array_field(cx, SignUp::TEAM)
            .remove_value(cx, 0);
    });
    window.run_until_parked();
    let moved = field(window, &slot, SignUp::TEAM.at(0).then(Invite::EMAIL));
    assert_eq!(moved.value().map(String::as_str), Some("b@y"));
    assert!(moved.meta().is_touched, "the row's state moved with it");
    assert_eq!(moved.errors(), ["Needs a domain"]);
    let last = field(window, &slot, SignUp::TEAM.at(1).then(Invite::EMAIL));
    assert!(!last.meta().is_touched);

    window.update(|_, cx| {
        form(&slot)
            .array_field(cx, SignUp::TEAM)
            .swap_values(cx, 0, 1);
    });
    let swapped = field(window, &slot, SignUp::TEAM.at(1).then(Invite::EMAIL));
    assert_eq!(swapped.value().map(String::as_str), Some("b@y"));
    assert!(swapped.meta().is_touched);
}

#[gpui::test]
fn a_failed_submit_focuses_the_first_invalid_field(cx: &mut TestAppContext) {
    let submitted = Rc::new(RefCell::new(Vec::new()));
    let log = submitted.clone();
    let (slot, window) = open(cx, move || {
        let log = log.clone();
        FormOptions::new(SignUp::default())
            .field(
                SignUp::NAME,
                Validators::new().on_change(|name: &String| name.is_empty().then_some("Required")),
            )
            .field(
                SignUp::EMAIL,
                Validators::new()
                    .on_submit(|email: &String| email.is_empty().then_some("Required")),
            )
            .validators(FormValidators::new().on_submit(|values: &SignUp| {
                (values.password != values.confirm)
                    .then(|| FormError::field(&SignUp::CONFIRM, "Passwords do not match"))
            }))
            .on_submit(move |values, _| {
                log.borrow_mut().push(values.email.clone());
                if values.email == "taken@example.com" {
                    gpui::Task::ready(Err(FormError::field(&SignUp::EMAIL, "Already registered")))
                } else {
                    gpui::Task::ready(Ok(()))
                }
            })
    });
    assert!(window.update(|window, cx| window.focused(cx).is_none()));
    window.update(|window, cx| form(&slot).submit(window, cx));
    window.run_until_parked();
    assert!(submitted.borrow().is_empty());
    let focused = window.update(|window, cx| {
        let form = form(&slot);
        let name = form.field(cx, SignUp::NAME);
        let email = form.field(cx, SignUp::EMAIL);
        assert_eq!(name.errors(), ["Required"]);
        assert_eq!(email.errors(), ["Required"]);
        window.focused(cx).is_some()
    });
    assert!(focused, "the name input has focus");

    // A server error lands on its field.
    window.update(|_, cx| {
        let form = form(&slot);
        form.set_value(cx, SignUp::NAME, "Ada".to_string());
        form.set_value(cx, SignUp::EMAIL, "taken@example.com".to_string());
    });
    window.update(|window, cx| form(&slot).submit(window, cx));
    window.run_until_parked();
    assert_eq!(*submitted.borrow(), ["taken@example.com"]);
    let email = field(window, &slot, SignUp::EMAIL);
    assert_eq!(
        email
            .meta()
            .form_error_map
            .get(&ValidationEvent::Server)
            .map(AsRef::as_ref),
        Some("Already registered")
    );
    assert!(!form(&slot).state().is_submitted);

    window.update(|_, cx| form(&slot).set_value(cx, SignUp::EMAIL, "ada@example.com".to_string()));
    window.update(|window, cx| form(&slot).submit(window, cx));
    window.run_until_parked();
    window.run_until_parked();
    assert!(form(&slot).state().is_submitted);
    assert!(form(&slot).is_dirty());

    window.update(|_, cx| form(&slot).reset(cx));
    window.run_until_parked();
    assert_eq!(form(&slot).values(), &SignUp::default());
    assert!(!form(&slot).is_dirty());
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
