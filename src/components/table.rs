//! Table: rows and cells, composed like shadcn/ui's `<Table>`.
//!
//! GPUI has no table layout, so columns are flex children: every cell grows
//! equally by default. Give the head and cells of a column the same width
//! (`TableCell::new(..).w(px(100.))`) to size it.
//!
//! ```ignore
//! Table::new()
//!     .child(TableCaption::new("A list of your recent invoices."))
//!     .child(TableHeader::new().child(TableRow::new()
//!         .child(TableHead::new("Invoice").w(px(100.)))
//!         .child(TableHead::new("Status"))
//!         .child(TableHead::new("Amount").text_right())))
//!     .child(TableBody::new().child(TableRow::new()
//!         .child(TableCell::new().w(px(100.)).child("INV001"))
//!         .child(TableCell::new().child("Paid"))
//!         .child(TableCell::new().text_right().child("$250.00"))))
//! ```

use gpui::{div, prelude::*, px, AnyElement, App, FontWeight, SharedString, StyleRefinement};

use super::direction::DirectionalStyled;
use crate::{component, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// The table container.
#[component]
pub fn Table(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .flex_dir()
        .flex_col()
        .w_full()
        .text_sm()
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The header rows.
#[component]
pub fn TableHeader(#[children] children: Vec<AnyElement>, cx: &mut App) -> impl IntoElement {
    div()
        .flex_dir()
        .flex_col()
        .border_b_1()
        .border_color(cx.theme().colors.border)
        .children(children)
}

/// The body rows. Each row but the last draws a bottom border.
#[component]
pub fn TableBody(#[children] children: Vec<AnyElement>) -> impl IntoElement {
    div().flex_dir().flex_col().children(children)
}

/// Summary rows under the body, on a muted background.
#[component]
pub fn TableFooter(#[children] children: Vec<AnyElement>, cx: &mut App) -> impl IntoElement {
    let colors = &cx.theme().colors;
    div()
        .flex_dir()
        .flex_col()
        .border_t_1()
        .border_color(colors.border)
        .bg(colors.muted.opacity(0.5))
        .font_weight(FontWeight::MEDIUM)
        .children(children)
}

/// One row. `selected(true)` highlights it.
#[component]
pub fn TableRow(
    #[prop(optional)] selected: bool,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    let colors = &cx.theme().colors;
    let muted = colors.muted;
    div()
        .flex_dir()
        .items_center()
        .w_full()
        .border_b_1()
        .border_color(colors.border)
        .when(selected, |row| row.bg(muted))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// A header cell.
#[component]
pub fn TableHead(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    div()
        .map(|head| column_sizing(head, &style_overrides))
        .min_w_0()
        .h(px(40.))
        .px(px(8.))
        .flex_dir()
        .items_center()
        .font_weight(FontWeight::MEDIUM)
        .text_color(cx.theme().colors.foreground)
        .whitespace_nowrap()
        .child(text)
        .apply_style_overrides(&style_overrides)
}

/// A body cell; holds any elements.
#[component]
pub fn TableCell(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
) -> impl IntoElement {
    div()
        .map(|cell| column_sizing(cell, &style_overrides))
        .min_w_0()
        .p(px(8.))
        .whitespace_nowrap()
        .overflow_hidden()
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// Muted caption under the table.
#[component]
pub fn TableCaption(text: SharedString, cx: &mut App) -> impl IntoElement {
    div()
        .mt(px(16.))
        .text_center()
        .text_sm()
        .text_color(cx.theme().colors.muted_foreground)
        .child(text)
}

/// Cells share the row equally unless the caller gives them a width. `flex_1`
/// sets a zero flex basis, which would override that width, so it is skipped.
fn column_sizing(cell: gpui::Div, style_overrides: &StyleRefinement) -> gpui::Div {
    if style_overrides.size.width.is_some() {
        cell.flex_none()
    } else {
        cell.flex_1()
    }
}
