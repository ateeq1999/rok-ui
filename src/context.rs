//! Values scoped to part of the tree: [`Provide`] makes a value available to every component
//! below it, and `cx.context::<T>()` reads the nearest one (React's context, Flutter's
//! `InheritedWidget`).
//!
//! ```no_run
//! use rok_ui::{context::Provide, prelude::*};
//!
//! /// Which account the pages below are about.
//! #[derive(Clone)]
//! struct Account {
//!     name: SharedString,
//! }
//!
//! #[component]
//! fn Greeting(cx: &mut Cx) -> impl IntoElement {
//!     let account = cx.expect_context::<Account>();
//!     div().child(format!("Signed in as {}", account.name))
//! }
//!
//! fn page() -> impl IntoElement {
//!     Provide::new()
//!         .value(Account { name: "Ada".into() })
//!         .child(Greeting::new())
//! }
//! ```
//!
//! Values are looked up by type, innermost provider first, so a nested `Provide` overrides
//! an outer one for its subtree. Wrap a value in a newtype to keep two of the same type
//! apart. Values are cloned on every read: provide cheap handles (`Rc`, `Arc`,
//! `SharedString`, entity handles), not large data.

use std::{
    any::{Any, TypeId},
    rc::Rc,
};

use gpui::{prelude::*, AnyElement, App, Window};

use crate::scope::{find, single, Frame, ScopeElement};

/// Makes values available to the components below it, by type; read them with
/// `cx.context::<T>()`. See the [module docs](self).
#[derive(IntoElement, Default)]
pub struct Provide {
    frame: Frame,
    children: Vec<AnyElement>,
}

impl Provide {
    /// A provider with no values yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Provide `value` as `T` to the subtree (replacing an earlier `T` on this provider).
    #[must_use]
    pub fn value<T: Clone + 'static>(mut self, value: T) -> Self {
        self.frame
            .insert(TypeId::of::<T>(), Rc::new(value) as Rc<dyn Any>);
        self
    }
}

impl std::fmt::Debug for Provide {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Provide")
            .field("values", &self.frame.len())
            .finish_non_exhaustive()
    }
}

impl ParentElement for Provide {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Provide {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        ScopeElement {
            frame: Rc::new(self.frame),
            child: single(self.children),
        }
    }
}

/// The nearest provided `T`, outside a component (in an event handler, the value is gone:
/// read it while rendering and move it into the handler).
#[must_use]
pub fn context<T: Clone + 'static>() -> Option<T> {
    find::<T>()
}
