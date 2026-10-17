//! Shaping field text: one line, or rows that rok-ui wraps itself for right-to-left text on
//! Windows (see [`crate::bidi`]), and positions in a multi-line field's rows.

use std::ops::Range;

use gpui::{point, px, App, Pixels, Point, ShapedLine, SharedString, TextRun, Window};

use super::InputState;

/// One visual row of a multi-line field that rok-ui lays out itself (right-to-left
/// text on Windows, see [`crate::bidi`]).
#[derive(Clone)]
#[doc(hidden)]
pub struct BidiRow {
    /// Byte range of the row in the field's content, including trailing spaces.
    pub range: Range<usize>,
    pub line: DisplayLine,
    /// How far the row is shifted right to align it (right-to-left alignment).
    pub left: Pixels,
}

/// Whether a multi-line field with `content` needs [`bidi_rows`] on this platform.
#[doc(hidden)]
#[must_use]
pub fn needs_bidi_rows(content: &str) -> bool {
    crate::bidi::platform_needs_reordering() && crate::bidi::has_rtl(content)
}

/// Wrap `text` into rows no wider than `width`, breaking between words in logical
/// order (explicit newlines always break), and shape each row for display.
#[doc(hidden)]
#[must_use]
pub fn bidi_rows(
    text: &str,
    run: &TextRun,
    font_size: Pixels,
    width: Option<Pixels>,
    align_right: bool,
    window: &Window,
) -> Vec<BidiRow> {
    let shape = |range: Range<usize>| {
        let row_text = SharedString::from(text[range].to_string());
        let run = TextRun {
            len: row_text.len(),
            ..run.clone()
        };
        DisplayLine::shape(row_text, &[run], font_size, window)
    };
    let mut ranges = Vec::new();
    let mut paragraph_start = 0;
    for paragraph in text.split('\n') {
        let paragraph_end = paragraph_start + paragraph.len();
        let mut row_start = paragraph_start;
        let mut row_end = paragraph_start;
        for word in paragraph.split_inclusive(' ') {
            let word_end = row_end + word.len();
            let fits = width.is_none_or(|width| {
                let candidate = text[row_start..word_end].trim_end();
                shape(row_start..row_start + candidate.len()).width() <= width
            });
            if !fits && row_end > row_start {
                ranges.push(row_start..row_end);
                row_start = row_end;
            }
            row_end = word_end;
        }
        ranges.push(row_start..paragraph_end);
        paragraph_start = paragraph_end + 1;
    }
    ranges
        .into_iter()
        .map(|range| {
            let line = shape(range.clone());
            let left = match width {
                Some(width) if align_right => (width - line.width()).max(px(0.)),
                _ => px(0.),
            };
            BidiRow { range, line, left }
        })
        .collect()
}

/// Where content `offset` is drawn, relative to the text's top-left.
fn bidi_row_position_for_offset(
    rows: &[BidiRow],
    offset: usize,
    line_height: Pixels,
) -> Option<Point<Pixels>> {
    for (index, row) in rows.iter().enumerate() {
        // At a soft wrap, the offset between two rows belongs to the second one.
        let continues_on_next_row = rows
            .get(index + 1)
            .is_some_and(|next| next.range.start == row.range.end);
        let inside = offset >= row.range.start
            && (offset < row.range.end || (offset == row.range.end && !continues_on_next_row));
        if inside {
            let x = row.left + row.line.x_for_index(offset - row.range.start);
            return Some(point(x, line_height * index as f32));
        }
    }
    None
}

/// The content offset nearest to `local` (relative to the text's top-left).
fn bidi_row_index_for_point(rows: &[BidiRow], local: Point<Pixels>, line_height: Pixels) -> usize {
    if rows.is_empty() {
        return 0;
    }
    let row_index = ((local.y / line_height).floor().max(0.) as usize).min(rows.len() - 1);
    let row = &rows[row_index];
    let index = row.line.closest_index_for_x(local.x - row.left);
    (row.range.start + index).min(row.range.end)
}

/// A shaped line of field text and, when it was reordered for display (right-to-left
/// text on Windows, see [`crate::bidi`]), the map from logical offsets to positions.
#[derive(Clone)]
#[doc(hidden)]
pub struct DisplayLine {
    pub(super) line: ShapedLine,
    pub(super) bidi: Option<crate::bidi::VisualLine>,
}

impl DisplayLine {
    /// Shape `text` (logical order). `runs` style the logical text and apply as-is
    /// when no reordering is needed; reordered text is drawn with the first run's style.
    pub(super) fn shape(
        text: SharedString,
        runs: &[TextRun],
        font_size: Pixels,
        window: &Window,
    ) -> Self {
        if crate::bidi::platform_needs_reordering() && crate::bidi::has_rtl(&text) {
            let direction = if crate::components::direction::is_rtl() {
                crate::components::direction::TextDirection::Rtl
            } else {
                crate::components::direction::TextDirection::Ltr
            };
            let visual = crate::bidi::visual_line_in_font(&text, direction, &runs[0].font.family);
            let run = TextRun {
                len: visual.text.len(),
                ..runs[0].clone()
            };
            let line = window.text_system().shape_line(
                visual.text.clone().into(),
                font_size,
                &[run],
                None,
            );
            return Self {
                line,
                bidi: Some(visual),
            };
        }
        let line = window.text_system().shape_line(text, font_size, runs, None);
        Self { line, bidi: None }
    }

    pub(super) fn width(&self) -> Pixels {
        self.line.width
    }

    /// The left and right edges of each logical character, with whether its run
    /// reads right to left.
    fn edges(&self) -> impl Iterator<Item = (&crate::bidi::VisualChar, Pixels, Pixels)> {
        self.bidi.iter().flat_map(move |bidi| {
            bidi.chars.iter().map(move |character| {
                let left = self.line.x_for_index(character.visual.start);
                let right = self.line.x_for_index(character.visual.end);
                (character, left.min(right), left.max(right))
            })
        })
    }

    /// Where the caret for logical offset `index` is drawn.
    pub(super) fn x_for_index(&self, index: usize) -> Pixels {
        if self.bidi.is_none() {
            return self.line.x_for_index(index);
        }
        let mut at_end = None;
        for (character, left, right) in self.edges() {
            // The caret before a character sits on its reading-start side.
            if character.logical.start == index {
                return if character.rtl { right } else { left };
            }
            if character.logical.end == index {
                at_end = Some(if character.rtl { left } else { right });
            }
        }
        at_end.unwrap_or(px(0.))
    }

    /// The logical offset whose caret position is closest to `x`.
    pub(super) fn closest_index_for_x(&self, x: Pixels) -> usize {
        if self.bidi.is_none() {
            return self.line.closest_index_for_x(x);
        }
        let mut best = (Pixels::MAX, 0);
        for (character, left, right) in self.edges() {
            let (left_index, right_index) = if character.rtl {
                (character.logical.end, character.logical.start)
            } else {
                (character.logical.start, character.logical.end)
            };
            for (edge, index) in [(left, left_index), (right, right_index)] {
                let distance = (x - edge).abs();
                if distance < best.0 {
                    best = (distance, index);
                }
            }
        }
        best.1
    }

    /// The logical offset at `x`, or `None` past either end of the text.
    pub(super) fn index_for_x(&self, x: Pixels) -> Option<usize> {
        if self.bidi.is_none() {
            return self.line.index_for_x(x);
        }
        (x >= px(0.) && x <= self.width()).then(|| self.closest_index_for_x(x))
    }

    /// Horizontal spans covering the logical `range`. Mixed-direction text can need
    /// several.
    pub(super) fn selection_spans(&self, range: Range<usize>) -> Vec<(Pixels, Pixels)> {
        if self.bidi.is_none() {
            return vec![(
                self.line.x_for_index(range.start),
                self.line.x_for_index(range.end),
            )];
        }
        let mut spans: Vec<(Pixels, Pixels)> = self
            .edges()
            .filter(|(character, _, _)| {
                character.logical.start >= range.start && character.logical.end <= range.end
            })
            .map(|(_, left, right)| (left, right))
            .collect();
        spans.sort_by(|a, b| f32::from(a.0).total_cmp(&f32::from(b.0)));
        let mut merged: Vec<(Pixels, Pixels)> = Vec::new();
        for (left, right) in spans {
            match merged.last_mut() {
                Some(last) if left <= last.1 + px(0.5) => last.1 = last.1.max(right),
                _ => merged.push((left, right)),
            }
        }
        merged
    }

    pub(super) fn paint(
        &self,
        origin: Point<Pixels>,
        line_height: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.line.paint(origin, line_height, window, cx).ok();
    }
}

/// Positions in a multi-line field, from the rows of its last paint.
impl InputState {
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
    fn row_offset(&self, paragraph: usize, y: Pixels) -> Pixels {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{prelude::*, Context};

    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            gpui::div()
        }
    }

    #[gpui::test]
    fn reordered_lines_map_carets_and_clicks(cx: &mut gpui::TestAppContext) {
        // Only Windows reorders; elsewhere the platform shaper handles direction.
        if !crate::bidi::platform_needs_reordering() {
            return;
        }
        cx.update(crate::init);
        let (_view, window_context) = cx.add_window_view(|_, _| EmptyView);
        window_context.update(|window, _| {
            let text = SharedString::from("مرحبا");
            let run = window.text_style().to_run(text.len());
            let line = crate::components::direction::with_direction(
                crate::components::TextDirection::Rtl,
                || DisplayLine::shape(text.clone(), &[run], px(16.), window),
            );
            let width = line.width();
            assert!(width > px(0.));
            // Right-to-left: the caret starts at the right and ends at the left.
            let (start, end) = (line.x_for_index(0), line.x_for_index(text.len()));
            assert!(start > end);
            // Each letter moves the caret leftward.
            let mut previous = line.x_for_index(0);
            for (offset, _) in text.char_indices().skip(1) {
                let x = line.x_for_index(offset);
                assert!(x < previous, "caret at {offset} did not move left");
                previous = x;
            }
            // Clicking an end puts the caret at the logical start or end.
            assert_eq!(line.closest_index_for_x(width + px(5.)), 0);
            assert_eq!(line.closest_index_for_x(px(-5.)), text.len());
            // Selecting everything highlights the whole line in one span.
            let spans = line.selection_spans(0..text.len());
            assert_eq!(spans.len(), 1);
            assert!((spans[0].0 - end).abs() < px(0.5) && (spans[0].1 - start).abs() < px(0.5));
        });
    }
}
