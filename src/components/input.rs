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

use std::ops::Range;

use gpui::{
    actions, div, fill, point, prelude::*, px, relative, size, App, Bounds, ClipboardItem, Context,
    ElementId, ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, GlobalElementId, Hsla, KeyBinding, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine, SharedString, Style,
    StyleRefinement, TextRun, UTF16Selection, UnderlineStyle, Window, WrappedLine,
};
use unicode_segmentation::UnicodeSegmentation;

mod textarea;
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
        DeleteBackward,
        DeleteForward,
        MoveLeft,
        MoveRight,
        SelectLeft,
        SelectRight,
        SelectAll,
        MoveToStart,
        MoveToEnd,
        Paste,
        Cut,
        Copy,
        Submit,
        MoveUp,
        MoveDown,
        SelectUp,
        SelectDown,
    ]
);

pub(crate) const INPUT_KEY_CONTEXT: &str = "RokUiInput";
pub(crate) const TEXTAREA_KEY_CONTEXT: &str = "RokUiTextarea";

pub(crate) fn bind_text_editing_keys(cx: &mut App) {
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

/// Events emitted by [`InputState`].
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// The text changed; carries the new text.
    Changed(SharedString),
    /// Enter was pressed (Ctrl/Cmd-Enter in a multi-line field); carries the current text.
    Submitted(SharedString),
}

/// The text, selection and focus of one text field.
pub struct InputState {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    masked: bool,
    multiline: bool,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    /// Wrapped paragraphs from the last paint of a multi-line field.
    last_wrapped_lines: Vec<WrappedLine>,
    /// Per paragraph, per visual row: the RTL right-alignment shift (empty in LTR).
    last_row_offsets: Vec<Vec<Pixels>>,
    last_line_height: Pixels,
    is_selecting: bool,
}

impl EventEmitter<InputEvent> for InputState {}

impl Focusable for InputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl InputState {
    /// `cx.new(InputState::new)`.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle().tab_stop(true).tab_index(0),
            content: SharedString::default(),
            placeholder: SharedString::default(),
            masked: false,
            multiline: false,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            last_wrapped_lines: Vec::new(),
            last_row_offsets: Vec::new(),
            last_line_height: px(20.),
            is_selecting: false,
        }
    }

    /// Text shown while the field is empty.
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Show bullets instead of the text (password fields).
    pub fn with_masked_text(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Allow line breaks: Enter inserts a newline, Up / Down move between lines.
    /// Use it for the state behind a [`super::Textarea`].
    pub fn with_multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// Start with some text.
    pub fn with_text(mut self, text: impl Into<SharedString>) -> Self {
        self.content = text.into();
        self.selected_range = self.content.len()..self.content.len();
        self
    }

    /// Whether the field accepts line breaks (see [`InputState::with_multiline`]).
    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    pub(crate) fn focus_handle_ref(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Current text.
    pub fn text(&self) -> &SharedString {
        &self.content
    }

    /// Replace the text, moving the cursor to the end.
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        self.selected_range = self.content.len()..self.content.len();
        self.marked_range = None;
        cx.emit(InputEvent::Changed(self.content.clone()));
        cx.notify();
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    fn move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_cursor_to(self.previous_grapheme_boundary(self.cursor_offset()), cx);
        } else {
            self.move_cursor_to(self.selected_range.start, cx)
        }
    }

    fn move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_cursor_to(self.next_grapheme_boundary(self.selected_range.end), cx);
        } else {
            self.move_cursor_to(self.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_grapheme_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_grapheme_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn move_to_start(&mut self, _: &MoveToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(0, cx);
    }

    fn move_to_end(&mut self, _: &MoveToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(self.content.len(), cx);
    }

    fn delete_backward(&mut self, _: &DeleteBackward, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_grapheme_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete_forward(&mut self, _: &DeleteForward, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_grapheme_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline && !window.modifiers().secondary() {
            self.replace_text_in_range(
                None, "
", window, cx,
            );
        } else {
            cx.emit(InputEvent::Submitted(self.content.clone()));
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text.replace("\r\n", "\n")
            } else {
                text.replace('\n', " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_cursor_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_cursor_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Text as drawn: bullets for masked fields. Byte offsets differ when masked,
    /// so masked fields map offsets through `displayed_offset`.
    fn displayed_text(&self) -> SharedString {
        if self.masked {
            "•".repeat(self.content.chars().count()).into()
        } else {
            self.content.clone()
        }
    }

    /// Content byte offset to displayed-text byte offset.
    fn displayed_offset(&self, content_offset: usize) -> usize {
        if self.masked {
            self.content[..content_offset].chars().count() * '•'.len_utf8()
        } else {
            content_offset
        }
    }

    /// Displayed-text byte offset to content byte offset.
    fn content_offset(&self, displayed_offset: usize) -> usize {
        if self.masked {
            let character_count = displayed_offset / '•'.len_utf8();
            self.content
                .char_indices()
                .nth(character_count)
                .map(|(byte_offset, _)| byte_offset)
                .unwrap_or(self.content.len())
        } else {
            displayed_offset
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        if self.multiline {
            return match self.last_bounds.as_ref() {
                Some(bounds) => self.multiline_index_for_point(position - bounds.origin),
                None => 0,
            };
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        self.content_offset(line.closest_index_for_x(position.x - bounds.left()))
    }

    /// Content offset nearest to `local` (relative to the text's top-left) in a
    /// multi-line field, using the wrapped lines from the last paint.
    fn multiline_index_for_point(&self, local: Point<Pixels>) -> usize {
        let line_height = self.last_line_height;
        let mut paragraph_start = 0;
        let mut line_top = px(0.);
        let line_count = self.last_wrapped_lines.len();
        for (index, line) in self.last_wrapped_lines.iter().enumerate() {
            let line_bottom = line_top + line.size(line_height).height;
            if local.y < line_bottom || index + 1 == line_count {
                let y_in_line = (local.y - line_top)
                    .max(px(0.))
                    .min(line_bottom - line_top - px(1.));
                // Undo this row's RTL alignment shift before hit testing.
                let shift = self.row_offset(index, y_in_line);
                let index_in_line = line
                    .closest_index_for_position(
                        point((local.x - shift).max(px(0.)), y_in_line),
                        line_height,
                    )
                    .unwrap_or_else(|index| index);
                return (paragraph_start + index_in_line).min(self.content.len());
            }
            paragraph_start += line.len() + 1;
            line_top = line_bottom;
        }
        self.content.len()
    }

    /// Where `offset` is drawn in a multi-line field, relative to the text's top-left.
    pub(crate) fn multiline_position_for_offset(&self, offset: usize) -> Option<Point<Pixels>> {
        let line_height = self.last_line_height;
        let mut paragraph_start = 0;
        let mut line_top = px(0.);
        for (index, line) in self.last_wrapped_lines.iter().enumerate() {
            if offset <= paragraph_start + line.len() {
                return line
                    .position_for_index(offset - paragraph_start, line_height)
                    .map(|position| {
                        let shift = self.row_offset(index, position.y);
                        point(position.x + shift, position.y + line_top)
                    });
            }
            paragraph_start += line.len() + 1;
            line_top += line.size(line_height).height;
        }
        None
    }

    /// Left edge of the text on the visual row at `y` (relative to the text's top).
    /// Zero in LTR; the right-alignment shift in RTL.
    pub(crate) fn multiline_row_left(&self, y: Pixels) -> Pixels {
        let line_height = self.last_line_height;
        let mut line_top = px(0.);
        for (index, line) in self.last_wrapped_lines.iter().enumerate() {
            let height = line.size(line_height).height;
            if y < line_top + height {
                return self.row_offset(index, y - line_top);
            }
            line_top += height;
        }
        px(0.)
    }

    /// How far visual row `y` (within paragraph `paragraph`) is shifted right by
    /// RTL alignment. Zero in LTR.
    fn row_offset(&self, paragraph: usize, y: Pixels) -> Pixels {
        let row = (y / self.last_line_height).floor().max(0.) as usize;
        self.last_row_offsets
            .get(paragraph)
            .and_then(|rows| rows.get(row))
            .copied()
            .unwrap_or(px(0.))
    }

    /// Offset one visual row above (`rows = -1`) or below (`rows = 1`) the cursor.
    fn vertical_target(&self, rows: f32) -> usize {
        let Some(position) = self.multiline_position_for_offset(self.cursor_offset()) else {
            return self.cursor_offset();
        };
        let target_y = position.y + self.last_line_height * (rows + 0.5);
        if target_y < px(0.) {
            return 0;
        }
        self.multiline_index_for_point(point(position.x, target_y))
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(self.vertical_target(-1.), cx);
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(self.vertical_target(1.), cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target(-1.), cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target(1.), cx);
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, utf16_offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for character in self.content.chars() {
            if utf16_count >= utf16_offset {
                break;
            }
            utf16_count += character.len_utf16();
            utf8_offset += character.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, utf8_offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for character in self.content.chars() {
            if utf8_count >= utf8_offset {
                break;
            }
            utf8_count += character.len_utf8();
            utf16_offset += character.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, utf16_range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(utf16_range.start)..self.offset_from_utf16(utf16_range.end)
    }

    fn previous_grapheme_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    fn next_grapheme_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.content.len())
    }
}

impl EntityInputHandler for InputState {
    fn text_for_range(
        &mut self,
        utf16_range: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&utf16_range);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        utf16_range: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = utf16_range
            .as_ref()
            .map(|utf16_range| self.range_from_utf16(utf16_range))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        cx.emit(InputEvent::Changed(self.content.clone()));
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        utf16_range: Option<Range<usize>>,
        new_text: &str,
        new_selected_utf16_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = utf16_range
            .as_ref()
            .map(|utf16_range| self.range_from_utf16(utf16_range))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.marked_range =
            (!new_text.is_empty()).then(|| range.start..range.start + new_text.len());
        self.selected_range = new_selected_utf16_range
            .as_ref()
            .map(|utf16_range| self.range_from_utf16(utf16_range))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        cx.emit(InputEvent::Changed(self.content.clone()));
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        utf16_range: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&utf16_range);
        if self.multiline {
            let start = self.multiline_position_for_offset(range.start)?;
            let end = self
                .multiline_position_for_offset(range.end)
                .unwrap_or(start);
            return Some(Bounds::from_corners(
                bounds.origin + start,
                bounds.origin + point(end.x.max(start.x), end.y + self.last_line_height),
            ));
        }
        let last_layout = self.last_layout.as_ref()?;
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(self.displayed_offset(range.start)),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(self.displayed_offset(range.end)),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        if self.multiline {
            let bounds = self.last_bounds?;
            let index = self.multiline_index_for_point(position - bounds.origin);
            return Some(self.offset_to_utf16(index));
        }
        let line_origin = self.last_bounds?.localize(&position)?;
        let last_layout = self.last_layout.as_ref()?;
        let displayed_index = last_layout.index_for_x(position.x - line_origin.x)?;
        Some(self.offset_to_utf16(self.content_offset(displayed_index)))
    }
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
/// ```ignore
/// let email = use_input_state("email", window, cx, |state| state.with_placeholder("m@example.com"));
/// Input::new(&email).leading_icon(IconName::Mail)
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
    pub fn leading_icon(mut self, icon: IconName) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Red border and ring, shadcn/ui's `aria-invalid` styling.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// For fields embedded in a larger control that draws its own ring.
    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    pub(crate) fn without_focus_ring(mut self) -> Self {
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

/// Low-level element that shapes and paints the text, cursor and selection.
struct InputTextElement {
    state: Entity<InputState>,
    placeholder_color: Hsla,
    cursor_color: Hsla,
    selection_color: Hsla,
}

struct InputTextPrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    /// How far the text is shifted right (RTL alignment).
    text_offset: Pixels,
}

impl IntoElement for InputTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InputTextElement {
    type RequestLayoutState = ();
    type PrepaintState = InputTextPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = self.state.read(cx);
        let content_is_empty = state.content.is_empty();
        let selected_range = state.displayed_offset(state.selected_range.start)
            ..state.displayed_offset(state.selected_range.end);
        let cursor_offset = state.displayed_offset(state.cursor_offset());
        let marked_range = state
            .marked_range
            .as_ref()
            .map(|range| state.displayed_offset(range.start)..state.displayed_offset(range.end));
        let text_style = window.text_style();

        let (display_text, text_color) = if content_is_empty {
            (state.placeholder.clone(), self.placeholder_color)
        } else {
            (state.displayed_text(), text_style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: text_style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = match marked_range {
            Some(marked_range) if !content_is_empty => vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect(),
            _ => vec![run],
        };

        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        // In RTL the text sits against the right edge. Shifting the bounds keeps
        // painting, the cursor, hit testing and IME positions consistent.
        let rtl = crate::components::direction::is_rtl();
        let text_offset = if rtl {
            (bounds.size.width - line.width).max(px(0.))
        } else {
            px(0.)
        };
        let bounds = shift_bounds(bounds, text_offset);

        let (selection, cursor) = if selected_range.is_empty() {
            let cursor_x = if content_is_empty {
                if rtl {
                    line.width
                } else {
                    px(0.)
                }
            } else {
                line.x_for_index(cursor_offset)
            };
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_x, bounds.top()),
                        size(px(1.5), bounds.bottom() - bounds.top()),
                    ),
                    self.cursor_color,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    self.selection_color,
                )),
                None,
            )
        };
        InputTextPrepaintState {
            line: Some(line),
            cursor,
            selection,
            text_offset,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let bounds = shift_bounds(bounds, prepaint.text_offset);
        let focus_handle = self.state.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let Some(line) = prepaint.line.take() else {
            return;
        };
        line.paint(bounds.origin, window.line_height(), window, cx)
            .ok();

        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }

        let content_is_empty = self.state.read(cx).content.is_empty();
        self.state.update(cx, |state, _| {
            // The placeholder's layout must not be used for hit testing real text.
            state.last_layout = (!content_is_empty).then_some(line);
            state.last_bounds = Some(bounds);
        });
    }
}

/// `bounds` with its left edge moved right by `offset`.
fn shift_bounds(bounds: Bounds<Pixels>, offset: Pixels) -> Bounds<Pixels> {
    Bounds::new(
        point(bounds.left() + offset, bounds.top()),
        size(bounds.size.width - offset, bounds.size.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RtlTextareaView(Entity<InputState>);

    impl Render for RtlTextareaView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::components::Direction::new(crate::components::TextDirection::Rtl).child(
                gpui::div()
                    .w(px(300.))
                    .child(crate::components::Textarea::new(&self.0)),
            )
        }
    }

    #[gpui::test]
    fn rtl_textarea_right_aligns_rows(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let state = cx.new(|cx| {
            InputState::new(cx)
                .with_multiline(true)
                .with_text("ab\nlonger line")
        });
        let view_state = state.clone();
        let (_view, window_context) =
            cx.add_window_view(move |_, _| RtlTextareaView(view_state.clone()));
        window_context.run_until_parked();
        state.read_with(window_context, |state, _| {
            let bounds = state.last_bounds.expect("textarea was laid out");
            let short = state.multiline_position_for_offset(0).unwrap();
            let long = state.multiline_position_for_offset(3).unwrap();
            // Both rows end at the right edge, so the shorter one starts further right.
            assert!(short.x > long.x, "{short:?} vs {long:?}");
            assert!(long.x > px(0.));
            let short_end = state.multiline_position_for_offset(2).unwrap();
            assert!((bounds.size.width - short_end.x).abs() < px(1.));
            // Hit-testing undoes the shift: clicking where offset 0 is drawn lands on it.
            assert_eq!(
                state.multiline_index_for_point(point(short.x + px(0.5), short.y + px(1.))),
                0
            );
            assert_eq!(state.multiline_row_left(px(1.)), short.x);
        });
    }

    #[gpui::test]
    fn typing_emits_changed_and_masks_offsets(cx: &mut gpui::TestAppContext) {
        let input_state = cx.new(|cx| InputState::new(cx).with_masked_text(true));
        let received_events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        cx.update(|cx| {
            let received_events = received_events.clone();
            cx.subscribe(&input_state, move |_, event: &InputEvent, _| {
                received_events.borrow_mut().push(event.clone());
            })
            .detach();
        });

        input_state.update(cx, |state, cx| state.set_text("héllo", cx));
        input_state.read_with(cx, |state, _| {
            assert_eq!(state.text().as_ref(), "héllo");
            assert_eq!(state.displayed_text().as_ref(), "•••••");
            // 'é' is two bytes; in the masked text every character is three bytes.
            assert_eq!(state.displayed_offset(3), 2 * '•'.len_utf8());
            assert_eq!(state.content_offset(2 * '•'.len_utf8()), 3);
        });
        assert_eq!(
            received_events.borrow().as_slice(),
            &[InputEvent::Changed("héllo".into())]
        );
    }
}
