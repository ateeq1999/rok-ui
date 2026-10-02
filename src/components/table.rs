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

use gpui::{div, prelude::*, AnyElement, SharedString, StyleRefinement};

use crate::sx::{Sx, SxStyled};
use crate::{component, styles, styles::ApplyStyleOverrides};

styles! {
    TABLE = {
        table: { display: flex, direction: column, width: full, text: sm },
        header: { display: flex, direction: column, border_bottom: 1, border_color: border },
        body: { display: flex, direction: column },
        footer: {
            display: flex,
            direction: column,
            border_top: 1,
            border_color: border,
            background: muted/50,
            font: medium,
        },
        row: {
            display: flex,
            align: center,
            width: full,
            border_bottom: 1,
            border_color: border,
        },
        selected: { background: muted },
        head: {
            min_width: 0,
            height: 10,
            padding_x: 2,
            display: flex,
            align: center,
            font: medium,
            color: foreground,
            whitespace: nowrap,
        },
        cell: { min_width: 0, padding: 2, whitespace: nowrap, overflow: hidden },
        caption: { margin_top: 4, text_align: center, text: sm, color: muted_foreground },
    }
}

/// The table container.
#[component]
pub fn Table(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TABLE.table, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// The header rows.
#[component]
pub fn TableHeader(#[children] children: Vec<AnyElement>, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&TABLE.header, &sx)).children(children)
}

/// The body rows. Each row but the last draws a bottom border.
#[component]
pub fn TableBody(#[children] children: Vec<AnyElement>, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&TABLE.body, &sx)).children(children)
}

/// Summary rows under the body, on a muted background.
#[component]
pub fn TableFooter(#[children] children: Vec<AnyElement>, #[sx] sx: Sx) -> impl IntoElement {
    div().sx((&TABLE.footer, &sx)).children(children)
}

/// One row. `selected(true)` highlights it.
#[component]
pub fn TableRow(
    #[prop(optional)] selected: bool,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&TABLE.row, selected.then_some(&TABLE.selected), &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// A header cell.
#[component]
pub fn TableHead(
    text: SharedString,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .map(|head| column_sizing(head, &style_overrides))
        .sx((&TABLE.head, &sx))
        .child(crate::components::bidi_text::text(text))
        .apply_style_overrides(&style_overrides)
}

/// A body cell; holds any elements.
#[component]
pub fn TableCell(
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .map(|cell| column_sizing(cell, &style_overrides))
        .sx((&TABLE.cell, &sx))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

/// Muted caption under the table.
#[component]
pub fn TableCaption(text: SharedString, #[sx] sx: Sx) -> impl IntoElement {
    div()
        .sx((&TABLE.caption, &sx))
        .child(crate::components::bidi_text::text(text))
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
