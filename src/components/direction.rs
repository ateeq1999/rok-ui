//! Direction: left-to-right or right-to-left layout for a subtree (shadcn/ui's
//! `<DirectionProvider>`).
//!
//! GPUI shapes text left to right only, so this does not reorder characters.
//! It right-aligns text and tells direction-aware components (breadcrumbs,
//! pagination, carousels, sidebars) to mirror their layout and arrows.
//! Read it in your own components with [`ActiveDirection::direction`].

use gpui::{
    div, prelude::*, AnyElement, App, Global, GlobalElementId, InspectorElementId, LayoutId,
    Pixels, Window,
};

/// Reading direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDirection {
    #[default]
    Ltr,
    Rtl,
}

impl TextDirection {
    pub fn is_rtl(self) -> bool {
        self == TextDirection::Rtl
    }
}

impl Global for TextDirection {}

/// Read the direction in effect while rendering: `cx.direction().is_rtl()`.
pub trait ActiveDirection {
    fn direction(&self) -> TextDirection;
}

impl ActiveDirection for App {
    fn direction(&self) -> TextDirection {
        self.try_global::<TextDirection>()
            .copied()
            .unwrap_or_default()
    }
}

/// Set the direction for the whole app (outside any [`Direction`] element).
pub fn set_text_direction(direction: TextDirection, cx: &mut App) {
    cx.set_global(direction);
    cx.refresh_windows();
}

/// Applies a direction to everything rendered inside it.
///
/// ```ignore
/// Direction::new(TextDirection::Rtl).child(Breadcrumb::new("path").link(..).page(..))
/// ```
pub struct Direction {
    direction: TextDirection,
    children: Vec<AnyElement>,
    content: Option<AnyElement>,
}

impl Direction {
    pub fn new(direction: TextDirection) -> Self {
        Self {
            direction,
            children: Vec::new(),
            content: None,
        }
    }
}

impl ParentElement for Direction {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl IntoElement for Direction {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Runs `body` with `direction` installed, then restores what was there.
fn with_direction<R>(
    direction: TextDirection,
    cx: &mut App,
    body: impl FnOnce(&mut App) -> R,
) -> R {
    let previous = cx.try_global::<TextDirection>().copied();
    cx.set_global(direction);
    let result = body(cx);
    match previous {
        Some(previous) => cx.set_global(previous),
        None => {
            cx.remove_global::<TextDirection>();
        }
    }
    result
}

impl Element for Direction {
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
        let is_rtl = self.direction.is_rtl();
        let mut content = div()
            .flex()
            .flex_col()
            .when(is_rtl, |content| content.text_right().items_end())
            .children(std::mem::take(&mut self.children))
            .into_any_element();
        // Components render while their layout is requested, so the direction
        // only needs to be installed for this step.
        let layout_id = with_direction(self.direction, cx, |cx| content.request_layout(window, cx));
        self.content = Some(content);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(content) = self.content.as_mut() {
            with_direction(self.direction, cx, |cx| content.prepaint(window, cx));
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(content) = self.content.as_mut() {
            with_direction(self.direction, cx, |cx| content.paint(window, cx));
        }
    }
}
