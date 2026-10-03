//! `InputOtp`: one-time-code entry, one box per character.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ElementId, SharedString, StyleRefinement, Window,
};

use super::{focus_ring_outline, overlay::child_id};
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Which characters a code accepts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OtpPattern {
    /// 0-9 only.
    #[default]
    Digits,
    /// Letters and digits; letters are uppercased.
    Alphanumeric,
}

impl OtpPattern {
    fn accept(self, character: char) -> Option<char> {
        match self {
            OtpPattern::Digits => character.is_ascii_digit().then_some(character),
            OtpPattern::Alphanumeric => character
                .is_ascii_alphanumeric()
                .then(|| character.to_ascii_uppercase()),
        }
    }

    /// The accepted characters of `text`, at most `limit` of them (used for paste).
    #[must_use]
    pub fn filter(self, text: &str, limit: usize) -> String {
        text.chars()
            .filter_map(|character| self.accept(character))
            .take(limit)
            .collect()
    }
}

/// Controlled: pass `value`, update it in `on_change`. Typing fills the next
/// box, Backspace clears the last one and Ctrl/Cmd-V pastes a whole code.
///
/// ```ignore
/// InputOtp::new("verification", 6)
///     .groups([3, 3])
///     .value(code.get(cx))
///     .on_change(move |value, _, cx| code.set(value.clone(), cx))
///     .on_complete(|value, _, cx| verify(value, cx))
/// ```
#[derive(IntoElement)]
pub struct InputOtp {
    id: ElementId,
    length: usize,
    groups: Vec<usize>,
    value: SharedString,
    pattern: OtpPattern,
    disabled: bool,
    invalid: bool,
    on_change: Option<EventHandler<SharedString>>,
    on_complete: Option<EventHandler<SharedString>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(InputOtp);

impl InputOtp {
    /// A code input of `length` cells. `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, length: usize) -> Self {
        Self {
            id: id.into(),
            length,
            groups: vec![length],
            value: SharedString::default(),
            pattern: OtpPattern::Digits,
            disabled: false,
            invalid: false,
            on_change: None,
            on_complete: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Split the boxes into groups with a separator between, like `[3, 3]`.
    #[must_use]
    pub fn groups(mut self, groups: impl IntoIterator<Item = usize>) -> Self {
        self.groups = groups.into_iter().filter(|size| *size > 0).collect();
        self
    }

    /// The code entered so far.
    #[must_use]
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Which characters cells accept.
    #[must_use]
    pub fn pattern(mut self, pattern: OtpPattern) -> Self {
        self.pattern = pattern;
        self
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Mark it invalid: a red border.
    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Receives the code after every edit.
    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Called once every box is filled.
    #[must_use]
    pub fn on_complete(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_complete = Some(Rc::new(handler));
        self
    }
}

styles! {
    INPUT_OTP = {
        // Codes read left to right in every direction.
        ltr: { display: flex, direction: row_ltr },
        root: { align: center, gap: 2 },
        interactive: { cursor: text },
        inert: { opacity: 0.5 },
        group: { align: center },
        // Neighbouring slots share a border, so each draws only its right edge.
        slot: {
            position: relative,
            align: center,
            justify: center,
            size: 9,
            border_y: 1,
            border_right: 1,
            border_color: input,
            text: sm,
        },
        slot_first: { border_left: 1, radius_left: md },
        slot_last: { radius_right: md },
        slot_active: { border_color: ring },
        invalid: { border_color: destructive_text },
        caret: { width: 0.25, height: 4, background: foreground },
    }
}

impl RenderOnce for InputOtp {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| {
                cx.focus_handle().tab_stop(true)
            })
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let theme = cx.theme();
        let colors = &theme.colors;
        let radius = theme.radius_medium();
        let length = self.length;
        let value: Vec<char> = self.value.chars().take(length).collect();
        let active_slot = value.len().min(length.saturating_sub(1));
        let accent = if self.invalid {
            colors.destructive_text
        } else {
            colors.ring
        };

        // Lay out groups, padding or trimming so they add up to `length`.
        let mut groups = self.groups.clone();
        let grouped: usize = groups.iter().sum();
        if grouped < length {
            groups.push(length - grouped);
        }

        let mut slot_index = 0;
        let mut children: Vec<AnyElement> = Vec::new();
        for (group_index, group_size) in groups.iter().enumerate() {
            if slot_index >= length {
                break;
            }
            if group_index > 0 {
                children.push(
                    Icon::new(IconName::Minus)
                        .size(px(16.))
                        .color(colors.muted_foreground)
                        .into_any_element(),
                );
            }
            let group_end = (slot_index + group_size).min(length);
            let slots = (slot_index..group_end).map(|index| {
                let character = value.get(index).copied();
                let is_active = is_focused && index == active_slot;
                let is_first = index == slot_index;
                let is_last = index + 1 == group_end;
                div()
                    .sx((
                        &INPUT_OTP.ltr,
                        &INPUT_OTP.slot,
                        is_first.then_some(&INPUT_OTP.slot_first),
                        is_last.then_some(&INPUT_OTP.slot_last),
                        is_active.then_some(&INPUT_OTP.slot_active),
                        self.invalid.then_some(&INPUT_OTP.invalid),
                    ))
                    .when(is_active, |slot| {
                        let ring_radius = if is_first || is_last { radius } else { px(0.) };
                        slot.child(focus_ring_outline(accent, ring_radius))
                    })
                    .when_some(character, |slot, character| {
                        slot.child(SharedString::from(character.to_string()))
                    })
                    .when(is_active && character.is_none(), |slot| {
                        slot.child(div().sx(&INPUT_OTP.caret))
                    })
            });
            children.push(
                div()
                    .sx((&INPUT_OTP.ltr, &INPUT_OTP.group))
                    .children(slots)
                    .into_any_element(),
            );
            slot_index = group_end;
        }

        let pattern = self.pattern;
        let on_change = self.on_change;
        let on_complete = self.on_complete;
        let current = self.value.clone();
        let focus_on_click = focus_handle.clone();
        div()
            .id(self.id)
            .sx((
                &INPUT_OTP.ltr,
                &INPUT_OTP.root,
                if self.disabled {
                    &INPUT_OTP.inert
                } else {
                    &INPUT_OTP.interactive
                },
                &self.sx,
            ))
            .when(!self.disabled, |otp| {
                otp.track_focus(&focus_handle)
                    .on_click(move |_, window, _| window.focus(&focus_on_click))
                    .on_key_down(move |event, window, cx| {
                        let keystroke = &event.keystroke;
                        let existing: String = current.chars().take(length).collect();
                        let next: Option<String> = if keystroke.key == "backspace" {
                            let mut characters: Vec<char> = existing.chars().collect();
                            characters.pop();
                            Some(characters.into_iter().collect())
                        } else if keystroke.modifiers.secondary() && keystroke.key == "v" {
                            cx.read_from_clipboard()
                                .and_then(|item| item.text())
                                .map(|text| pattern.filter(&text, length))
                                .filter(|pasted| !pasted.is_empty())
                        } else if keystroke.modifiers.secondary() || keystroke.modifiers.alt {
                            None
                        } else {
                            keystroke
                                .key_char
                                .as_deref()
                                .and_then(|typed| typed.chars().next())
                                .and_then(|character| pattern.accept(character))
                                .filter(|_| existing.chars().count() < length)
                                .map(|character| format!("{existing}{character}"))
                        };
                        let Some(next) = next else {
                            return;
                        };
                        cx.stop_propagation();
                        let next = SharedString::from(next);
                        if let Some(handler) = on_change.as_ref() {
                            handler(&next, window, cx);
                        }
                        if next.chars().count() == length {
                            if let Some(handler) = on_complete.as_ref() {
                                handler(&next, window, cx);
                            }
                        }
                    })
            })
            .children(children)
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::OtpPattern;

    #[test]
    fn patterns_filter_pasted_codes() {
        assert_eq!(OtpPattern::Digits.filter("12-34 56 78", 6), "123456");
        assert_eq!(OtpPattern::Alphanumeric.filter("ab-12c", 6), "AB12C");
        assert_eq!(OtpPattern::Digits.filter("abc", 6), "");
    }
}
