//! Small interaction helpers shared by several components: pointer drags that
//! keep tracking outside the element, bounds measurement, keyboard activation
//! and the modal layer behind dialogs, sheets and drawers.

// Shared by optional components; parts go unused in partial feature builds.
#![cfg_attr(not(feature = "full"), allow(dead_code, unused_imports))]

use std::{cell::Cell, rc::Rc};

use gpui::{
    canvas, div, point, prelude::*, px, AnyElement, App, Bounds, Corner, DispatchPhase, Div,
    ElementId, MouseMoveEvent, MouseUpEvent, Pixels, Point, Stateful, Window,
};

use super::direction::DirectionalStyled;
use super::layer::layer_at;
use super::overlay::child_id;
use crate::{
    motion::{use_presence, Presence, Transition},
    theme::ActiveTheme,
};

/// A handler that takes no event, shared between several listeners.
pub(crate) type Callback = Rc<dyn Fn(&mut Window, &mut App)>;

/// Records the bounds of the element it is placed in (absolutely, full size).
pub(crate) fn measure_bounds(bounds: Rc<Cell<Bounds<Pixels>>>) -> impl IntoElement {
    canvas(move |measured, _, _| bounds.set(measured), |_, _, _, _| {})
        .absolute()
        .top_0()
        .left_0()
        .size_full()
}

/// Receives each pointer position during a drag.
pub(crate) type PointerHandler = Rc<dyn Fn(Point<Pixels>, &mut Window, &mut App)>;

/// While `active`, follow the pointer anywhere in the window: `on_move` gets
/// every position and `on_end` runs once on mouse up. Place it anywhere in the tree.
pub(crate) fn track_drag(
    active: bool,
    on_move: PointerHandler,
    on_end: Callback,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            if !active {
                return;
            }
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble && event.dragging() {
                    on_move(event.position, window, cx);
                }
            });
            window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble {
                    on_end(window, cx);
                }
            });
        },
    )
    .absolute()
    .size_0()
}

/// Run `handler` on a mouse click, and on Enter / Space while the element (or a
/// focusable child, like a [`super::Button`] used as a trigger) has focus.
pub(crate) fn on_activate(element: Stateful<Div>, handler: Callback) -> Stateful<Div> {
    let key_handler = handler.clone();
    element
        .on_click(move |event, window, cx| {
            if !event.is_keyboard() {
                handler(window, cx)
            }
        })
        .on_key_down(move |event, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                cx.stop_propagation();
                key_handler(window, cx);
            }
        })
}

/// Where a modal panel sits in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModalPlacement {
    Center,
    Top,
    Bottom,
    Left,
    Right,
}

/// Draw `panel` over a scrim covering the window, with focus moved inside.
/// Escape calls `on_escape`; a press on the scrim calls `on_backdrop` (if any).
/// Enter / exit progress for a modal: keeps it mounted while it animates out.
/// Call it on every render, open or not, so the transition has a start value.
pub(crate) fn modal_presence(
    id: &ElementId,
    open: bool,
    window: &mut Window,
    cx: &mut App,
) -> Presence {
    use_presence(
        child_id(id, "presence"),
        window,
        cx,
        open,
        Transition::ease_out(180),
    )
}

/// Offset of a modal panel at `progress`: dialogs rise a little, sheets and
/// drawers slide in from their edge.
pub(crate) fn modal_offset(placement: ModalPlacement, progress: f32) -> (Pixels, Pixels) {
    let remaining = 1. - progress;
    match placement {
        ModalPlacement::Center => (px(0.), px(8. * remaining)),
        ModalPlacement::Top => (px(0.), px(-320. * remaining)),
        ModalPlacement::Bottom => (px(0.), px(320. * remaining)),
        ModalPlacement::Left => (px(-384. * remaining), px(0.)),
        ModalPlacement::Right => (px(384. * remaining), px(0.)),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_modal(
    id: ElementId,
    placement: ModalPlacement,
    progress: f32,
    panel: Div,
    on_escape: Option<Callback>,
    on_backdrop: Option<Callback>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    // Focus moves inside once, when the modal opens. (Element state is dropped
    // while the modal is closed, so reopening starts fresh.) Grabbing focus on
    // every frame instead would make stacked modals fight over it forever.
    let (focus_handle, focus_pending) = window
        .use_keyed_state(child_id(&id, "modal-focus"), cx, |_, cx| {
            (cx.focus_handle(), Rc::new(Cell::new(true)))
        })
        .read(cx)
        .clone();
    if focus_pending.replace(false) {
        let focus_handle_to_focus = focus_handle.clone();
        window.defer(cx, move |window, _| window.focus(&focus_handle_to_focus));
    }

    let theme = cx.theme();
    let viewport_size = window.viewport_size();
    let (offset_x, offset_y) = modal_offset(placement, progress);
    let panel = panel
        .relative()
        .left(offset_x)
        .top(offset_y)
        .track_focus(&focus_handle)
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_key_down(move |event, window, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                if let Some(on_escape) = on_escape.as_ref() {
                    on_escape(window, cx);
                }
            }
        });

    let scrim = div()
        .id(id)
        .occlude()
        .w(viewport_size.width)
        .h(viewport_size.height)
        .flex_ltr()
        .bg(theme.colors.overlay.opacity(progress))
        .font_family(theme.font_family.clone())
        .text_size(theme.font_size)
        .text_color(theme.colors.foreground)
        .when(super::direction::is_rtl(), |scrim| scrim.text_right())
        .map(|scrim| match placement {
            ModalPlacement::Center => scrim.items_center().justify_center().p(px(16.)),
            ModalPlacement::Top => scrim.flex_col().justify_start(),
            ModalPlacement::Bottom => scrim.flex_col().justify_end(),
            ModalPlacement::Left => scrim.justify_start(),
            ModalPlacement::Right => scrim.justify_end(),
        })
        .when_some(on_backdrop, |scrim, on_backdrop| {
            scrim.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                on_backdrop(window, cx)
            })
        })
        .child(panel.opacity(progress));

    layer_at(point(px(0.), px(0.)), Corner::TopLeft, px(0.), scrim, 1, cx)
}
