//! Calendar: a month grid for picking a date or a range, and the small date
//! type it works with.

use std::{fmt, rc::Rc};

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, FontWeight, SharedString,
    StyleRefinement, Window,
};

use super::{button::Button, overlay::child_id};
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::IconName,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A calendar day (proleptic Gregorian), without time or time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    pub year: i32,
    /// 1 to 12.
    pub month: u32,
    /// 1 to the length of the month.
    pub day: u32,
}

impl CalendarDate {
    /// `None` when the day does not exist (`2026-02-30`).
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        ((1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month))
            .then_some(Self { year, month, day })
    }

    /// Today's date in UTC (rok-ui has no time-zone database).
    pub fn today() -> Self {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);
        Self::from_days_since_epoch(seconds.div_euclid(86_400))
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub fn days_since_epoch(self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(self.month);
        let day_of_year =
            (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
    pub fn from_days_since_epoch(days: i64) -> Self {
        let days = days + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;
        let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
        let month = if month_index < 10 {
            month_index + 3
        } else {
            month_index - 9
        } as u32;
        let year = (year_of_era + era * 400 + i64::from(month <= 2)) as i32;
        Self { year, month, day }
    }

    /// 0 = Sunday … 6 = Saturday.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday.
        (self.days_since_epoch() + 4).rem_euclid(7) as u32
    }

    pub fn add_days(self, days: i64) -> Self {
        Self::from_days_since_epoch(self.days_since_epoch() + days)
    }

    /// Same day `months` later, clamped to the end of shorter months.
    pub fn add_months(self, months: i32) -> Self {
        let month_index = self.year * 12 + self.month as i32 - 1 + months;
        let year = month_index.div_euclid(12);
        let month = month_index.rem_euclid(12) as u32 + 1;
        Self {
            year,
            month,
            day: self.day.min(days_in_month(year, month)),
        }
    }

    /// The first of this date's month.
    pub fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    /// "October 2, 2026".
    pub fn format_long(self) -> String {
        format!("{} {}, {}", month_name(self.month), self.day, self.year)
    }

    /// "Oct 2, 2026".
    pub fn format_short(self) -> String {
        format!(
            "{} {}, {}",
            &month_name(self.month)[..3],
            self.day,
            self.year
        )
    }
}

/// ISO 8601: `2026-10-02`.
impl fmt::Display for CalendarDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// English month name for `1..=12`.
pub fn month_name(month: u32) -> &'static str {
    MONTH_NAMES[(month.clamp(1, 12) - 1) as usize]
}

/// A selected range. `end` is `None` while only the first day has been picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateRange {
    pub start: CalendarDate,
    pub end: Option<CalendarDate>,
}

impl DateRange {
    pub fn contains(&self, date: CalendarDate) -> bool {
        match self.end {
            Some(end) => self.start <= date && date <= end,
            None => self.start == date,
        }
    }

    /// The range after the user clicks `date`: a click starts a new range unless
    /// the current one is waiting for its end; ends before the start swap in.
    pub fn after_click(current: Option<DateRange>, date: CalendarDate) -> DateRange {
        match current {
            Some(DateRange { start, end: None }) if date >= start => DateRange {
                start,
                end: Some(date),
            },
            Some(DateRange { start, end: None }) => DateRange {
                start: date,
                end: Some(start),
            },
            _ => DateRange {
                start: date,
                end: None,
            },
        }
    }

    /// "Oct 2, 2026 – Oct 9, 2026".
    pub fn format_short(&self) -> String {
        match self.end {
            Some(end) => format!("{} – {}", self.start.format_short(), end.format_short()),
            None => self.start.format_short(),
        }
    }
}

/// The 6 × 7 grid of days shown for a month, starting on Sunday or Monday.
pub fn month_grid(year: i32, month: u32, week_starts_on_monday: bool) -> Vec<CalendarDate> {
    let first = CalendarDate {
        year,
        month,
        day: 1,
    };
    let week_start = u32::from(week_starts_on_monday);
    let leading_days = (first.weekday() + 7 - week_start) % 7;
    let grid_start = first.add_days(-i64::from(leading_days));
    (0..42).map(|offset| grid_start.add_days(offset)).collect()
}

#[derive(Clone)]
enum CalendarSelection {
    Single {
        selected: Option<CalendarDate>,
        on_select: Option<EventHandler<CalendarDate>>,
    },
    Range {
        range: Option<DateRange>,
        on_select: Option<EventHandler<DateRange>>,
    },
}

type DateFilter = Rc<dyn Fn(&CalendarDate) -> bool>;

/// Controlled: pass the selection, update it in the handler. The visible month
/// is remembered per id and starts at the selection (or today).
///
/// ```ignore
/// Calendar::new("due-date")
///     .selected(due_date.get(cx))
///     .on_select(move |date, _, cx| due_date.set(Some(*date), cx))
///
/// Calendar::new("trip").number_of_months(2)
///     .range(trip.get(cx))
///     .on_range_select(move |range, _, cx| trip.set(Some(*range), cx))
/// ```
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    selection: CalendarSelection,
    number_of_months: usize,
    week_starts_on_monday: bool,
    min: Option<CalendarDate>,
    max: Option<CalendarDate>,
    disabled_dates: Option<DateFilter>,
    today: CalendarDate,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Calendar);

impl Calendar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selection: CalendarSelection::Single {
                selected: None,
                on_select: None,
            },
            number_of_months: 1,
            week_starts_on_monday: false,
            min: None,
            max: None,
            disabled_dates: None,
            today: CalendarDate::today(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The selected day (single mode).
    pub fn selected(mut self, selected: Option<CalendarDate>) -> Self {
        let on_select = match self.selection {
            CalendarSelection::Single { on_select, .. } => on_select,
            CalendarSelection::Range { .. } => None,
        };
        self.selection = CalendarSelection::Single {
            selected,
            on_select,
        };
        self
    }

    /// Receives the clicked day (single mode).
    pub fn on_select(
        mut self,
        handler: impl Fn(&CalendarDate, &mut Window, &mut App) + 'static,
    ) -> Self {
        let selected = match self.selection {
            CalendarSelection::Single { selected, .. } => selected,
            CalendarSelection::Range { .. } => None,
        };
        self.selection = CalendarSelection::Single {
            selected,
            on_select: Some(Rc::new(handler)),
        };
        self
    }

    /// The selected range (range mode).
    pub fn range(mut self, range: Option<DateRange>) -> Self {
        let on_select = match self.selection {
            CalendarSelection::Range { on_select, .. } => on_select,
            CalendarSelection::Single { .. } => None,
        };
        self.selection = CalendarSelection::Range { range, on_select };
        self
    }

    /// Receives the range after each click (range mode). A first click gives a
    /// range with no `end`; the second completes it.
    pub fn on_range_select(
        mut self,
        handler: impl Fn(&DateRange, &mut Window, &mut App) + 'static,
    ) -> Self {
        let range = match self.selection {
            CalendarSelection::Range { range, .. } => range,
            CalendarSelection::Single { .. } => None,
        };
        self.selection = CalendarSelection::Range {
            range,
            on_select: Some(Rc::new(handler)),
        };
        self
    }

    /// Show several months side by side.
    pub fn number_of_months(mut self, count: usize) -> Self {
        self.number_of_months = count.max(1);
        self
    }

    pub fn week_starts_on_monday(mut self, monday: bool) -> Self {
        self.week_starts_on_monday = monday;
        self
    }

    /// Days before `min` cannot be picked.
    pub fn min(mut self, min: CalendarDate) -> Self {
        self.min = Some(min);
        self
    }

    /// Days after `max` cannot be picked.
    pub fn max(mut self, max: CalendarDate) -> Self {
        self.max = Some(max);
        self
    }

    /// Days for which `is_disabled` returns true cannot be picked.
    pub fn disabled_dates(mut self, is_disabled: impl Fn(&CalendarDate) -> bool + 'static) -> Self {
        self.disabled_dates = Some(Rc::new(is_disabled));
        self
    }

    /// Override "today" (highlighting and the initial month), mainly for tests.
    pub fn today(mut self, today: CalendarDate) -> Self {
        self.today = today;
        self
    }

    fn is_disabled(&self, date: CalendarDate) -> bool {
        self.min.is_some_and(|min| date < min)
            || self.max.is_some_and(|max| date > max)
            || self
                .disabled_dates
                .as_ref()
                .is_some_and(|is_disabled| is_disabled(&date))
    }

    fn anchor_date(&self) -> CalendarDate {
        match &self.selection {
            CalendarSelection::Single {
                selected: Some(date),
                ..
            } => *date,
            CalendarSelection::Range {
                range: Some(range), ..
            } => range.start,
            _ => self.today,
        }
    }
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let anchor = self.anchor_date().first_of_month();
        let visible_month = use_keyed_state(child_id(&self.id, "month"), window, cx, || anchor);
        let first_month = visible_month.get(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let cell_radius = theme.radius_medium();
        let ring_color = colors.ring;
        let weekday_labels: Vec<&str> = if self.week_starts_on_monday {
            vec!["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        } else {
            vec!["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
        };
        let cell = px(32.);

        let months = (0..self.number_of_months).map(|month_offset| {
            let month = first_month.add_months(month_offset as i32);
            let days = month_grid(month.year, month.month, self.week_starts_on_monday);
            let is_first = month_offset == 0;
            let is_last = month_offset + 1 == self.number_of_months;

            let previous_state = visible_month.clone();
            let next_state = visible_month.clone();
            let header = div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .h(cell)
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child(format!("{} {}", month_name(month.month), month.year)),
                )
                .when(is_first, |header| {
                    header.child(
                        div().absolute().left_0().child(
                            Button::new("calendar-previous")
                                .ghost()
                                .icon_only(IconName::ChevronLeft)
                                .w(cell)
                                .h(cell)
                                .tooltip("Previous month")
                                .on_click(move |_, _, cx| {
                                    previous_state.update(cx, |month| *month = month.add_months(-1))
                                }),
                        ),
                    )
                })
                .when(is_last, |header| {
                    header.child(
                        div().absolute().right_0().child(
                            Button::new("calendar-next")
                                .ghost()
                                .icon_only(IconName::ChevronRight)
                                .w(cell)
                                .h(cell)
                                .tooltip("Next month")
                                .on_click(move |_, _, cx| {
                                    next_state.update(cx, |month| *month = month.add_months(1))
                                }),
                        ),
                    )
                });

            let weekday_row = div().flex().children(weekday_labels.iter().map(|label| {
                div()
                    .w(cell)
                    .flex()
                    .justify_center()
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child(*label)
            }));

            let weeks = days.chunks(7).map(|week| {
                div().flex().children(week.iter().map(|date| {
                    let date = *date;
                    let is_outside = date.month != month.month;
                    let is_disabled = self.is_disabled(date);
                    let is_today = date == self.today;
                    let (is_selected, is_range_middle) = match &self.selection {
                        CalendarSelection::Single { selected, .. } => {
                            (*selected == Some(date), false)
                        }
                        CalendarSelection::Range { range, .. } => match range {
                            Some(range) => {
                                let is_end = date == range.start || Some(date) == range.end;
                                (is_end, range.contains(date) && !is_end)
                            }
                            None => (false, false),
                        },
                    };
                    let selection = self.selection.clone();
                    let mut day = div()
                        .id(("calendar-day", date.days_since_epoch() as u64))
                        .size(cell)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(cell_radius)
                        .text_sm()
                        .child(SharedString::from(date.day.to_string()));
                    day = if is_selected {
                        day.bg(colors.primary).text_color(colors.primary_foreground)
                    } else if is_range_middle {
                        day.rounded_none()
                            .bg(colors.accent)
                            .text_color(colors.accent_foreground)
                    } else if is_today {
                        day.bg(colors.accent).text_color(colors.accent_foreground)
                    } else if is_outside {
                        day.text_color(colors.muted_foreground)
                    } else {
                        day
                    };
                    if is_disabled {
                        day.opacity(0.5).line_through()
                    } else {
                        day.cursor(CursorStyle::PointingHand)
                            .tab_index(0)
                            .border_1()
                            .border_color(gpui::transparent_black())
                            .focus(move |style| style.border_color(ring_color))
                            .when(!is_selected, |day| {
                                day.hover(|style| style.bg(colors.accent))
                            })
                            .on_click(move |_, window, cx| match &selection {
                                CalendarSelection::Single {
                                    on_select: Some(handler),
                                    ..
                                } => handler(&date, window, cx),
                                CalendarSelection::Range {
                                    range,
                                    on_select: Some(handler),
                                } => handler(&DateRange::after_click(*range, date), window, cx),
                                _ => {}
                            })
                    }
                    .into_any_element()
                }))
            });

            div()
                .id(("calendar-month", month_offset))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(header)
                .child(weekday_row)
                .child(div().flex().flex_col().gap(px(2.)).children(weeks))
        });
        let months: Vec<AnyElement> = months.map(IntoElement::into_any_element).collect();

        div()
            .id(self.id)
            .flex()
            .gap(px(16.))
            .p(px(12.))
            .children(months)
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> CalendarDate {
        CalendarDate::new(year, month, day).unwrap()
    }

    #[test]
    fn days_round_trip_and_weekdays() {
        assert_eq!(date(1970, 1, 1).days_since_epoch(), 0);
        assert_eq!(date(2000, 3, 1).days_since_epoch(), 11_017);
        for days in [-800_000, -1, 0, 59, 11_016, 20_000, 2_000_000] {
            assert_eq!(
                CalendarDate::from_days_since_epoch(days).days_since_epoch(),
                days
            );
        }
        assert_eq!(date(1970, 1, 1).weekday(), 4); // Thursday
        assert_eq!(date(2026, 10, 2).weekday(), 5); // Friday
        assert_eq!(date(2024, 2, 29).weekday(), 4); // Thursday
    }

    #[test]
    fn month_lengths_and_validation() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert!(CalendarDate::new(2026, 2, 29).is_none());
        assert!(CalendarDate::new(2026, 13, 1).is_none());
    }

    #[test]
    fn add_months_clamps_the_day() {
        assert_eq!(date(2026, 1, 31).add_months(1), date(2026, 2, 28));
        assert_eq!(date(2026, 12, 15).add_months(1), date(2027, 1, 15));
        assert_eq!(date(2026, 1, 15).add_months(-1), date(2025, 12, 15));
    }

    #[test]
    fn month_grid_starts_on_the_right_weekday() {
        // October 2026 starts on a Thursday.
        let sunday_grid = month_grid(2026, 10, false);
        assert_eq!(sunday_grid.len(), 42);
        assert_eq!(sunday_grid[0], date(2026, 9, 27));
        assert_eq!(sunday_grid[4], date(2026, 10, 1));
        let monday_grid = month_grid(2026, 10, true);
        assert_eq!(monday_grid[0], date(2026, 9, 28));
    }

    #[test]
    fn range_clicks_start_extend_and_restart() {
        let first = DateRange::after_click(None, date(2026, 10, 5));
        assert_eq!(first.end, None);
        let completed = DateRange::after_click(Some(first), date(2026, 10, 9));
        assert_eq!(completed.end, Some(date(2026, 10, 9)));
        assert!(completed.contains(date(2026, 10, 7)));
        let backwards = DateRange::after_click(Some(first), date(2026, 10, 1));
        assert_eq!(
            (backwards.start, backwards.end),
            (date(2026, 10, 1), Some(date(2026, 10, 5)))
        );
        let restarted = DateRange::after_click(Some(completed), date(2026, 11, 1));
        assert_eq!((restarted.start, restarted.end), (date(2026, 11, 1), None));
    }

    #[test]
    fn formats() {
        assert_eq!(date(2026, 10, 2).to_string(), "2026-10-02");
        assert_eq!(date(2026, 10, 2).format_long(), "October 2, 2026");
        assert_eq!(date(2026, 10, 2).format_short(), "Oct 2, 2026");
    }
}
