//! Field: labels, controls, help text and errors composed into form fields.
//!
//! ```ignore
//! FieldSet::new()
//!     .child(FieldLegend::new("Payment method"))
//!     .child(FieldDescription::new("All transactions are secure and encrypted."))
//!     .child(FieldGroup::new()
//!         .child(Field::new()
//!             .child(FieldLabel::new("Name on card"))
//!             .child(Input::new(&name)))
//!         .child(Field::new()
//!             .invalid(true)
//!             .child(FieldLabel::new("Card number"))
//!             .child(Input::new(&number).invalid(true))
//!             .child(FieldError::new("Enter a valid card number."))))
//! ```

use gpui::{div, prelude::*, AnyElement, SharedString, StyleRefinement};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles, styles::ApplyStyleOverrides};

/// Label above the control, or beside it when horizontal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FieldOrientation {
    #[default]
    Vertical,
    Horizontal,
}

styles! {
    FIELD = {
        field: { display: flex, width: full, gap: 3 },
        orientation(FieldOrientation): {
            Vertical: { direction: column },
            Horizontal: { align: center },
        },
        disabled: { opacity: 0.5 },
        invalid: { color: destructive_text },
        label: { text: sm, font: medium, line_height: 4 },
        title: { text: sm, font: medium },
        description: { text: sm, color: muted_foreground },
        error: { text: sm, color: destructive_text },
        content: { display: flex, direction: column, flex: 1, gap: 1.5 },
        group: { display: flex, direction: column, width: full, gap: 7 },
        set: { display: flex, direction: column, gap: 6 },
        legend: { margin_bottom: -3, text: base, font: medium },
        separator: { display: flex, align: center, gap: 2, height: 5 },
        separator_line: { flex: 1, height: 0.25, background: border },
        separator_text: { text: sm, color: muted_foreground },
    }
}

/// One form field: a label, a control and optional description / error.
#[component]
pub fn Field(
    #[prop(optional)] orientation: FieldOrientation,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] invalid: bool,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((
            &FIELD.field,
            FIELD.orientation(orientation),
            disabled.then_some(&FIELD.disabled),
            invalid.then_some(&FIELD.invalid),
            &sx,
        ))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The label of a [`Field`].
#[component]
pub fn FieldLabel(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&FIELD.label, &sx))
        .child(crate::components::bidi_text::text(text))
        .apply_style_overrides(&style_overrides)
}

/// A bolder title for fields whose control is a card or a choice group.
#[component]
pub fn FieldTitle(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div()
        .sx((&FIELD.title, &sx))
        .child(crate::components::bidi_text::text(text))
}

/// Help text under the control.
#[component]
pub fn FieldDescription(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&FIELD.description, &sx))
        .child(crate::components::bidi_text::text(text))
        .apply_style_overrides(&style_overrides)
}

/// Validation message, in the error color.
#[component]
pub fn FieldError(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div()
        .sx((&FIELD.error, &sx))
        .child(crate::components::bidi_text::text(text))
}

/// Stacks label and description next to a horizontal control.
#[component]
pub fn FieldContent(#[children] children: Vec<AnyElement>, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&FIELD.content, &sx)).children(children)
}

/// A vertical stack of fields.
#[component]
pub fn FieldGroup(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&FIELD.group, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// A group of related fields with a [`FieldLegend`] (HTML's `<fieldset>`).
#[component]
pub fn FieldSet(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&FIELD.set, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The heading of a [`FieldSet`].
#[component]
pub fn FieldLegend(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div()
        .sx((&FIELD.legend, &sx))
        .child(crate::components::bidi_text::text(text))
}

/// A divider between fields, with optional centered text ("Or continue with").
#[component]
pub fn FieldSeparator(
    #[prop(optional)] text: Option<SharedString>,
    #[sx] sx: Sx,
) -> impl IntoElement {
    let line = || div().sx(&FIELD.separator_line);
    div()
        .sx((&FIELD.separator, &sx))
        .child(line())
        .when_some(text, |separator, text| {
            separator
                .child(
                    div()
                        .sx(&FIELD.separator_text)
                        .child(crate::components::bidi_text::text(text)),
                )
                .child(line())
        })
}
