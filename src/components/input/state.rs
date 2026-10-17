//! [`InputState`]: the text, selection and focus of a field, and the editing actions.

use std::ops::Range;

use gpui::{
    point, px, App, Bounds, ClipboardItem, Context, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, Window,
    WrappedLine,
};
use unicode_segmentation::UnicodeSegmentation;

use super::{
    bidi_row_index_for_point, bidi_row_position_for_offset, BidiRow, Copy, Cut, DeleteBackward,
    DeleteForward, DisplayLine, MoveDown, MoveLeft, MoveRight, MoveToEnd, MoveToStart, MoveUp,
    Paste, SelectAll, SelectDown, SelectLeft, SelectRight, SelectUp, Submit,
};

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
    pub(super) focus_handle: FocusHandle,
    pub(super) content: SharedString,
    pub(super) placeholder: SharedString,
    pub(super) masked: bool,
    pub(super) multiline: bool,
    pub(super) selected_range: Range<usize>,
    pub(super) selection_reversed: bool,
    pub(super) marked_range: Option<Range<usize>>,
    pub(super) last_layout: Option<DisplayLine>,
    pub(super) last_bounds: Option<Bounds<Pixels>>,
    /// Wrapped paragraphs from the last paint of a multi-line field.
    pub(super) last_wrapped_lines: Vec<WrappedLine>,
    /// Per paragraph, per visual row: the RTL right-alignment shift (empty in LTR).
    pub(super) last_row_offsets: Vec<Vec<Pixels>>,
    /// Rows laid out by rok-ui instead of GPUI's wrapping, for right-to-left text on
    /// Windows (see [`crate::bidi`]). When set, they replace `last_wrapped_lines`.
    pub(super) last_bidi_rows: Option<Vec<BidiRow>>,
    pub(super) last_line_height: Pixels,
    pub(super) is_selecting: bool,
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
            last_bidi_rows: None,
            last_line_height: px(20.),
            is_selecting: false,
        }
    }

    /// Text shown while the field is empty.
    #[must_use]
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Show bullets instead of the text (password fields).
    #[must_use]
    pub fn with_masked_text(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Allow line breaks: Enter inserts a newline, Up / Down move between lines.
    /// Use it for the state behind a [`super::Textarea`].
    #[must_use]
    pub fn with_multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// Start with some text.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<SharedString>) -> Self {
        self.content = text.into();
        self.selected_range = self.content.len()..self.content.len();
        self
    }

    /// Whether the field accepts line breaks (see [`InputState::with_multiline`]).
    #[must_use]
    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    #[doc(hidden)]
    #[must_use]
    pub fn focus_handle_ref(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Current text.
    #[must_use]
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

    /// Text shown while the input is empty.
    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub(super) fn move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_cursor_to(self.previous_grapheme_boundary(self.cursor_offset()), cx);
        } else {
            self.move_cursor_to(self.selected_range.start, cx);
        }
    }

    pub(super) fn move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_cursor_to(self.next_grapheme_boundary(self.selected_range.end), cx);
        } else {
            self.move_cursor_to(self.selected_range.end, cx);
        }
    }

    pub(super) fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_grapheme_boundary(self.cursor_offset()), cx);
    }

    pub(super) fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_grapheme_boundary(self.cursor_offset()), cx);
    }

    pub(super) fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    pub(super) fn move_to_start(
        &mut self,
        _: &MoveToStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_cursor_to(0, cx);
    }

    pub(super) fn move_to_end(&mut self, _: &MoveToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(self.content.len(), cx);
    }

    pub(super) fn delete_backward(
        &mut self,
        _: &DeleteBackward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_grapheme_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub(super) fn delete_forward(
        &mut self,
        _: &DeleteForward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_grapheme_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub(super) fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline && !window.modifiers().secondary() {
            self.replace_text_in_range(
                None, "
", window, cx,
            );
        } else {
            cx.emit(InputEvent::Submitted(self.content.clone()));
        }
    }

    pub(super) fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text.replace("\r\n", "\n")
            } else {
                text.replace('\n', " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    pub(super) fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub(super) fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    pub(super) fn on_mouse_down(
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
            self.move_cursor_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    pub(super) fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    pub(super) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    pub(super) fn move_cursor_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify();
    }

    pub(super) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Text as drawn: bullets for masked fields. Byte offsets differ when masked,
    /// so masked fields map offsets through `displayed_offset`.
    pub(super) fn displayed_text(&self) -> SharedString {
        if self.masked {
            "•".repeat(self.content.chars().count()).into()
        } else {
            self.content.clone()
        }
    }

    /// Content byte offset to displayed-text byte offset.
    pub(super) fn displayed_offset(&self, content_offset: usize) -> usize {
        if self.masked {
            self.content[..content_offset].chars().count() * '•'.len_utf8()
        } else {
            content_offset
        }
    }

    /// Displayed-text byte offset to content byte offset.
    pub(super) fn content_offset(&self, displayed_offset: usize) -> usize {
        if self.masked {
            let character_count = displayed_offset / '•'.len_utf8();
            self.content
                .char_indices()
                .nth(character_count)
                .map_or(self.content.len(), |(byte_offset, _)| byte_offset)
        } else {
            displayed_offset
        }
    }

    pub(super) fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
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
    pub(super) fn multiline_index_for_point(&self, local: Point<Pixels>) -> usize {
        if let Some(rows) = self.last_bidi_rows.as_ref() {
            return bidi_row_index_for_point(rows, local, self.last_line_height)
                .min(self.content.len());
        }
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
    #[doc(hidden)]
    #[must_use]
    pub fn multiline_position_for_offset(&self, offset: usize) -> Option<Point<Pixels>> {
        if let Some(rows) = self.last_bidi_rows.as_ref() {
            return bidi_row_position_for_offset(rows, offset, self.last_line_height);
        }
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
    #[doc(hidden)]
    #[must_use]
    pub fn multiline_row_left(&self, y: Pixels) -> Pixels {
        if let Some(rows) = self.last_bidi_rows.as_ref() {
            let row = (y / self.last_line_height).floor().max(0.) as usize;
            return rows.get(row).map_or(px(0.), |row| row.left);
        }
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
    pub(super) fn row_offset(&self, paragraph: usize, y: Pixels) -> Pixels {
        let row = (y / self.last_line_height).floor().max(0.) as usize;
        self.last_row_offsets
            .get(paragraph)
            .and_then(|rows| rows.get(row))
            .copied()
            .unwrap_or(px(0.))
    }

    /// Offset one visual row above (`rows = -1`) or below (`rows = 1`) the cursor.
    pub(super) fn vertical_target(&self, rows: f32) -> usize {
        let Some(position) = self.multiline_position_for_offset(self.cursor_offset()) else {
            return self.cursor_offset();
        };
        let target_y = position.y + self.last_line_height * (rows + 0.5);
        if target_y < px(0.) {
            return 0;
        }
        self.multiline_index_for_point(point(position.x, target_y))
    }

    pub(super) fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(self.vertical_target(-1.), cx);
    }

    pub(super) fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_cursor_to(self.vertical_target(1.), cx);
    }

    pub(super) fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target(-1.), cx);
    }

    pub(super) fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target(1.), cx);
    }

    pub(super) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    pub(super) fn offset_from_utf16(&self, utf16_offset: usize) -> usize {
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

    pub(super) fn offset_to_utf16(&self, utf8_offset: usize) -> usize {
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

    pub(super) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub(super) fn range_from_utf16(&self, utf16_range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(utf16_range.start)..self.offset_from_utf16(utf16_range.end)
    }

    pub(super) fn previous_grapheme_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    pub(super) fn next_grapheme_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.content.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{prelude::*, Context, Entity};

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
    fn rtl_textarea_on_windows_wraps_and_maps_arabic(cx: &mut gpui::TestAppContext) {
        if !crate::bidi::platform_needs_reordering() {
            return;
        }
        cx.update(crate::init);
        let text = "مرحبا بكم في واجهة روك للمكونات وهي مكتبة لتطبيقات سطح المكتب";
        let state = cx.new(|cx| InputState::new(cx).with_multiline(true).with_text(text));
        let view_state = state.clone();
        let (_view, window_context) =
            cx.add_window_view(move |_, _| RtlTextareaView(view_state.clone()));
        window_context.run_until_parked();
        state.read_with(window_context, |state, _| {
            let rows = state
                .last_bidi_rows
                .as_ref()
                .expect("rok-ui lays out the rows");
            assert!(rows.len() > 1, "a long paragraph wraps");
            // Rows tile the text in logical order.
            assert_eq!(rows[0].range.start, 0);
            assert_eq!(rows.last().unwrap().range.end, text.len());
            for pair in rows.windows(2) {
                assert_eq!(pair[0].range.end, pair[1].range.start);
            }
            // The first row starts at the right and the second sits one line lower.
            let first = state.multiline_position_for_offset(0).unwrap();
            let second = state
                .multiline_position_for_offset(rows[1].range.start)
                .unwrap();
            assert!(first.x > second.x - px(1.) || first.y < second.y);
            assert_eq!(second.y, state.last_line_height);
            // Offsets inside a row survive a round trip through their position.
            for (offset, _) in text[..rows[0].range.end].char_indices().skip(1) {
                let position = state.multiline_position_for_offset(offset).unwrap();
                let back = state.multiline_index_for_point(point(position.x, position.y + px(1.)));
                assert_eq!(back, offset, "offset {offset} at {position:?}");
            }
        });
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
