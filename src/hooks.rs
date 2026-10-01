//! React-style hooks on top of GPUI's element state.
//!
//! GPUI already keeps state alive across frames for an element at a stable
//! position (`Window::use_state`). These helpers wrap it in a small [`State`]
//! handle with `get` / `set` / `update`, so component code reads like React:
//!
//! ```ignore
//! #[component]
//! fn Counter(window: &mut Window, cx: &mut App) -> impl IntoElement {
//!     let count = use_state(window, cx, || 0);
//!     Button::new("increment", format!("Clicked {} times", count.get(cx)))
//!         .on_click(move |_, _, cx| count.update(cx, |value| *value += 1))
//! }
//! ```
//!
//! Rules (same spirit as React's rules of hooks):
//! - Call hooks unconditionally, in the same order, during render.
//! - Inside loops, use [`use_keyed_state`] with a stable key per item.
//! - Updating a [`State`] re-renders the view that owns the component.

use std::rc::Rc;

use gpui::{App, ElementId, Entity, Window};

/// An event callback, the GPUI equivalent of a React `onX` prop.
///
/// `#[component]` gives props of this type a builder that accepts a closure directly.
pub type EventHandler<Event> = Rc<dyn Fn(&Event, &mut Window, &mut App) + 'static>;

/// A handle to a piece of component state. Cheap to clone and `'static`, so it
/// can be moved into event handlers.
pub struct State<Value: 'static> {
    entity: Entity<Value>,
}

impl<Value: 'static> Clone for State<Value> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity.clone(),
        }
    }
}

impl<Value: 'static> State<Value> {
    /// Wrap an existing entity, for state owned by a parent view.
    pub fn from_entity(entity: Entity<Value>) -> Self {
        Self { entity }
    }

    /// Borrow the current value.
    pub fn read<'context>(&self, cx: &'context App) -> &'context Value {
        self.entity.read(cx)
    }

    /// Copy out the current value.
    pub fn get(&self, cx: &App) -> Value
    where
        Value: Clone,
    {
        self.entity.read(cx).clone()
    }

    /// Replace the value and re-render. Like React's `setValue(next)`.
    pub fn set(&self, value: Value, cx: &mut App) {
        self.entity.update(cx, |current_value, cx| {
            *current_value = value;
            cx.notify();
        });
    }

    /// Change the value in place and re-render. Like React's `setValue(previous => next)`.
    pub fn update(&self, cx: &mut App, change: impl FnOnce(&mut Value)) {
        self.entity.update(cx, |current_value, cx| {
            change(current_value);
            cx.notify();
        });
    }

    /// The underlying GPUI entity, for `cx.observe` and friends.
    pub fn entity(&self) -> &Entity<Value> {
        &self.entity
    }
}

/// `useState`: state tied to this call site. The initializer runs once.
#[track_caller]
pub fn use_state<Value: 'static>(
    window: &mut Window,
    cx: &mut App,
    initial_value: impl FnOnce() -> Value,
) -> State<Value> {
    let entity = window.use_state(cx, |_, _| initial_value());
    State { entity }
}

/// `useState` with an explicit key, for state created inside loops.
pub fn use_keyed_state<Value: 'static>(
    key: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
    initial_value: impl FnOnce() -> Value,
) -> State<Value> {
    let entity = window.use_keyed_state(key, cx, |_, _| initial_value());
    State { entity }
}
