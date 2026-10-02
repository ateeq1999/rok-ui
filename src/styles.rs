//! Shared styling helpers: sizes and `className`-style overrides.

use gpui::{Refineable, StyleRefinement, Styled};

/// Control size, shared by buttons, inputs, badges and toggles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ComponentSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Apply caller-supplied style overrides on top of a component's own styles,
/// the way `className` merges into a shadcn/ui component through `cn()`.
pub trait ApplyStyleOverrides: Styled + Sized {
    fn apply_style_overrides(mut self, style_overrides: &StyleRefinement) -> Self {
        self.style().refine(style_overrides);
        self
    }
}

impl<Element: Styled> ApplyStyleOverrides for Element {}

/// Implement [`Styled`] (and `.sx(..)`) for a component that stores
/// `style_overrides: StyleRefinement` and `sx: Sx` fields, so callers can chain
/// `.w_full()`, `.mt_4()` and the rest of the GPUI style API, or pass `styles!` styles.
#[macro_export]
macro_rules! implement_style_overrides {
    ($component:ty) => {
        impl $crate::gpui::Styled for $component {
            fn style(&mut self) -> &mut $crate::gpui::StyleRefinement {
                &mut self.style_overrides
            }
        }

        // StyleX-style `.sx(..)` overrides, stored in the component's `sx`
        // field and applied last in its render.
        impl $crate::sx::SxStyled for $component {
            fn apply_sx(mut self, sx: $crate::sx::Sx) -> Self {
                self.sx.merge(&sx);
                self
            }
        }
    };
}
