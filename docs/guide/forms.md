# Forms

`rok_ui::form` (feature `form`, part of the `forms` group and of `full`) is a headless,
type-safe form library modeled on TanStack Form: the form owns typed values, fields read their
slice through typed paths, validators run on the events you choose, and bound controls render
the state.

| TanStack Form | rok-ui |
|---|---|
| `useForm({ defaultValues, validators, onSubmit })` | `form::use_form(cx, FormOptions::new(defaults).validators(..).on_submit(..))` |
| `<form.Field name="email" validators={..}>` | `form.field(cx, SignUp::EMAIL)` / `form.field_with(cx, path, validators)` |
| `field.state.value`, `field.state.meta.errors` | `field.value()`, `field.meta().errors()` |
| `field.handleChange`, `field.handleBlur` | `field.handle_change(cx, value)`, `field.handle_blur(cx)` |
| `onChange`, `onBlur`, `onSubmit`, `onChangeAsyncDebounceMs` | `on_change`, `on_blur`, `on_submit`, `on_change_async_debounce` |
| `onChangeListenTo` | `.listen_to(&SignUp::PASSWORD)` |
| `form.Field mode="array"` + `pushValue`, `removeValue`, .. | `form.array_field(cx, path)` + `push_value`, `remove_value`, .. |
| `form.state.canSubmit`, `isSubmitting` | `form.state().can_submit`, `.is_submitting` |
| Standard Schema adapters | `FormValidators::schema(..)`, `GardeSchema` (feature `form-garde`) |

## Defining a form

`#[derive(FormValues)]` adds a typed field constant per struct field: `email: String` becomes
`SignUp::EMAIL`, a `Field<SignUp, String>`. Options are a value built by a function:

```
use rok_ui::{
    form::{FormError, FormOptions, FormValidators, FormValues, Validators},
    gpui::Task,
};

#[derive(FormValues, Clone, Debug, Default)]
pub struct SignUp {
    pub email: String,
    pub password: String,
    pub confirm: String,
    pub accept_terms: bool,
}

pub fn sign_up_form() -> FormOptions<SignUp> {
    FormOptions::new(SignUp::default())
        .field(
            SignUp::EMAIL,
            Validators::new()
                .on_change(|email: &String| email.is_empty().then_some("Required"))
                .on_blur(|email: &String| (!email.contains('@')).then_some("Enter a valid email")),
        )
        .field(
            SignUp::CONFIRM,
            // Re-checked when the password changes too.
            Validators::new().listen_to(&SignUp::PASSWORD),
        )
        .validators(FormValidators::new().on_submit(|values: &SignUp| {
            (values.password != values.confirm)
                .then(|| FormError::field(&SignUp::CONFIRM, "Passwords do not match"))
        }))
        .on_submit(|values, _cx| {
            // Call a procedure or save the values; errors map back onto fields.
            let _ = values;
            Task::ready(Ok(()))
        })
}
# let _ = sign_up_form();
```

## Rendering fields

```no_run
# use rok_ui::{prelude::*, form::{self, CheckboxField, FormOptions, FormValues, SubmitButton, TextField}};
# #[derive(FormValues, Clone, Default)]
# pub struct SignUp { email: String, password: String, accept_terms: bool }
#[component]
fn SignUpForm(cx: &mut Cx) -> impl IntoElement {
    let form = form::use_form(cx, FormOptions::new(SignUp::default()));
    let email = form.field(cx, SignUp::EMAIL);
    let password = form.field(cx, SignUp::PASSWORD);
    let terms = form.field(cx, SignUp::ACCEPT_TERMS);
    FieldGroup::new()
        .child(TextField::new(&email, "Email").leading_icon(IconName::Mail))
        .child(TextField::new(&password, "Password").password())
        .child(CheckboxField::new(&terms, "I accept the terms"))
        .child(SubmitButton::new(&form, "Create account"))
}
```

- `TextField` (and the bare `BoundInput`) binds an `Input` to a `String` field: typing
  changes the value, leaving the field runs blur validators, and Enter submits the form.
- `CheckboxField` and `SwitchField` bind `bool` fields; any control with an `on_change` that
  passes the new value binds with `.on_change(field.change_handler())`.
- Errors show once the field is touched or a submit was attempted
  (`field.should_show_errors()`); custom layouts read `field.meta()` directly.
- `SubmitButton` spins while the form submits and is disabled while it cannot.
  `FormErrors::new(&form)` lists errors that are not tied to a field.

## Field and form state

| Field | Meaning |
|---|---|
| `value()` | The typed value at the field's path |
| `meta().errors()`, `meta().error_map`, `meta().form_error_map` | Errors, by the event that produced them |
| `meta().is_touched`, `is_blurred`, `is_dirty`, `is_pristine()` | Interaction state |
| `meta().is_validating` | An async validator is running |
| `handle_change(cx, value)`, `handle_blur(cx)`, `change_handler()` | What bound controls call |

| Form | Meaning |
|---|---|
| `values()` | Every value |
| `state().can_submit`, `is_submitting`, `is_submitted`, `submission_attempts` | Submit lifecycle |
| `state().is_valid`, `is_dirty`, `is_touched`, `is_validating`, `errors` | Aggregates |
| `submit(window, cx)`, `submit_handler()`, `reset(cx)`, `reset_field(cx, path)`, `set_value(cx, path, value)`, `validate(cx, event)` | Imperative API |

## Validation

- **Events.** `on_mount`, `on_change`, `on_blur` and `on_submit`. Errors are kept per event
  (`error_map`), so a blur error does not hide a submit error. Changing a field clears its
  submit and server errors; each event's errors last until that event runs again.
- **Async.** `on_change_async_debounce(duration, |value| async { .. })` runs once the value
  has been still for `duration`, only after the synchronous `on_change` passes, and only the
  latest run's result counts. `on_blur_async` and `on_submit_async` run without a debounce.
- **Form validators** (`FormValidators`) see all values and return a `FormError` with errors
  for any field (`FormError::field(&SignUp::CONFIRM, ..)`) or the whole form.
- **Linked fields.** `.listen_to(&OTHER)` re-runs a field's change validators when `OTHER`
  changes.
- **Schemas.** `FormValidators::schema(..)` validates with anything implementing `Schema`.
  With the `form-garde` feature, `GardeSchema` uses the values' `garde::Validate` derive, so
  one set of rules covers the form and the procedure that receives it.
- **Submit.** Submitting runs every validator. If anything is invalid, the first invalid bound
  field gets focus; otherwise the submit handler runs, and the `FormError` it returns shows on
  its fields as `ValidationEvent::Server` errors.

## Nested and list fields

Paths compose: `SignUp::TEAM.at(0).then(Invite::EMAIL)` is the first invite's email, keyed
`team.0.email`. List fields keep each row's state with its row when rows move:

```no_run
# use rok_ui::{prelude::*, form::{self, FormOptions, FormValues, TextField}};
# #[derive(FormValues, Clone, Default)] pub struct Invite { email: String }
# #[derive(FormValues, Clone, Default)] pub struct SignUp { team: Vec<Invite> }
#[component]
fn Team(cx: &mut Cx) -> impl IntoElement {
    let form = form::use_form(cx, FormOptions::new(SignUp::default()));
    let team = form.array_field(cx, SignUp::TEAM);
    let rows: Vec<AnyElement> = (0..team.len())
        .map(|index| {
            let email = form.field(cx, team.at(index).then(Invite::EMAIL));
            div()
                .flex()
                .gap_2()
                .child(TextField::new(&email, "Email"))
                .child(
                    Button::new(("remove", index))
                        .ghost()
                        .icon(IconName::Trash)
                        .on_click(team.remove_handler(index)),
                )
                .into_any_element()
        })
        .collect();
    div()
        .children(rows)
        .child(
            Button::new("add")
                .outline()
                .label("Add teammate")
                .on_click(team.push_handler(Invite::default())),
        )
}
```

`push_value`, `insert_value`, `remove_value`, `swap_values`, `move_value` and `replace_value`
move the rows' touched, dirty and error state (and their inputs) along with them.

## Unsaved changes

`form.is_dirty()` plugs into the router's blocker: `router::use_blocker(cx, form.is_dirty())`
asks before navigating away from a changed form.
