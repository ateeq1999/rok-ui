//! The devtools overlay lists live forms with their status and errors.

use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rok_ui::{
    devtools::{self, Devtools},
    form::{self, Form, FormOptions, FormValues, TextField, Validators},
    prelude::*,
};

#[derive(FormValues, Clone, Default)]
struct SignIn {
    email: String,
}

type Slot = Rc<RefCell<Option<Form<SignIn>>>>;

struct Page {
    slot: Slot,
    show_form: Rc<RefCell<bool>>,
}

impl Render for Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show = *self.show_form.borrow();
        let mut cx = Cx::new(window, cx);
        let root = AppRoot::new().child(Devtools::new());
        if !show {
            return root;
        }
        let form = form::use_form(
            &mut cx,
            FormOptions::new(SignIn::default()).field(
                SignIn::EMAIL,
                Validators::new()
                    .on_submit(|email: &String| email.is_empty().then_some("Required")),
            ),
        );
        let email = form.field(&mut cx, SignIn::EMAIL);
        *self.slot.borrow_mut() = Some(form);
        root.child(TextField::new(&email, "Email"))
    }
}

#[gpui::test]
fn live_forms_show_their_status_and_errors(cx: &mut TestAppContext) {
    cx.update(|cx| {
        rok_ui::init(cx);
        devtools::toggle(cx);
    });
    let slot: Slot = Rc::default();
    let show = Rc::new(RefCell::new(true));
    let (view_slot, view_show) = (slot.clone(), show.clone());
    let (view, window) = cx.add_window_view(move |_, _| Page {
        slot: view_slot,
        show_form: view_show,
    });
    window.run_until_parked();
    let forms = window.update(|_, cx| form::live_forms(cx));
    assert_eq!(forms.len(), 1);
    assert_eq!(forms[0].name, "SignIn");
    assert!(forms[0].is_valid && !forms[0].is_dirty);

    window.update(|window, cx| slot.borrow().clone().unwrap().submit(window, cx));
    window.run_until_parked();
    let forms = window.update(|_, cx| form::live_forms(cx));
    assert_eq!(forms[0].attempts, 1);
    assert!(!forms[0].is_valid);
    assert_eq!(forms[0].field_errors, [("email".into(), "Required".into())]);

    // Once the form is gone, so is its entry (and the form itself: its inputs hold it weakly).
    *show.borrow_mut() = false;
    slot.borrow_mut().take();
    // Element state is dropped one frame after it is no longer rendered.
    for _ in 0..2 {
        window.update(|_, cx| view.update(cx, |_, cx| cx.notify()));
        window.run_until_parked();
    }
    let forms = window.update(|_, cx| form::live_forms(cx));
    assert_eq!(forms.len(), 0);
}
