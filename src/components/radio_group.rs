//! RadioGroup: pick exactly one option from a short list.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, CursorStyle, ElementId, SharedString, StyleRefinement, Window,
};

use super::extra_small_shadow;
use crate::{hooks::EventHandler, styles::ApplyStyleOverrides, theme::ActiveTheme};

#[derive(Clone)]
struct RadioOption {
    value: SharedString,
    label: SharedString,
    description: Option<SharedString>,
    disabled: bool,
}

/// Controlled: pass `value`, update it in `on_change`. Arrow keys change the
/// selection while an option has focus.
///
/// ```ignore
/// RadioGroup::new("density")
///     .option("default", "Default")
///     .option("comfortable", "Comfortable")
///     .option("compact", "Compact")
///     .value(density.get(cx))
///     .on_change(move |value, _, cx| density.set(value.clone(), cx))
/// ```
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    options: Vec<RadioOption>,
    value: Option<SharedString>,
    horizontal: bool,
    disabled: bool,
    on_change: Option<EventHandler<SharedString>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(RadioGroup);

impl RadioGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            value: None,
            horizontal: false,
            disabled: false,
            on_change: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.options.push(RadioOption {
            value: value.into(),
            label: label.into(),
            description: None,
            disabled: false,
        });
        self
    }

    /// An option with a line of help text under its label.
    pub fn option_with_description(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        self.options.push(RadioOption {
            value: value.into(),
            label: label.into(),
            description: Some(description.into()),
            disabled: false,
        });
        self
    }

    /// Disable the option added last.
    pub fn disable_last(mut self) -> Self {
        if let Some(option) = self.options.last_mut() {
            option.disabled = true;
        }
        self
    }

    pub fn value(mut self, value: Option<impl Into<SharedString>>) -> Self {
        self.value = value.map(Into::into);
        self
    }

    /// Lay the options out in a row.
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Receives the chosen option's value.
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let ring_color = colors.ring;
        let enabled_values: Rc<Vec<SharedString>> = Rc::new(
            self.options
                .iter()
                .filter(|option| !option.disabled && !self.disabled)
                .map(|option| option.value.clone())
                .collect(),
        );

        let options = self.options.into_iter().enumerate().map(|(index, option)| {
            let is_selected = self.value.as_ref() == Some(&option.value);
            let is_interactive = !option.disabled && !self.disabled;
            let indicator = div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(16.))
                .mt(px(2.))
                .rounded_full()
                .border_1()
                .border_color(if is_selected {
                    colors.primary
                } else {
                    colors.input
                })
                .bg(colors.background)
                .shadow(extra_small_shadow())
                .when(is_selected, |indicator| {
                    indicator.child(div().size(px(8.)).rounded_full().bg(colors.primary))
                });

            let on_change = self.on_change.clone();
            let key_on_change = self.on_change.clone();
            let value = option.value.clone();
            let enabled_values = enabled_values.clone();
            let horizontal = self.horizontal;
            div()
                .id(index)
                .flex()
                .items_start()
                .gap(px(12.))
                .rounded(px(4.))
                .text_sm()
                .child(indicator)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(option.label)
                        .when_some(option.description, |text, description| {
                            text.child(div().text_color(colors.muted_foreground).child(description))
                        }),
                )
                .when(is_interactive, |row| {
                    row.tab_index(0)
                        .cursor(CursorStyle::PointingHand)
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .focus(move |style| style.border_color(ring_color))
                        .on_click({
                            let value = value.clone();
                            move |_, window, cx| {
                                if let Some(handler) = on_change.as_ref() {
                                    handler(&value, window, cx);
                                }
                            }
                        })
                        .on_key_down(move |event, window, cx| {
                            let step: isize = match (event.keystroke.key.as_str(), horizontal) {
                                ("down", false) | ("right", true) => 1,
                                ("up", false) | ("left", true) => -1,
                                _ => return,
                            };
                            let Some(position) =
                                enabled_values.iter().position(|enabled| *enabled == value)
                            else {
                                return;
                            };
                            cx.stop_propagation();
                            let count = enabled_values.len() as isize;
                            let next = (position as isize + step).rem_euclid(count) as usize;
                            if let Some(handler) = key_on_change.as_ref() {
                                handler(&enabled_values[next], window, cx);
                            }
                        })
                })
                .when(!is_interactive, |row| row.opacity(0.5))
        });

        div()
            .id(self.id)
            .flex()
            .gap(px(12.))
            .when(!self.horizontal, |group| group.flex_col())
            .children(options)
            .apply_style_overrides(&self.style_overrides)
    }
}
