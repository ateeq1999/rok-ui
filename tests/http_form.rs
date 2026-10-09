//! API errors on forms: `use_api_form` and `Form::apply_server_errors`.

use std::{cell::RefCell, rc::Rc};

use gpui::{TestAppContext, VisualTestContext};
use rok_ui::{
    form::{
        self, ApiForm, ApiFormOptions, FormOptions, FormValues, ServerErrorOptions, TextField,
        ValidationEvent,
    },
    http::{ApiError, FieldDetails, FieldIssue},
    prelude::*,
};

#[derive(FormValues, Clone, Debug, Default, PartialEq)]
struct SignUp {
    email: String,
    password: String,
}

type Slot = Rc<RefCell<Option<ApiForm<SignUp>>>>;
type Answers = Rc<RefCell<Vec<Result<u32, ApiError>>>>;

struct SignUpView {
    slot: Slot,
    answers: Answers,
    successes: Rc<RefCell<Vec<u32>>>,
}

impl Render for SignUpView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cx = Cx::new(window, cx);
        let answers = self.answers.clone();
        let successes = self.successes.clone();
        let form = form::use_api_form(
            &mut cx,
            ApiFormOptions::new(
                FormOptions::new(SignUp::default()),
                move |_values: SignUp| {
                    let answer = answers.borrow_mut().remove(0);
                    async move { answer }
                },
            )
            .server_errors(
                ServerErrorOptions::new()
                    .map("emailAddress", &SignUp::EMAIL)
                    .unauthorized_field(&SignUp::PASSWORD),
            )
            .on_success(move |id, _| successes.borrow_mut().push(id)),
        );
        let email = form.field(&mut cx, SignUp::EMAIL);
        *self.slot.borrow_mut() = Some(form.clone());
        div().child(TextField::new(&email, "Email"))
    }
}

fn server_error(
    window: &mut VisualTestContext,
    slot: &Slot,
    path: form::Field<SignUp, String>,
) -> Option<String> {
    window.update(|_, cx| {
        let form = slot.borrow().clone().unwrap();
        form.field(cx, path)
            .meta()
            .form_error_map
            .get(&ValidationEvent::Server)
            .map(ToString::to_string)
    })
}

fn submit(window: &mut VisualTestContext, slot: &Slot) {
    window.update(|window, cx| slot.borrow().clone().unwrap().submit(window, cx));
    window.run_until_parked();
}

#[gpui::test]
fn api_forms_sort_errors_and_call_on_success(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let slot: Slot = Rc::default();
    let answers: Answers = Rc::default();
    let successes = Rc::new(RefCell::new(Vec::new()));
    let details: FieldDetails = [(
        "emailAddress".to_string(),
        vec![FieldIssue {
            message: "Already registered".into(),
            ..FieldIssue::default()
        }],
    )]
    .into_iter()
    .collect();
    answers.borrow_mut().extend([
        Err(ApiError::new(422, "invalid", "Check the form").with_details(details)),
        Err(ApiError::new(401, "unauthorized", "Wrong password")),
        Err(ApiError::new(500, "boom", "The server failed")),
        Err(ApiError::cancelled()),
        Ok(7),
    ]);
    let (view_slot, view_answers, view_successes) =
        (slot.clone(), answers.clone(), successes.clone());
    let (_, window) = cx.add_window_view(move |_, _| SignUpView {
        slot: view_slot,
        answers: view_answers,
        successes: view_successes,
    });
    window.run_until_parked();

    submit(window, &slot);
    assert_eq!(
        server_error(window, &slot, SignUp::EMAIL).as_deref(),
        Some("Already registered")
    );
    assert_eq!(slot.borrow().as_ref().unwrap().form_error(), None);

    // The user fixes the field, then submits again.
    window.update(|_, cx| {
        slot.borrow()
            .clone()
            .unwrap()
            .set_value(cx, SignUp::EMAIL, "ada@example.com".to_string());
    });
    assert_eq!(
        server_error(window, &slot, SignUp::EMAIL),
        None,
        "editing clears the field's error"
    );
    submit(window, &slot);
    assert_eq!(
        server_error(window, &slot, SignUp::PASSWORD).as_deref(),
        Some("Wrong password")
    );

    window.update(|_, cx| {
        slot.borrow()
            .clone()
            .unwrap()
            .set_value(cx, SignUp::PASSWORD, "hunter2".to_string());
    });
    submit(window, &slot);
    {
        let form = slot.borrow().clone().unwrap();
        assert_eq!(
            form.form_error().map(ToString::to_string).as_deref(),
            Some("The server failed")
        );
        assert_eq!(form.form_error_status(), Some(500));
    }
    window.update(|_, cx| slot.borrow().clone().unwrap().clear_form_error(cx));
    window.run_until_parked();
    assert_eq!(slot.borrow().as_ref().unwrap().form_error(), None);

    submit(window, &slot);
    assert_eq!(
        slot.borrow().as_ref().unwrap().form_error(),
        None,
        "cancelled: nothing shown"
    );
    assert!(
        successes.borrow().is_empty(),
        "on_success runs only after a success"
    );

    submit(window, &slot);
    assert_eq!(*successes.borrow(), [7]);
    assert!(slot.borrow().as_ref().unwrap().state().is_submitted);
}

#[gpui::test]
fn server_errors_apply_to_any_form(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let slot: Slot = Rc::default();
    let view_slot = slot.clone();
    let (_, window) = cx.add_window_view(move |_, _| SignUpView {
        slot: view_slot,
        answers: Rc::new(RefCell::new(vec![Ok(1)])),
        successes: Rc::default(),
    });
    window.run_until_parked();
    let error = ApiError::new(409, "taken", "Already registered");
    let errors = form::to_server_errors(
        &error,
        &ServerErrorOptions::new().conflict_field(&SignUp::EMAIL),
    );
    window.update(|_, cx| {
        slot.borrow()
            .clone()
            .unwrap()
            .apply_server_errors(cx, &errors);
    });
    window.run_until_parked();
    assert_eq!(
        server_error(window, &slot, SignUp::EMAIL).as_deref(),
        Some("Already registered")
    );

    // A message for the whole form does not block the retry.
    let error = ApiError::new(503, "down", "Try again later");
    let errors = form::to_server_errors(&error, &ServerErrorOptions::new());
    window.update(|_, cx| {
        let form = slot.borrow().clone().unwrap();
        form.set_value(cx, SignUp::EMAIL, "new@example.com".to_string());
        form.apply_server_errors(cx, &errors);
    });
    window.run_until_parked();
    assert_eq!(
        slot.borrow().as_ref().unwrap().state().errors,
        ["Try again later"]
    );
    submit(window, &slot);
    assert_eq!(
        slot.borrow().as_ref().unwrap().state().errors,
        Vec::<SharedString>::new()
    );
}
