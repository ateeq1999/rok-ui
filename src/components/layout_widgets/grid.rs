//! Children in rows and columns: `Wrap` and `GridView`.

use gpui::{div, prelude::*, px, AnyElement, App, Div, ElementId, Pixels, StyleRefinement, Window};

use super::{CrossAxisAlignment, LayoutBuilder, MainAxisAlignment};
use crate::components::direction::{is_rtl, DirectionalStyled};
use crate::{
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// Children in runs that wrap onto new lines when a run is full, like words in a
/// paragraph (chips, tags, buttons).
#[derive(IntoElement)]
pub struct Wrap {
    alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    spacing: Pixels,
    run_spacing: Pixels,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Wrap);

impl Wrap {
    /// Children that wrap onto new lines.
    #[must_use]
    pub fn new() -> Self {
        Self {
            alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Start,
            spacing: px(0.),
            run_spacing: px(0.),
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Placement of children within a run. Default: `Start`.
    #[must_use]
    pub fn alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Placement of children of different heights within a run. Default: `Start`.
    #[must_use]
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }

    /// Space between children in a run.
    #[must_use]
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into();
        self
    }

    /// Space between runs.
    #[must_use]
    pub fn run_spacing(mut self, run_spacing: impl Into<Pixels>) -> Self {
        self.run_spacing = run_spacing.into();
        self
    }
}

impl Default for Wrap {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Wrap {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Wrap {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut wrap = div()
            .flex_dir()
            .flex_wrap()
            .w_full()
            .gap_x(self.spacing)
            .gap_y(self.run_spacing);
        wrap.style().justify_content = Some(self.alignment.justify_content());
        wrap.style().align_items = Some(self.cross_axis_alignment.align_items(false));
        wrap.children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// How a [`GridView`] picks its column count.
#[derive(Clone, Copy, Debug, PartialEq)]
enum GridColumns {
    Count(usize),
    MaxExtent(Pixels),
}

/// Children in a grid of equal-width columns, filled in reading order (right to
/// left in RTL). Wrap it in a scroll container for long grids.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let photos: Vec<AnyElement> = Vec::new();
/// # let cards: Vec<AnyElement> = Vec::new();
/// let fixed_columns = GridView::count(3).spacing(px(12.)).children(photos);
/// let fitted_columns = GridView::extent("products", px(220.)).spacing(px(16.)).children(cards);
/// # }
/// ```
#[derive(IntoElement)]
pub struct GridView {
    id: Option<ElementId>,
    columns: GridColumns,
    main_axis_spacing: Pixels,
    cross_axis_spacing: Pixels,
    row_height: Option<Pixels>,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(GridView);

impl GridView {
    fn with_columns(id: Option<ElementId>, columns: GridColumns) -> Self {
        Self {
            id,
            columns,
            main_axis_spacing: px(0.),
            cross_axis_spacing: px(0.),
            row_height: None,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A fixed number of columns (Flutter's `GridView.count`).
    #[must_use]
    pub fn count(columns: usize) -> Self {
        Self::with_columns(None, GridColumns::Count(columns.max(1)))
    }

    /// As many columns as fit with each at most `max_extent` wide (Flutter's
    /// `GridView.extent`). The column count follows the grid's own width.
    pub fn extent(id: impl Into<ElementId>, max_extent: impl Into<Pixels>) -> Self {
        Self::with_columns(Some(id.into()), GridColumns::MaxExtent(max_extent.into()))
    }

    /// Space between rows and between columns.
    #[must_use]
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        let spacing = spacing.into();
        self.main_axis_spacing = spacing;
        self.cross_axis_spacing = spacing;
        self
    }

    /// Space between rows.
    #[must_use]
    pub fn main_axis_spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.main_axis_spacing = spacing.into();
        self
    }

    /// Space between columns.
    #[must_use]
    pub fn cross_axis_spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.cross_axis_spacing = spacing.into();
        self
    }

    /// A fixed height for every row. Rows otherwise fit their tallest child.
    #[must_use]
    pub fn row_height(mut self, height: impl Into<Pixels>) -> Self {
        self.row_height = Some(height.into());
        self
    }
}

impl ParentElement for GridView {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// The number of columns at most `max_extent` wide that fill `width`.
fn columns_for_extent(width: Pixels, max_extent: Pixels, spacing: Pixels) -> usize {
    let per_column = f32::from(max_extent + spacing).max(1.);
    (f32::from(width + spacing) / per_column).ceil().max(1.) as usize
}

fn render_grid(
    columns: usize,
    children: Vec<AnyElement>,
    main_axis_spacing: Pixels,
    cross_axis_spacing: Pixels,
    row_height: Option<Pixels>,
) -> Div {
    let rtl = is_rtl();
    let cells = children.into_iter().enumerate().map(move |(index, child)| {
        let (row, column) = (index / columns, index % columns);
        let column = if rtl { columns - 1 - column } else { column };
        div()
            .flex()
            .flex_col()
            .min_w(px(0.))
            .col_start(column as i16 + 1)
            .row_start(row as i16 + 1)
            .when_some(row_height, gpui::Styled::h)
            .child(child)
    });
    div()
        .grid()
        .grid_cols(columns as u16)
        .w_full()
        .gap_x(cross_axis_spacing)
        .gap_y(main_axis_spacing)
        .children(cells)
}

impl RenderOnce for GridView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            columns,
            main_axis_spacing,
            cross_axis_spacing,
            row_height,
            children,
            sx,
            style_overrides,
        } = self;
        match (columns, id) {
            (GridColumns::MaxExtent(max_extent), Some(id)) => {
                LayoutBuilder::new(id, move |constraints, _, _| {
                    let columns =
                        columns_for_extent(constraints.max_width, max_extent, cross_axis_spacing);
                    render_grid(
                        columns,
                        children,
                        main_axis_spacing,
                        cross_axis_spacing,
                        row_height,
                    )
                    .into_any_element()
                })
                .sx(sx)
                .apply_style_overrides(&style_overrides)
                .into_any_element()
            }
            (columns, _) => {
                let columns = match columns {
                    GridColumns::Count(count) => count,
                    GridColumns::MaxExtent(_) => 1,
                };
                render_grid(
                    columns,
                    children,
                    main_axis_spacing,
                    cross_axis_spacing,
                    row_height,
                )
                .sx(&sx)
                .apply_style_overrides(&style_overrides)
                .into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extent_grids_fit_as_many_columns_as_the_width_allows() {
        // 3 columns of 200 with 2 gaps of 10 = 620.
        assert_eq!(columns_for_extent(px(620.), px(200.), px(10.)), 3);
        // A little wider and the columns would exceed 200, so a fourth is added.
        assert_eq!(columns_for_extent(px(640.), px(200.), px(10.)), 4);
        assert_eq!(columns_for_extent(px(50.), px(200.), px(10.)), 1);
    }
}
