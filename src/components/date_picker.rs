//! DatePicker: a button that opens a [`Calendar`] in a popover.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, CursorStyle, ElementId, FontWeight, SharedString, StyleRefinement,
    Window,
};

use super::direction::DirectionalStyled;
use super::{
    calendar::{Calendar, CalendarDate, DateRange},
    extra_small_shadow, focus_ring_shadow,
    overlay::{
        dismissable, floating, popover_surface, trigger_wrapper, use_open_state, Align, Side,
    },
};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
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
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(DatePicker);

impl DatePicker {
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
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The picked date (single mode).
    pub fn date(mut self, date: Option<CalendarDate>) -> Self {
        let on_change = match self.value {
            PickerValue::Single { on_change, .. } => on_change,
            PickerValue::Range { .. } => None,
        };
        self.value = PickerValue::Single { date, on_change };
        self
    }

    /// Receives the picked date (single mode).
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

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// A quick pick listed beside the calendar (single mode).
    pub fn preset(mut self, label: impl Into<SharedString>, date: CalendarDate) -> Self {
        self.presets.push((label.into(), date));
        self
    }

    pub fn number_of_months(mut self, count: usize) -> Self {
        self.number_of_months = count;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for DatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state = use_open_state(&self.id, None, None, window, cx);
        let is_open = open_state.is_open(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let ring_color = colors.ring;

        let label: Option<String> = match &self.value {
            PickerValue::Single { date, .. } => date.map(CalendarDate::format_long),
            PickerValue::Range { range, .. } => range.map(|range| range.format_short()),
        };

        let trigger = div()
            .id("date-picker-trigger")
            .flex_dir()
            .items_center()
            .gap(px(8.))
            .h(px(36.))
            .w(px(240.))
            .px(px(12.))
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(colors.input)
            .bg(colors.background)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .shadow(extra_small_shadow())
            .when(!self.disabled, |trigger| {
                trigger
                    .tab_index(0)
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(colors.accent))
                    .focus(move |style| {
                        style
                            .border_color(ring_color)
                            .shadow(focus_ring_shadow(ring_color))
                    })
            })
            .when(self.disabled, |trigger| trigger.opacity(0.5))
            .child(
                Icon::new(IconName::Calendar)
                    .size(px(16.))
                    .color(colors.muted_foreground),
            )
            .child(match label {
                Some(label) => div().truncate().child(label),
                None => div()
                    .truncate()
                    .text_color(colors.muted_foreground)
                    .child(self.placeholder),
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
                Some(
                    div()
                        .flex_dir()
                        .flex_col()
                        .gap(px(2.))
                        .p(px(8.))
                        .border_e_1()
                        .border_color(colors.border)
                        .children(buttons),
                )
            }
            _ => None,
        };

        let panel = popover_surface(cx.theme())
            .flex_row()
            .children(presets)
            .child(calendar);
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(Side::Bottom, Align::Start, panel, cx))
    }
}
