//! The element that shapes and paints a single-line field's text, cursor and selection.

use gpui::{
    fill, point, prelude::*, px, relative, size, App, Bounds, ElementId, ElementInputHandler,
    Entity, GlobalElementId, Hsla, LayoutId, PaintQuad, Pixels, Style, TextRun, UnderlineStyle,
    Window,
};

use super::{DisplayLine, InputState};

/// Low-level element that shapes and paints the text, cursor and selection.
pub(super) struct InputTextElement {
    pub(super) state: Entity<InputState>,
    pub(super) placeholder_color: Hsla,
    pub(super) cursor_color: Hsla,
    pub(super) selection_color: Hsla,
}

pub(super) struct InputTextPrepaintState {
    line: Option<DisplayLine>,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
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
        let line = DisplayLine::shape(display_text, &runs, font_size, window);

        // In RTL the text sits against the right edge. Shifting the bounds keeps
        // painting, the cursor, hit testing and IME positions consistent.
        let rtl = crate::components::direction::is_rtl();
        let text_offset = if rtl {
            (bounds.size.width - line.width()).max(px(0.))
        } else {
            px(0.)
        };
        let bounds = shift_bounds(bounds, text_offset);

        let (selection, cursor) = if selected_range.is_empty() {
            let cursor_x = if content_is_empty {
                if rtl {
                    line.width()
                } else {
                    px(0.)
                }
            } else {
                line.x_for_index(cursor_offset)
            };
            (
                Vec::new(),
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
                line.selection_spans(selected_range)
                    .into_iter()
                    .map(|(start, end)| {
                        fill(
                            Bounds::from_corners(
                                point(bounds.left() + start, bounds.top()),
                                point(bounds.left() + end, bounds.bottom()),
                            ),
                            self.selection_color,
                        )
                    })
                    .collect(),
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
        for selection in prepaint.selection.drain(..) {
            window.paint_quad(selection);
        }
        let Some(line) = prepaint.line.take() else {
            return;
        };
        line.paint(bounds.origin, window.line_height(), window, cx);

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
