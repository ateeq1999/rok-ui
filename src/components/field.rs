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

use gpui::{div, prelude::*, px, AnyElement, App, FontWeight, SharedString, StyleRefinement};

use crate::{component, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// Label above the control, or beside it when horizontal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FieldOrientation {
    #[default]
    Vertical,
    Horizontal,
}

/// One form field: a label, a control and optional description / error.
#[component]
pub fn Field(
    #[prop(optional)] orientation: FieldOrientation,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] invalid: bool,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    let destructive_text = cx.theme().colors.destructive_text;
    div()
        .flex()
        .w_full()
        .gap(px(12.))
        .map(|field| match orientation {
            FieldOrientation::Vertical => field.flex_col(),
            FieldOrientation::Horizontal => field.items_center(),
        })
        .when(disabled, |field| field.opacity(0.5))
        .when(invalid, |field| field.text_color(destructive_text))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The label of a [`Field`].
#[component]
pub fn FieldLabel(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .line_height(px(16.))
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// A bolder title for fields whose control is a card or a choice group.
#[component]
pub fn FieldTitle(text: SharedString) -> impl IntoElement {
    div().text_sm().font_weight(FontWeight::MEDIUM).child(text)
}

/// Help text under the control.
#[component]
pub fn FieldDescription(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(cx.theme().colors.muted_foreground)
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// Validation message, in the error color.
#[component]
pub fn FieldError(text: SharedString, cx: &mut App) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(cx.theme().colors.destructive_text)
        .child(text)
}

/// Stacks label and description next to a horizontal control.
#[component]
pub fn FieldContent(#[children] children: Vec<AnyElement>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .gap(px(6.))
        .children(children)
}

/// A vertical stack of fields.
#[component]
pub fn FieldGroup(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .gap(px(28.))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// A group of related fields with a [`FieldLegend`] (HTML's `<fieldset>`).
#[component]
pub fn FieldSet(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(24.))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The heading of a [`FieldSet`].
#[component]
pub fn FieldLegend(text: SharedString) -> impl IntoElement {
    div()
        .mb(px(-12.))
        .text_base()
        .font_weight(FontWeight::MEDIUM)
        .child(text)
}

/// A divider between fields, with optional centered text ("Or continue with").
#[component]
pub fn FieldSeparator(
    #[prop(optional)] text: Option<SharedString>,
    cx: &mut App,
) -> impl IntoElement {
    let colors = cx.theme().colors.clone();
    let line = || div().flex_1().h(px(1.)).bg(colors.border);
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(20.))
        .child(line())
        .when_some(text, |separator, text| {
            separator
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.muted_foreground)
                        .child(text),
                )
                .child(line())
        })
}
