//! # rok-ui
//!
//! A shadcn/ui-style component system for [GPUI](https://gpui.rs) desktop apps,
//! with a React-like developer experience:
//!
//! | React / shadcn/ui                    | rok-ui                                              |
//! |--------------------------------------|-----------------------------------------------------|
//! | `function Card(props) { … }`         | `#[component] fn Card(…) -> impl IntoElement { … }` |
//! | `<Button variant="outline" size="sm">` | `Button::new("id").outline().small()`             |
//! | `props.children`                     | `#[children] children: Vec<AnyElement>`             |
//! | `className="w-full mt-4"`            | `.w_full().mt_4()` on any component                 |
//! | `onClick={() => …}`                  | `.on_click(\|event, window, cx\| …)`                 |
//! | `const [n, setN] = useState(0)`      | `let n = use_state(window, cx, \|\| 0)`              |
//! | CSS variables (`--primary`)          | `cx.theme().colors.primary`                         |
//!
//! ```no_run
//! use rok_ui::prelude::*;
//!
//! struct MyView;
//!
//! impl Render for MyView {
//!     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
//!         AppRoot::new().child(Button::new("hello").label("Hello"))
//!     }
//! }
//!
//! fn main() {
//!     Application::new().with_assets(rok_ui::Assets).run(|cx: &mut App| {
//!         rok_ui::init(cx);
//!         cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| MyView)).unwrap();
//!     });
//! }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

// Lets `#[component]` expand to `::rok_ui::…` paths inside this crate too.
extern crate self as rok_ui;

pub mod bidi;
#[cfg(feature = "bloc")]
pub mod bloc;
pub mod components;
pub mod context;
pub mod cx;
#[cfg(feature = "db")]
pub mod db;
#[cfg(feature = "devtools")]
pub mod devtools;
pub mod fonts;
#[cfg(feature = "form")]
pub mod form;
pub mod hooks;
#[cfg(feature = "http")]
pub mod http;
pub mod icon;
pub mod keyed;
pub mod motion;
#[cfg(feature = "persist")]
pub mod persist;
pub mod prelude;
#[cfg(feature = "query")]
pub mod query;
#[cfg(feature = "router")]
pub mod router;
#[cfg(feature = "runtime")]
pub mod runtime;
mod scope;
#[cfg(feature = "state")]
pub mod state;
pub mod styles;
pub mod sx;
pub mod theme;

pub use cx::Cx;
pub use gpui;
pub use icon::{Assets, AssetsWithFallback, Icon, IconName};
pub use keyed::Keyed;
pub use rok_ui_macros::{children, component, keyframes, style, styles, view};
#[cfg(feature = "query")]
pub use rok_ui_macros::{memoize, procedure};

/// Support code for the `view!` and `children!` macros. Not public API.
#[doc(hidden)]
pub mod __private {
    use std::cell::Cell;

    use gpui::{AnyElement, IntoElement, SharedString};

    /// A child expression from markup. Text (anything `Into<SharedString>`) becomes
    /// a [`crate::components::BidiText`] through [`TextChild`]; other elements go
    /// through [`ElementChild`]. Method resolution tries `TextChild` first because
    /// it needs one reference less (autoref specialization).
    pub struct Child<T>(Cell<Option<T>>);

    impl<T> Child<T> {
        pub fn new(value: T) -> Self {
            Self(Cell::new(Some(value)))
        }

        fn take(&self) -> T {
            self.0.take().expect("a markup child is converted once")
        }
    }

    pub trait TextChild {
        fn take_child(&self) -> AnyElement;
    }

    impl<T: Into<SharedString>> TextChild for Child<T> {
        fn take_child(&self) -> AnyElement {
            crate::components::BidiText::new(self.take()).into_any_element()
        }
    }

    pub trait ElementChild {
        fn take_child(&self) -> AnyElement;
    }

    impl<T: IntoElement> ElementChild for &Child<T> {
        fn take_child(&self) -> AnyElement {
            self.take().into_any_element()
        }
    }
}

/// Install the default theme and the key bindings rok-ui components rely on
/// (text editing, Tab focus navigation). Call once at startup.
pub fn init(cx: &mut gpui::App) {
    theme::init(cx);
    // Keyboard input makes focus rings visible (see `sx::focus_visible`).
    cx.observe_keystrokes(|_, window, _| {
        if sx::set_keyboard_modality(true) {
            window.refresh();
        }
    })
    .detach();
    components::app_root::bind_focus_navigation_keys(cx);
    #[cfg(feature = "input")]
    components::input::bind_text_editing_keys(cx);
    #[cfg(feature = "router")]
    router::init(cx);
    #[cfg(feature = "state")]
    state::init(cx);
    #[cfg(feature = "devtools")]
    devtools::init(cx);
}
