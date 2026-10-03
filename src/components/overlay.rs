//! Shared plumbing for floating surfaces: popovers, menus, selects and hover cards.
//!
//! - [`OpenState`]: open/closed state that is uncontrolled by default (kept per
//!   element id, like Radix) and becomes controlled when the caller passes `open`.
//! - [`floating`]: places content next to its trigger, on top of everything, and
//!   keeps it inside the window.
//! - [`popover_surface`]: the bordered, shadowed panel every floating surface uses.

// Shared by optional components; parts go unused in partial feature builds.
#![cfg_attr(not(feature = "full"), allow(dead_code, unused_imports))]

use std::{cell::Cell, rc::Rc};

use gpui::{
    div, prelude::*, px, relative, App, Corner, Div, ElementId, FocusHandle, Pixels, Point, Window,
};

use super::layer::layer_at_marker;
use crate::motion::{presets, Motion, MotionExt, MotionSide};

use crate::sx::SxStyled;
use crate::{
    hooks::{EventHandler, State},
    styles,
};

/// Which side of the trigger a floating surface opens on. `Left` and `Right`
/// swap in RTL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Side {
    /// Above the trigger.
    Top,
    /// Below the trigger.
    #[default]
    Bottom,
    /// Left of the trigger.
    Left,
    /// Right of the trigger.
    Right,
}

/// How a floating surface lines up with its trigger along the trigger's edge.
/// Logical: `Start` is the left edge in LTR and the right edge in RTL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Align {
    /// Left edges line up (top edges for `Left` / `Right` sides).
    #[default]
    Start,
    /// Right edges line up (bottom edges for `Left` / `Right` sides).
    End,
}

/// `ElementId` for a named part of a component, derived from the component's id.
#[doc(hidden)]
#[must_use]
pub fn child_id(parent: &ElementId, part: &'static str) -> ElementId {
    ElementId::NamedChild(Box::new(parent.clone()), part.into())
}

/// Per-instance bookkeeping that survives between frames.
#[doc(hidden)]
pub struct OverlayMemory {
    pub open: bool,
    /// Set when the surface opens, so focus moves into it on the next render only.
    pub focus_requested: Rc<Cell<bool>>,
    pub focus_handle: FocusHandle,
    /// Width of the trigger, measured each frame, for surfaces that match it.
    pub trigger_width: Rc<Cell<Pixels>>,
    /// When this surface opened relative to the others (see [`next_open_order`]);
    /// 0 while closed.
    pub open_order: Rc<Cell<u64>>,
}

/// Open/closed state for one floating surface. Cheap to clone into handlers.
#[derive(Clone)]
#[doc(hidden)]
pub struct OpenState {
    memory: State<OverlayMemory>,
    controlled_open: Option<bool>,
    on_open_change: Option<EventHandler<bool>>,
}

impl OpenState {
    pub fn is_open(&self, cx: &App) -> bool {
        self.controlled_open
            .unwrap_or_else(|| self.memory.read(cx).open)
    }

    /// Open or close, notifying `on_open_change`. Opening also moves focus inside.
    pub fn set_open(&self, open: bool, window: &mut Window, cx: &mut App) {
        if open {
            self.memory.read(cx).focus_requested.set(true);
        }
        self.memory.update(cx, |memory| memory.open = open);
        if let Some(handler) = self.on_open_change.as_ref() {
            handler(&open, window, cx);
        }
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.memory.read(cx).focus_handle.clone()
    }

    pub fn trigger_width(&self, cx: &App) -> Rc<Cell<Pixels>> {
        self.memory.read(cx).trigger_width.clone()
    }

    /// Call while rendering the open surface: focuses it once, right after it opens.
    pub fn focus_if_requested(&self, window: &mut Window, cx: &mut App) {
        let memory = self.memory.read(cx);
        if memory.focus_requested.replace(false) {
            let focus_handle = memory.focus_handle.clone();
            window.defer(cx, move |window, _| window.focus(&focus_handle));
        }
    }
}

/// Open state for the component with `id`. Pass `controlled_open` to control it.
#[doc(hidden)]
pub fn use_open_state(
    id: &ElementId,
    controlled_open: Option<bool>,
    on_open_change: Option<EventHandler<bool>>,
    window: &mut Window,
    cx: &mut App,
) -> OpenState {
    let memory = window.use_keyed_state(child_id(id, "overlay"), cx, |_, cx| OverlayMemory {
        open: false,
        focus_requested: Rc::new(Cell::new(controlled_open == Some(true))),
        focus_handle: cx.focus_handle(),
        trigger_width: Rc::new(Cell::new(px(0.))),
        open_order: Rc::new(Cell::new(0)),
    });
    let memory = State::from_entity(memory);
    let open_state = OpenState {
        memory,
        controlled_open,
        on_open_change,
    };
    if !open_state.is_open(cx) {
        open_state.memory.read(cx).open_order.set(0);
    }
    open_state
}

thread_local! {
    static OPEN_COUNTER: Cell<u64> = const { Cell::new(0) };
    /// The latest mouse press that landed inside a floating surface: where it was,
    /// and the open order of the newest surface under it.
    static CLAIMED_PRESS: Cell<Option<(Point<Pixels>, u64)>> = const { Cell::new(None) };
}

/// A number larger than that of every surface opened before.
fn next_open_order() -> u64 {
    OPEN_COUNTER.with(|counter| {
        counter.set(counter.get() + 1);
        counter.get()
    })
}

/// Position `content` against the edge of the element this is a child of. The
/// parent must be `relative()`. The content draws above everything else and is
/// shifted back inside the window when it would overflow.
#[doc(hidden)]
pub fn floating(side: Side, align: Align, content: impl IntoElement, cx: &mut App) -> Div {
    // Sides and alignment are logical: in RTL, start is the right edge and a
    // surface opening "right" (a submenu) opens left.
    let (side, align) = if super::direction::is_rtl() {
        let side = match side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            other => other,
        };
        let align = match align {
            Align::Start => Align::End,
            Align::End => Align::Start,
        };
        (side, align)
    } else {
        (side, align)
    };
    let gap = px(4.);
    let anchor = match (side, align) {
        (Side::Bottom | Side::Right, Align::Start) => Corner::TopLeft,
        (Side::Bottom, Align::End) | (Side::Left, Align::Start) => Corner::TopRight,
        (Side::Top, Align::Start) | (Side::Right, Align::End) => Corner::BottomLeft,
        (Side::Top | Side::Left, Align::End) => Corner::BottomRight,
    };
    let content = div()
        .map(|spacer| match side {
            Side::Top => spacer.pb(gap),
            Side::Bottom => spacer.pt(gap),
            Side::Left => spacer.pr(gap),
            Side::Right => spacer.pl(gap),
        })
        .child(
            div()
                .child(content)
                .motion("rok-ui-floating-enter", enter_motion(side)),
        );
    let layer = layer_at_marker(anchor, content, cx);

    // A zero-size marker at the anchor point; `anchored` opens from its origin.
    let marker = div().absolute().size_0();
    let marker = match (side, align) {
        (Side::Bottom, Align::Start) => marker.left_0().top(relative(1.)),
        (Side::Bottom, Align::End) => marker.right_0().top(relative(1.)),
        (Side::Top, Align::Start) => marker.left_0().top_0(),
        (Side::Top, Align::End) => marker.right_0().top_0(),
        (Side::Right, Align::Start) => marker.left(relative(1.)).top_0(),
        (Side::Right, Align::End) => marker.left(relative(1.)).bottom_0(),
        (Side::Left, Align::Start) => marker.left_0().top_0(),
        (Side::Left, Align::End) => marker.left_0().bottom_0(),
    };
    marker.child(layer)
}

styles! {
    OVERLAY = {
        // Layers lay out apart from the window root, so text alignment is set here.
        surface: {
            text_align: start,
            display: flex,
            direction: column,
            radius: md,
            border: 1,
            border_color: border,
            background: popover,
            color: popover_foreground,
            font_family: sans,
            text: theme,
            shadow: md,
        },
    }
}

/// The panel style shared by popovers, menus and hover cards (`bg-popover`,
/// border, `rounded-md`, `shadow-md`).
#[doc(hidden)]
#[must_use]
pub fn popover_surface() -> Div {
    div().sx(&OVERLAY.surface)
}

/// Records the width of the element it is placed in (absolutely, full size) into `width`.
#[doc(hidden)]
pub fn measure_width(width: Rc<Cell<Pixels>>) -> impl IntoElement {
    gpui::canvas(
        move |bounds, _, _| width.set(bounds.size.width),
        |_, (), _, _| {},
    )
    .absolute()
    .size_full()
}

/// The element that wraps a trigger: toggles `open_state` on mouse down and on
/// Enter / Space / Down while the trigger (not the open surface) has focus.
///
/// Toggling on mouse down with the open value captured at render time means a
/// click on the trigger while open closes the surface instead of reopening it
/// (the surface's own click-outside handler has already run by then).
#[doc(hidden)]
pub fn trigger_wrapper(
    id: ElementId,
    open_state: &OpenState,
    disabled: bool,
    cx: &App,
) -> gpui::Stateful<Div> {
    let was_open = open_state.is_open(cx);
    let mouse_state = open_state.clone();
    let key_state = open_state.clone();
    div().id(id).relative().when(!disabled, |wrapper| {
        wrapper
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                mouse_state.set_open(!was_open, window, cx);
            })
            .on_key_down(move |event, window, cx| {
                if key_state.focus_handle(cx).contains_focused(window, cx) {
                    return;
                }
                let key = event.keystroke.key.as_str();
                if matches!(key, "enter" | "space" | "down") && !was_open {
                    cx.stop_propagation();
                    key_state.set_open(true, window, cx);
                }
            })
    })
}

/// Make `panel` behave like an open floating surface: it takes focus when it
/// opens, and closes on Escape or on a mouse press outside it.
#[doc(hidden)]
pub fn dismissable(panel: Div, open_state: &OpenState, window: &mut Window, cx: &mut App) -> Div {
    open_state.focus_if_requested(window, cx);
    let focus_handle = open_state.focus_handle(cx);
    let escape_state = open_state.clone();
    let outside_state = open_state.clone();
    let order_cell = open_state.memory.read(cx).open_order.clone();
    if order_cell.get() == 0 {
        order_cell.set(next_open_order());
    }
    let order = order_cell.get();
    panel
        .track_focus(&focus_handle)
        .occlude()
        .on_key_down(move |event, window, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                escape_state.set_open(false, window, cx);
            }
        })
        // A press inside this surface claims it, so the surfaces it was opened from
        // (which see the press as outside them) stay open. Nested surfaces paint
        // later, so their claim lands after an outer surface's outside check runs;
        // that check is deferred until the whole event has been dispatched.
        .capture_any_mouse_down(move |event, window, cx| {
            let position = event.position;
            CLAIMED_PRESS.with(|claimed| {
                let newest = match claimed.get() {
                    Some((at, claimed_order)) if at == position => claimed_order.max(order),
                    _ => order,
                };
                claimed.set(Some((position, newest)));
            });
            window.defer(cx, |_, _| CLAIMED_PRESS.with(|claimed| claimed.set(None)));
        })
        .on_mouse_down_out(move |event, window, cx| {
            let position = event.position;
            let outside_state = outside_state.clone();
            window.defer(cx, move |window, cx| {
                let claimed_by_newer = CLAIMED_PRESS.with(|claimed| {
                    claimed
                        .get()
                        .is_some_and(|(at, claimed_order)| at == position && claimed_order > order)
                });
                if !claimed_by_newer {
                    outside_state.set_open(false, window, cx);
                }
            });
        })
}

/// Floating surfaces fade in while sliding 4px away from their trigger.
#[doc(hidden)]
#[must_use]
pub fn enter_motion(side: Side) -> Motion {
    let from = match side {
        Side::Bottom => MotionSide::Top,
        Side::Top => MotionSide::Bottom,
        Side::Left => MotionSide::Right,
        Side::Right => MotionSide::Left,
    };
    presets::slide_in(from, 1.).duration_ms(150)
}
