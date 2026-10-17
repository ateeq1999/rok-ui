//! `DataTable`: a [`super::Table`] with filtering, sorting, row selection,
//! column visibility and pagination (shadcn/ui's `TanStack` Table example).

use std::{collections::BTreeSet, rc::Rc};

use gpui::{div, prelude::*, AnyElement, App, ElementId, SharedString, StyleRefinement, Window};

use super::{
    button::{Button, IconPosition},
    checkbox::Checkbox,
    input::{use_input_state, Input},
    menu::{DropdownMenu, Menu, MenuItem},
    overlay::{child_id, Align},
};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler, State},
    icon::IconName,
    styles,
    styles::ApplyStyleOverrides,
};

mod columns;

pub use columns::{compare_cells, visible_rows, ColumnAlign, DataColumn, SortDirection};

struct DataTableMemory {
    sort: Option<(usize, SortDirection)>,
    selected: BTreeSet<usize>,
    hidden_columns: BTreeSet<usize>,
    page: usize,
}

type RowActions = Rc<dyn Fn(usize) -> Menu>;

/// Rows are lists of cell text, in column order. Selection, sorting, paging and
/// hidden columns are kept per id; `on_selection_change` reports the selected
/// rows (indices into the rows you passed).
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # struct Payment { status: &'static str, email: &'static str, amount: &'static str }
/// # let payments: Vec<Payment> = Vec::new();
/// let table = DataTable::new("payments")
///     .column(DataColumn::new("status", "Status"))
///     .column(DataColumn::new("email", "Email").sortable())
///     .column(DataColumn::new("amount", "Amount").sortable().align_end())
///     .rows(payments.iter().map(|p| vec![p.status.into(), p.email.into(), p.amount.into()]))
///     .filter_column("email", "Filter emails...")
///     .selectable(true)
///     .row_actions(|_row| Menu::new().item(MenuItem::new("Copy payment ID")));
/// # }
/// ```
#[derive(IntoElement)]
pub struct DataTable {
    id: ElementId,
    columns: Vec<DataColumn>,
    rows: Vec<Vec<SharedString>>,
    filter: Option<(SharedString, SharedString)>,
    selectable: bool,
    page_size: usize,
    row_actions: Option<RowActions>,
    on_selection_change: Option<EventHandler<Vec<usize>>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(DataTable);

impl DataTable {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            columns: Vec::new(),
            rows: Vec::new(),
            filter: None,
            selectable: false,
            page_size: 10,
            row_actions: None,
            on_selection_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add a column.
    #[must_use]
    pub fn column(mut self, column: DataColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// The rows, one cell per column.
    #[must_use]
    pub fn rows(mut self, rows: impl IntoIterator<Item = Vec<SharedString>>) -> Self {
        self.rows = rows.into_iter().collect();
        self
    }

    /// Show a search field that filters rows by the column with `key`.
    #[must_use]
    pub fn filter_column(
        mut self,
        key: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
    ) -> Self {
        self.filter = Some((key.into(), placeholder.into()));
        self
    }

    /// Add a checkbox column with select-all.
    #[must_use]
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Rows per page. Defaults to 10.
    #[must_use]
    pub fn page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size.max(1);
        self
    }

    /// A "…" menu at the end of each row; receives the row's index.
    #[must_use]
    pub fn row_actions(mut self, menu: impl Fn(usize) -> Menu + 'static) -> Self {
        self.row_actions = Some(Rc::new(menu));
        self
    }

    /// Called with the selected row indices.
    #[must_use]
    pub fn on_selection_change(
        mut self,
        handler: impl Fn(&Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_selection_change = Some(Rc::new(handler));
        self
    }
}

fn report_selection(
    memory: &State<DataTableMemory>,
    handler: Option<&EventHandler<Vec<usize>>>,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(handler) = handler {
        let selected: Vec<usize> = memory.read(cx).selected.iter().copied().collect();
        handler(&selected, window, cx);
    }
}

styles! {
    DATA_TABLE = {
        root: { display: flex, direction: column, gap: 4, width: full },
        toolbar: { display: flex, align: center, gap: 2 },
        filter: { max_width: 96 },
        spacer: { flex: 1 },
        table: {
            display: flex,
            direction: column,
            radius: md,
            border: 1,
            border_color: border,
            overflow: hidden,
            text: sm,
            cursor: default,
        },
        header: {
            display: flex,
            align: center,
            height: 10,
            border_bottom: 1,
            border_color: border,
            font: medium,
        },
        row: {
            display: flex,
            align: center,
            min_height: 12,
            border_bottom: 1,
            border_color: border,
            hover: { background: muted/50 },
        },
        row_selected: { background: muted },
        cell: { min_width: 0, padding_x: 2, display: flex, align: center },
        cell_end: { justify: end },
        body_cell: { padding_y: 2 },
        text: { truncate: true },
        select_cell: { width: 10, flex: none, display: flex, justify: center },
        actions_cell: { width: 12, flex: none },
        centered: { display: flex, justify: center },
        // Lines the sort button's label up with the cells below it.
        sort_button: { margin_start: -2 },
        actions_button: { width: 8, height: 8 },
        empty: {
            height: 24,
            display: flex,
            align: center,
            justify: center,
            color: muted_foreground,
        },
        footer: { display: flex, align: center, gap: 2, text: sm, color: muted_foreground },
    }
}

impl RenderOnce for DataTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let memory: State<DataTableMemory> =
            use_keyed_state(child_id(&self.id, "data-table"), window, cx, || {
                DataTableMemory {
                    sort: None,
                    selected: BTreeSet::new(),
                    hidden_columns: BTreeSet::new(),
                    page: 0,
                }
            });
        let filter_placeholder = self
            .filter
            .as_ref()
            .map(|(_, placeholder)| placeholder.clone())
            .unwrap_or_default();
        let filter_input = use_input_state(child_id(&self.id, "filter"), window, cx, |state| {
            state.with_placeholder(filter_placeholder)
        });
        let query = filter_input.read(cx).text().clone();
        let filter_column = self
            .filter
            .as_ref()
            .and_then(|(key, _)| self.columns.iter().position(|column| &column.key == key));

        let (sort, selected, hidden_columns, stored_page) = {
            let memory = memory.read(cx);
            (
                memory.sort,
                memory.selected.clone(),
                memory.hidden_columns.clone(),
                memory.page,
            )
        };
        let row_order = visible_rows(&self.rows, filter_column, &query, sort);
        let page_count = row_order.len().div_ceil(self.page_size).max(1);
        let page = stored_page.min(page_count - 1);
        let page_rows: Vec<usize> = row_order
            .iter()
            .skip(page * self.page_size)
            .take(self.page_size)
            .copied()
            .collect();

        let visible_columns: Vec<(usize, DataColumn)> = self
            .columns
            .iter()
            .cloned()
            .enumerate()
            .filter(|(index, _)| !hidden_columns.contains(index))
            .collect();
        let on_selection_change = self.on_selection_change.clone();

        let cell = |column: &DataColumn| {
            div()
                .sx((
                    &DATA_TABLE.cell,
                    (column.align == ColumnAlign::End).then_some(&DATA_TABLE.cell_end),
                ))
                .map(|cell| match column.width {
                    Some(width) => cell.w(width).flex_none(),
                    None => cell.flex_1(),
                })
        };

        // Header.
        let all_on_page_selected =
            !page_rows.is_empty() && page_rows.iter().all(|row| selected.contains(row));
        let mut header = div().sx(&DATA_TABLE.header);
        if self.selectable {
            let memory = memory.clone();
            let page_rows = page_rows.clone();
            let on_selection_change = on_selection_change.clone();
            header = header.child(
                div().sx(&DATA_TABLE.select_cell).child(
                    Checkbox::new("data-table-select-all")
                        .checked(all_on_page_selected)
                        .on_change(move |checked, window, cx| {
                            memory.update(cx, |memory| {
                                for row in &page_rows {
                                    if *checked {
                                        memory.selected.insert(*row);
                                    } else {
                                        memory.selected.remove(row);
                                    }
                                }
                            });
                            report_selection(&memory, on_selection_change.as_ref(), window, cx);
                        }),
                ),
            );
        }
        for (column_index, column) in &visible_columns {
            let column_index = *column_index;
            let title = column.title.clone();
            let head = if column.sortable {
                let memory = memory.clone();
                let icon = match sort {
                    Some((sorted, SortDirection::Ascending)) if sorted == column_index => {
                        IconName::ArrowUp
                    }
                    Some((sorted, SortDirection::Descending)) if sorted == column_index => {
                        IconName::ArrowDown
                    }
                    _ => IconName::ArrowUpDown,
                };
                Button::new(("data-table-sort", column_index))
                    .ghost()
                    .small()
                    .label(title)
                    .icon(icon)
                    .icon_position(IconPosition::End)
                    .sx(&DATA_TABLE.sort_button)
                    .on_click(move |_, _, cx| {
                        memory.update(cx, |memory| {
                            memory.sort = match memory.sort {
                                Some((sorted, SortDirection::Ascending))
                                    if sorted == column_index =>
                                {
                                    Some((column_index, SortDirection::Descending))
                                }
                                Some((sorted, SortDirection::Descending))
                                    if sorted == column_index =>
                                {
                                    None
                                }
                                _ => Some((column_index, SortDirection::Ascending)),
                            };
                        });
                    })
                    .into_any_element()
            } else {
                div()
                    .child(crate::components::bidi_text::text(title))
                    .into_any_element()
            };
            header = header.child(cell(column).child(head));
        }
        if self.row_actions.is_some() {
            header = header.child(div().sx(&DATA_TABLE.actions_cell));
        }

        // Body.
        let body_rows: Vec<AnyElement> = page_rows
            .iter()
            .map(|row_index| {
                let row_index = *row_index;
                let is_selected = selected.contains(&row_index);
                let mut row = div().id(("data-table-row", row_index)).sx((
                    &DATA_TABLE.row,
                    is_selected.then_some(&DATA_TABLE.row_selected),
                ));
                if self.selectable {
                    let memory = memory.clone();
                    let on_selection_change = on_selection_change.clone();
                    row = row.child(
                        div().sx(&DATA_TABLE.select_cell).child(
                            Checkbox::new(("data-table-select", row_index))
                                .checked(is_selected)
                                .on_change(move |checked, window, cx| {
                                    memory.update(cx, |memory| {
                                        if *checked {
                                            memory.selected.insert(row_index);
                                        } else {
                                            memory.selected.remove(&row_index);
                                        }
                                    });
                                    report_selection(
                                        &memory,
                                        on_selection_change.as_ref(),
                                        window,
                                        cx,
                                    );
                                }),
                        ),
                    );
                }
                for (column_index, column) in &visible_columns {
                    let text = self.rows[row_index]
                        .get(*column_index)
                        .cloned()
                        .unwrap_or_default();
                    row = row.child(
                        cell(column).sx(&DATA_TABLE.body_cell).child(
                            div()
                                .sx(&DATA_TABLE.text)
                                .child(crate::components::bidi_text::text(text)),
                        ),
                    );
                }
                if let Some(row_actions) = self.row_actions.as_ref() {
                    row = row.child(
                        div()
                            .sx((&DATA_TABLE.actions_cell, &DATA_TABLE.centered))
                            .child(
                                DropdownMenu::new(("data-table-actions", row_index))
                                    .align(Align::End)
                                    .trigger(
                                        Button::new(("data-table-actions-trigger", row_index))
                                            .ghost()
                                            .icon_only(IconName::Ellipsis)
                                            .sx(&DATA_TABLE.actions_button)
                                            .tooltip("Open menu"),
                                    )
                                    .menu(row_actions(row_index)),
                            ),
                    );
                }
                row.into_any_element()
            })
            .collect();
        let body = if body_rows.is_empty() {
            vec![div()
                .sx(&DATA_TABLE.empty)
                .child("No results.")
                .into_any_element()]
        } else {
            body_rows
        };

        // Toolbar.
        let hideable: Vec<(usize, DataColumn)> = self
            .columns
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, column)| column.hideable)
            .collect();
        let columns_menu = (!hideable.is_empty()).then(|| {
            let menu = hideable
                .into_iter()
                .fold(Menu::new(), |menu, (column_index, column)| {
                    let memory = memory.clone();
                    let is_visible = !hidden_columns.contains(&column_index);
                    menu.item(MenuItem::new(column.title).checked(is_visible).on_select(
                        move |(), _, cx| {
                            memory.update(cx, |memory| {
                                if !memory.hidden_columns.remove(&column_index) {
                                    memory.hidden_columns.insert(column_index);
                                }
                            });
                        },
                    ))
                });
            DropdownMenu::new(child_id(&self.id, "columns"))
                .align(Align::End)
                .trigger(
                    Button::new("data-table-columns")
                        .outline()
                        .label("Columns")
                        .icon(IconName::ChevronDown)
                        .icon_position(IconPosition::End),
                )
                .menu(menu)
        });
        let toolbar = div()
            .sx(&DATA_TABLE.toolbar)
            .when(self.filter.is_some(), |toolbar| {
                toolbar.child(Input::new(&filter_input).sx(&DATA_TABLE.filter))
            })
            .child(div().sx(&DATA_TABLE.spacer))
            .children(columns_menu);

        // Footer.
        let previous_memory = memory.clone();
        let next_memory = memory.clone();
        let footer = div()
            .sx(&DATA_TABLE.footer)
            .child(div().sx(&DATA_TABLE.spacer).child(if self.selectable {
                format!("{} of {} row(s) selected.", selected.len(), self.rows.len())
            } else {
                format!("Page {} of {}", page + 1, page_count)
            }))
            .child(
                Button::new("data-table-previous")
                    .outline()
                    .small()
                    .label("Previous")
                    .disabled(page == 0)
                    .on_click(move |_, _, cx| {
                        previous_memory.update(cx, |memory| memory.page = page.saturating_sub(1));
                    }),
            )
            .child(
                Button::new("data-table-next")
                    .outline()
                    .small()
                    .label("Next")
                    .disabled(page + 1 >= page_count)
                    .on_click(move |_, _, cx| {
                        next_memory.update(cx, |memory| memory.page = page + 1);
                    }),
            );

        div()
            .id(self.id)
            .sx((&DATA_TABLE.root, &self.sx))
            .child(toolbar)
            .child(div().sx(&DATA_TABLE.table).child(header).children(body))
            .child(footer)
            .apply_style_overrides(&self.style_overrides)
    }
}
