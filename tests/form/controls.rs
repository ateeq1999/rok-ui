//! List fields and the bound controls: pickers, text areas and one-time codes.

use std::{cell::RefCell, rc::Rc};

use gpui::{TestAppContext, VisualTestContext};
use rok_ui::{
    form::{Form, FormOptions, FormValues, Validators},
    prelude::*,
};

use super::{field, form, open, Invite, SignUp};

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

#[derive(FormValues, Clone, Debug, Default, PartialEq)]
struct Profile {
    plan: String,
    bio: String,
    country: String,
    volume: f32,
}

type ProfileSlot = Rc<RefCell<Option<Form<Profile>>>>;

/// A radio group and a text area bound to a form.
struct ProfileView {
    slot: ProfileSlot,
    submitted: Rc<RefCell<Vec<Profile>>>,
}

impl Render for ProfileView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cx = Cx::new(window, cx);
        let log = self.submitted.clone();
        let form = form::use_form(
            &mut cx,
            FormOptions::new(Profile::default())
                .field(
                    Profile::PLAN,
                    Validators::new()
                        .on_blur(|plan: &String| (plan == "team").then_some("Not yet")),
                )
                .on_submit(move |values, _| {
                    log.borrow_mut().push(values);
                    gpui::Task::ready(Ok(()))
                }),
        );
        let plan = form.field(&mut cx, Profile::PLAN);
        let bio = form.field(&mut cx, Profile::BIO);
        let country = form.field(&mut cx, Profile::COUNTRY);
        let volume = form.field(&mut cx, Profile::VOLUME);
        *self.slot.borrow_mut() = Some(form);
        // AppRoot moves focus with Tab.
        rok_ui::components::AppRoot::new()
            .child(
                form::RadioGroupField::new(&plan, "Plan")
                    .option("free", "Free")
                    .option("pro", "Pro")
                    .option("team", "Team"),
            )
            .child(form::TextareaField::new(&bio, "Bio"))
            .child(
                form::SelectField::new(&country, "Country")
                    .placeholder("Choose one")
                    .option("eg", "Egypt")
                    .option("sd", "Sudan"),
            )
            .child(
                form::SliderField::new(&volume, "Volume")
                    .range(0., 10.)
                    .step(1.),
            )
    }
}

#[gpui::test]
fn pickers_and_text_areas_edit_their_fields(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let slot: ProfileSlot = Rc::new(RefCell::new(None));
    let submitted = Rc::new(RefCell::new(Vec::new()));
    let (view_slot, view_submitted) = (slot.clone(), submitted.clone());
    let (_, window) = cx.add_window_view(move |_, _| ProfileView {
        slot: view_slot.clone(),
        submitted: view_submitted.clone(),
    });
    window.run_until_parked();
    let profile = |window: &mut VisualTestContext| {
        window.update(|_, _| slot.borrow().clone().expect("rendered").values().clone())
    };

    // Arrow keys pick the next option; a pick counts as leaving the field.
    window.simulate_keystrokes("tab down");
    window.run_until_parked();
    assert_eq!(profile(window).plan, "pro");
    window.simulate_keystrokes("tab down");
    window.run_until_parked();
    let plan = window.update(|_, cx| {
        slot.borrow()
            .clone()
            .expect("rendered")
            .field(cx, Profile::PLAN)
    });
    assert_eq!(plan.value().map(String::as_str), Some("team"));
    assert!(plan.meta().is_blurred, "a pick counts as leaving the field");
    assert_eq!(plan.errors(), ["Not yet"], "blur validators ran");

    // The text area takes several lines.
    window.simulate_keystrokes("tab tab");
    window.simulate_input("Hello");
    window.simulate_keystrokes("enter");
    window.simulate_input("world");
    window.run_until_parked();
    assert_eq!(profile(window).bio, "Hello\nworld");
    assert!(submitted.borrow().is_empty(), "Enter adds a line");

    // Picking a valid plan clears the error; Ctrl/Cmd-Enter in the text area submits.
    window.simulate_keystrokes("shift-tab up");
    window.run_until_parked();
    assert_eq!(profile(window).plan, "pro");
    window.simulate_keystrokes("tab");
    // The text area reads the held modifiers, which the platform reports separately.
    window.simulate_modifiers_change(gpui::Modifiers::secondary_key());
    window.simulate_keystrokes("secondary-enter");
    window.run_until_parked();
    assert_eq!(
        *submitted.borrow(),
        [Profile {
            plan: "pro".into(),
            bio: "Hello\nworld".into(),
            ..Profile::default()
        }]
    );

    // The select and the slider render values set from code.
    window.update(|_, cx| {
        let form = slot.borrow().clone().expect("rendered");
        form.set_value(cx, Profile::COUNTRY, "sd".to_string());
        form.set_value(cx, Profile::VOLUME, 7.);
    });
    window.run_until_parked();
    let values = profile(window);
    assert_eq!((values.country.as_str(), values.volume), ("sd", 7.));
}

#[derive(FormValues, Clone, Debug, Default, PartialEq)]
struct Verify {
    code: String,
    language: String,
    due: Option<rok_ui::components::CalendarDate>,
}

type VerifySlot = Rc<RefCell<Option<Form<Verify>>>>;

/// A code input, a combobox and a date picker bound to a form.
struct VerifyView(VerifySlot);

impl Render for VerifyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cx = Cx::new(window, cx);
        let form = form::use_form(
            &mut cx,
            FormOptions::new(Verify::default()).field(
                Verify::CODE,
                Validators::new()
                    .on_blur(|code: &String| (code != "123456").then_some("Wrong code")),
            ),
        );
        let code = form.field(&mut cx, Verify::CODE);
        let language = form.field(&mut cx, Verify::LANGUAGE);
        let due = form.field(&mut cx, Verify::DUE);
        *self.0.borrow_mut() = Some(form);
        rok_ui::components::AppRoot::new()
            .child(form::InputOtpField::new(&code, "Code", 6))
            .child(
                form::ComboboxField::new(&language, "Language")
                    .option("rust", "Rust")
                    .option("zig", "Zig"),
            )
            .child(form::DatePickerField::new(&due, "Due"))
    }
}

#[gpui::test]
fn code_inputs_fill_their_field_and_check_it_when_complete(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let slot: VerifySlot = Rc::new(RefCell::new(None));
    let view_slot = slot.clone();
    let (_, window) = cx.add_window_view(move |_, _| VerifyView(view_slot.clone()));
    window.run_until_parked();
    let code = |window: &mut VisualTestContext| {
        window.update(|_, cx| {
            slot.borrow()
                .clone()
                .expect("rendered")
                .field(cx, Verify::CODE)
        })
    };

    // One character per frame, as a person types: the boxes read the code they rendered.
    let type_code = |window: &mut VisualTestContext, digits: &str| {
        for digit in digits.chars() {
            window.simulate_input(&digit.to_string());
            window.run_until_parked();
        }
    };
    window.simulate_keystrokes("tab");
    type_code(window, "123");
    let partial = code(window);
    assert_eq!(partial.value().map(String::as_str), Some("123"));
    assert!(
        !partial.meta().is_blurred,
        "typing does not leave the field"
    );

    type_code(window, "999");
    let full = code(window);
    assert_eq!(full.value().map(String::as_str), Some("123999"));
    assert!(
        full.meta().is_blurred,
        "filling the last box counts as leaving the field"
    );
    assert_eq!(
        full.errors(),
        ["Wrong code"],
        "a full code runs blur validators"
    );

    // The combobox and the date picker render values set from code.
    let date = rok_ui::components::CalendarDate::new(2026, 10, 3);
    window.update(|_, cx| {
        let form = slot.borrow().clone().expect("rendered");
        form.set_value(cx, Verify::LANGUAGE, "zig".to_string());
        form.set_value(cx, Verify::DUE, date);
    });
    window.run_until_parked();
    let values = window.update(|_, _| slot.borrow().clone().expect("rendered").values().clone());
    assert_eq!((values.language.as_str(), values.due), ("zig", date));
}
