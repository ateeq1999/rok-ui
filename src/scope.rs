//! Scopes: values a provider makes available to the elements below it while they render,
//! lay out and paint. Used by `context::Provide` and the bloc providers.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

use gpui::{
    div, prelude::*, AnyElement, App, Bounds, ElementId, GlobalElementId, InspectorElementId,
    LayoutId, Pixels, Window,
};

/// Values one provider makes available, by type.
pub(crate) type Frame = HashMap<TypeId, Rc<dyn Any>>;

thread_local! {
    /// The providers around the element being laid out, innermost last.
    static SCOPES: RefCell<Vec<Rc<Frame>>> = const { RefCell::new(Vec::new()) };
}

/// Run `body` with `frame` as the innermost scope.
pub(crate) fn within<R>(frame: &Rc<Frame>, body: impl FnOnce() -> R) -> R {
    SCOPES.with(|scopes| scopes.borrow_mut().push(frame.clone()));
    let result = body();
    SCOPES.with(|scopes| scopes.borrow_mut().pop());
    result
}

/// The innermost provided value of type `T`.
pub(crate) fn find<T: Clone + 'static>() -> Option<T> {
    SCOPES.with(|scopes| {
        scopes
            .borrow()
            .iter()
            .rev()
            .find_map(|frame| frame.get(&TypeId::of::<T>()))
            .and_then(|value| value.downcast_ref::<T>().cloned())
    })
}

/// Renders its child with a provider's values in scope while it lays out and paints.
pub(crate) struct ScopeElement {
    pub(crate) frame: Rc<Frame>,
    pub(crate) child: AnyElement,
}

impl IntoElement for ScopeElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ScopeElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
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
        let child = &mut self.child;
        (within(&self.frame, || child.request_layout(window, cx)), ())
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
        let child = &mut self.child;
        within(&self.frame, || child.prepaint(window, cx));
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
        let child = &mut self.child;
        within(&self.frame, || child.paint(window, cx));
    }
}

/// One child as is, or several in a column.
pub(crate) fn single(mut children: Vec<AnyElement>) -> AnyElement {
    if children.len() == 1 {
        children.remove(0)
    } else {
        div()
            .flex()
            .flex_col()
            .size_full()
            .children(children)
            .into_any_element()
    }
}
