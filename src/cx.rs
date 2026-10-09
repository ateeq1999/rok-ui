//! [`Cx`]: one context handle for app code.
//!
//! GPUI passes components a `&mut Window` and a `&mut App`. [`Cx`] carries both, so hooks,
//! queries and mutations take one argument, the way they do in TanStack and topcoat:
//!
//! ```no_run
//! use rok_ui::prelude::*;
//!
//! #[component]
//! fn Greeting(name: SharedString, cx: &mut Cx) -> impl IntoElement {
//!     let visits = cx.use_state(|| 0_u32);
//!     let count = visits.get(cx);
//!     div().child(format!("Hello, {name} ({count})"))
//! }
//! ```
//!
//! `Cx` dereferences to [`App`], so everything that takes `&App` or `&mut App` accepts a
//! `&mut Cx` too.

use std::ops::{Deref, DerefMut};

use gpui::{App, ElementId, Global, Window};

use crate::hooks::{use_keyed_state, use_state, State};

/// The window and app being rendered, as one handle.
pub struct Cx<'a> {
    /// The window being rendered.
    pub window: &'a mut Window,
    /// The app.
    pub app: &'a mut App,
}

impl<'a> Cx<'a> {
    /// Wrap a window and app, for code that receives them separately (a view's `render`).
    pub fn new(window: &'a mut Window, app: &'a mut App) -> Self {
        Self { window, app }
    }

    /// App-wide context by type: a [`Global`] set with `cx.set_global(..)`.
    ///
    /// # Panics
    ///
    /// Panics if no value of type `T` was set. Use [`Cx::try_get`] when it is optional.
    #[must_use]
    pub fn get<T: Global>(&self) -> &T {
        self.app.global::<T>()
    }

    /// App-wide context by type, if it was set.
    #[must_use]
    pub fn try_get<T: Global>(&self) -> Option<&T> {
        self.app.try_global::<T>()
    }

    /// `use_state` for this call site: state created on the first render and kept while the
    /// component stays rendered.
    #[track_caller]
    pub fn use_state<T: 'static>(&mut self, initial_value: impl FnOnce() -> T) -> State<T> {
        use_state(self.window, self.app, initial_value)
    }

    /// `use_state` with an explicit key, for state created inside loops.
    pub fn use_keyed_state<T: 'static>(
        &mut self,
        key: impl Into<ElementId>,
        initial_value: impl FnOnce() -> T,
    ) -> State<T> {
        use_keyed_state(key, self.window, self.app, initial_value)
    }

    /// Re-render this window on the next frame.
    pub fn refresh(&mut self) {
        self.window.refresh();
    }

    /// The nearest `T` provided above this component with
    /// [`Provide`](crate::context::Provide), or `None`.
    #[must_use]
    pub fn context<T: Clone + 'static>(&self) -> Option<T> {
        crate::context::context::<T>()
    }

    /// The nearest `T` provided above this component.
    ///
    /// # Panics
    ///
    /// Panics, naming the type, when no `Provide` above provides a `T`.
    #[must_use]
    pub fn expect_context<T: Clone + 'static>(&self) -> T {
        self.context::<T>().unwrap_or_else(|| {
            panic!(
                "no `{}` is provided here; wrap this part of the tree in \
                 `Provide::new().value(..)`",
                std::any::type_name::<T>()
            )
        })
    }

    /// State with a stable identity of your choosing, for rows in a list: the same key keeps
    /// the same state when rows move, are inserted or are removed.
    ///
    /// ```no_run
    /// use rok_ui::prelude::*;
    ///
    /// #[component]
    /// fn Rows(ids: Vec<u64>, cx: &mut Cx) -> impl IntoElement {
    ///     let rows: Vec<AnyElement> = ids
    ///         .into_iter()
    ///         .map(|id| {
    ///             let expanded = cx.keyed(("row", id as usize)).use_state(|| false);
    ///             div().child(format!("{id}: {}", expanded.get(cx))).into_any_element()
    ///         })
    ///         .collect();
    ///     div().children(rows)
    /// }
    /// ```
    pub fn keyed(&mut self, key: impl Into<ElementId>) -> KeyedCx<'_, 'a> {
        KeyedCx {
            cx: self,
            key: key.into(),
        }
    }
}

/// A [`Cx`] with a key, from [`Cx::keyed`].
pub struct KeyedCx<'c, 'a> {
    cx: &'c mut Cx<'a>,
    key: ElementId,
}

impl KeyedCx<'_, '_> {
    /// State for this key, created on the first render with it.
    pub fn use_state<T: 'static>(self, initial_value: impl FnOnce() -> T) -> State<T> {
        use_keyed_state(self.key, self.cx.window, self.cx.app, initial_value)
    }

    /// The key.
    #[must_use]
    pub fn key(&self) -> &ElementId {
        &self.key
    }
}

impl std::fmt::Debug for KeyedCx<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KeyedCx")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl Deref for Cx<'_> {
    type Target = App;

    fn deref(&self) -> &App {
        self.app
    }
}

impl DerefMut for Cx<'_> {
    fn deref_mut(&mut self) -> &mut App {
        self.app
    }
}
