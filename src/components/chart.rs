//! Chart: bar, line, area, pie and donut charts drawn with GPUI, with a grid,
//! a legend and a hover tooltip (shadcn/ui's Recharts-based charts).

use std::{cell::Cell, rc::Rc};

use gpui::{
    div, hsla, prelude::*, px, App, Bounds, ElementId, Hsla, Pixels, SharedString, StyleRefinement,
    Window,
};

use super::overlay::child_id;
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, State},
    styles,
    styles::ApplyStyleOverrides,
    theme::{ActiveTheme, ThemeMode},
};

mod draw;

use draw::format_number;

/// shadcn/ui's `--chart-1` … `--chart-5` for the given mode.
#[must_use]
pub fn chart_palette(mode: ThemeMode) -> [Hsla; 5] {
    match mode {
        ThemeMode::Light => [
            hsla(20. / 360., 1.0, 0.48, 1.),
            hsla(175. / 360., 1.0, 0.29, 1.),
            hsla(196. / 360., 0.72, 0.23, 1.),
            hsla(44. / 360., 1.0, 0.50, 1.),
            hsla(36. / 360., 1.0, 0.50, 1.),
        ],
        ThemeMode::Dark => [
            hsla(225. / 360., 0.84, 0.49, 1.),
            hsla(160. / 360., 1.0, 0.37, 1.),
            hsla(36. / 360., 1.0, 0.50, 1.),
            hsla(273. / 360., 1.0, 0.64, 1.),
            hsla(345. / 360., 1.0, 0.56, 1.),
        ],
    }
}

/// The `index`-th chart color of the active theme (wraps after five).
pub fn chart_color(index: usize, cx: &App) -> Hsla {
    chart_palette(cx.theme().mode)[index % 5]
}

/// The smallest "nice" number (1, 2 or 5 × 10ⁿ) at or above `value`, for axis maxima.
#[must_use]
pub fn nice_ceiling(value: f32) -> f32 {
    if value <= 0. {
        return 1.;
    }
    let magnitude = 10_f32.powf(value.log10().floor());
    let normalized = value / magnitude;
    let nice = if normalized <= 1. {
        1.
    } else if normalized <= 2. {
        2.
    } else if normalized <= 5. {
        5.
    } else {
        10.
    };
    nice * magnitude
}

/// What kind of chart to draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChartKind {
    /// Vertical bars, grouped per category.
    #[default]
    Bar,
    /// Lines through each series' points.
    Line,
    /// Lines with the area below them filled.
    Area,
    /// Slices of the first series, one per category.
    Pie,
    /// A pie with a hole; see [`Chart::center_label`].
    Donut,
}

/// One named set of values, one value per category.
#[derive(Clone)]
pub struct ChartSeries {
    label: SharedString,
    values: Vec<f32>,
    color: Option<Hsla>,
}

impl ChartSeries {
    /// A named series of values, one per category.
    pub fn new(label: impl Into<SharedString>, values: impl IntoIterator<Item = f32>) -> Self {
        Self {
            label: label.into(),
            values: values.into_iter().collect(),
            color: None,
        }
    }

    /// Override the palette color.
    #[must_use]
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let visitors = Chart::new("visitors", ChartKind::Bar)
///     .categories(["January", "February", "March"])
///     .series(ChartSeries::new("Desktop", [186., 305., 237.]))
///     .series(ChartSeries::new("Mobile", [80., 200., 120.]))
///     .legend(true);
///
/// let browsers = Chart::new("browsers", ChartKind::Donut)
///     .categories(["Chrome", "Safari", "Firefox"])
///     .series(ChartSeries::new("Visitors", [275., 200., 187.]))
///     .center_label("662", "Visitors");
/// # }
/// ```
#[derive(IntoElement)]
pub struct Chart {
    id: ElementId,
    kind: ChartKind,
    categories: Vec<SharedString>,
    series: Vec<ChartSeries>,
    height: Pixels,
    show_grid: bool,
    show_legend: bool,
    show_y_axis: bool,
    stacked: bool,
    center_label: Option<(SharedString, SharedString)>,
    value_format: Rc<dyn Fn(f32) -> SharedString>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Chart);

impl Chart {
    /// A chart of `kind`. `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, kind: ChartKind) -> Self {
        Self {
            id: id.into(),
            kind,
            categories: Vec::new(),
            series: Vec::new(),
            height: px(240.),
            show_grid: true,
            show_legend: false,
            show_y_axis: false,
            stacked: false,
            center_label: None,
            value_format: Rc::new(|value| format_number(value).into()),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The x-axis labels (or slice labels for pie charts).
    #[must_use]
    pub fn categories(
        mut self,
        categories: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.categories = categories.into_iter().map(Into::into).collect();
        self
    }

    /// Add a series.
    #[must_use]
    pub fn series(mut self, series: ChartSeries) -> Self {
        self.series.push(series);
        self
    }

    /// Height of the plot area. Defaults to 240px.
    #[must_use]
    pub fn height(mut self, height: impl Into<Pixels>) -> Self {
        self.height = height.into();
        self
    }

    /// Show horizontal grid lines. Default: shown.
    #[must_use]
    pub fn grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// Show the legend. Default: shown.
    #[must_use]
    pub fn legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Value labels on the left edge.
    #[must_use]
    pub fn y_axis(mut self, show: bool) -> Self {
        self.show_y_axis = show;
        self
    }

    /// Stack bar series instead of grouping them.
    #[must_use]
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Big value and caption in the middle of a donut.
    #[must_use]
    pub fn center_label(
        mut self,
        value: impl Into<SharedString>,
        caption: impl Into<SharedString>,
    ) -> Self {
        self.center_label = Some((value.into(), caption.into()));
        self
    }

    /// How values are printed in tooltips and on the axis.
    #[must_use]
    pub fn value_format(mut self, format: impl Fn(f32) -> SharedString + 'static) -> Self {
        self.value_format = Rc::new(format);
        self
    }
}

struct ChartMemory {
    hovered: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

// Charts read left to right in every direction: every flex container starts from
// `ltr`, and tables that stack set a column direction after it.
styles! {
    CHART = {
        ltr: { display: flex, direction: row_ltr },
        column: { direction: column, width: full },
        full_width: { width: full },
        fill: { position: absolute, top: 0, left: 0, size: full },
        legend: { wrap: true, justify: center, gap: 4, padding_top: 3, text: xs },
        legend_item: { align: center, gap: 1.5 },
        legend_swatch: { size: 2, radius: 0.5 },
        grid_line: {
            position: absolute,
            left: 0,
            width: full,
            height: 0.25,
            background: border/60,
        },
        bars: { align: end },
        bar_column: { flex: 1, height: full, align: end, justify: center, padding_x: 1 },
        grouped: { gap: 1 },
        dimmed: { opacity: 0.6 },
        stack: { direction: column_reverse, width: 70%, max_width: 12, height: full },
        stack_segment: { width: full },
        bar: { flex: 1, max_width: 8 },
        bar_top: { radius_top: sm },
        plot: { position: relative, flex: 1 },
        y_axis: { position: relative, width: 10, text: xs, color: muted_foreground },
        y_tick: { position: absolute, right: 2, margin_top: -2 },
        x_labels: { padding_top: 2, text: xs, color: muted_foreground },
        x_labels_offset: { padding_left: 10 },
        x_label: {
            flex: 1,
            min_width: 0,
            justify: center,
            overflow: hidden,
            whitespace: nowrap,
        },
        tooltip: {
            padding_x: 2.5,
            padding_y: 1.5,
            radius: md,
            border: 1,
            border_color: border,
            background: background,
            shadow: lg,
            text: xs,
        },
        tooltip_card: { direction: column, gap: 1.5, min_width: 32 },
        tooltip_anchor: { position: absolute, top: 2, width: 0 },
        tooltip_anchor_end: { justify: end },
        tooltip_before: { margin_right: 2 },
        tooltip_after: { margin_left: 2 },
        tooltip_row: { align: center, gap: 2 },
        tooltip_swatch: { size: 2.5, radius: 0.5 },
        tooltip_label: { flex: 1, color: muted_foreground },
        tooltip_value: { font: medium },
        pie_label: { color: muted_foreground },
        pie: { position: relative, margin_x: auto },
        pie_tooltip: { position: absolute, top: 2, right: 2, align: center, gap: 2 },
        center_label: { direction: column, align: center, justify: center },
        center_value: { text: 28.0, font: bold },
        center_caption: { text: xs, color: muted_foreground },
    }
}

impl RenderOnce for Chart {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let memory: State<ChartMemory> =
            use_keyed_state(child_id(&self.id, "chart"), window, cx, || ChartMemory {
                hovered: None,
                bounds: Rc::new(Cell::new(Bounds::default())),
            });
        let hovered = memory.read(cx).hovered;
        let bounds = memory.read(cx).bounds.clone();
        let theme = cx.theme();
        let palette = chart_palette(theme.mode);
        let series_colors: Vec<Hsla> = self
            .series
            .iter()
            .enumerate()
            .map(|(index, series)| series.color.unwrap_or(palette[index % 5]))
            .collect();
        let is_circular = matches!(self.kind, ChartKind::Pie | ChartKind::Donut);
        let category_count = self.categories.len().max(
            self.series
                .iter()
                .map(|series| series.values.len())
                .max()
                .unwrap_or(0),
        );
        let value = |series: &ChartSeries, index: usize| -> f32 {
            series.values.get(index).copied().unwrap_or(0.).max(0.)
        };

        let plot = if is_circular {
            self.render_circular(&memory, hovered, bounds.clone(), &palette, cx)
        } else {
            let raw_max = (0..category_count)
                .map(|index| {
                    if self.stacked && self.kind == ChartKind::Bar {
                        self.series.iter().map(|series| value(series, index)).sum()
                    } else {
                        self.series
                            .iter()
                            .map(|series| value(series, index))
                            .fold(0., f32::max)
                    }
                })
                .fold(0., f32::max);
            let max = nice_ceiling(raw_max);
            self.render_cartesian(
                &memory,
                hovered,
                bounds.clone(),
                &series_colors,
                category_count,
                max,
                cx,
            )
        };

        let legend_items: Vec<(SharedString, Hsla)> = if is_circular {
            self.categories
                .iter()
                .enumerate()
                .map(|(index, label)| (label.clone(), palette[index % 5]))
                .collect()
        } else {
            self.series
                .iter()
                .zip(series_colors.iter())
                .map(|(series, color)| (series.label.clone(), *color))
                .collect()
        };
        let legend = self.show_legend.then(|| {
            div()
                .sx((&CHART.ltr, &CHART.legend))
                .children(legend_items.into_iter().map(|(label, color)| {
                    div()
                        .sx((&CHART.ltr, &CHART.legend_item))
                        .child(div().sx(&CHART.legend_swatch).bg(color))
                        .child(crate::components::bidi_text::text(label))
                }))
        });

        div()
            .id(self.id.clone())
            .sx((&CHART.ltr, &CHART.column, &self.sx))
            .child(plot)
            .children(legend)
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::nice_ceiling;

    #[test]
    #[allow(clippy::float_cmp)] // Exact round numbers.
    fn nice_ceiling_rounds_up_to_one_two_or_five() {
        assert_eq!(nice_ceiling(0.), 1.);
        assert_eq!(nice_ceiling(7.), 10.);
        assert_eq!(nice_ceiling(186.), 200.);
        assert_eq!(nice_ceiling(305.), 500.);
        assert_eq!(nice_ceiling(1000.), 1000.);
        assert_eq!(nice_ceiling(1200.), 2000.);
    }
}
