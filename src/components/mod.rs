//! Every rok-ui component. Each one follows the same conventions:
//!
//! - Built with a constructor, configured with builder methods (`Button::new("save").outline()`).
//! - Controlled, like React: the component never owns its value. You pass `checked`,
//!   `selected_index` or `value` in and get changes back through `on_change`.
//! - Implements GPUI's `Styled` where it makes sense, so `.w_full()` and friends
//!   override its defaults the way `className` does in shadcn/ui.
//! - Reads colors, radius and fonts from the active [`crate::theme::Theme`].

pub mod alert;
pub mod app_root;
pub mod avatar;
pub mod badge;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod dialog;
pub mod input;
pub mod keyboard_shortcut;
pub mod label;
pub mod progress;
pub mod separator;
pub mod skeleton;
pub mod spinner;
pub mod switch;
pub mod tabs;
pub mod tooltip;

pub use alert::{Alert, AlertVariant};
pub use app_root::AppRoot;
pub use avatar::Avatar;
pub use badge::{Badge, BadgeVariant};
pub use button::{Button, ButtonSize, ButtonVariant, IconPosition};
pub use card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
pub use checkbox::Checkbox;
pub use dialog::Dialog;
pub use input::{use_input_state, Input, InputEvent, InputState};
pub use keyboard_shortcut::KeyboardShortcut;
pub use label::Label;
pub use progress::Progress;
pub use separator::{Separator, SeparatorOrientation};
pub use skeleton::Skeleton;
pub use spinner::Spinner;
pub use switch::Switch;
pub use tabs::Tabs;
pub use tooltip::Tooltip;

use gpui::{point, px, BoxShadow, Hsla};

/// shadcn/ui's focus ring (`ring-[3px] ring-ring/50`), drawn as a spread shadow.
/// GPUI skips shadows with zero blur, so a 1px blur plus 2px spread gives the 3px ring.
pub(crate) fn focus_ring_shadow(ring_color: Hsla) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: ring_color.opacity(0.5),
        offset: point(px(0.), px(0.)),
        blur_radius: px(1.),
        spread_radius: px(2.),
    }]
}

/// shadcn/ui's `shadow-xs`, used on outlined controls.
pub(crate) fn extra_small_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: gpui::black().opacity(0.05),
        offset: point(px(0.), px(1.)),
        blur_radius: px(2.),
        spread_radius: px(0.),
    }]
}
