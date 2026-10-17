//! Calendar: a month grid for picking a date or a range, and the small date
//! type it works with.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ElementId, SharedString, StyleRefinement, Window};

use super::{button::Button, overlay::child_id};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::IconName,
    styles,
    styles::ApplyStyleOverrides,
};

mod date;

pub use date::{days_in_month, is_leap_year, month_grid, month_name, CalendarDate, DateRange};

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
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let due_date = use_state(window, cx, || None::<CalendarDate>);
/// let trip = use_state(window, cx, || None::<DateRange>);
/// let single = Calendar::new("due-date")
///     .selected(due_date.get(cx))
///     .on_select(move |date, _, cx| due_date.set(Some(*date), cx));
///
/// let range = Calendar::new("trip")
///     .number_of_months(2)
///     .range(trip.get(cx))
///     .on_range_select(move |range, _, cx| trip.set(Some(*range), cx));
/// # }
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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Calendar);

impl Calendar {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
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
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The selected day (single mode).
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn number_of_months(mut self, count: usize) -> Self {
        self.number_of_months = count.max(1);
        self
    }

    /// Start weeks on Monday instead of Sunday.
    #[must_use]
    pub fn week_starts_on_monday(mut self, monday: bool) -> Self {
        self.week_starts_on_monday = monday;
        self
    }

    /// Days before `min` cannot be picked.
    #[must_use]
    pub fn min(mut self, min: CalendarDate) -> Self {
        self.min = Some(min);
        self
    }

    /// Days after `max` cannot be picked.
    #[must_use]
    pub fn max(mut self, max: CalendarDate) -> Self {
        self.max = Some(max);
        self
    }

    /// Days for which `is_disabled` returns true cannot be picked.
    #[must_use]
    pub fn disabled_dates(mut self, is_disabled: impl Fn(&CalendarDate) -> bool + 'static) -> Self {
        self.disabled_dates = Some(Rc::new(is_disabled));
        self
    }

    /// Override "today" (highlighting and the initial month), mainly for tests.
    #[must_use]
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

styles! {
    CALENDAR = {
        root: { display: flex, gap: 4, padding: 3 },
        month: { display: flex, direction: column, gap: 2 },
        header: {
            position: relative,
            display: flex,
            align: center,
            justify: center,
            height: 8,
        },
        caption: { text: sm, font: medium },
        previous_slot: { position: absolute, inset_start: 0 },
        next_slot: { position: absolute, inset_end: 0 },
        nav_button: { width: 8, height: 8 },
        weekdays: { display: flex },
        weekday: {
            width: 8,
            display: flex,
            justify: center,
            text: xs,
            color: muted_foreground,
        },
        weeks: { display: flex, direction: column, gap: 0.5 },
        week: { display: flex },
        day: {
            size: 8,
            display: flex,
            align: center,
            justify: center,
            radius: md,
            text: sm,
        },
        selected: { background: primary, color: primary_foreground },
        range_middle: { radius: none, background: accent, color: accent_foreground },
        today: { background: accent, color: accent_foreground },
        outside: { color: muted_foreground },
        disabled: { opacity: 0.5, line_through: true },
        interactive: {
            cursor: pointer,
            border: 1,
            border_color: transparent,
            focus: { border_color: ring },
        },
        hoverable: { hover: { background: accent } },
    }
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let anchor = self.anchor_date().first_of_month();
        let visible_month = use_keyed_state(child_id(&self.id, "month"), window, cx, || anchor);
        let first_month = visible_month.get(cx);
        let weekday_labels: Vec<&str> = if self.week_starts_on_monday {
            vec!["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        } else {
            vec!["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
        };

        let months = (0..self.number_of_months).map(|month_offset| {
            let month = first_month.add_months(month_offset as i32);
            let days = month_grid(month.year, month.month, self.week_starts_on_monday);
            let is_first = month_offset == 0;
            let is_last = month_offset + 1 == self.number_of_months;

            let previous_state = visible_month.clone();
            let next_state = visible_month.clone();
            let header = div()
                .sx(&CALENDAR.header)
                .child(div().sx(&CALENDAR.caption).child(format!(
                    "{} {}",
                    month_name(month.month),
                    month.year
                )))
                .when(is_first, |header| {
                    header.child(
                        div().sx(&CALENDAR.previous_slot).child(
                            Button::new("calendar-previous")
                                .ghost()
                                .icon_only(IconName::ChevronLeft.for_direction())
                                .sx(&CALENDAR.nav_button)
                                .tooltip("Previous month")
                                .on_click(move |_, _, cx| {
                                    previous_state
                                        .update(cx, |month| *month = month.add_months(-1));
                                }),
                        ),
                    )
                })
                .when(is_last, |header| {
                    header.child(
                        div().sx(&CALENDAR.next_slot).child(
                            Button::new("calendar-next")
                                .ghost()
                                .icon_only(IconName::ChevronRight.for_direction())
                                .sx(&CALENDAR.nav_button)
                                .tooltip("Next month")
                                .on_click(move |_, _, cx| {
                                    next_state.update(cx, |month| *month = month.add_months(1));
                                }),
                        ),
                    )
                });

            let weekday_row = div().sx(&CALENDAR.weekdays).children(
                weekday_labels
                    .iter()
                    .map(|label| div().sx(&CALENDAR.weekday).child(*label)),
            );

            let weeks = days.chunks(7).map(|week| {
                div().sx(&CALENDAR.week).children(week.iter().map(|date| {
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
                    let state = if is_selected {
                        Some(&CALENDAR.selected)
                    } else if is_range_middle {
                        Some(&CALENDAR.range_middle)
                    } else if is_today {
                        Some(&CALENDAR.today)
                    } else if is_outside {
                        Some(&CALENDAR.outside)
                    } else {
                        None
                    };
                    let selection = self.selection.clone();
                    let day = div()
                        .id(("calendar-day", date.days_since_epoch() as u64))
                        .sx((
                            &CALENDAR.day,
                            state,
                            if is_disabled {
                                &CALENDAR.disabled
                            } else {
                                &CALENDAR.interactive
                            },
                            (!is_disabled && !is_selected).then_some(&CALENDAR.hoverable),
                        ))
                        .child(SharedString::from(date.day.to_string()));
                    if is_disabled {
                        day
                    } else {
                        day.tab_index(0)
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
                .sx(&CALENDAR.month)
                .child(header)
                .child(weekday_row)
                .child(div().sx(&CALENDAR.weeks).children(weeks))
        });
        let months: Vec<AnyElement> = months.map(IntoElement::into_any_element).collect();

        div()
            .id(self.id)
            .sx((&CALENDAR.root, &self.sx))
            .children(months)
            .apply_style_overrides(&self.style_overrides)
    }
}
