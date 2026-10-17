//! Input: a single-line text field.
//!
//! Text editing needs state that outlives a frame (content, selection, IME
//! composition), so it is split in two, like a React ref plus a component:
//! - [`InputState`]: a GPUI entity holding the text. Create it with
//!   [`use_input_state`] inside a component, or `cx.new(InputState::new)` in a view.
//! - [`Input`]: the styled element that draws an `InputState`.
//!
//! Listen for edits with `cx.subscribe(&state, |_, _, event: &InputEvent, _| ..)`
//! or read `state.read(cx).text()` when you need the value.

use gpui::{
    actions, div, prelude::*, px, App, Context, ElementId, Entity, KeyBinding, MouseButton,
    StyleRefinement, Window,
};

mod element;
mod ime;
mod layout;
mod state;
mod textarea;

use element::InputTextElement;
use layout::{bidi_row_index_for_point, bidi_row_position_for_offset};
#[doc(hidden)]
pub use layout::{bidi_rows, needs_bidi_rows, BidiRow, DisplayLine};
pub use state::{InputEvent, InputState};
pub use textarea::{use_textarea_state, Textarea};

use super::focus_ring_outline;
use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

actions!(
    rok_ui_input,
    [
        /// Delete the selection, or the grapheme before the cursor.
        DeleteBackward,
        /// Delete the selection, or the grapheme after the cursor.
        DeleteForward,
        /// Move the cursor one grapheme left.
        MoveLeft,
        /// Move the cursor one grapheme right.
        MoveRight,
        /// Extend the selection one grapheme left.
        SelectLeft,
        /// Extend the selection one grapheme right.
        SelectRight,
        /// Select all text.
        SelectAll,
        /// Move the cursor to the start of the text.
        MoveToStart,
        /// Move the cursor to the end of the text.
        MoveToEnd,
        /// Paste from the clipboard.
        Paste,
        /// Cut the selection to the clipboard.
        Cut,
        /// Copy the selection to the clipboard.
        Copy,
        /// Submit the input (Enter).
        Submit,
        /// Move the cursor up one line (textareas).
        MoveUp,
        /// Move the cursor down one line (textareas).
        MoveDown,
        /// Extend the selection up one line (textareas).
        SelectUp,
        /// Extend the selection down one line (textareas).
        SelectDown,
    ]
);

#[doc(hidden)]
pub const INPUT_KEY_CONTEXT: &str = "RokUiInput";
#[doc(hidden)]
pub const TEXTAREA_KEY_CONTEXT: &str = "RokUiTextarea";

#[doc(hidden)]
pub fn bind_text_editing_keys(cx: &mut App) {
    let context = Some(INPUT_KEY_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("backspace", DeleteBackward, context),
        KeyBinding::new("delete", DeleteForward, context),
        KeyBinding::new("left", MoveLeft, context),
        KeyBinding::new("right", MoveRight, context),
        KeyBinding::new("shift-left", SelectLeft, context),
        KeyBinding::new("shift-right", SelectRight, context),
        KeyBinding::new("secondary-a", SelectAll, context),
        KeyBinding::new("secondary-v", Paste, context),
        KeyBinding::new("secondary-c", Copy, context),
        KeyBinding::new("secondary-x", Cut, context),
        KeyBinding::new("home", MoveToStart, context),
        KeyBinding::new("end", MoveToEnd, context),
        KeyBinding::new("enter", Submit, context),
        KeyBinding::new("secondary-enter", Submit, context),
    ]);
    let textarea_context = Some(TEXTAREA_KEY_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("up", MoveUp, textarea_context),
        KeyBinding::new("down", MoveDown, textarea_context),
        KeyBinding::new("shift-up", SelectUp, textarea_context),
        KeyBinding::new("shift-down", SelectDown, textarea_context),
    ]);
}

/// `useRef`-style hook: an [`InputState`] that lives as long as the component.
pub fn use_input_state(
    key: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    configure: impl FnOnce(InputState) -> InputState,
) -> Entity<InputState> {
    window.use_keyed_state(key, cx, |_, cx| configure(InputState::new(cx)))
}

/// The styled text field (shadcn/ui's `<Input>`).
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let email = use_input_state("email", window, cx, |state| state.with_placeholder("m@example.com"));
/// let field = Input::new(&email).leading_icon(IconName::Mail);
/// # }
/// ```
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    leading_icon: Option<IconName>,
    disabled: bool,
    invalid: bool,
    focus_ring: bool,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Input);

impl Input {
    /// An input for `state` (create it with `use_input_state` or `cx.new(InputState::new)`).
    #[must_use]
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            leading_icon: None,
            disabled: false,
            invalid: false,
            focus_ring: true,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Icon drawn inside the field, before the text.
    #[must_use]
    pub fn leading_icon(mut self, icon: IconName) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Red border and ring, shadcn/ui's `aria-invalid` styling.
    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// For fields embedded in a larger control that draws its own ring.
    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    #[doc(hidden)]
    #[must_use]
    pub fn without_focus_ring(mut self) -> Self {
        self.focus_ring = false;
        self
    }
}

styles! {
    INPUT = {
        // No shadows: GPUI paints them under the element, so they would tint the
        // transparent field. The focus ring is an outline instead.
        field: {
            position: relative,
            display: flex,
            align: center,
            gap: 2,
            width: full,
            height: 9,
            padding_x: 3,
            radius: md,
            border: 1,
            border_color: input,
            background: transparent,
            color: foreground,
            text: sm,
            line_height: 5,
        },
        focused: { border_color: ring },
        invalid: { border_color: destructive_text },
        enabled: { cursor: text },
        disabled: { opacity: 0.5 },
        text: { flex: 1, overflow: hidden },
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);
        let ring_color = if self.invalid {
            colors.destructive_text
        } else {
            colors.ring
        };

        let state = self.state.clone();
        div()
            .id(ElementId::NamedInteger(
                "rok-ui-input".into(),
                self.state.entity_id().as_u64(),
            ))
            .sx((
                &INPUT.field,
                is_focused.then_some(&INPUT.focused),
                self.invalid.then_some(&INPUT.invalid),
                if self.disabled {
                    &INPUT.disabled
                } else {
                    &INPUT.enabled
                },
                &self.sx,
            ))
            .when(is_focused && self.focus_ring, |field| {
                field.child(focus_ring_outline(ring_color, theme.radius_medium()))
            })
            .when_some(self.leading_icon, |field, icon| {
                field.child(Icon::new(icon).size(px(16.)).color(colors.muted_foreground))
            })
            .when(!self.disabled, |field| {
                field
                    .key_context(INPUT_KEY_CONTEXT)
                    .track_focus(&focus_handle)
                    .on_action(forward_to_state(&state, InputState::delete_backward))
                    .on_action(forward_to_state(&state, InputState::delete_forward))
                    .on_action(forward_to_state(&state, InputState::move_left))
                    .on_action(forward_to_state(&state, InputState::move_right))
                    .on_action(forward_to_state(&state, InputState::select_left))
                    .on_action(forward_to_state(&state, InputState::select_right))
                    .on_action(forward_to_state(&state, InputState::select_all))
                    .on_action(forward_to_state(&state, InputState::move_to_start))
                    .on_action(forward_to_state(&state, InputState::move_to_end))
                    .on_action(forward_to_state(&state, InputState::paste))
                    .on_action(forward_to_state(&state, InputState::cut))
                    .on_action(forward_to_state(&state, InputState::copy))
                    .on_action(forward_to_state(&state, InputState::submit))
                    .on_mouse_down(
                        MouseButton::Left,
                        forward_to_state(&state, InputState::on_mouse_down),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        forward_to_state(&state, InputState::on_mouse_up),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        forward_to_state(&state, InputState::on_mouse_up),
                    )
                    .on_mouse_move(forward_to_state(&state, InputState::on_mouse_move))
            })
            .child(div().sx(&INPUT.text).child(InputTextElement {
                state: self.state,
                placeholder_color: colors.muted_foreground,
                cursor_color: colors.foreground,
                selection_color: colors.ring.opacity(0.3),
            }))
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Route an action or mouse event from the field element to its [`InputState`].
fn forward_to_state<Event: 'static>(
    state: &Entity<InputState>,
    handler: fn(&mut InputState, &Event, &mut Window, &mut Context<InputState>),
) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
    let state = state.clone();
    move |event, window, cx| state.update(cx, |state, cx| handler(state, event, window, cx))
}
