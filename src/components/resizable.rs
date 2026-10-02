//! Resizable: panels separated by draggable handles.

use std::{cell::Cell, rc::Rc};

use gpui::{
    div, prelude::*, px, relative, AnyElement, App, Bounds, CursorStyle, ElementId, MouseButton,
    Pixels, StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::{
    focus_ring_shadow,
    interaction::{measure_bounds, track_drag},
    overlay::child_id,
};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, State},
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Which way the panels are laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResizableDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// One panel. Sizes are percentages of the group.
pub struct ResizablePanel {
    default_size: Option<f32>,
    min_size: f32,
    max_size: f32,
    children: Vec<AnyElement>,
}

impl ResizablePanel {
    pub fn new() -> Self {
        Self {
            default_size: None,
            min_size: 0.,
            max_size: 100.,
            children: Vec::new(),
        }
    }

    /// Starting size in percent. Panels without one share what is left.
    pub fn default_size(mut self, percent: f32) -> Self {
        self.default_size = Some(percent);
        self
    }

    pub fn min_size(mut self, percent: f32) -> Self {
        self.min_size = percent;
        self
    }

    pub fn max_size(mut self, percent: f32) -> Self {
        self.max_size = percent;
        self
    }
}

impl Default for ResizablePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for ResizablePanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Starting sizes: explicit defaults, with the remainder split between the rest.
pub(crate) fn initial_sizes(defaults: &[Option<f32>]) -> Vec<f32> {
    let assigned: f32 = defaults.iter().flatten().sum();
    let unassigned = defaults.iter().filter(|size| size.is_none()).count();
    let share = if unassigned > 0 {
        ((100. - assigned) / unassigned as f32).max(0.)
    } else {
        0.
    };
    defaults.iter().map(|size| size.unwrap_or(share)).collect()
}

/// Move the boundary after panel `handle` by `delta` percent, within the limits
/// of both neighbours. Returns the new sizes.
pub(crate) fn resize(sizes: &[f32], limits: &[(f32, f32)], handle: usize, delta: f32) -> Vec<f32> {
    let mut next = sizes.to_vec();
    let (before, after) = (handle, handle + 1);
    if after >= sizes.len() {
        return next;
    }
    let (before_min, before_max) = limits[before];
    let (after_min, after_max) = limits[after];
    let lowest = (before_min - sizes[before]).max(sizes[after] - after_max);
    let highest = (before_max - sizes[before]).min(sizes[after] - after_min);
    let delta = delta.clamp(lowest.min(0.), highest.max(0.));
    next[before] += delta;
    next[after] -= delta;
    next
}

struct ResizableMemory {
    sizes: Vec<f32>,
    dragging_handle: Option<usize>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

/// ```ignore
/// ResizablePanelGroup::new("layout")
///     .with_handle(true)
///     .panel(ResizablePanel::new().default_size(25.).min_size(15.).child(sidebar))
///     .panel(ResizablePanel::new().child(editor))
/// ```
#[derive(IntoElement)]
pub struct ResizablePanelGroup {
    id: ElementId,
    direction: ResizableDirection,
    panels: Vec<ResizablePanel>,
    with_handle: bool,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ResizablePanelGroup);

impl ResizablePanelGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            direction: ResizableDirection::Horizontal,
            panels: Vec::new(),
            with_handle: false,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn direction(mut self, direction: ResizableDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Stack panels top to bottom.
    pub fn vertical(self) -> Self {
        self.direction(ResizableDirection::Vertical)
    }

    pub fn panel(mut self, panel: ResizablePanel) -> Self {
        self.panels.push(panel);
        self
    }

    /// Draw a grip on each handle.
    pub fn with_handle(mut self, with_handle: bool) -> Self {
        self.with_handle = with_handle;
        self
    }
}

impl RenderOnce for ResizablePanelGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let defaults: Vec<Option<f32>> =
            self.panels.iter().map(|panel| panel.default_size).collect();
        let memory: State<ResizableMemory> =
            use_keyed_state(child_id(&self.id, "resizable"), window, cx, || {
                ResizableMemory {
                    sizes: initial_sizes(&defaults),
                    dragging_handle: None,
                    bounds: Rc::new(Cell::new(Bounds::default())),
                }
            });
        if memory.read(cx).sizes.len() != self.panels.len() {
            memory.update(cx, |memory| memory.sizes = initial_sizes(&defaults));
        }
        let sizes = memory.read(cx).sizes.clone();
        let dragging_handle = memory.read(cx).dragging_handle;
        let bounds = memory.read(cx).bounds.clone();
        let limits: Rc<Vec<(f32, f32)>> = Rc::new(
            self.panels
                .iter()
                .map(|panel| (panel.min_size, panel.max_size))
                .collect(),
        );
        let is_horizontal = self.direction == ResizableDirection::Horizontal;
        // Captured while rendering: handlers run outside the `Direction` scope.
        let rtl = is_horizontal && super::direction::is_rtl();
        let colors = cx.theme().colors.clone();
        let ring_color = colors.ring;
        let panel_count = self.panels.len();

        let mut children: Vec<AnyElement> = Vec::new();
        for (index, panel) in self.panels.into_iter().enumerate() {
            let basis = relative(sizes[index] / 100.);
            children.push(
                div()
                    .flex_dir()
                    .flex_col()
                    .flex_grow()
                    .flex_shrink()
                    .flex_basis(basis)
                    .min_w_0()
                    .min_h_0()
                    .overflow_hidden()
                    .children(panel.children)
                    .into_any_element(),
            );
            if index + 1 == panel_count {
                continue;
            }
            let press_memory = memory.clone();
            let key_memory = memory.clone();
            let key_limits = limits.clone();
            let grip = self.with_handle.then(|| {
                div()
                    .absolute()
                    .flex_dir()
                    .items_center()
                    .justify_center()
                    .rounded(px(2.))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.border)
                    .map(|grip| {
                        if is_horizontal {
                            grip.h(px(16.)).w(px(12.)).left(px(-6.))
                        } else {
                            grip.w(px(16.)).h(px(12.)).top(px(-6.))
                        }
                    })
                    .child(Icon::new(IconName::GripVertical).size(px(10.)))
            });
            children.push(
                div()
                    .id(("resizable-handle", index))
                    .relative()
                    .flex_dir()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .bg(if dragging_handle == Some(index) {
                        colors.ring
                    } else {
                        colors.border
                    })
                    .map(|handle| {
                        if is_horizontal {
                            handle
                                .w(px(1.))
                                .h_full()
                                .cursor(CursorStyle::ResizeLeftRight)
                        } else {
                            handle.h(px(1.)).w_full().cursor(CursorStyle::ResizeUpDown)
                        }
                    })
                    .tab_index(0)
                    .focus(move |style| style.shadow(focus_ring_shadow(ring_color)))
                    // A wider invisible strip makes the 1px line easy to grab.
                    .child(
                        div()
                            .id(("resizable-hit-area", index))
                            .absolute()
                            .map(|area| {
                                if is_horizontal {
                                    area.top_0()
                                        .h_full()
                                        .w(px(9.))
                                        .left(px(-4.))
                                        .cursor(CursorStyle::ResizeLeftRight)
                                } else {
                                    area.left_0()
                                        .w_full()
                                        .h(px(9.))
                                        .top(px(-4.))
                                        .cursor(CursorStyle::ResizeUpDown)
                                }
                            })
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                cx.stop_propagation();
                                press_memory
                                    .update(cx, |memory| memory.dragging_handle = Some(index));
                            }),
                    )
                    .children(grip)
                    .on_key_down(move |event, _, cx| {
                        let delta = match (event.keystroke.key.as_str(), is_horizontal) {
                            // Panels run right to left in RTL.
                            ("left", true) if rtl => 5.,
                            ("right", true) if rtl => -5.,
                            ("left", true) | ("up", false) => -5.,
                            ("right", true) | ("down", false) => 5.,
                            _ => return,
                        };
                        cx.stop_propagation();
                        key_memory.update(cx, |memory| {
                            memory.sizes = resize(&memory.sizes, &key_limits, index, delta)
                        });
                    })
                    .into_any_element(),
            );
        }

        let move_memory = memory.clone();
        let move_bounds = bounds.clone();
        let end_memory = memory.clone();
        div()
            .id(self.id)
            .relative()
            .flex_dir()
            .size_full()
            .when(!is_horizontal, |group| group.flex_col())
            .child(measure_bounds(bounds))
            .children(children)
            .child(track_drag(
                dragging_handle.is_some(),
                Rc::new(move |position, _, cx| {
                    let Some(handle) = dragging_handle else {
                        return;
                    };
                    let group_bounds = move_bounds.get();
                    let (offset, length) = if is_horizontal {
                        let from_start = if rtl {
                            group_bounds.right() - position.x
                        } else {
                            position.x - group_bounds.left()
                        };
                        (from_start, group_bounds.size.width)
                    } else {
                        (position.y - group_bounds.top(), group_bounds.size.height)
                    };
                    if length <= px(0.) {
                        return;
                    }
                    let pointer_percent = offset / length * 100.;
                    move_memory.update(cx, |memory| {
                        let boundary: f32 = memory.sizes[..=handle].iter().sum();
                        memory.sizes =
                            resize(&memory.sizes, &limits, handle, pointer_percent - boundary);
                    });
                }),
                Rc::new(move |_, cx| end_memory.update(cx, |memory| memory.dragging_handle = None)),
            ))
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::{initial_sizes, resize};

    #[test]
    fn initial_sizes_split_the_remainder() {
        assert_eq!(
            initial_sizes(&[Some(25.), None, None]),
            vec![25., 37.5, 37.5]
        );
        assert_eq!(initial_sizes(&[None, None]), vec![50., 50.]);
        assert_eq!(initial_sizes(&[Some(30.), Some(70.)]), vec![30., 70.]);
    }

    #[test]
    fn resize_respects_neighbour_limits() {
        let limits = [(10., 100.), (20., 100.)];
        assert_eq!(resize(&[50., 50.], &limits, 0, 10.), vec![60., 40.]);
        // The second panel cannot go below 20%.
        assert_eq!(resize(&[50., 50.], &limits, 0, 45.), vec![80., 20.]);
        // The first panel cannot go below 10%.
        assert_eq!(resize(&[50., 50.], &limits, 0, -45.), vec![10., 90.]);
    }
}
