//! The values a declaration takes: lengths, colors, radii, shadows and layout keywords.

use gpui::{
    px, relative, AbsoluteLength, DefiniteLength, Hsla, Length, Pixels, Rgba, SharedString,
};

use crate::theme::{Theme, ThemeColors};

/// A length on Tailwind's scale. Numbers are multiples of 4px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SxLength {
    /// Spacing units: `6.0` is 24px.
    Units(f32),
    /// Exact pixels.
    Px(Pixels),
    /// A fraction of the parent: `0.5` is 50%.
    Relative(f32),
    /// 100% of the parent.
    Full,
    /// Sized by content (`auto`).
    Auto,
}

impl SxLength {
    pub(super) fn length(self) -> Length {
        match self {
            SxLength::Units(units) => px(units * 4.).into(),
            SxLength::Px(pixels) => pixels.into(),
            SxLength::Relative(fraction) => relative(fraction).into(),
            SxLength::Full => relative(1.).into(),
            SxLength::Auto => Length::Auto,
        }
    }

    pub(super) fn definite(self) -> DefiniteLength {
        match self {
            SxLength::Units(units) => px(units * 4.).into(),
            SxLength::Px(pixels) => pixels.into(),
            SxLength::Relative(fraction) => relative(fraction),
            SxLength::Full => relative(1.),
            SxLength::Auto => px(0.).into(),
        }
    }

    pub(super) fn absolute(self) -> AbsoluteLength {
        match self {
            SxLength::Units(units) => px(units * 4.).into(),
            SxLength::Px(pixels) => pixels.into(),
            _ => px(0.).into(),
        }
    }
}

impl From<f32> for SxLength {
    fn from(units: f32) -> Self {
        SxLength::Units(units)
    }
}

impl From<i32> for SxLength {
    fn from(units: i32) -> Self {
        SxLength::Units(units as f32)
    }
}

impl From<Pixels> for SxLength {
    fn from(pixels: Pixels) -> Self {
        SxLength::Px(pixels)
    }
}

impl From<DefiniteLength> for SxLength {
    fn from(length: DefiniteLength) -> Self {
        match length {
            DefiniteLength::Fraction(fraction) => SxLength::Relative(fraction),
            DefiniteLength::Absolute(AbsoluteLength::Pixels(pixels)) => SxLength::Px(pixels),
            DefiniteLength::Absolute(AbsoluteLength::Rems(rems)) => SxLength::Px(px(rems.0 * 16.)),
        }
    }
}

macro_rules! color_tokens {
    ($($variant:ident => $field:ident),* $(,)?) => {
        /// A color from the active theme, one per [`ThemeColors`] field.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ColorToken {
            $(
                #[doc = concat!("The theme's `", stringify!($field), "` color.")]
                $variant,
            )*
        }

        impl ColorToken {
            /// The color this token stands for in `colors`.
            pub fn resolve(self, colors: &ThemeColors) -> Hsla {
                match self {
                    $(ColorToken::$variant => colors.$field,)*
                }
            }
        }
    };
}

color_tokens! {
    Background => background,
    Foreground => foreground,
    Card => card,
    CardForeground => card_foreground,
    Popover => popover,
    PopoverForeground => popover_foreground,
    Primary => primary,
    PrimaryForeground => primary_foreground,
    Secondary => secondary,
    SecondaryForeground => secondary_foreground,
    Muted => muted,
    MutedForeground => muted_foreground,
    Accent => accent,
    AccentForeground => accent_foreground,
    Destructive => destructive,
    DestructiveForeground => destructive_foreground,
    DestructiveText => destructive_text,
    Border => border,
    Input => input,
    Ring => ring,
    Overlay => overlay,
}

impl ColorToken {
    /// The token at `alpha` opacity (Tailwind's `bg-primary/90` is `alpha(0.9)`).
    #[must_use]
    pub fn alpha(self, alpha: f32) -> SxColor {
        SxColor::Token(self, alpha)
    }
}

/// A theme token (with opacity) or a fixed color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SxColor {
    /// A theme color at an opacity from 0 to 1.
    Token(ColorToken, f32),
    /// A fixed color.
    Value(Hsla),
}

impl SxColor {
    /// Fully transparent.
    pub const TRANSPARENT: SxColor = SxColor::Value(Hsla {
        h: 0.,
        s: 0.,
        l: 0.,
        a: 0.,
    });

    pub(super) fn resolve(self, colors: &ThemeColors) -> Hsla {
        match self {
            SxColor::Token(token, alpha) => {
                let color = token.resolve(colors);
                color.opacity(alpha)
            }
            SxColor::Value(color) => color,
        }
    }
}

impl From<ColorToken> for SxColor {
    fn from(token: ColorToken) -> Self {
        SxColor::Token(token, 1.)
    }
}

impl From<Hsla> for SxColor {
    fn from(color: Hsla) -> Self {
        SxColor::Value(color)
    }
}

impl From<Rgba> for SxColor {
    fn from(color: Rgba) -> Self {
        SxColor::Value(color.into())
    }
}

/// A corner radius from the theme, or an exact length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SxRadius {
    /// Square corners.
    None,
    /// `rounded-sm`: radius − 4px.
    Sm,
    /// `rounded-md`: radius − 2px.
    Md,
    /// `rounded-lg`: the theme radius.
    Lg,
    /// `rounded-xl`: radius + 4px.
    Xl,
    /// Fully round (pills and circles).
    Full,
    /// An exact radius.
    Length(SxLength),
}

impl SxRadius {
    #[doc(hidden)]
    #[must_use]
    pub fn resolve(self, theme: &Theme) -> AbsoluteLength {
        match self {
            SxRadius::None => px(0.).into(),
            SxRadius::Sm => theme.radius_small().into(),
            SxRadius::Md => theme.radius_medium().into(),
            SxRadius::Lg => theme.radius_large().into(),
            SxRadius::Xl => theme.radius_extra_large().into(),
            SxRadius::Full => px(9999.).into(),
            SxRadius::Length(length) => length.absolute(),
        }
    }
}

impl<T: Into<SxLength>> From<T> for SxRadius {
    fn from(length: T) -> Self {
        SxRadius::Length(length.into())
    }
}

/// Box shadows matching shadcn/ui's scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxShadow {
    /// No shadow.
    None,
    /// `shadow-xs`.
    Xs,
    /// `shadow-sm`.
    Sm,
    /// `shadow-md`.
    Md,
    /// `shadow-lg`.
    Lg,
    /// `shadow-xl`.
    Xl,
    /// The 3px focus ring in the theme's ring color.
    Ring,
}

/// Text sizes matching Tailwind's scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SxText {
    /// The theme's base size (`theme.font_size`).
    Theme,
    /// 12px.
    Xs,
    /// 14px.
    Sm,
    /// 16px.
    Base,
    /// 18px.
    Lg,
    /// 20px.
    Xl,
    /// 24px.
    Xl2,
    /// 30px.
    Xl3,
    /// An exact size.
    Px(Pixels),
}

/// Font family from the theme.
#[derive(Clone, Debug, PartialEq)]
pub enum SxFont {
    /// `theme.font_family`.
    Sans,
    /// `theme.monospace_font_family`.
    Mono,
    /// A family by name, such as `"Cairo"`.
    Named(SharedString),
}

/// Which edges a padding, margin, inset or border applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edges {
    /// Every side.
    All,
    /// Left and right.
    X,
    /// Top and bottom.
    Y,
    /// The top.
    Top,
    /// The right side.
    Right,
    /// The bottom.
    Bottom,
    /// The left side.
    Left,
    /// Left in LTR, right in RTL.
    Start,
    /// Right in LTR, left in RTL.
    End,
}

impl Edges {
    /// Logical edges resolved for the current direction.
    pub(super) fn physical(self) -> Edges {
        let rtl = crate::components::direction::is_rtl();
        match (self, rtl) {
            (Edges::Start, false) | (Edges::End, true) => Edges::Left,
            (Edges::Start, true) | (Edges::End, false) => Edges::Right,
            (other, _) => other,
        }
    }
}

/// Which corners a radius applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Corners {
    /// Every corner.
    All,
    /// Both top corners.
    Top,
    /// Both bottom corners.
    Bottom,
    /// Both left corners.
    Left,
    /// Both right corners.
    Right,
    /// The top-left corner.
    TopLeft,
    /// The top-right corner.
    TopRight,
    /// The bottom-left corner.
    BottomLeft,
    /// The bottom-right corner.
    BottomRight,
    /// The starting side's corners (left in LTR).
    Start,
    /// The ending side's corners (right in LTR).
    End,
    /// The top corner at the start.
    TopStart,
    /// The top corner at the end.
    TopEnd,
    /// The bottom corner at the start.
    BottomStart,
    /// The bottom corner at the end.
    BottomEnd,
}

impl Corners {
    /// Logical corners resolved for the current direction.
    pub(super) fn physical(self) -> Corners {
        let rtl = crate::components::direction::is_rtl();
        let (start, end) = if rtl {
            (Corners::Right, Corners::Left)
        } else {
            (Corners::Left, Corners::Right)
        };
        let (top_start, top_end, bottom_start, bottom_end) = if rtl {
            (
                Corners::TopRight,
                Corners::TopLeft,
                Corners::BottomRight,
                Corners::BottomLeft,
            )
        } else {
            (
                Corners::TopLeft,
                Corners::TopRight,
                Corners::BottomLeft,
                Corners::BottomRight,
            )
        };
        match self {
            Corners::Start => start,
            Corners::End => end,
            Corners::TopStart => top_start,
            Corners::TopEnd => top_end,
            Corners::BottomStart => bottom_start,
            Corners::BottomEnd => bottom_end,
            other => other,
        }
    }
}

/// How an element is displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxDisplay {
    /// A flex container.
    Flex,
    /// A block.
    Block,
    /// Not displayed and takes no space.
    Hidden,
}

/// The direction flex children are laid out in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxDirection {
    /// A row in the reading direction: right to left in RTL.
    Row,
    /// Top to bottom.
    Column,
    /// A row against the reading direction.
    RowReverse,
    /// Bottom to top.
    ColumnReverse,
    /// A row that runs left to right in every direction (charts, codes, numbers).
    RowLtr,
}

/// Cross-axis alignment (`align-items`, `align-self`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxAlign {
    /// At the start.
    Start,
    /// Centered.
    Center,
    /// At the end.
    End,
    /// Filling the cross axis.
    Stretch,
    /// Text baselines lined up.
    Baseline,
}

/// Main-axis distribution (`justify-content`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxJustify {
    /// Packed at the start.
    Start,
    /// Packed in the center.
    Center,
    /// Packed at the end.
    End,
    /// Space between children only.
    Between,
    /// Equal space around each child.
    Around,
    /// Equal space between children and at the ends.
    Evenly,
}

/// The `flex` shorthand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxFlex {
    /// `flex: 1 1 0%`.
    One,
    /// `flex: 1 1 auto`.
    Auto,
    /// `flex: 0 1 auto`.
    Initial,
    /// `flex: none`.
    None,
}

/// Horizontal text alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxTextAlign {
    /// Left, in every direction.
    Left,
    /// Centered.
    Center,
    /// Right, in every direction.
    Right,
    /// At the reading-direction start.
    Start,
    /// At the reading-direction end.
    End,
}
