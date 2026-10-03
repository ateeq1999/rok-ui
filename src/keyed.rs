//! [`Keyed`]: stable identity for an element in a list.

use std::hash::{DefaultHasher, Hash, Hasher};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window,
};

/// Gives its child a stable identity: hook state (`use_state`, `use_query`, forms) inside it is
/// keyed by `key`, so it follows the item when a list is reordered, instead of staying at a
/// position. It adds no layout box of its own.
///
/// `view!` and `children!` write this for `#[key(item.id)] for item in items { .. }`.
///
/// ```
/// # use rok_ui::{prelude::*, Keyed};
/// let rows: Vec<Keyed> = [(3_u64, "Milk"), (7, "Eggs")]
///     .into_iter()
///     .map(|(id, name)| Keyed::new(&id, div().child(name)))
///     .collect();
/// # let _ = rows;
/// ```
pub struct Keyed {
    id: ElementId,
    child: AnyElement,
}

impl Keyed {
    /// `child`, identified by `key` (any hashable value: an id, a name, a tuple).
    pub fn new(key: &impl Hash, child: impl IntoElement) -> Self {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        Self {
            id: ElementId::NamedInteger("rok-key".into(), hasher.finish()),
            child: child.into_any_element(),
        }
    }
}

impl IntoElement for Keyed {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Keyed {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
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
        (self.child.request_layout(window, cx), ())
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
        self.child.prepaint(window, cx);
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
        self.child.paint(window, cx);
    }
}
