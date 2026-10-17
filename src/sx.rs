//! StyleX-style styling: style objects defined once, merged at use, last wins.
//!
//! ```no_run
//! # use rok_ui::{prelude::*, sx};
//! styles! {
//!     pub CARD = {
//!         base: { display: flex, direction: column, gap: 6, padding: 6, radius: xl,
//!                 border: 1, border_color: border, background: card,
//!                 hover: { border_color: ring } },
//!         compact: { padding: 3 },
//!     }
//! }
//!
//! # fn example(compact: bool) {
//! let card = div().sx((&CARD.base, compact.then_some(&CARD.compact)));
//! let same = div().sx(sx![CARD.base, compact => CARD.compact]);
//! # }
//! ```
//!
//! An [`Sx`] is an ordered list of declarations per state (base, hover, focus,
//! active). Merging appends, so later declarations override earlier ones, like
//! `stylex.props(a, b)`. Theme tokens (`ColorToken::Primary`, `RadiusToken::Md`)
//! are resolved against the active theme when the style is applied, so the
//! same static style follows light/dark and preset switches.
//!
//! Lengths use Tailwind's spacing scale: plain numbers are multiples of 4px
//! (`gap(6.)` is 24px). Use `px(..)` for exact pixels and `relative(..)` for
//! fractions.

use gpui::{
    CursorStyle, Div, FontWeight, InteractiveElement, Refineable, Stateful, StyleRefinement, Styled,
};

use crate::theme::Theme;

mod ambient;
mod decl;
mod values;

pub use ambient::{
    current_theme, focus_visible, set_focus_ring_mode, set_keyboard_modality, set_theme_snapshot,
    FocusRingMode,
};
use decl::resolve;
pub use decl::Decl;
pub use values::{
    ColorToken, Corners, Edges, SxAlign, SxColor, SxDirection, SxDisplay, SxFlex, SxFont,
    SxJustify, SxLength, SxRadius, SxShadow, SxText, SxTextAlign,
};

// ---------------------------------------------------------------------------
// Sx.

/// A style object: declarations for the resting state and each interaction
/// state. Build one with `styles!`, `style!`, or the builder methods below.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sx {
    base: Vec<Decl>,
    hover: Vec<Decl>,
    focus: Vec<Decl>,
    active: Vec<Decl>,
}

/// Generates `Sx` builder methods that push one declaration.
macro_rules! decl_methods {
    ($($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) => $decl:expr;)*) => {
        $(
            $(#[$doc])*
            #[must_use]
            pub fn $name(mut self, $($arg: $ty),*) -> Self {
                self.base.push($decl);
                self
            }
        )*
    };
}

impl Sx {
    /// An empty style.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a declaration (later declarations win).
    #[must_use]
    pub fn decl(mut self, decl: Decl) -> Self {
        self.base.push(decl);
        self
    }

    /// Styles while the pointer is over the element.
    #[must_use]
    pub fn hover(mut self, styles: impl FnOnce(Sx) -> Sx) -> Self {
        let state = styles(Sx::new());
        self.hover.extend(state.base);
        self
    }

    /// Styles while the element has keyboard focus (focusable elements only).
    #[must_use]
    pub fn focus(mut self, styles: impl FnOnce(Sx) -> Sx) -> Self {
        let state = styles(Sx::new());
        self.focus.extend(state.base);
        self
    }

    /// Styles while the element is pressed (elements with an id only).
    #[must_use]
    pub fn active(mut self, styles: impl FnOnce(Sx) -> Sx) -> Self {
        let state = styles(Sx::new());
        self.active.extend(state.base);
        self
    }

    /// Append `other`'s declarations, so they override this style's.
    pub fn merge(&mut self, other: &Sx) {
        self.base.extend(other.base.iter().cloned());
        self.hover.extend(other.hover.iter().cloned());
        self.focus.extend(other.focus.iter().cloned());
        self.active.extend(other.active.iter().cloned());
    }

    /// Whether the style has no declarations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
            && self.hover.is_empty()
            && self.focus.is_empty()
            && self.active.is_empty()
    }

    /// The resting-state declarations resolved against `theme`.
    #[must_use]
    pub fn resolve_base(&self, theme: &Theme) -> StyleRefinement {
        resolve(&self.base, theme)
    }

    decl_methods! {
        /// `display: flex`.
        flex() => Decl::Display(SxDisplay::Flex);
        /// `display: block`.
        block() => Decl::Display(SxDisplay::Block);
        /// `display: none`: the element takes no space.
        hidden() => Decl::Display(SxDisplay::Hidden);
        /// The flex direction.
        direction(direction: SxDirection) => Decl::Direction(direction);
        /// Lay children out in a row.
        flex_row() => Decl::Direction(SxDirection::Row);
        /// Lay children out in a column.
        flex_col() => Decl::Direction(SxDirection::Column);
        /// Whether flex children wrap onto new lines.
        wrap(wrap: bool) => Decl::Wrap(wrap);
        /// Grow and shrink from a zero basis (`flex: 1 1 0%`).
        flex_1() => Decl::Flex(SxFlex::One);
        /// Grow and shrink from the content size (`flex: 1 1 auto`).
        flex_auto() => Decl::Flex(SxFlex::Auto);
        /// Neither grow nor shrink (`flex: none`).
        flex_none() => Decl::Flex(SxFlex::None);
        /// The flex grow factor.
        grow(grow: f32) => Decl::Grow(grow);
        /// The flex shrink factor.
        shrink(shrink: f32) => Decl::Shrink(shrink);
        /// The flex basis.
        basis(length: impl Into<SxLength>) => Decl::Basis(length.into());
        /// Cross-axis alignment of the children (`align-items`).
        align(align: SxAlign) => Decl::Align(align);
        /// This element's cross-axis alignment in its parent (`align-self`).
        align_self(align: SxAlign) => Decl::AlignSelf(align);
        /// Main-axis distribution of the children (`justify-content`).
        justify(justify: SxJustify) => Decl::Justify(justify);
        /// Space between children on both axes.
        gap(length: impl Into<SxLength>) => Decl::Gap(length.into());
        /// Space between columns.
        gap_x(length: impl Into<SxLength>) => Decl::GapX(length.into());
        /// Space between rows.
        gap_y(length: impl Into<SxLength>) => Decl::GapY(length.into());
        /// Position in normal flow (`position: relative`).
        relative() => Decl::Absolute(false);
        /// Position relative to the parent, out of flow (`position: absolute`).
        absolute() => Decl::Absolute(true);
        /// Offset an absolutely positioned element from `edges`.
        inset(edges: Edges, length: impl Into<SxLength>) => Decl::Inset(edges, length.into());
        /// Clip content that overflows the element.
        overflow_hidden() => Decl::OverflowHidden(Edges::All);
        /// Width.
        w(length: impl Into<SxLength>) => Decl::Width(length.into());
        /// Height.
        h(length: impl Into<SxLength>) => Decl::Height(length.into());
        /// Minimum width.
        min_w(length: impl Into<SxLength>) => Decl::MinWidth(length.into());
        /// Maximum width.
        max_w(length: impl Into<SxLength>) => Decl::MaxWidth(length.into());
        /// Minimum height.
        min_h(length: impl Into<SxLength>) => Decl::MinHeight(length.into());
        /// Maximum height.
        max_h(length: impl Into<SxLength>) => Decl::MaxHeight(length.into());
        /// Width divided by height.
        aspect_ratio(ratio: f32) => Decl::AspectRatio(ratio);
        /// Padding on every side.
        p(length: impl Into<SxLength>) => Decl::Padding(Edges::All, length.into());
        /// Horizontal padding.
        px(length: impl Into<SxLength>) => Decl::Padding(Edges::X, length.into());
        /// Vertical padding.
        py(length: impl Into<SxLength>) => Decl::Padding(Edges::Y, length.into());
        /// Padding on `edges`.
        padding(edges: Edges, length: impl Into<SxLength>) => Decl::Padding(edges, length.into());
        /// Margin on every side.
        m(length: impl Into<SxLength>) => Decl::Margin(Edges::All, length.into());
        /// Margin on `edges`.
        margin(edges: Edges, length: impl Into<SxLength>) => Decl::Margin(edges, length.into());
        /// Background color.
        bg(color: impl Into<SxColor>) => Decl::Background(color.into());
        /// Text color.
        text_color(color: impl Into<SxColor>) => Decl::TextColor(color.into());
        /// Border color.
        border_color(color: impl Into<SxColor>) => Decl::BorderColor(color.into());
        /// Border width on every side, in pixels.
        border(width: f32) => Decl::BorderWidth(Edges::All, gpui::px(width));
        /// Border width on `edges`, in pixels.
        border_on(edges: Edges, width: f32) => Decl::BorderWidth(edges, gpui::px(width));
        /// Corner radius on every corner.
        rounded(radius: impl Into<SxRadius>) => Decl::Radius(Corners::All, radius.into());
        /// Corner radius on `corners`.
        rounded_on(corners: Corners, radius: impl Into<SxRadius>) => Decl::Radius(corners, radius.into());
        /// Box shadow.
        shadow(shadow: SxShadow) => Decl::Shadow(shadow);
        /// Opacity from 0 (invisible) to 1.
        opacity(opacity: f32) => Decl::Opacity(opacity);
        /// Mouse cursor over the element.
        cursor(cursor: CursorStyle) => Decl::Cursor(cursor);
        /// Font size.
        text(size: SxText) => Decl::Text(size);
        /// Font weight.
        font_weight(weight: FontWeight) => Decl::FontWeight(weight);
        /// Font family.
        font(font: SxFont) => Decl::Font(font);
        /// Line height.
        line_height(length: impl Into<SxLength>) => Decl::LineHeight(length.into());
        /// Horizontal text alignment.
        text_align(align: SxTextAlign) => Decl::TextAlign(align);
        /// Keep text on one line.
        nowrap() => Decl::NoWrap(true);
        /// One line, cut off with an ellipsis.
        truncate() => Decl::Truncate;
        /// At most `lines` lines, then an ellipsis.
        line_clamp(lines: usize) => Decl::LineClamp(lines);
        /// Italic text.
        italic() => Decl::Italic;
        /// Underlined text.
        underline() => Decl::Underline;
        /// Struck-through text.
        line_through() => Decl::LineThrough;
        /// Border style (solid or dashed).
        border_style(border_style: gpui::BorderStyle) => Decl::BorderStyle(border_style);
    }
}

impl Sx {
    /// `rounded(SxRadius::Full)`.
    #[must_use]
    pub fn rounded_full(self) -> Self {
        self.rounded(SxRadius::Full)
    }
}

// ---------------------------------------------------------------------------
// Arguments to `.sx()`.

/// Anything `.sx()` accepts: a style, an optional style, an owned style,
/// arrays and tuples of those (merged left to right).
pub trait SxArg {
    /// Merge this style into `target` (later declarations win).
    fn merge_into(self, target: &mut Sx);
}

/// References work at any depth (`&CARD.base`, `CARD.variant(v)`, `&Some(&style)`).
impl<T: SxArg + Clone> SxArg for &T {
    fn merge_into(self, target: &mut Sx) {
        self.clone().merge_into(target);
    }
}

impl SxArg for Sx {
    fn merge_into(self, target: &mut Sx) {
        if target.is_empty() {
            *target = self;
        } else {
            target.merge(&self);
        }
    }
}

impl<T: SxArg> SxArg for Option<T> {
    fn merge_into(self, target: &mut Sx) {
        if let Some(style) = self {
            style.merge_into(target);
        }
    }
}

impl<T: SxArg, const N: usize> SxArg for [T; N] {
    fn merge_into(self, target: &mut Sx) {
        for style in self {
            style.merge_into(target);
        }
    }
}

impl<T: SxArg> SxArg for Vec<T> {
    fn merge_into(self, target: &mut Sx) {
        for style in self {
            style.merge_into(target);
        }
    }
}

macro_rules! tuple_sx_arg {
    ($($name:ident),+) => {
        impl<$($name: SxArg),+> SxArg for ($($name,)+) {
            #[allow(non_snake_case)]
            fn merge_into(self, target: &mut Sx) {
                let ($($name,)+) = self;
                $($name.merge_into(target);)+
            }
        }
    };
}

tuple_sx_arg!(A);
tuple_sx_arg!(A, B);
tuple_sx_arg!(A, B, C);
tuple_sx_arg!(A, B, C, D);
tuple_sx_arg!(A, B, C, D, E);
tuple_sx_arg!(A, B, C, D, E, F);
tuple_sx_arg!(A, B, C, D, E, F, G);
tuple_sx_arg!(A, B, C, D, E, F, G, H);

/// Merge any [`SxArg`] into one owned style.
pub fn merge(styles: impl SxArg) -> Sx {
    let mut merged = Sx::new();
    styles.merge_into(&mut merged);
    merged
}

// ---------------------------------------------------------------------------
// `.sx()` on elements and components.

/// Apply StyleX-style styles. On GPUI elements the styles are applied at once;
/// on rok-ui components they are stored and applied last, over the
/// component's own styles (`StyleX`'s `xstyle` prop).
pub trait SxStyled: Sized {
    /// Receive a merged style.
    #[must_use]
    fn apply_sx(self, sx: Sx) -> Self;

    /// Merge `styles` left to right (last wins) and apply them.
    #[must_use]
    fn sx(self, styles: impl SxArg) -> Self {
        self.apply_sx(merge(styles))
    }
}

/// Apply a merged style to an interactive element: the base layer refines its
/// style, the state layers become its hover / focus styles.
fn apply_to_interactive<E: Styled + InteractiveElement>(mut element: E, sx: Sx) -> E {
    if sx.is_empty() {
        return element;
    }
    let theme = current_theme();
    let base = resolve(&sx.base, &theme);
    element.style().refine(&base);
    if !sx.hover.is_empty() {
        let hover = resolve(&sx.hover, &theme);
        element = element.hover(move |style| style.refined(hover));
    }
    if !sx.focus.is_empty() && focus_visible() {
        let focus = resolve(&sx.focus, &theme);
        element = element.focus(move |style| style.refined(focus));
    }
    element
}

impl SxStyled for Div {
    fn apply_sx(self, sx: Sx) -> Self {
        apply_to_interactive(self, sx)
    }
}

impl SxStyled for gpui::Svg {
    fn apply_sx(self, sx: Sx) -> Self {
        apply_to_interactive(self, sx)
    }
}

impl SxStyled for gpui::Img {
    fn apply_sx(self, sx: Sx) -> Self {
        apply_to_interactive(self, sx)
    }
}

impl SxStyled for Stateful<Div> {
    fn apply_sx(self, sx: Sx) -> Self {
        use gpui::StatefulInteractiveElement;
        let active = (!sx.active.is_empty()).then(|| resolve(&sx.active, &current_theme()));
        let element = apply_to_interactive(self, sx);
        match active {
            Some(active) => element.active(move |style| style.refined(active)),
            None => element,
        }
    }
}

/// `sx![A.base, condition => A.extra, other]`: merge styles, including some
/// only when a condition holds. Produces an owned [`Sx`].
#[macro_export]
macro_rules! sx {
    ($($tail:tt)*) => {{
        let mut merged = $crate::sx::Sx::new();
        $crate::__sx_merge!(merged; $($tail)*);
        merged
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __sx_merge {
    ($merged:ident;) => {};
    ($merged:ident; $condition:expr => $style:expr $(, $($tail:tt)*)?) => {
        if $condition {
            $crate::sx::SxArg::merge_into(&$style, &mut $merged);
        }
        $($crate::__sx_merge!($merged; $($tail)*);)?
    };
    ($merged:ident; $style:expr $(, $($tail:tt)*)?) => {
        $crate::sx::SxArg::merge_into(&$style, &mut $merged);
        $($crate::__sx_merge!($merged; $($tail)*);)?
    };
}

#[cfg(test)]
mod tests {
    use gpui::{px, AbsoluteLength, DefiniteLength, Length};

    use super::*;
    use crate::theme::{ThemeMode, ThemePreset};

    fn theme() -> Theme {
        Theme::from_preset(ThemePreset::Neutral, ThemeMode::Light)
    }

    #[test]
    fn rtl_mirrors_rows_and_logical_properties() {
        use crate::components::direction::{with_direction, TextDirection};
        use gpui::FlexDirection;

        let row = Sx::new()
            .flex()
            .padding(Edges::Start, 2.)
            .margin(Edges::End, 1.)
            .text_align(SxTextAlign::Start);
        let column = Sx::new().flex_col().flex();
        let theme = theme();

        let ltr = row.resolve_base(&theme);
        assert_eq!(ltr.flex_direction, None);
        assert_eq!(
            ltr.padding.left,
            Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(8.))))
        );
        assert_eq!(ltr.padding.right, None);

        with_direction(TextDirection::Rtl, || {
            let rtl = row.resolve_base(&theme);
            assert_eq!(rtl.flex_direction, Some(FlexDirection::RowReverse));
            assert_eq!(
                rtl.padding.right,
                Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(8.))))
            );
            assert_eq!(rtl.padding.left, None);
            assert_eq!(rtl.margin.left, Some(Length::Definite(px(4.).into())));
            // An explicit column stays a column.
            assert_eq!(
                column.resolve_base(&theme).flex_direction,
                Some(FlexDirection::Column)
            );
        });
    }

    #[test]
    fn numbers_follow_the_four_pixel_scale() {
        let style = Sx::new()
            .gap(6.)
            .w(px(10.))
            .h(SxLength::Full)
            .resolve_base(&theme());
        assert_eq!(
            style.gap.width,
            Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(24.))))
        );
        assert_eq!(
            style.size.width,
            Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(10.))
            )))
        );
        assert_eq!(
            style.size.height,
            Some(Length::Definite(DefiniteLength::Fraction(1.)))
        );
    }

    #[test]
    fn later_styles_win_and_tokens_resolve() {
        let base = Sx::new().p(4.).bg(ColorToken::Card);
        let compact = Sx::new().p(2.);
        let selected = Sx::new().bg(ColorToken::Primary.alpha(0.5));
        let theme = theme();

        let merged = merge((&base, Some(&compact), None::<&Sx>, &selected));
        let style = merged.resolve_base(&theme);
        assert_eq!(
            style.padding.top,
            Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(8.))))
        );
        assert_eq!(
            style.background,
            Some(theme.colors.primary.opacity(0.5).into())
        );

        let with_macro = crate::sx![base, false => compact, true => selected];
        assert_eq!(with_macro, merged_without_compact(&base, &selected));
    }

    fn merged_without_compact(base: &Sx, selected: &Sx) -> Sx {
        merge((base, selected))
    }

    #[test]
    fn states_are_kept_apart() {
        let style = Sx::new()
            .bg(ColorToken::Background)
            .hover(|state| state.bg(ColorToken::Accent));
        assert_eq!(style.base.len(), 1);
        assert_eq!(style.hover.len(), 1);
    }
}
