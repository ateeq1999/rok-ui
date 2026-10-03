//! Textarea: a multi-line text field that grows with its content.

use gpui::{
    div, fill, point, prelude::*, px, relative, size, App, AvailableSpace, Bounds, ElementId,
    ElementInputHandler, Entity, GlobalElementId, Hsla, LayoutId, MouseButton, PaintQuad, Pixels,
    SharedString, Size, Style, StyleRefinement, TextAlign, TextRun, Window, WrappedLine,
};

use super::{
    bidi_rows, forward_to_state, needs_bidi_rows, BidiRow, InputState, INPUT, INPUT_KEY_CONTEXT,
    TEXTAREA_KEY_CONTEXT,
};
use crate::sx::SxStyled;
use crate::{
    components::focus_ring_outline, styles, styles::ApplyStyleOverrides, theme::ActiveTheme,
};

/// `useRef`-style hook for a multi-line [`InputState`].
pub fn use_textarea_state(
    key: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    configure: impl FnOnce(InputState) -> InputState,
) -> Entity<InputState> {
    window.use_keyed_state(key, cx, |_, cx| {
        configure(InputState::new(cx).with_multiline(true))
    })
}

/// shadcn/ui's `<Textarea>`. Enter adds a line; Ctrl/Cmd-Enter emits
/// [`super::InputEvent::Submitted`]. It is at least 64px tall and grows with the text.
///
/// ```ignore
/// let message = use_textarea_state("message", window, cx, |state| {
///     state.with_placeholder("Type your message here.")
/// });
/// Textarea::new(&message)
/// ```
#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<InputState>,
    disabled: bool,
    invalid: bool,
    focus_ring: bool,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Textarea);

impl Textarea {
    /// `state` should be multi-line: create it with [`use_textarea_state`] or
    /// `InputState::new(cx).with_multiline(true)`.
    #[must_use]
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            disabled: false,
            invalid: false,
            focus_ring: true,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Red border and ring, for validation errors.
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
    TEXTAREA = {
        outer: { width: full },
        field: {
            position: relative,
            display: flex,
            width: full,
            min_height: 16,
            padding_x: 3,
            padding_y: 2,
            radius: md,
            border: 1,
            border_color: input,
            color: foreground,
            text: sm,
            line_height: 5,
        },
    }
}

impl RenderOnce for Textarea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);
        let accent = if self.invalid {
            colors.destructive_text
        } else {
            colors.ring
        };
        let state = self.state.clone();

        let field = div()
            .id(ElementId::NamedInteger(
                "rok-ui-textarea".into(),
                self.state.entity_id().as_u64(),
            ))
            .sx((
                &TEXTAREA.field,
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
                field.child(focus_ring_outline(accent, theme.radius_medium()))
            })
            .when(!self.disabled, |field| {
                field
                    .key_context(INPUT_KEY_CONTEXT)
                    .track_focus(&focus_handle)
                    .on_action(forward_to_state(&state, InputState::delete_backward))
                    .on_action(forward_to_state(&state, InputState::delete_forward))
                    .on_action(forward_to_state(&state, InputState::move_left))
                    .on_action(forward_to_state(&state, InputState::move_right))
                    .on_action(forward_to_state(&state, InputState::move_up))
                    .on_action(forward_to_state(&state, InputState::move_down))
                    .on_action(forward_to_state(&state, InputState::select_left))
                    .on_action(forward_to_state(&state, InputState::select_right))
                    .on_action(forward_to_state(&state, InputState::select_up))
                    .on_action(forward_to_state(&state, InputState::select_down))
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
            .child(TextareaTextElement {
                state: self.state,
                placeholder_color: colors.muted_foreground,
                cursor_color: colors.foreground,
                selection_color: colors.ring.opacity(0.3),
            })
            .apply_style_overrides(&self.style_overrides);

        // Up / Down bindings live in this outer context so plain inputs keep them free.
        div()
            .key_context(TEXTAREA_KEY_CONTEXT)
            .sx(&TEXTAREA.outer)
            .child(field)
    }
}

/// Shapes, wraps and paints the text, selection and cursor of a multi-line field.
struct TextareaTextElement {
    state: Entity<InputState>,
    placeholder_color: Hsla,
    cursor_color: Hsla,
    selection_color: Hsla,
}

struct TextareaPrepaint {
    lines: Vec<WrappedLine>,
    /// Rows laid out by rok-ui (right-to-left text on Windows); replace `lines`.
    rows: Vec<BidiRow>,
    line_height: Pixels,
    selection: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    rtl: bool,
}

/// How far each visual row of `line` is pushed right when right-aligned to `width`
/// (mirrors gpui's `TextAlign::Right` placement).
fn right_aligned_row_offsets(line: &WrappedLine, width: Pixels) -> Vec<Pixels> {
    let layout = &line.unwrapped_layout;
    let boundary_x = |b: &gpui::WrapBoundary| layout.runs[b.run_ix].glyphs[b.glyph_ix].position.x;
    let mut starts = vec![px(0.)];
    starts.extend(line.wrap_boundaries.iter().map(boundary_x));
    let mut ends: Vec<Pixels> = starts.iter().skip(1).copied().collect();
    ends.push(layout.width);
    starts
        .iter()
        .zip(ends)
        .map(|(start, end)| (width - (end - *start)).max(px(0.)))
        .collect()
}

impl IntoElement for TextareaTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl TextareaTextElement {
    /// The text to draw and its runs: the content, or the placeholder when empty.
    fn display_text(&self, window: &Window, cx: &App) -> (SharedString, Vec<TextRun>) {
        let state = self.state.read(cx);
        let text_style = window.text_style();
        let (text, color) = if state.content.is_empty() {
            let direction = if crate::components::direction::is_rtl() {
                crate::components::direction::TextDirection::Rtl
            } else {
                crate::components::direction::TextDirection::Ltr
            };
            (
                crate::bidi::display_text(state.placeholder.clone(), direction),
                self.placeholder_color,
            )
        } else {
            (state.content.clone(), text_style.color)
        };
        let run = TextRun {
            len: text.len(),
            font: text_style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        (text, vec![run])
    }
}

fn shape(
    text: SharedString,
    runs: &[TextRun],
    wrap_width: Option<Pixels>,
    window: &Window,
) -> Vec<WrappedLine> {
    let font_size = window.text_style().font_size.to_pixels(window.rem_size());
    window
        .text_system()
        .shape_text(text, font_size, runs, wrap_width, None)
        .map(Vec::from_iter)
        .unwrap_or_default()
}

fn total_height(lines: &[WrappedLine], line_height: Pixels) -> Pixels {
    lines
        .iter()
        .map(|line| line.size(line_height).height)
        .fold(px(0.), |total, height| total + height)
        .max(line_height)
}

impl Element for TextareaTextElement {
    type RequestLayoutState = ();
    type PrepaintState = TextareaPrepaint;

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
        let (text, runs) = self.display_text(window, cx);
        let line_height = window.line_height();
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let reorder = !self.state.read(cx).content.is_empty() && needs_bidi_rows(&text);
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let layout_id =
            window.request_measured_layout(style, move |known, available, window, _| {
                let wrap_width = known.width.or(match available.width {
                    AvailableSpace::Definite(width) => Some(width),
                    _ => None,
                });
                let height = if reorder {
                    let rows = bidi_rows(&text, &runs[0], font_size, wrap_width, false, window);
                    (line_height * rows.len() as f32).max(line_height)
                } else {
                    total_height(&shape(text.clone(), &runs, wrap_width, window), line_height)
                };
                size(wrap_width.unwrap_or(px(0.)), height)
            });
        (layout_id, ())
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
        let (text, runs) = self.display_text(window, cx);
        let line_height = window.line_height();
        let content_is_empty = self.state.read(cx).content.is_empty();
        let rtl = crate::components::direction::is_rtl();
        // Right-to-left text on Windows: rok-ui wraps and reorders the rows itself.
        let reorder = !content_is_empty && needs_bidi_rows(&text);
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let rows = if reorder {
            bidi_rows(
                &text,
                &runs[0],
                font_size,
                Some(bounds.size.width),
                rtl,
                window,
            )
        } else {
            Vec::new()
        };
        let lines = if reorder {
            Vec::new()
        } else {
            shape(text, &runs, Some(bounds.size.width), window)
        };

        // Store the layout first so offset <-> position lookups use this frame's wrapping.
        self.state.update(cx, |state, _| {
            state.last_bidi_rows = reorder.then(|| rows.clone());
            state.last_wrapped_lines = if content_is_empty {
                Vec::new()
            } else {
                lines.clone()
            };
            state.last_row_offsets = if rtl && !content_is_empty {
                lines
                    .iter()
                    .map(|line| right_aligned_row_offsets(line, bounds.size.width))
                    .collect()
            } else {
                Vec::new()
            };
            state.last_line_height = line_height;
            state.last_bounds = Some(bounds);
        });

        let state = self.state.read(cx);
        let position = |offset: usize| -> Size<Pixels> {
            let point = state
                .multiline_position_for_offset(offset)
                .unwrap_or_default();
            size(point.x, point.y)
        };
        let to_quad = |left: Pixels, top: Pixels, right: Pixels| {
            fill(
                Bounds::from_corners(
                    point(bounds.left() + left, bounds.top() + top),
                    point(bounds.left() + right, bounds.top() + top + line_height),
                ),
                self.selection_color,
            )
        };

        let range = state.selected_range.clone();
        let mut selection = Vec::new();
        let mut cursor = None;
        if range.is_empty() || content_is_empty {
            let at = if content_is_empty {
                // An empty RTL field starts typing from the right edge.
                let x = if rtl {
                    bounds.size.width - px(1.5)
                } else {
                    px(0.)
                };
                size(x, px(0.))
            } else {
                position(state.cursor_offset())
            };
            cursor = Some(fill(
                Bounds::new(
                    point(bounds.left() + at.width, bounds.top() + at.height),
                    size(px(1.5), line_height),
                ),
                self.cursor_color,
            ));
        } else if reorder {
            // Mixed-direction rows can need several highlight spans each.
            for (index, row) in rows.iter().enumerate() {
                let start = range.start.max(row.range.start);
                let end = range.end.min(row.range.end);
                if start >= end {
                    continue;
                }
                let top = line_height * index as f32;
                for (left, right) in row
                    .line
                    .selection_spans(start - row.range.start..end - row.range.start)
                {
                    selection.push(to_quad(row.left + left, top, row.left + right));
                }
            }
        } else {
            let start = position(range.start);
            let end = position(range.end);
            if start.height == end.height {
                selection.push(to_quad(start.width, start.height, end.width));
            } else {
                selection.push(to_quad(start.width, start.height, bounds.size.width));
                let mut row = start.height + line_height;
                while row < end.height {
                    selection.push(to_quad(px(0.), row, bounds.size.width));
                    row += line_height;
                }
                let last_left = state.multiline_row_left(end.height + line_height / 2.);
                selection.push(to_quad(last_left.min(end.width), end.height, end.width));
            }
        }

        TextareaPrepaint {
            lines,
            rows,
            line_height,
            selection,
            cursor,
            rtl,
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
        let focus_handle = self.state.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );
        for quad in prepaint.selection.drain(..) {
            window.paint_quad(quad);
        }
        for (index, row) in prepaint.rows.iter().enumerate() {
            let top = bounds.top() + prepaint.line_height * index as f32;
            row.line.paint(
                point(bounds.left() + row.left, top),
                prepaint.line_height,
                window,
                cx,
            );
        }
        let mut line_top = bounds.top();
        for line in &prepaint.lines {
            line.paint(
                point(bounds.left(), line_top),
                prepaint.line_height,
                if prepaint.rtl {
                    TextAlign::Right
                } else {
                    TextAlign::Left
                },
                Some(bounds),
                window,
                cx,
            )
            .ok();
            line_top += line.size(prepaint.line_height).height;
        }
        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }
    }
}
