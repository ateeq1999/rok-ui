//! Textarea: a multi-line text field that grows with its content.

use gpui::{
    div, fill, point, prelude::*, px, relative, size, App, AvailableSpace, Bounds, CursorStyle,
    ElementId, ElementInputHandler, Entity, GlobalElementId, Hsla, LayoutId, MouseButton,
    PaintQuad, Pixels, SharedString, Size, Style, StyleRefinement, TextAlign, TextRun, Window,
    WrappedLine,
};

use super::{forward_to_state, InputState, INPUT_KEY_CONTEXT, TEXTAREA_KEY_CONTEXT};
use crate::components::direction::DirectionalStyled;
use crate::sx::SxStyled;
use crate::{components::focus_ring_outline, styles::ApplyStyleOverrides, theme::ActiveTheme};

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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Red border and ring, for validation errors.
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
            .flex_dir()
            .w_full()
            .min_h(px(64.))
            .px(px(12.))
            .py(px(8.))
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(if self.invalid || is_focused {
                accent
            } else {
                colors.input
            })
            .text_color(colors.foreground)
            .text_sm()
            .line_height(px(20.))
            .relative()
            .when(is_focused && self.focus_ring, |field| {
                field.child(focus_ring_outline(accent, theme.radius_medium()))
            })
            .when(self.disabled, |field| field.opacity(0.5))
            .when(!self.disabled, |field| {
                field
                    .key_context(INPUT_KEY_CONTEXT)
                    .track_focus(&focus_handle)
                    .cursor(CursorStyle::IBeam)
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
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides);

        // Up / Down bindings live in this outer context so plain inputs keep them free.
        div()
            .key_context(TEXTAREA_KEY_CONTEXT)
            .w_full()
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
    line_height: Pixels,
    selection: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
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
            (state.placeholder.clone(), self.placeholder_color)
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
        .map(|lines| lines.into_vec())
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
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let layout_id =
            window.request_measured_layout(style, move |known, available, window, _| {
                let wrap_width = known.width.or(match available.width {
                    AvailableSpace::Definite(width) => Some(width),
                    _ => None,
                });
                let lines = shape(text.clone(), &runs, wrap_width, window);
                size(
                    wrap_width.unwrap_or(px(0.)),
                    total_height(&lines, line_height),
                )
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
        let lines = shape(text, &runs, Some(bounds.size.width), window);
        let content_is_empty = self.state.read(cx).content.is_empty();

        // Store the layout first so offset <-> position lookups use this frame's wrapping.
        self.state.update(cx, |state, _| {
            state.last_wrapped_lines = if content_is_empty {
                Vec::new()
            } else {
                lines.clone()
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
                size(px(0.), px(0.))
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
                selection.push(to_quad(px(0.), end.height, end.width));
            }
        }

        TextareaPrepaint {
            lines,
            line_height,
            selection,
            cursor,
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
        let mut line_top = bounds.top();
        for line in &prepaint.lines {
            line.paint(
                point(bounds.left(), line_top),
                prepaint.line_height,
                TextAlign::Left,
                None,
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
