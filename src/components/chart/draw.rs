//! Drawing a chart: axes, bars, lines and areas, and the pie, with their hover state.

use std::{cell::Cell, f32::consts::PI, rc::Rc};

use gpui::{
    canvas, div, point, prelude::*, px, relative, AnyElement, App, Bounds, Hsla, PathBuilder,
    Pixels, Point,
};

use super::{Chart, ChartKind, ChartMemory, ChartSeries, CHART};
use crate::components::interaction::measure_bounds;
use crate::sx::SxStyled;
use crate::{hooks::State, theme::ActiveTheme};

pub(super) fn format_number(value: f32) -> String {
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

impl Chart {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_cartesian(
        &self,
        memory: &State<ChartMemory>,
        hovered: Option<usize>,
        bounds: Rc<Cell<Bounds<Pixels>>>,
        series_colors: &[Hsla],
        category_count: usize,
        max: f32,
        cx: &App,
    ) -> AnyElement {
        let height = self.height;
        let value = |series: &ChartSeries, index: usize| -> f32 {
            series.values.get(index).copied().unwrap_or(0.).max(0.)
        };

        let grid = self.show_grid.then(|| {
            div().sx(&CHART.fill).children(
                (0..=4).map(|step| div().sx(&CHART.grid_line).top(relative(step as f32 / 4.))),
            )
        });

        let marks: AnyElement = match self.kind {
            ChartKind::Bar => div()
                .sx((&CHART.ltr, &CHART.fill, &CHART.bars))
                .children((0..category_count).map(|index| {
                    let is_dimmed = hovered.is_some_and(|hovered| hovered != index);
                    let column = div().sx((
                        &CHART.ltr,
                        &CHART.bar_column,
                        is_dimmed.then_some(&CHART.dimmed),
                        (!self.stacked).then_some(&CHART.grouped),
                    ));
                    if self.stacked {
                        column.child(div().sx((&CHART.ltr, &CHART.stack)).children(
                            self.series.iter().zip(series_colors).enumerate().map(
                                |(series_index, (series, color))| {
                                    let is_top = series_index + 1 == self.series.len();
                                    div()
                                        .sx((
                                            &CHART.stack_segment,
                                            is_top.then_some(&CHART.bar_top),
                                        ))
                                        .h(relative(value(series, index) / max))
                                        .bg(*color)
                                },
                            ),
                        ))
                    } else {
                        column.children(self.series.iter().zip(series_colors).map(
                            |(series, color)| {
                                div()
                                    .sx((&CHART.bar, &CHART.bar_top))
                                    .h(relative(value(series, index) / max))
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
                let cursor_color = cx.theme().colors.border;
                canvas(
                    |_, _, _| {},
                    move |plot_bounds, (), window, _| {
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
                            .sx((&CHART.ltr, &CHART.tooltip_row))
                            .child(div().sx(&CHART.tooltip_swatch).bg(*color))
                            .child(div().sx(&CHART.tooltip_label).child(series.label.clone()))
                            .child(
                                div()
                                    .sx(&CHART.tooltip_value)
                                    .child((self.value_format)(value(series, index))),
                            )
                    });
                let card = div()
                    .sx((&CHART.ltr, &CHART.tooltip, &CHART.tooltip_card))
                    .child(
                        div()
                            .sx(&CHART.tooltip_value)
                            .child(crate::components::bidi_text::text(label)),
                    )
                    .children(rows);
                // Show the card on whichever side of the cursor has room.
                let column = div()
                    .sx((&CHART.ltr, &CHART.tooltip_anchor))
                    .left(relative(fraction));
                if fraction > 0.6 {
                    column
                        .sx(&CHART.tooltip_anchor_end)
                        .child(div().sx(&CHART.tooltip_before).child(card))
                } else {
                    column.child(div().sx(&CHART.tooltip_after).child(card))
                }
            });

        let move_memory = memory.clone();
        let move_bounds = bounds.clone();
        let leave_memory = memory.clone();
        let plot = div()
            .id("chart-plot")
            .sx(&CHART.plot)
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
                .sx(&CHART.y_axis)
                .h(height)
                .children((0..=4).map(|step| {
                    let fraction = step as f32 / 4.;
                    div()
                        .sx(&CHART.y_tick)
                        .top(relative(fraction))
                        .child((self.value_format)(max * (1. - fraction)))
                }))
        });

        let x_labels = div()
            .sx((
                &CHART.ltr,
                &CHART.x_labels,
                self.show_y_axis.then_some(&CHART.x_labels_offset),
            ))
            .children(self.categories.iter().map(|label| {
                div()
                    .sx((&CHART.ltr, &CHART.x_label))
                    .child(crate::components::bidi_text::text(label.clone()))
            }));

        div()
            .sx((&CHART.ltr, &CHART.column))
            .child(
                div()
                    .sx((&CHART.ltr, &CHART.full_width))
                    .children(y_axis)
                    .child(plot),
            )
            .child(x_labels)
            .into_any_element()
    }

    pub(super) fn render_circular(
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
            move |pie_bounds, (), window, _| {
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
                        .sx((&CHART.ltr, &CHART.fill, &CHART.center_label))
                        .child(
                            div()
                                .sx(&CHART.center_value)
                                .child(crate::components::bidi_text::text(value)),
                        )
                        .child(
                            div()
                                .sx(&CHART.center_caption)
                                .child(crate::components::bidi_text::text(caption)),
                        )
                });

        let tooltip = hovered.filter(|index| *index < values.len()).map(|index| {
            let label = self.categories.get(index).cloned().unwrap_or_default();
            div()
                .sx((&CHART.ltr, &CHART.tooltip, &CHART.pie_tooltip))
                .child(div().sx(&CHART.tooltip_swatch).bg(palette[index % 5]))
                .child(
                    div()
                        .sx(&CHART.pie_label)
                        .child(crate::components::bidi_text::text(label)),
                )
                .child(
                    div()
                        .sx(&CHART.tooltip_value)
                        .child((self.value_format)(values[index])),
                )
        });

        let move_memory = memory.clone();
        let move_bounds = bounds.clone();
        let leave_memory = memory.clone();
        let slice_values = values.clone();
        div()
            .id("chart-pie")
            .sx(&CHART.pie)
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
