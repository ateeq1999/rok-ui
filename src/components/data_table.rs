//! DataTable: a [`super::Table`] with filtering, sorting, row selection,
//! column visibility and pagination (shadcn/ui's TanStack Table example).

use std::{cmp::Ordering, collections::BTreeSet, rc::Rc};

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, FontWeight, SharedString,
    StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::{
    button::{Button, IconPosition},
    checkbox::Checkbox,
    input::{use_input_state, Input},
    menu::{DropdownMenu, Menu, MenuItem},
    overlay::{child_id, Align},
};
use crate::{
    hooks::{use_keyed_state, EventHandler, State},
    icon::IconName,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Horizontal alignment of a column's cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColumnAlign {
    #[default]
    Start,
    End,
}

/// One column of a [`DataTable`].
#[derive(Clone)]
pub struct DataColumn {
    key: SharedString,
    title: SharedString,
    sortable: bool,
    hideable: bool,
    align: ColumnAlign,
    width: Option<gpui::Pixels>,
}

impl DataColumn {
    /// `key` identifies the column (filtering, visibility); `title` is its header.
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            sortable: false,
            hideable: true,
            align: ColumnAlign::Start,
            width: None,
        }
    }

    /// Clicking the header sorts by this column (numbers numerically, `$1,200` included).
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    /// Keep the column out of the "Columns" menu.
    pub fn always_visible(mut self) -> Self {
        self.hideable = false;
        self
    }

    pub fn align_end(mut self) -> Self {
        self.align = ColumnAlign::End;
        self
    }

    pub fn width(mut self, width: impl Into<gpui::Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }
}

/// Sort direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Compare two cells: as numbers when both parse (ignoring `$`, `,`, `%` and
/// spaces), otherwise as case-insensitive text.
pub fn compare_cells(a: &str, b: &str) -> Ordering {
    let as_number = |text: &str| -> Option<f64> {
        let cleaned: String = text
            .chars()
            .filter(|character| !matches!(character, '$' | ',' | '%' | ' ' | '€' | '£'))
            .collect();
        cleaned.parse::<f64>().ok()
    };
    match (as_number(a), as_number(b)) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

/// Indices of `rows` that contain `query` in column `filter_column`, sorted by
/// `sort`. Shared by rendering and tests.
pub fn visible_rows(
    rows: &[Vec<SharedString>],
    filter_column: Option<usize>,
    query: &str,
    sort: Option<(usize, SortDirection)>,
) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let mut indices: Vec<usize> = (0..rows.len())
        .filter(|row_index| {
            query.is_empty()
                || match filter_column {
                    Some(column) => rows[*row_index]
                        .get(column)
                        .is_some_and(|cell| cell.to_lowercase().contains(&query)),
                    None => rows[*row_index]
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&query)),
                }
        })
        .collect();
    if let Some((column, direction)) = sort {
        indices.sort_by(|a, b| {
            let empty = SharedString::default();
            let left = rows[*a].get(column).unwrap_or(&empty);
            let right = rows[*b].get(column).unwrap_or(&empty);
            let ordering = compare_cells(left, right);
            match direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });
    }
    indices
}

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
/// ```ignore
/// DataTable::new("payments")
///     .column(DataColumn::new("status", "Status"))
///     .column(DataColumn::new("email", "Email").sortable())
///     .column(DataColumn::new("amount", "Amount").sortable().align_end())
///     .rows(payments.iter().map(|p| vec![p.status.into(), p.email.into(), p.amount.into()]))
///     .filter_column("email", "Filter emails…")
///     .selectable(true)
///     .row_actions(|row| Menu::new().item(MenuItem::new("Copy payment ID")))
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
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(DataTable);

impl DataTable {
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
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn column(mut self, column: DataColumn) -> Self {
        self.columns.push(column);
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = Vec<SharedString>>) -> Self {
        self.rows = rows.into_iter().collect();
        self
    }

    /// Show a search field that filters rows by the column with `key`.
    pub fn filter_column(
        mut self,
        key: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
    ) -> Self {
        self.filter = Some((key.into(), placeholder.into()));
        self
    }

    /// Add a checkbox column with select-all.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Rows per page. Defaults to 10.
    pub fn page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size.max(1);
        self
    }

    /// A "…" menu at the end of each row; receives the row's index.
    pub fn row_actions(mut self, menu: impl Fn(usize) -> Menu + 'static) -> Self {
        self.row_actions = Some(Rc::new(menu));
        self
    }

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

        let theme = cx.theme();
        let colors = theme.colors.clone();
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
                .min_w_0()
                .px(px(8.))
                .flex_dir()
                .items_center()
                .map(|cell| match column.width {
                    Some(width) => cell.w(width).flex_none(),
                    None => cell.flex_1(),
                })
                .when(column.align == ColumnAlign::End, |cell| cell.justify_end())
        };

        // Header.
        let all_on_page_selected =
            !page_rows.is_empty() && page_rows.iter().all(|row| selected.contains(row));
        let mut header = div()
            .flex_dir()
            .items_center()
            .h(px(40.))
            .border_b_1()
            .border_color(colors.border)
            .font_weight(FontWeight::MEDIUM);
        if self.selectable {
            let memory = memory.clone();
            let page_rows = page_rows.clone();
            let on_selection_change = on_selection_change.clone();
            header = header.child(
                div()
                    .w(px(40.))
                    .flex_none()
                    .flex_dir()
                    .justify_center()
                    .child(
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
                    .ms(px(-8.))
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
                        })
                    })
                    .into_any_element()
            } else {
                div().child(title).into_any_element()
            };
            header = header.child(cell(column).child(head));
        }
        if self.row_actions.is_some() {
            header = header.child(div().w(px(48.)).flex_none());
        }

        // Body.
        let body_rows: Vec<AnyElement> = page_rows
            .iter()
            .map(|row_index| {
                let row_index = *row_index;
                let is_selected = selected.contains(&row_index);
                let mut row = div()
                    .id(("data-table-row", row_index))
                    .flex_dir()
                    .items_center()
                    .min_h(px(48.))
                    .border_b_1()
                    .border_color(colors.border)
                    .hover(|style| style.bg(colors.muted.opacity(0.5)))
                    .when(is_selected, |row| row.bg(colors.muted));
                if self.selectable {
                    let memory = memory.clone();
                    let on_selection_change = on_selection_change.clone();
                    row = row.child(
                        div()
                            .w(px(40.))
                            .flex_none()
                            .flex_dir()
                            .justify_center()
                            .child(
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
                    row = row.child(cell(column).py(px(8.)).child(div().truncate().child(text)));
                }
                if let Some(row_actions) = self.row_actions.as_ref() {
                    row = row.child(
                        div()
                            .w(px(48.))
                            .flex_none()
                            .flex_dir()
                            .justify_center()
                            .child(
                                DropdownMenu::new(("data-table-actions", row_index))
                                    .align(Align::End)
                                    .trigger(
                                        Button::new(("data-table-actions-trigger", row_index))
                                            .ghost()
                                            .icon_only(IconName::Ellipsis)
                                            .w(px(32.))
                                            .h(px(32.))
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
                .h(px(96.))
                .flex_dir()
                .items_center()
                .justify_center()
                .text_color(colors.muted_foreground)
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
                        move |_, _, cx| {
                            memory.update(cx, |memory| {
                                if !memory.hidden_columns.remove(&column_index) {
                                    memory.hidden_columns.insert(column_index);
                                }
                            })
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
            .flex_dir()
            .items_center()
            .gap(px(8.))
            .when(self.filter.is_some(), |toolbar| {
                toolbar.child(Input::new(&filter_input).max_w(px(384.)))
            })
            .child(div().flex_1())
            .children(columns_menu);

        // Footer.
        let previous_memory = memory.clone();
        let next_memory = memory.clone();
        let footer = div()
            .flex_dir()
            .items_center()
            .gap(px(8.))
            .text_sm()
            .text_color(colors.muted_foreground)
            .child(div().flex_1().child(if self.selectable {
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
                        previous_memory.update(cx, |memory| memory.page = page.saturating_sub(1))
                    }),
            )
            .child(
                Button::new("data-table-next")
                    .outline()
                    .small()
                    .label("Next")
                    .disabled(page + 1 >= page_count)
                    .on_click(move |_, _, cx| {
                        next_memory.update(cx, |memory| memory.page = page + 1)
                    }),
            );

        div()
            .id(self.id)
            .flex_dir()
            .flex_col()
            .gap(px(16.))
            .w_full()
            .child(toolbar)
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .rounded(theme.radius_medium())
                    .border_1()
                    .border_color(colors.border)
                    .overflow_hidden()
                    .text_sm()
                    .cursor(CursorStyle::Arrow)
                    .child(header)
                    .children(body),
            )
            .child(footer)
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Vec<SharedString>> {
        [
            ["success", "ken99@example.com", "$316.00"],
            ["processing", "abe45@example.com", "$1,242.00"],
            ["failed", "carmella@example.com", "$721.00"],
        ]
        .iter()
        .map(|row| row.iter().map(|cell| SharedString::from(*cell)).collect())
        .collect()
    }

    #[test]
    fn numbers_sort_numerically_and_text_alphabetically() {
        assert_eq!(compare_cells("$1,242.00", "$316.00"), Ordering::Greater);
        assert_eq!(compare_cells("apple", "Banana"), Ordering::Less);
    }

    #[test]
    fn rows_filter_and_sort() {
        let rows = rows();
        assert_eq!(visible_rows(&rows, Some(1), "", None), vec![0, 1, 2]);
        assert_eq!(visible_rows(&rows, Some(1), "CAR", None), vec![2]);
        assert_eq!(
            visible_rows(&rows, None, "", Some((2, SortDirection::Descending))),
            vec![1, 2, 0]
        );
        assert_eq!(
            visible_rows(&rows, None, "", Some((0, SortDirection::Ascending))),
            vec![2, 1, 0]
        );
    }
}
