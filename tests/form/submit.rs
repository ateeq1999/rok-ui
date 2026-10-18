//! Submitting: focusing and scrolling to the first invalid field, and server errors.

use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rok_ui::{
    form::{
        Form, FormError, FormOptions, FormValidators, FormValues, TextField, ValidationEvent,
        Validators,
    },
    prelude::*,
};

use super::{field, form, open, SignUp};

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

#[derive(FormValues, Clone, Debug, Default, PartialEq)]
struct Long {
    email: String,
}

/// A short scroll area with the only required field far below the fold.
struct LongView(Rc<RefCell<Option<Form<Long>>>>, gpui::ScrollHandle);

impl Render for LongView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cx = Cx::new(window, cx);
        let form = form::use_form(
            &mut cx,
            FormOptions::new(Long::default())
                .scroll_handle(self.1.clone())
                .field(
                    Long::EMAIL,
                    Validators::new()
                        .on_submit(|email: &String| email.is_empty().then_some("Required")),
                ),
        );
        let email = form.field(&mut cx, Long::EMAIL);
        *self.0.borrow_mut() = Some(form);
        rok_ui::components::ScrollArea::new("long")
            .track_scroll(&self.1)
            .h(px(200.))
            .w(px(300.))
            .child(div().h(px(1500.)))
            .child(TextField::new(&email, "Email"))
    }
}

#[gpui::test]
fn a_failed_submit_scrolls_to_the_first_invalid_field(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let slot = Rc::new(RefCell::new(None));
    let handle = gpui::ScrollHandle::new();
    let (view_slot, view_handle) = (slot.clone(), handle.clone());
    let (_, window) =
        cx.add_window_view(move |_, _| LongView(view_slot.clone(), view_handle.clone()));
    window.run_until_parked();
    assert_eq!(handle.offset().y, px(0.));

    window.update(|window, cx| {
        slot.borrow().clone().expect("rendered").submit(window, cx);
    });
    window.run_until_parked();
    let offset = -handle.offset().y;
    assert!(
        offset > px(1300.) && offset <= px(1500.),
        "scrolled to the field: {offset:?}"
    );
}
