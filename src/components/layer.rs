//! Top layers: how dialogs, menus, popovers and toasts draw above everything.
//!
//! GPUI's `deferred` paints an element after the rest of the window, but GPUI
//! 0.2 panics if a deferred element contains another one. Floating surfaces
//! nest all the time (a select in a dialog, a submenu in a menu), so only the
//! outermost one is deferred. It is wrapped in a [`LayerRoot`], and every
//! floating surface rendered inside it becomes a *portal*: the root lays it out
//! and paints it after its own content, at the position its trigger recorded.

use std::{cell::Cell, rc::Rc};

use gpui::{
    anchored, deferred, div, point, prelude::*, px, size, AnyElement, App, AvailableSpace, Bounds,
    Corner, Global, GlobalElementId, InspectorElementId, LayoutId, Pixels, Point, Window,
};

/// Where a portal is attached.
enum PortalAnchor {
    /// Window coordinates known up front (context menus, modals at 0,0).
    Fixed(Point<Pixels>),
    /// A point recorded while the trigger was prepainted.
    Marker(Rc<Cell<Point<Pixels>>>),
}

struct Portal {
    anchor: PortalAnchor,
    corner: Corner,
    margin: Pixels,
    content: AnyElement,
}

/// Collects portals while a [`LayerRoot`] lays out and prepaints its content.
#[derive(Default)]
struct LayerContext {
    depth: usize,
    portals: Vec<Portal>,
}

impl Global for LayerContext {}

fn inside_layer(cx: &App) -> bool {
    cx.try_global::<LayerContext>()
        .is_some_and(|context| context.depth > 0)
}

/// Run `body` with a layer active, so floating surfaces created inside become portals.
fn within_layer<R>(cx: &mut App, body: impl FnOnce(&mut App) -> R) -> R {
    cx.default_global::<LayerContext>().depth += 1;
    let result = body(cx);
    cx.default_global::<LayerContext>().depth -= 1;
    result
}

fn take_portals(cx: &mut App) -> Vec<Portal> {
    std::mem::take(&mut cx.default_global::<LayerContext>().portals)
}

/// Put `content` on the top layer with one corner at `position` (window
/// coordinates), kept `margin` inside the window. Used by context menus and modals.
pub(crate) fn layer_at(
    position: Point<Pixels>,
    corner: Corner,
    margin: Pixels,
    content: impl IntoElement,
    priority: usize,
    _cx: &mut App,
) -> AnyElement {
    LayerSlot {
        anchor: SlotAnchor::Fixed(position),
        corner,
        margin,
        priority,
        content: Some(content.into_any_element()),
        inner: None,
        marker: None,
    }
    .into_any_element()
}

/// Put `content` on the top layer with `corner` at this element's position
/// (the caller places it in a zero-size marker next to the trigger).
pub(crate) fn layer_at_marker(
    corner: Corner,
    content: impl IntoElement,
    _cx: &mut App,
) -> AnyElement {
    LayerSlot {
        anchor: SlotAnchor::Here,
        corner,
        margin: px(8.),
        priority: 1,
        content: Some(content.into_any_element()),
        inner: None,
        marker: None,
    }
    .into_any_element()
}

enum SlotAnchor {
    Fixed(Point<Pixels>),
    Here,
}

/// Where a floating surface is declared. Whether it becomes a deferred layer
/// or a portal of an enclosing layer is decided during layout: only then is
/// it known whether this slot ended up inside another layer.
struct LayerSlot {
    anchor: SlotAnchor,
    corner: Corner,
    margin: Pixels,
    priority: usize,
    content: Option<AnyElement>,
    inner: Option<AnyElement>,
    /// Set when this slot became a portal anchored here.
    marker: Option<Rc<Cell<Point<Pixels>>>>,
}

impl IntoElement for LayerSlot {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for LayerSlot {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let content = self
            .content
            .take()
            .unwrap_or_else(|| div().into_any_element());
        let mut inner = if inside_layer(cx) {
            let anchor = match self.anchor {
                SlotAnchor::Fixed(position) => PortalAnchor::Fixed(position),
                SlotAnchor::Here => {
                    let marker = Rc::new(Cell::new(Point::default()));
                    self.marker = Some(marker.clone());
                    PortalAnchor::Marker(marker)
                }
            };
            cx.default_global::<LayerContext>().portals.push(Portal {
                anchor,
                corner: self.corner,
                margin: self.margin,
                content,
            });
            div().absolute().size_0().into_any_element()
        } else {
            let anchored = anchored()
                .anchor(self.corner)
                .snap_to_window_with_margin(self.margin);
            let anchored = match self.anchor {
                SlotAnchor::Fixed(position) => anchored.position(position),
                SlotAnchor::Here => anchored,
            };
            deferred(LayerRoot::new(anchored.child(content)))
                .with_priority(self.priority)
                .into_any_element()
        };
        let layout_id = inner.request_layout(window, cx);
        self.inner = Some(inner);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // The enclosing layer prepaints its portals after its content, so the
        // anchor is recorded before the portal is placed.
        if let Some(marker) = self.marker.as_ref() {
            marker.set(bounds.origin);
        }
        if let Some(inner) = self.inner.as_mut() {
            inner.prepaint(window, cx);
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(inner) = self.inner.as_mut() {
            inner.paint(window, cx);
        }
    }
}

/// Origin for a box of `content_size` whose `corner` sits at `anchor`, moved
/// back inside `viewport` (keeping `margin` from its edges) when it would overflow.
pub(crate) fn place_in_viewport(
    anchor: Point<Pixels>,
    corner: Corner,
    content_size: gpui::Size<Pixels>,
    viewport: gpui::Size<Pixels>,
    margin: Pixels,
) -> Point<Pixels> {
    let mut origin = match corner {
        Corner::TopLeft => anchor,
        Corner::TopRight => point(anchor.x - content_size.width, anchor.y),
        Corner::BottomLeft => point(anchor.x, anchor.y - content_size.height),
        Corner::BottomRight => point(
            anchor.x - content_size.width,
            anchor.y - content_size.height,
        ),
    };
    if origin.x + content_size.width > viewport.width - margin {
        origin.x = viewport.width - margin - content_size.width;
    }
    if origin.y + content_size.height > viewport.height - margin {
        origin.y = viewport.height - margin - content_size.height;
    }
    origin.x = origin.x.max(margin);
    origin.y = origin.y.max(margin);
    origin
}

/// The outermost top-layer element: lays out and paints the portals created
/// inside its content, after (so above) that content.
pub(crate) struct LayerRoot {
    content: AnyElement,
    /// Portals created while the content was laid out, waiting for prepaint.
    pending: Vec<Portal>,
    /// Portals laid out and prepainted, painted after the content.
    portals: Vec<AnyElement>,
}

impl LayerRoot {
    fn new(content: impl IntoElement) -> Self {
        Self {
            content: content.into_any_element(),
            pending: Vec::new(),
            portals: Vec::new(),
        }
    }
}

impl IntoElement for LayerRoot {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for LayerRoot {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Keep this root's portals apart from any other layer laid out in the same frame.
        let outer = take_portals(cx);
        let layout_id = within_layer(cx, |cx| self.content.request_layout(window, cx));
        self.pending = take_portals(cx);
        cx.default_global::<LayerContext>().portals = outer;
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        within_layer(cx, |cx| {
            let outer = take_portals(cx);
            let mut pending = std::mem::take(&mut self.pending);
            // Prepainting the content records marker positions, and may lay out
            // more elements (list items) that create portals too.
            self.content.prepaint(window, cx);
            pending.extend(take_portals(cx));
            let viewport = window.viewport_size();
            // Laying out a portal can create more (a submenu inside a menu).
            while !pending.is_empty() {
                for mut portal in std::mem::take(&mut pending) {
                    let anchor = match &portal.anchor {
                        PortalAnchor::Fixed(position) => *position,
                        PortalAnchor::Marker(recorded) => recorded.get(),
                    };
                    let content_size = portal.content.layout_as_root(
                        size(AvailableSpace::MinContent, AvailableSpace::MinContent),
                        window,
                        cx,
                    );
                    let origin = place_in_viewport(
                        anchor,
                        portal.corner,
                        content_size,
                        viewport,
                        portal.margin,
                    );
                    portal.content.prepaint_at(origin, window, cx);
                    self.portals.push(portal.content);
                    pending.extend(take_portals(cx));
                }
            }
            cx.default_global::<LayerContext>().portals = outer;
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.content.paint(window, cx);
        for portal in &mut self.portals {
            portal.paint(window, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, size, Corner};

    use super::place_in_viewport;

    #[test]
    fn layers_stay_inside_the_window() {
        let viewport = size(px(800.), px(600.));
        let menu = size(px(200.), px(300.));
        // Fits: placed as asked.
        assert_eq!(
            place_in_viewport(
                point(px(100.), px(100.)),
                Corner::TopLeft,
                menu,
                viewport,
                px(8.)
            ),
            point(px(100.), px(100.))
        );
        // Right edge overflow: shifted left.
        assert_eq!(
            place_in_viewport(
                point(px(700.), px(100.)),
                Corner::TopLeft,
                menu,
                viewport,
                px(8.)
            )
            .x,
            px(592.)
        );
        // Anchored by its top-right corner.
        assert_eq!(
            place_in_viewport(
                point(px(400.), px(100.)),
                Corner::TopRight,
                menu,
                viewport,
                px(8.)
            )
            .x,
            px(200.)
        );
        // Bottom overflow: shifted up.
        assert_eq!(
            place_in_viewport(
                point(px(100.), px(500.)),
                Corner::TopLeft,
                menu,
                viewport,
                px(8.)
            )
            .y,
            px(292.)
        );
    }
}
