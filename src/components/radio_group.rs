//! `RadioGroup`: pick exactly one option from a short list.

use std::rc::Rc;

use gpui::{div, prelude::*, App, ElementId, SharedString, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{hooks::EventHandler, styles, styles::ApplyStyleOverrides};

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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(RadioGroup);

impl RadioGroup {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            value: None,
            horizontal: false,
            disabled: false,
            on_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add an option with its value and label.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn disable_last(mut self) -> Self {
        if let Some(option) = self.options.last_mut() {
            option.disabled = true;
        }
        self
    }

    /// The selected value (controlled).
    #[must_use]
    pub fn value(mut self, value: Option<impl Into<SharedString>>) -> Self {
        self.value = value.map(Into::into);
        self
    }

    /// Lay the options out in a row.
    #[must_use]
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Receives the chosen option's value.
    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

styles! {
    RADIO = {
        group: { display: flex, gap: 3 },
        vertical: { direction: column },
        option: { display: flex, align: start, gap: 3, radius: 1, text: sm },
        interactive: {
            cursor: pointer,
            border: 1,
            border_color: transparent,
            focus: { border_color: ring },
        },
        inert: { opacity: 0.5 },
        indicator: {
            display: flex,
            flex: none,
            align: center,
            justify: center,
            size: 4,
            margin_top: 0.5,
            radius: full,
            border: 1,
            border_color: input,
            background: background,
            shadow: xs,
        },
        indicator_selected: { border_color: primary },
        dot: { size: 2, radius: full, background: primary },
        text: { display: flex, direction: column, gap: 1 },
        description: { color: muted_foreground },
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
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
                .sx((
                    &RADIO.indicator,
                    is_selected.then_some(&RADIO.indicator_selected),
                ))
                .when(is_selected, |indicator| {
                    indicator.child(div().sx(&RADIO.dot))
                });

            let on_change = self.on_change.clone();
            let key_on_change = self.on_change.clone();
            let value = option.value.clone();
            let enabled_values = enabled_values.clone();
            let horizontal = self.horizontal;
            // Captured while rendering: handlers run outside the `Direction` scope.
            let rtl = super::direction::is_rtl();
            div()
                .id(index)
                .sx((
                    &RADIO.option,
                    if is_interactive {
                        &RADIO.interactive
                    } else {
                        &RADIO.inert
                    },
                ))
                .child(indicator)
                .child(
                    div()
                        .sx(&RADIO.text)
                        .child(crate::components::bidi_text::text(option.label))
                        .when_some(option.description, |text, description| {
                            text.child(
                                div()
                                    .sx(&RADIO.description)
                                    .child(crate::components::bidi_text::text(description)),
                            )
                        }),
                )
                .when(is_interactive, |row| {
                    row.tab_index(0)
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
                            // A horizontal group runs right to left in RTL.
                            let step = if horizontal && rtl { -step } else { step };
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
        });

        div()
            .id(self.id)
            .sx((
                &RADIO.group,
                (!self.horizontal).then_some(&RADIO.vertical),
                &self.sx,
            ))
            .children(options)
            .apply_style_overrides(&self.style_overrides)
    }
}
