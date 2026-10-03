//! Small interaction helpers shared by several components: pointer drags that
//! keep tracking outside the element, bounds measurement, keyboard activation
//! and the modal layer behind dialogs, sheets and drawers.

// Shared by optional components; parts go unused in partial feature builds.
#![cfg_attr(not(feature = "full"), allow(dead_code, unused_imports))]

use std::{cell::Cell, rc::Rc};

use gpui::{
    canvas, div, point, prelude::*, px, AnyElement, App, Bounds, Corner, DispatchPhase, Div,
    ElementId, FocusHandle, MouseMoveEvent, MouseUpEvent, Pixels, Point, Stateful, Window,
};

use super::app_root::{FocusNextElement, FocusPreviousElement};
use super::layer::layer_at;
use super::overlay::child_id;
use crate::sx::SxStyled;
use crate::{
    motion::{use_presence, Presence, Transition},
    styles,
    theme::ActiveTheme,
};

/// A handler that takes no event, shared between several listeners.
pub(crate) type Callback = Rc<dyn Fn(&mut Window, &mut App)>;

/// Where a modal is in moving focus inside when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModalFocus {
    /// Just opened: focus goes to the panel, so Escape works at once.
    Opening,
    /// The panel is focused; once it has been laid out, focus moves to its first field.
    PanelFocused,
    /// Done. Focus is left to the user (moving it every frame would make stacked
    /// modals fight over it).
    Settled,
}

/// Call on every render of a modal. Moves focus to the panel when the modal opens,
/// then on the following render, when the panel's elements are in the tab order,
/// to the first focusable element inside it.
pub(crate) fn focus_modal_on_open(
    panel: &FocusHandle,
    phase: &Cell<ModalFocus>,
    window: &mut Window,
    cx: &mut App,
) {
    match phase.get() {
        ModalFocus::Opening => {
            phase.set(ModalFocus::PanelFocused);
            let panel = panel.clone();
            window.defer(cx, move |window, _| window.focus(&panel));
        }
        ModalFocus::PanelFocused => {
            phase.set(ModalFocus::Settled);
            let panel = panel.clone();
            window.defer(cx, move |window, cx| {
                // Leave focus alone if something inside already took it.
                if panel.is_focused(window) {
                    focus_first_inside(&panel, window, cx);
                }
            });
        }
        ModalFocus::Settled => {}
    }
}

/// Keep Tab and Shift-Tab inside `panel`: leaving past the last element wraps to
/// the first, and back past the first wraps to the last.
pub(crate) fn trap_focus<E: InteractiveElement>(element: E, panel: &FocusHandle) -> E {
    let forward_panel = panel.clone();
    let backward_panel = panel.clone();
    element
        .on_action(move |_: &FocusNextElement, window, cx| {
            window.focus_next();
            if !is_inside(&forward_panel, window, cx) {
                focus_first_inside(&forward_panel, window, cx);
            }
        })
        .on_action(move |_: &FocusPreviousElement, window, cx| {
            window.focus_prev();
            if !is_inside(&backward_panel, window, cx) {
                focus_last_inside(&backward_panel, window, cx);
            }
        })
}

/// Whether focus is on an element inside `panel` (not the panel itself).
fn is_inside(panel: &FocusHandle, window: &Window, cx: &App) -> bool {
    panel.contains_focused(window, cx) && !panel.is_focused(window)
}

/// Tab order runs through the panel's descendants right after the panel itself.
fn focus_first_inside(panel: &FocusHandle, window: &mut Window, cx: &App) {
    window.focus(panel);
    window.focus_next();
    if !is_inside(panel, window, cx) {
        // Nothing focusable inside: keep focus on the panel.
        window.focus(panel);
    }
}

fn focus_last_inside(panel: &FocusHandle, window: &mut Window, cx: &App) {
    focus_first_inside(panel, window, cx);
    let mut last = window.focused(cx);
    // Bounded, in case the tab order changes under us.
    for _ in 0..1024 {
        window.focus_next();
        let current = window.focused(cx);
        if !is_inside(panel, window, cx) || current == last {
            break;
        }
        last = current;
    }
    match last {
        Some(handle) => window.focus(&handle),
        None => window.focus(panel),
    }
}

/// Records the bounds of the element it is placed in (absolutely, full size).
pub(crate) fn measure_bounds(bounds: Rc<Cell<Bounds<Pixels>>>) -> impl IntoElement {
    canvas(move |measured, _, _| bounds.set(measured), |_, (), _, _| {})
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
        move |_, (), window, _| {
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
                handler(window, cx);
            }
        })
        .on_key_down(move |event, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                cx.stop_propagation();
                key_handler(window, cx);
            }
        })
}

styles! {
    MODAL = {
        // Layers lay out apart from the window root, so text alignment is set here.
        // Panels slide in from physical edges (sheets already swap sides for RTL).
        scrim: {
            display: flex,
            direction: row_ltr,
            color: foreground,
            font_family: sans,
            text: theme,
            text_align: start,
        },
        placement(ModalPlacement): {
            Center: { align: center, justify: center, padding: 4 },
            Top: { direction: column, justify: start },
            Bottom: { direction: column, justify: end },
            Left: { justify: start },
            Right: { justify: end },
        },
    }
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
            (cx.focus_handle(), Rc::new(Cell::new(ModalFocus::Opening)))
        })
        .read(cx)
        .clone();
    focus_modal_on_open(&focus_handle, &focus_pending, window, cx);

    let theme = cx.theme();
    let viewport_size = window.viewport_size();
    let (offset_x, offset_y) = modal_offset(placement, progress);
    let panel = trap_focus(panel, &focus_handle);
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
        .sx((&MODAL.scrim, MODAL.placement(placement)))
        .bg(theme.colors.overlay.opacity(progress))
        .when_some(on_backdrop, |scrim, on_backdrop| {
            scrim.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                on_backdrop(window, cx);
            })
        })
        .child(panel.opacity(progress));

    layer_at(point(px(0.), px(0.)), Corner::TopLeft, px(0.), scrim, 1, cx)
}
