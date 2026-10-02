//! Chart: bar, line, area, pie and donut charts drawn with GPUI, with a grid,
//! a legend and a hover tooltip (shadcn/ui's Recharts-based charts).

use std::{cell::Cell, f32::consts::PI, rc::Rc};

use gpui::{
    canvas, div, hsla, point, prelude::*, px, relative, AnyElement, App, Bounds, ElementId,
    FontWeight, Hsla, PathBuilder, Pixels, Point, SharedString, StyleRefinement, Window,
};

use super::{interaction::measure_bounds, overlay::child_id};
use crate::{
    hooks::{use_keyed_state, State},
    styles::ApplyStyleOverrides,
    theme::{ActiveTheme, ThemeMode},
};

/// shadcn/ui's `--chart-1` … `--chart-5` for the given mode.
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
    #[default]
    Bar,
    Line,
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
    pub fn new(label: impl Into<SharedString>, values: impl IntoIterator<Item = f32>) -> Self {
        Self {
            label: label.into(),
            values: values.into_iter().collect(),
            color: None,
        }
    }

    /// Override the palette color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// ```ignore
/// Chart::new("visitors", ChartKind::Bar)
///     .categories(["January", "February", "March"])
///     .series(ChartSeries::new("Desktop", [186., 305., 237.]))
///     .series(ChartSeries::new("Mobile", [80., 200., 120.]))
///     .legend(true)
///
/// Chart::new("browsers", ChartKind::Donut)
///     .categories(["Chrome", "Safari", "Firefox"])
///     .series(ChartSeries::new("Visitors", [275., 200., 187.]))
///     .center_label("662", "Visitors")
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
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Chart);

impl Chart {
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
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The x-axis labels (or slice labels for pie charts).
    pub fn categories(
        mut self,
        categories: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.categories = categories.into_iter().map(Into::into).collect();
        self
    }

    pub fn series(mut self, series: ChartSeries) -> Self {
        self.series.push(series);
        self
    }

    /// Height of the plot area. Defaults to 240px.
    pub fn height(mut self, height: impl Into<Pixels>) -> Self {
        self.height = height.into();
        self
    }

    pub fn grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    pub fn legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Value labels on the left edge.
    pub fn y_axis(mut self, show: bool) -> Self {
        self.show_y_axis = show;
        self
    }

    /// Stack bar series instead of grouping them.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Big value and caption in the middle of a donut.
    pub fn center_label(
        mut self,
        value: impl Into<SharedString>,
        caption: impl Into<SharedString>,
    ) -> Self {
        self.center_label = Some((value.into(), caption.into()));
        self
    }

    /// How values are printed in tooltips and on the axis.
    pub fn value_format(mut self, format: impl Fn(f32) -> SharedString + 'static) -> Self {
        self.value_format = Rc::new(format);
        self
    }
}

fn format_number(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

/// Points of a smooth curve through `points` (Catmull-Rom as cubic Béziers).
fn add_smooth_curve(path: &mut PathBuilder, points: &[Point<Pixels>]) {
    for index in 0..points.len().saturating_sub(1) {
        let previous = points[index.saturating_sub(1)];
        let current = points[index];
        let next = points[index + 1];
        let after = points[(index + 2).min(points.len() - 1)];
        let control_a = point(
            current.x + (next.x - previous.x) / 6.,
            current.y + (next.y - previous.y) / 6.,
        );
        let control_b = point(
            next.x - (after.x - current.x) / 6.,
            next.y - (after.y - current.y) / 6.,
        );
        path.cubic_bezier_to(next, control_a, control_b);
    }
}

struct ChartMemory {
    hovered: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
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
                .flex()
                .flex_wrap()
                .justify_center()
                .gap(px(16.))
                .pt(px(12.))
                .text_xs()
                .children(legend_items.into_iter().map(|(label, color)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(div().size(px(8.)).rounded(px(2.)).bg(color))
                        .child(label)
                }))
        });

        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .w_full()
            .child(plot)
            .children(legend)
            .apply_style_overrides(&self.style_overrides)
    }
}

impl Chart {
    #[allow(clippy::too_many_arguments)]
    fn render_cartesian(
        &self,
        memory: &State<ChartMemory>,
        hovered: Option<usize>,
        bounds: Rc<Cell<Bounds<Pixels>>>,
        series_colors: &[Hsla],
        category_count: usize,
        max: f32,
        cx: &App,
    ) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let radius = cx.theme().radius_small();
        let height = self.height;
        let value = |series: &ChartSeries, index: usize| -> f32 {
            series.values.get(index).copied().unwrap_or(0.).max(0.)
        };

        let grid = self.show_grid.then(|| {
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .children((0..=4).map(|step| {
                    div()
                        .absolute()
                        .left_0()
                        .w_full()
                        .h(px(1.))
                        .top(relative(step as f32 / 4.))
                        .bg(colors.border.opacity(0.6))
                }))
        });

        let marks: AnyElement = match self.kind {
            ChartKind::Bar => div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_end()
                .children((0..category_count).map(|index| {
                    let is_dimmed = hovered.is_some_and(|hovered| hovered != index);
                    let column = div()
                        .flex_1()
                        .h_full()
                        .flex()
                        .items_end()
                        .justify_center()
                        .px(px(4.))
                        .when(is_dimmed, |column| column.opacity(0.6));
                    if self.stacked {
                        column.child(
                            div()
                                .flex()
                                .flex_col_reverse()
                                .w(relative(0.7))
                                .max_w(px(48.))
                                .h_full()
                                .children(self.series.iter().zip(series_colors).enumerate().map(
                                    |(series_index, (series, color))| {
                                        let is_top = series_index + 1 == self.series.len();
                                        div()
                                            .w_full()
                                            .h(relative(value(series, index) / max))
                                            .bg(*color)
                                            .when(is_top, |bar| bar.rounded_t(radius))
                                    },
                                )),
                        )
                    } else {
                        column
                            .gap(px(4.))
                            .children(self.series.iter().zip(series_colors).map(
                                |(series, color)| {
                                    div()
                                        .flex_1()
                                        .max_w(px(32.))
                                        .h(relative(value(series, index) / max))
                                        .rounded_t(radius)
                                        .bg(*color)
                                },
                            ))
                    }
                }))
                .into_any_element(),
            ChartKind::Line | ChartKind::Area => {
                let series: Vec<(Vec<f32>, Hsla)> = self
                    .series
                    .iter()
                    .zip(series_colors)
                    .map(|(series, color)| {
                        (
                            (0..category_count)
                                .map(|index| value(series, index))
                                .collect(),
                            *color,
                        )
                    })
                    .collect();
                let filled = self.kind == ChartKind::Area;
                let cursor_color = colors.border;
                canvas(
                    |_, _, _| {},
                    move |plot_bounds, _, window, _| {
                        let count = category_count.max(1) as f32;
                        let to_point = |index: usize, value: f32| {
                            point(
                                plot_bounds.left()
                                    + plot_bounds.size.width * ((index as f32 + 0.5) / count),
                                plot_bounds.bottom() - plot_bounds.size.height * (value / max),
                            )
                        };
                        if let Some(hovered) = hovered {
                            let x = to_point(hovered, 0.).x;
                            let mut cursor = PathBuilder::stroke(px(1.));
                            cursor.move_to(point(x, plot_bounds.top()));
                            cursor.line_to(point(x, plot_bounds.bottom()));
                            if let Ok(path) = cursor.build() {
                                window.paint_path(path, cursor_color);
                            }
                        }
                        for (values, color) in &series {
                            let points: Vec<Point<Pixels>> = values
                                .iter()
                                .enumerate()
                                .map(|(index, value)| to_point(index, *value))
                                .collect();
                            let (Some(first), Some(last)) = (points.first(), points.last()) else {
                                continue;
                            };
                            if filled {
                                let mut area = PathBuilder::fill();
                                area.move_to(point(first.x, plot_bounds.bottom()));
                                area.line_to(*first);
                                add_smooth_curve(&mut area, &points);
                                area.line_to(point(last.x, plot_bounds.bottom()));
                                area.close();
                                if let Ok(path) = area.build() {
                                    window.paint_path(path, color.opacity(0.35));
                                }
                            }
                            let mut line = PathBuilder::stroke(px(2.));
                            line.move_to(*first);
                            add_smooth_curve(&mut line, &points);
                            if let Ok(path) = line.build() {
                                window.paint_path(path, *color);
                            }
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .into_any_element()
            }
            ChartKind::Pie | ChartKind::Donut => div().into_any_element(),
        };

        let tooltip = hovered
            .filter(|index| *index < category_count)
            .map(|index| {
                let label = self.categories.get(index).cloned().unwrap_or_default();
                let fraction = (index as f32 + 0.5) / category_count.max(1) as f32;
                let rows = self
                    .series
                    .iter()
                    .zip(series_colors)
                    .map(|(series, color)| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().size(px(10.)).rounded(px(2.)).bg(*color))
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(colors.muted_foreground)
                                    .child(series.label.clone()),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child((self.value_format)(value(series, index))),
                            )
                    });
                let card = div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .min_w(px(128.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(cx.theme().radius_medium())
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.background)
                    .shadow_lg()
                    .text_xs()
                    .child(div().font_weight(FontWeight::MEDIUM).child(label))
                    .children(rows);
                // Show the card on whichever side of the cursor has room.
                let column = div().absolute().top(px(8.)).w(px(0.)).flex();
                if fraction > 0.6 {
                    column
                        .left(relative(fraction))
                        .justify_end()
                        .child(div().mr(px(8.)).child(card))
                } else {
                    column
                        .left(relative(fraction))
                        .child(div().ml(px(8.)).child(card))
                }
            });

        let move_memory = memory.clone();
        let move_bounds = bounds.clone();
        let leave_memory = memory.clone();
        let plot = div()
            .id("chart-plot")
            .relative()
            .flex_1()
            .h(height)
            .child(measure_bounds(bounds))
            .children(grid)
            .child(marks)
            .children(tooltip)
            .on_mouse_move(move |event, _, cx| {
                let plot_bounds = move_bounds.get();
                if plot_bounds.size.width <= px(0.) || category_count == 0 {
                    return;
                }
                let fraction = (event.position.x - plot_bounds.left()) / plot_bounds.size.width;
                let index = ((fraction * category_count as f32).floor().max(0.) as usize)
                    .min(category_count - 1);
                if move_memory.read(cx).hovered != Some(index) {
                    move_memory.update(cx, |memory| memory.hovered = Some(index));
                }
            })
            .on_hover(move |hovered, _, cx| {
                if !*hovered {
                    leave_memory.update(cx, |memory| memory.hovered = None);
                }
            });

        let y_axis = self.show_y_axis.then(|| {
            div()
                .relative()
                .w(px(40.))
                .h(height)
                .text_xs()
                .text_color(colors.muted_foreground)
                .children((0..=4).map(|step| {
                    let fraction = step as f32 / 4.;
                    div()
                        .absolute()
                        .right(px(8.))
                        .top(relative(fraction))
                        .mt(px(-8.))
                        .child((self.value_format)(max * (1. - fraction)))
                }))
        });

        let x_labels = div()
            .flex()
            .pt(px(8.))
            .when(self.show_y_axis, |labels| labels.pl(px(40.)))
            .text_xs()
            .text_color(colors.muted_foreground)
            .children(self.categories.iter().map(|label| {
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .justify_center()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(label.clone())
            }));

        div()
            .flex()
            .flex_col()
            .w_full()
            .child(div().flex().w_full().children(y_axis).child(plot))
            .child(x_labels)
            .into_any_element()
    }

    fn render_circular(
        &self,
        memory: &State<ChartMemory>,
        hovered: Option<usize>,
        bounds: Rc<Cell<Bounds<Pixels>>>,
        palette: &[Hsla; 5],
        cx: &App,
    ) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let values: Vec<f32> = self
            .series
            .first()
            .map(|series| series.values.iter().map(|value| value.max(0.)).collect())
            .unwrap_or_default();
        let total: f32 = values.iter().sum();
        let is_donut = self.kind == ChartKind::Donut;
        let size = self.height;
        let slice_colors: Vec<Hsla> = (0..values.len()).map(|index| palette[index % 5]).collect();
        let background = colors.background;

        let drawing_values = values.clone();
        let pie = canvas(
            |_, _, _| {},
            move |pie_bounds, _, window, _| {
                if total <= 0. {
                    return;
                }
                let center = pie_bounds.center();
                let outer = pie_bounds.size.width.min(pie_bounds.size.height) / 2.;
                let inner = if is_donut { outer * 0.6 } else { px(0.) };
                let at = |radius: Pixels, angle: f32| {
                    point(
                        center.x + radius * angle.cos(),
                        center.y + radius * angle.sin(),
                    )
                };
                let mut start_angle = -PI / 2.;
                for (index, value) in drawing_values.iter().enumerate() {
                    let sweep = value / total * 2. * PI;
                    if sweep <= 0. {
                        continue;
                    }
                    // Draw in halves so a single full slice still has a defined arc.
                    let pieces = if sweep > PI { 2 } else { 1 };
                    let piece_sweep = sweep / pieces as f32;
                    let grow = if hovered == Some(index) {
                        px(4.)
                    } else {
                        px(0.)
                    };
                    for piece in 0..pieces {
                        let from = start_angle + piece_sweep * piece as f32;
                        let to = from + piece_sweep;
                        let outer_radius = outer - px(4.) + grow;
                        let radii = point(outer_radius, outer_radius);
                        let mut path = PathBuilder::fill();
                        if is_donut {
                            path.move_to(at(outer_radius, from));
                            path.arc_to(radii, px(0.), false, true, at(outer_radius, to));
                            path.line_to(at(inner, to));
                            path.arc_to(point(inner, inner), px(0.), false, false, at(inner, from));
                        } else {
                            path.move_to(center);
                            path.line_to(at(outer_radius, from));
                            path.arc_to(radii, px(0.), false, true, at(outer_radius, to));
                        }
                        path.close();
                        if let Ok(path) = path.build() {
                            window.paint_path(path, slice_colors[index]);
                        }
                    }
                    // A thin gap between slices in the background color.
                    let mut gap = PathBuilder::stroke(px(2.));
                    gap.move_to(at(inner, start_angle));
                    gap.line_to(at(outer + px(4.), start_angle));
                    if let Ok(path) = gap.build() {
                        window.paint_path(path, background);
                    }
                    start_angle += sweep;
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let center_label =
            (is_donut)
                .then(|| self.center_label.clone())
                .flatten()
                .map(|(value, caption)| {
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(28.))
                                .font_weight(FontWeight::BOLD)
                                .child(value),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(colors.muted_foreground)
                                .child(caption),
                        )
                });

        let tooltip = hovered.filter(|index| *index < values.len()).map(|index| {
            let label = self.categories.get(index).cloned().unwrap_or_default();
            div()
                .absolute()
                .top(px(8.))
                .right(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(10.))
                .py(px(6.))
                .rounded(cx.theme().radius_medium())
                .border_1()
                .border_color(colors.border)
                .bg(colors.background)
                .shadow_lg()
                .text_xs()
                .child(div().size(px(10.)).rounded(px(2.)).bg(palette[index % 5]))
                .child(div().text_color(colors.muted_foreground).child(label))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .child((self.value_format)(values[index])),
                )
        });

        let move_memory = memory.clone();
        let move_bounds = bounds.clone();
        let leave_memory = memory.clone();
        let slice_values = values.clone();
        div()
            .id("chart-pie")
            .relative()
            .mx_auto()
            .size(size)
            .child(measure_bounds(bounds))
            .child(pie)
            .children(center_label)
            .children(tooltip)
            .on_mouse_move(move |event, _, cx| {
                let pie_bounds = move_bounds.get();
                let center = pie_bounds.center();
                let offset = event.position - center;
                let (dx, dy) = (f32::from(offset.x), f32::from(offset.y));
                let distance = (dx * dx + dy * dy).sqrt();
                let outer = f32::from(pie_bounds.size.width.min(pie_bounds.size.height)) / 2.;
                let inner = if is_donut { outer * 0.6 } else { 0. };
                let index = if distance > outer || distance < inner || total <= 0. {
                    None
                } else {
                    // Angle clockwise from 12 o'clock, 0..2π.
                    let angle = (dy.atan2(dx) + PI / 2.).rem_euclid(2. * PI);
                    let mut cumulative = 0.;
                    slice_values.iter().position(|value| {
                        cumulative += value / total * 2. * PI;
                        angle < cumulative
                    })
                };
                if move_memory.read(cx).hovered != index {
                    move_memory.update(cx, |memory| memory.hovered = index);
                }
            })
            .on_hover(move |hovered, _, cx| {
                if !*hovered {
                    leave_memory.update(cx, |memory| memory.hovered = None);
                }
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::nice_ceiling;

    #[test]
    fn nice_ceiling_rounds_up_to_one_two_or_five() {
        assert_eq!(nice_ceiling(0.), 1.);
        assert_eq!(nice_ceiling(7.), 10.);
        assert_eq!(nice_ceiling(186.), 200.);
        assert_eq!(nice_ceiling(305.), 500.);
        assert_eq!(nice_ceiling(1000.), 1000.);
        assert_eq!(nice_ceiling(1200.), 2000.);
    }
}
