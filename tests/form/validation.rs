//! Validation events, debounced and async validators.

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::TestAppContext;
use rok_ui::form::{FormOptions, ValidationEvent, Validators};

use super::{field, form, open, SignUp};

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
fn a_form_with_an_async_validator_submits_once_it_passes(cx: &mut TestAppContext) {
    let submitted = Rc::new(Cell::new(0));
    let count = submitted.clone();
    let (slot, window) = open(cx, move || {
        let count = count.clone();
        FormOptions::new(SignUp::default())
            .field(
                SignUp::NAME,
                Validators::new().on_change_async_debounce(
                    Duration::from_millis(300),
                    |name: String| async move { (name == "taken").then_some("That name is taken") },
                ),
            )
            .on_submit(move |_, _| {
                count.set(count.get() + 1);
                gpui::Task::ready(Ok(()))
            })
    });
    let name = field(window, &slot, SignUp::NAME);
    window.update(|_, cx| name.handle_change(cx, "ada".into()));
    window.executor().advance_clock(Duration::from_millis(300));
    window.run_until_parked();
    assert!(field(window, &slot, SignUp::NAME).meta().is_valid());

    // Submitting runs the async validator again and submits when it passes.
    window.update(|window, cx| form(&slot).submit(window, cx));
    assert!(form(&slot).state().is_submitting);
    window.executor().advance_clock(Duration::from_millis(300));
    window.run_until_parked();
    assert_eq!(submitted.get(), 1);
    assert!(form(&slot).state().is_submitted);

    // When it fails, nothing is submitted.
    window.update(|_, cx| name.handle_change(cx, "taken".into()));
    window.update(|window, cx| form(&slot).submit(window, cx));
    window.executor().advance_clock(Duration::from_millis(300));
    window.run_until_parked();
    assert_eq!(submitted.get(), 1);
    assert!(!form(&slot).state().is_submitting);
    assert_eq!(
        field(window, &slot, SignUp::NAME).errors(),
        ["That name is taken"]
    );
}

#[gpui::test]
fn a_value_that_fails_its_sync_check_stops_validating(cx: &mut TestAppContext) {
    let (slot, window) = open(cx, || {
        FormOptions::new(SignUp::default()).field(
            SignUp::NAME,
            Validators::new()
                .on_change(|name: &String| name.is_empty().then_some("Required"))
                .on_change_async_debounce(Duration::from_millis(300), |_: String| async {
                    None::<&str>
                }),
        )
    });
    let name = field(window, &slot, SignUp::NAME);
    window.update(|_, cx| name.handle_change(cx, "a".into()));
    assert!(field(window, &slot, SignUp::NAME).meta().is_validating);
    // Cleared before the async check ran: that run is dropped.
    window.update(|_, cx| name.handle_change(cx, String::new()));
    window.executor().advance_clock(Duration::from_millis(300));
    window.run_until_parked();
    let name = field(window, &slot, SignUp::NAME);
    assert_eq!(name.errors(), ["Required"]);
    assert!(!name.meta().is_validating);
    assert!(!form(&slot).state().is_validating);

    // Resetting a field drops its running check, so its result cannot land on the fresh field.
    window.update(|_, cx| name.handle_change(cx, "a".into()));
    window.update(|_, cx| form(&slot).reset_field(cx, SignUp::NAME));
    assert!(!form(&slot).state().is_validating);
}
