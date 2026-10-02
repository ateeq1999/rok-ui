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
//! ```ignore
//! use rok_ui::prelude::*;
//!
//! fn main() {
//!     Application::new().with_assets(rok_ui::Assets).run(|cx: &mut App| {
//!         rok_ui::init(cx);
//!         cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| MyView)).unwrap();
//!     });
//! }
//! ```

// Lets `#[component]` expand to `::rok_ui::…` paths inside this crate too.
extern crate self as rok_ui;

pub mod bidi;
pub mod components;
pub mod fonts;
pub mod hooks;
pub mod icon;
pub mod motion;
pub mod prelude;
pub mod styles;
pub mod sx;
pub mod theme;

pub use gpui;
pub use icon::{Assets, AssetsWithFallback, Icon, IconName};
pub use rok_ui_macros::{children, component, keyframes, style, styles, view};

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
}
