//! The platform text input handler: typing, IME composition and the candidate window.

use std::ops::Range;

use gpui::{point, Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window};

use super::{state::utf8_offset, InputEvent, InputState};

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
        // The new selection is relative to `new_text`.
        self.selected_range = new_selected_utf16_range.map_or_else(
            || range.start + new_text.len()..range.start + new_text.len(),
            |selected| {
                range.start + utf8_offset(new_text, selected.start)
                    ..range.start + utf8_offset(new_text, selected.end)
            },
        );
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
