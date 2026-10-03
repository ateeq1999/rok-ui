//! A sign-up form with `rok_ui::form`: typed fields, validation on change, blur and submit,
//! an async check, a list of invites and a submit handler that reports server errors.
//!
//! Run with `cargo run --example sign_up`.

use std::time::Duration;

use rok_ui::{
    form::{
        self, CheckboxField, FormError, FormErrors, FormOptions, FormValidators, FormValues,
        SubmitButton, TextField, Validators,
    },
    prelude::*,
};

#[derive(FormValues, Clone, Debug, Default)]
struct Invite {
    email: String,
}

#[derive(FormValues, Clone, Debug, Default)]
struct SignUp {
    name: String,
    email: String,
    password: String,
    confirm: String,
    accept_terms: bool,
    team: Vec<Invite>,
}

fn required(value: &str) -> Option<&'static str> {
    value.trim().is_empty().then_some("Required")
}

fn sign_up_form() -> FormOptions<SignUp> {
    FormOptions::new(SignUp::default())
        .field(
            SignUp::NAME,
            Validators::new().on_change(|name: &String| required(name)),
        )
        .field(
            SignUp::EMAIL,
            Validators::new()
                .on_change(|email: &String| required(email))
                .on_blur(|email: &String| (!email.contains('@')).then_some("Enter a valid email"))
                .on_change_async_debounce(Duration::from_millis(400), |email: String| async move {
                    (email == "ada@example.com").then_some("This email is already registered")
                }),
        )
        .field(
            SignUp::PASSWORD,
            Validators::new().on_blur(|password: &String| {
                (password.chars().count() < 8).then_some("Use at least 8 characters")
            }),
        )
        .field(
            SignUp::CONFIRM,
            Validators::new().listen_to(&SignUp::PASSWORD),
        )
        .validators(
            FormValidators::new()
                .on_change(|values: &SignUp| {
                    (!values.confirm.is_empty() && values.password != values.confirm)
                        .then(|| FormError::field(&SignUp::CONFIRM, "Passwords do not match"))
                })
                .on_submit(|values: &SignUp| {
                    (!values.accept_terms).then(|| {
                        FormError::field(&SignUp::ACCEPT_TERMS, "Accept the terms to continue")
                    })
                }),
        )
        .on_submit(|values, cx| {
            // A real app would call a procedure here; this one pretends to be a server.
            let delay = cx.background_executor().timer(Duration::from_millis(600));
            cx.spawn(async move |cx| {
                delay.await;
                if values.name.eq_ignore_ascii_case("admin") {
                    return Err(FormError::field(&SignUp::NAME, "That name is reserved"));
                }
                cx.update(|cx| toast(cx, Toast::success(format!("Welcome, {}!", values.name))))
                    .ok();
                Ok(())
            })
        })
}

#[component]
fn SignUpCard(cx: &mut Cx) -> impl IntoElement {
    let form = form::use_form(cx, sign_up_form());
    let name = form.field(cx, SignUp::NAME);
    let email = form.field(cx, SignUp::EMAIL);
    let password = form.field(cx, SignUp::PASSWORD);
    let confirm = form.field(cx, SignUp::CONFIRM);
    let terms = form.field(cx, SignUp::ACCEPT_TERMS);
    let team = form.array_field(cx, SignUp::TEAM);
    let invites: Vec<AnyElement> = (0..team.len())
        .map(|index| {
            let invite = form.field_with(
                cx,
                team.at(index).then(Invite::EMAIL),
                Validators::new().on_blur(|email: &String| {
                    (!email.contains('@')).then_some("Enter a valid email")
                }),
            );
            div()
                .flex()
                .items_end()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .child(TextField::new(&invite, format!("Teammate {}", index + 1))),
                )
                .child(
                    Button::new(("remove-invite", index))
                        .ghost()
                        .icon(IconName::Trash)
                        .on_click(team.remove_handler(index)),
                )
                .into_any_element()
        })
        .collect();

    Card::new().w(px(440.)).child(
        CardContent::new().child(
            FieldGroup::new()
                .child(TextField::new(&name, "Name").placeholder("Ada Lovelace"))
                .child(TextField::new(&email, "Email").leading_icon(IconName::Mail))
                .child(TextField::new(&password, "Password").password())
                .child(TextField::new(&confirm, "Confirm password").password())
                .children(invites)
                .child(
                    Button::new("add-invite")
                        .outline()
                        .label("Invite a teammate")
                        .on_click(team.push_handler(Invite::default())),
                )
                .child(CheckboxField::new(&terms, "I accept the terms"))
                .child(FormErrors::new(&form))
                .child(SubmitButton::new(&form, "Create account")),
        ),
    )
}

struct SignUpWindow;

impl Render for SignUpWindow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .items_center()
            .justify_center()
            .child(SignUpCard::new())
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut App| {
            rok_ui::init(cx);
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| SignUpWindow))
                .expect("the window opens");
        });
}
