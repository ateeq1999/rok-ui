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
