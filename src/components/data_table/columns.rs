//! Columns, and sorting and filtering rows by them.

use std::cmp::Ordering;

#[cfg(doc)]
use super::DataTable;
use gpui::SharedString;

/// Horizontal alignment of a column's cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColumnAlign {
    /// Aligned to the reading-direction start (text).
    #[default]
    Start,
    /// Aligned to the reading-direction end (numbers).
    End,
}

/// One column of a [`DataTable`].
#[derive(Clone)]
pub struct DataColumn {
    pub(super) key: SharedString,
    pub(super) title: SharedString,
    pub(super) sortable: bool,
    pub(super) hideable: bool,
    pub(super) align: ColumnAlign,
    pub(super) width: Option<gpui::Pixels>,
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
    #[must_use]
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    /// Keep the column out of the "Columns" menu.
    #[must_use]
    pub fn always_visible(mut self) -> Self {
        self.hideable = false;
        self
    }

    /// Align cells to the reading-direction end (for numbers).
    #[must_use]
    pub fn align_end(mut self) -> Self {
        self.align = ColumnAlign::End;
        self
    }

    /// A fixed column width.
    #[must_use]
    pub fn width(mut self, width: impl Into<gpui::Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }
}

/// Sort direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Smallest first.
    Ascending,
    /// Largest first.
    Descending,
}

/// Compare two cells: as numbers when both parse (ignoring `$`, `,`, `%` and
/// spaces), otherwise as case-insensitive text.
#[must_use]
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
#[must_use]
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
