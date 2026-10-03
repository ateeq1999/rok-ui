//! `DatePicker`: a button that opens a [`Calendar`] in a popover.

use std::rc::Rc;

use gpui::{div, prelude::*, px, App, ElementId, SharedString, StyleRefinement, Window};

use super::{
    calendar::{Calendar, CalendarDate, DateRange},
    overlay::{
        dismissable, floating, popover_surface, trigger_wrapper, use_open_state, Align, Side,
    },
};
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

#[derive(Clone)]
enum PickerValue {
    Single {
        date: Option<CalendarDate>,
        on_change: Option<EventHandler<CalendarDate>>,
    },
    Range {
        range: Option<DateRange>,
        on_change: Option<EventHandler<DateRange>>,
    },
}

/// Controlled like [`Calendar`]. Single mode closes after a pick; range mode
/// closes once both ends are chosen. `.preset(..)` adds quick picks beside the calendar.
///
/// ```ignore
/// DatePicker::new("due")
///     .date(due.get(cx))
///     .on_change(move |date, _, cx| due.set(Some(*date), cx))
///     .preset("Today", CalendarDate::today())
///     .preset("In a week", CalendarDate::today().add_days(7))
///
/// DatePicker::new("trip").range(trip.get(cx)).number_of_months(2)
///     .on_range_change(move |range, _, cx| trip.set(Some(*range), cx))
/// ```
#[derive(IntoElement)]
pub struct DatePicker {
    id: ElementId,
    value: PickerValue,
    placeholder: SharedString,
    presets: Vec<(SharedString, CalendarDate)>,
    number_of_months: usize,
    disabled: bool,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(DatePicker);

impl DatePicker {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: PickerValue::Single {
                date: None,
                on_change: None,
            },
            placeholder: "Pick a date".into(),
            presets: Vec::new(),
            number_of_months: 1,
            disabled: false,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The picked date (single mode).
    #[must_use]
    pub fn date(mut self, date: Option<CalendarDate>) -> Self {
        let on_change = match self.value {
            PickerValue::Single { on_change, .. } => on_change,
            PickerValue::Range { .. } => None,
        };
        self.value = PickerValue::Single { date, on_change };
        self
    }

    /// Receives the picked date (single mode).
    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&CalendarDate, &mut Window, &mut App) + 'static,
    ) -> Self {
        let date = match self.value {
            PickerValue::Single { date, .. } => date,
            PickerValue::Range { .. } => None,
        };
        self.value = PickerValue::Single {
            date,
            on_change: Some(Rc::new(handler)),
        };
        self
    }

    /// The picked range (range mode).
    #[must_use]
    pub fn range(mut self, range: Option<DateRange>) -> Self {
        let on_change = match self.value {
            PickerValue::Range { on_change, .. } => on_change,
            PickerValue::Single { .. } => None,
        };
        self.value = PickerValue::Range { range, on_change };
        if self.placeholder.as_ref() == "Pick a date" {
            self.placeholder = "Pick a date range".into();
        }
        self
    }

    /// Receives the range after each click (range mode).
    #[must_use]
    pub fn on_range_change(
        mut self,
        handler: impl Fn(&DateRange, &mut Window, &mut App) + 'static,
    ) -> Self {
        let range = match self.value {
            PickerValue::Range { range, .. } => range,
            PickerValue::Single { .. } => None,
        };
        self.value = PickerValue::Range {
            range,
            on_change: Some(Rc::new(handler)),
        };
        self
    }

    /// Text shown while nothing is entered or selected.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// A quick pick listed beside the calendar (single mode).
    #[must_use]
    pub fn preset(mut self, label: impl Into<SharedString>, date: CalendarDate) -> Self {
        self.presets.push((label.into(), date));
        self
    }

    /// How many months the popover shows side by side.
    #[must_use]
    pub fn number_of_months(mut self, count: usize) -> Self {
        self.number_of_months = count;
        self
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

styles! {
    DATE_PICKER = {
        trigger: {
            display: flex,
            align: center,
            gap: 2,
            height: 9,
            width: 60,
            padding_x: 3,
            radius: md,
            border: 1,
            border_color: input,
            background: background,
            text: sm,
            font: medium,
            shadow: xs,
        },
        interactive: {
            cursor: pointer,
            hover: { background: accent },
            focus: { border_color: ring, shadow: ring },
        },
        inert: { opacity: 0.5 },
        value: { truncate: true },
        placeholder: { truncate: true, color: muted_foreground },
        presets: {
            display: flex,
            direction: column,
            gap: 0.5,
            padding: 2,
            border_end: 1,
            border_color: border,
        },
    }
}

impl RenderOnce for DatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state = use_open_state(&self.id, None, None, window, cx);
        let is_open = open_state.is_open(cx);
        let muted_foreground = cx.theme().colors.muted_foreground;

        let label: Option<String> = match &self.value {
            PickerValue::Single { date, .. } => date.map(CalendarDate::format_long),
            PickerValue::Range { range, .. } => range.map(|range| range.format_short()),
        };

        let trigger = div()
            .id("date-picker-trigger")
            // The caller's `sx` is merged into the same call: GPUI allows a single
            // hover / focus style per element.
            .sx((
                &DATE_PICKER.trigger,
                if self.disabled {
                    &DATE_PICKER.inert
                } else {
                    &DATE_PICKER.interactive
                },
                &self.sx,
            ))
            .when(!self.disabled, |trigger| trigger.tab_index(0))
            .child(
                Icon::new(IconName::Calendar)
                    .size(px(16.))
                    .color(muted_foreground),
            )
            .child(match label {
                Some(label) => div()
                    .sx(&DATE_PICKER.value)
                    .child(crate::components::bidi_text::text(label)),
                None => div()
                    .sx(&DATE_PICKER.placeholder)
                    .child(crate::components::bidi_text::text(self.placeholder)),
            })
            .apply_style_overrides(&self.style_overrides);

        let wrapper =
            trigger_wrapper(self.id.clone(), &open_state, self.disabled, cx).child(trigger);
        if !is_open {
            return wrapper;
        }

        let calendar_id = super::overlay::child_id(&self.id, "calendar");
        let calendar = match self.value.clone() {
            PickerValue::Single { date, on_change } => {
                let close_state = open_state.clone();
                Calendar::new(calendar_id)
                    .number_of_months(self.number_of_months)
                    .selected(date)
                    .on_select(move |date, window, cx| {
                        if let Some(handler) = on_change.as_ref() {
                            handler(date, window, cx);
                        }
                        close_state.set_open(false, window, cx);
                    })
            }
            PickerValue::Range { range, on_change } => {
                let close_state = open_state.clone();
                Calendar::new(calendar_id)
                    .number_of_months(self.number_of_months)
                    .range(range)
                    .on_range_select(move |range, window, cx| {
                        if let Some(handler) = on_change.as_ref() {
                            handler(range, window, cx);
                        }
                        if range.end.is_some() {
                            close_state.set_open(false, window, cx);
                        }
                    })
            }
        };

        let presets = match &self.value {
            PickerValue::Single { on_change, .. } if !self.presets.is_empty() => {
                let buttons = self
                    .presets
                    .iter()
                    .enumerate()
                    .map(|(index, (label, date))| {
                        let on_change = on_change.clone();
                        let close_state = open_state.clone();
                        let date = *date;
                        super::Button::new(("date-preset", index))
                            .ghost()
                            .small()
                            .label(label.clone())
                            .justify_start()
                            .on_click(move |_, window, cx| {
                                if let Some(handler) = on_change.as_ref() {
                                    handler(&date, window, cx);
                                }
                                close_state.set_open(false, window, cx);
                            })
                    });
                Some(div().sx(&DATE_PICKER.presets).children(buttons))
            }
            _ => None,
        };

        let panel = popover_surface()
            .flex_row()
            .children(presets)
            .child(calendar);
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(Side::Bottom, Align::Start, panel, cx))
    }
}
