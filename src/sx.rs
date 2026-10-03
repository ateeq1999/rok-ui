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

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    px, relative, AbsoluteLength, AlignItems, AlignSelf, CursorStyle, DefiniteLength, Div,
    FontWeight, Hsla, InteractiveElement, JustifyContent, Length, Pixels, Refineable, Rgba,
    SharedString, Stateful, StyleRefinement, Styled,
};

use crate::{
    components::direction::DirectionalStyled,
    components::{extra_small_shadow, focus_ring_shadow},
    theme::{Theme, ThemeColors},
};

// ---------------------------------------------------------------------------
// The active theme, for resolving tokens without a context.

thread_local! {
    static THEME_SNAPSHOT: RefCell<Option<Rc<Theme>>> = const { RefCell::new(None) };
}

/// Record the theme that `.sx()` resolves tokens against. Called by
/// [`Theme::set_global`] and [`crate::init`]; GPUI renders on one thread.
#[doc(hidden)]
pub fn set_theme_snapshot(theme: &Theme) {
    THEME_SNAPSHOT.with(|snapshot| *snapshot.borrow_mut() = Some(Rc::new(theme.clone())));
}

/// The theme `.sx()` resolves against (the default theme before `init`).
#[must_use]
pub fn current_theme() -> Rc<Theme> {
    THEME_SNAPSHOT.with(|snapshot| {
        snapshot
            .borrow_mut()
            .get_or_insert_with(|| Rc::new(Theme::default()))
            .clone()
    })
}

// ---------------------------------------------------------------------------
// Focus visibility, like CSS `:focus-visible`.

/// When `focus:` styles (focus rings) show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusRingMode {
    /// Only after keyboard input, like CSS `:focus-visible`: clicking a button
    /// focuses it without a ring, tabbing to it shows one.
    #[default]
    KeyboardOnly,
    /// Whenever the element is focused, however focus got there.
    Always,
}

thread_local! {
    static FOCUS_RING_MODE: Cell<FocusRingMode> = const { Cell::new(FocusRingMode::KeyboardOnly) };
    /// Whether the most recent input was the keyboard (rather than a pointer).
    static KEYBOARD_MODALITY: Cell<bool> = const { Cell::new(false) };
}

/// Choose when focus rings show. The default is [`FocusRingMode::KeyboardOnly`].
pub fn set_focus_ring_mode(mode: FocusRingMode) {
    FOCUS_RING_MODE.with(|current| current.set(mode));
}

/// Whether `focus:` styles apply right now. Text inputs show their ring whenever
/// they are focused, as browsers do, and do not depend on this.
pub fn focus_visible() -> bool {
    FOCUS_RING_MODE.with(Cell::get) == FocusRingMode::Always || KEYBOARD_MODALITY.with(Cell::get)
}

/// Record whether the latest input came from the keyboard. Returns whether that
/// changed, in which case the window needs a redraw.
#[doc(hidden)]
#[must_use]
pub fn set_keyboard_modality(keyboard: bool) -> bool {
    KEYBOARD_MODALITY.with(|current| current.replace(keyboard) != keyboard)
}

// ---------------------------------------------------------------------------
// Values.

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
    fn length(self) -> Length {
        match self {
            SxLength::Units(units) => px(units * 4.).into(),
            SxLength::Px(pixels) => pixels.into(),
            SxLength::Relative(fraction) => relative(fraction).into(),
            SxLength::Full => relative(1.).into(),
            SxLength::Auto => Length::Auto,
        }
    }

    fn definite(self) -> DefiniteLength {
        match self {
            SxLength::Units(units) => px(units * 4.).into(),
            SxLength::Px(pixels) => pixels.into(),
            SxLength::Relative(fraction) => relative(fraction),
            SxLength::Full => relative(1.),
            SxLength::Auto => px(0.).into(),
        }
    }

    fn absolute(self) -> AbsoluteLength {
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

    fn resolve(self, colors: &ThemeColors) -> Hsla {
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
    fn physical(self) -> Edges {
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
    fn physical(self) -> Corners {
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

// ---------------------------------------------------------------------------
// Declarations.

/// One style declaration. You rarely build these by hand: use the [`Sx`]
/// builder methods or the `styles!` / `style!` macros.
#[derive(Clone, Debug, PartialEq)]
pub enum Decl {
    /// Display mode ([`Sx::flex`], [`Sx::hidden`]).
    Display(SxDisplay),
    /// Flex direction ([`Sx::direction`]).
    Direction(SxDirection),
    /// Flex wrapping ([`Sx::wrap`]).
    Wrap(bool),
    /// The `flex` shorthand ([`Sx::flex_1`]).
    Flex(SxFlex),
    /// Flex grow ([`Sx::grow`]).
    Grow(f32),
    /// Flex shrink ([`Sx::shrink`]).
    Shrink(f32),
    /// Flex basis ([`Sx::basis`]).
    Basis(SxLength),
    /// `align-items` ([`Sx::align`]).
    Align(SxAlign),
    /// `align-self` ([`Sx::align_self`]).
    AlignSelf(SxAlign),
    /// `justify-content` ([`Sx::justify`]).
    Justify(SxJustify),
    /// Gap on both axes ([`Sx::gap`]).
    Gap(SxLength),
    /// Column gap ([`Sx::gap_x`]).
    GapX(SxLength),
    /// Row gap ([`Sx::gap_y`]).
    GapY(SxLength),
    /// Absolute (`true`) or relative positioning ([`Sx::absolute`]).
    Absolute(bool),
    /// Offsets of a positioned element ([`Sx::inset`]).
    Inset(Edges, SxLength),
    /// Clip overflow ([`Sx::overflow_hidden`]).
    OverflowHidden(Edges),
    /// Width ([`Sx::w`]).
    Width(SxLength),
    /// Height ([`Sx::h`]).
    Height(SxLength),
    /// Minimum width ([`Sx::min_w`]).
    MinWidth(SxLength),
    /// Maximum width ([`Sx::max_w`]).
    MaxWidth(SxLength),
    /// Minimum height ([`Sx::min_h`]).
    MinHeight(SxLength),
    /// Maximum height ([`Sx::max_h`]).
    MaxHeight(SxLength),
    /// Aspect ratio ([`Sx::aspect_ratio`]).
    AspectRatio(f32),
    /// Padding ([`Sx::padding`]).
    Padding(Edges, SxLength),
    /// Margin ([`Sx::margin`]).
    Margin(Edges, SxLength),
    /// Background color ([`Sx::bg`]).
    Background(SxColor),
    /// Text color ([`Sx::text_color`]).
    TextColor(SxColor),
    /// Border color ([`Sx::border_color`]).
    BorderColor(SxColor),
    /// Border width ([`Sx::border_on`]).
    BorderWidth(Edges, Pixels),
    /// Corner radius ([`Sx::rounded_on`]).
    Radius(Corners, SxRadius),
    /// Box shadow ([`Sx::shadow`]).
    Shadow(SxShadow),
    /// Opacity ([`Sx::opacity`]).
    Opacity(f32),
    /// Mouse cursor ([`Sx::cursor`]).
    Cursor(CursorStyle),
    /// Font size ([`Sx::text`]).
    Text(SxText),
    /// Font weight ([`Sx::font_weight`]).
    FontWeight(FontWeight),
    /// Font family ([`Sx::font`]).
    Font(SxFont),
    /// Line height ([`Sx::line_height`]).
    LineHeight(SxLength),
    /// Text alignment ([`Sx::text_align`]).
    TextAlign(SxTextAlign),
    /// Keep text on one line ([`Sx::nowrap`]).
    NoWrap(bool),
    /// One line with an ellipsis ([`Sx::truncate`]).
    Truncate,
    /// Clamp to lines ([`Sx::line_clamp`]).
    LineClamp(usize),
    /// Italic ([`Sx::italic`]).
    Italic,
    /// Underline ([`Sx::underline`]).
    Underline,
    /// Strike-through ([`Sx::line_through`]).
    LineThrough,
    /// Border style ([`Sx::border_style`]).
    BorderStyle(gpui::BorderStyle),
}

/// Wraps a refinement so the GPUI `Styled` helpers can write into it.
struct Layer(StyleRefinement);

impl Styled for Layer {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.0
    }
}

fn apply_edges<L>(
    layer: Layer,
    edges: Edges,
    value: L,
    all: impl FnOnce(Layer, L) -> Layer,
    each: impl Fn(Layer, Edges, L) -> Layer,
) -> Layer
where
    L: Copy,
{
    match edges.physical() {
        Edges::All => all(layer, value),
        Edges::X => each(each(layer, Edges::Left, value), Edges::Right, value),
        Edges::Y => each(each(layer, Edges::Top, value), Edges::Bottom, value),
        side => each(layer, side, value),
    }
}

impl Decl {
    fn apply(&self, layer: Layer, theme: &Theme) -> Layer {
        let colors = &theme.colors;
        match self {
            // Rows flow in the reading direction, like CSS flex in `dir="rtl"`.
            Decl::Display(SxDisplay::Flex) => {
                let layer = layer.flex();
                if crate::components::direction::is_rtl() && layer.0.flex_direction.is_none() {
                    layer.flex_row_reverse()
                } else {
                    layer
                }
            }
            Decl::Display(SxDisplay::Block) => layer.block(),
            Decl::Display(SxDisplay::Hidden) => layer.hidden(),
            Decl::Direction(SxDirection::Row) if crate::components::direction::is_rtl() => {
                layer.flex_row_reverse()
            }
            Decl::Direction(SxDirection::RowReverse) if crate::components::direction::is_rtl() => {
                layer.flex_row()
            }
            Decl::Direction(SxDirection::Row) => layer.flex_row(),
            Decl::Direction(SxDirection::Column) => layer.flex_col(),
            Decl::Direction(SxDirection::RowReverse) => layer.flex_row_reverse(),
            Decl::Direction(SxDirection::ColumnReverse) => layer.flex_col_reverse(),
            Decl::Direction(SxDirection::RowLtr) => layer.flex_row(),
            Decl::Wrap(true) => layer.flex_wrap(),
            Decl::Wrap(false) => layer.flex_nowrap(),
            Decl::Flex(SxFlex::One) => layer.flex_1(),
            Decl::Flex(SxFlex::Auto) => layer.flex_auto(),
            Decl::Flex(SxFlex::Initial) => layer.flex_initial(),
            Decl::Flex(SxFlex::None) => layer.flex_none(),
            Decl::Grow(grow) => {
                let mut layer = layer;
                layer.0.flex_grow = Some(*grow);
                layer
            }
            Decl::Shrink(shrink) => {
                let mut layer = layer;
                layer.0.flex_shrink = Some(*shrink);
                layer
            }
            Decl::Basis(length) => layer.flex_basis(length.length()),
            Decl::Align(align) => {
                let mut layer = layer;
                layer.0.align_items = Some(match align {
                    SxAlign::Start => AlignItems::FlexStart,
                    SxAlign::Center => AlignItems::Center,
                    SxAlign::End => AlignItems::FlexEnd,
                    SxAlign::Stretch => AlignItems::Stretch,
                    SxAlign::Baseline => AlignItems::Baseline,
                });
                layer
            }
            Decl::AlignSelf(align) => {
                let mut layer = layer;
                layer.0.align_self = Some(match align {
                    SxAlign::Start => AlignSelf::FlexStart,
                    SxAlign::Center => AlignSelf::Center,
                    SxAlign::End => AlignSelf::FlexEnd,
                    SxAlign::Stretch => AlignSelf::Stretch,
                    SxAlign::Baseline => AlignSelf::Baseline,
                });
                layer
            }
            Decl::Justify(justify) => {
                let mut layer = layer;
                layer.0.justify_content = Some(match justify {
                    SxJustify::Start => JustifyContent::FlexStart,
                    SxJustify::Center => JustifyContent::Center,
                    SxJustify::End => JustifyContent::FlexEnd,
                    SxJustify::Between => JustifyContent::SpaceBetween,
                    SxJustify::Around => JustifyContent::SpaceAround,
                    SxJustify::Evenly => JustifyContent::SpaceEvenly,
                });
                layer
            }
            Decl::Gap(length) => layer.gap(length.definite()),
            Decl::GapX(length) => layer.gap_x(length.definite()),
            Decl::GapY(length) => layer.gap_y(length.definite()),
            Decl::Absolute(true) => layer.absolute(),
            Decl::Absolute(false) => layer.relative(),
            Decl::Inset(edges, length) => apply_edges(
                layer,
                *edges,
                length.length(),
                |layer, value| layer.top(value).right(value).bottom(value).left(value),
                |layer, edge, value| match edge {
                    Edges::Top => layer.top(value),
                    Edges::Right => layer.right(value),
                    Edges::Bottom => layer.bottom(value),
                    _ => layer.left(value),
                },
            ),
            Decl::OverflowHidden(Edges::X) => layer.overflow_x_hidden(),
            Decl::OverflowHidden(Edges::Y) => layer.overflow_y_hidden(),
            Decl::OverflowHidden(_) => layer.overflow_hidden(),
            Decl::Width(length) => layer.w(length.length()),
            Decl::Height(length) => layer.h(length.length()),
            Decl::MinWidth(length) => layer.min_w(length.length()),
            Decl::MaxWidth(length) => layer.max_w(length.length()),
            Decl::MinHeight(length) => layer.min_h(length.length()),
            Decl::MaxHeight(length) => layer.max_h(length.length()),
            Decl::AspectRatio(ratio) => {
                let mut layer = layer;
                layer.0.aspect_ratio = Some(*ratio);
                layer
            }
            Decl::Padding(edges, length) => apply_edges(
                layer,
                *edges,
                length.definite(),
                gpui::Styled::p,
                |layer, edge, value| match edge {
                    Edges::Top => layer.pt(value),
                    Edges::Right => layer.pr(value),
                    Edges::Bottom => layer.pb(value),
                    _ => layer.pl(value),
                },
            ),
            Decl::Margin(edges, length) => apply_edges(
                layer,
                *edges,
                length.length(),
                gpui::Styled::m,
                |layer, edge, value| match edge {
                    Edges::Top => layer.mt(value),
                    Edges::Right => layer.mr(value),
                    Edges::Bottom => layer.mb(value),
                    _ => layer.ml(value),
                },
            ),
            Decl::Background(color) => layer.bg(color.resolve(colors)),
            Decl::TextColor(color) => layer.text_color(color.resolve(colors)),
            Decl::BorderColor(color) => layer.border_color(color.resolve(colors)),
            Decl::BorderWidth(edges, width) => {
                let mut layer = layer;
                let width = Some(AbsoluteLength::from(*width));
                let widths = &mut layer.0.border_widths;
                match edges.physical() {
                    Edges::All => {
                        widths.top = width;
                        widths.right = width;
                        widths.bottom = width;
                        widths.left = width;
                    }
                    Edges::X => {
                        widths.left = width;
                        widths.right = width;
                    }
                    Edges::Y => {
                        widths.top = width;
                        widths.bottom = width;
                    }
                    Edges::Top => widths.top = width,
                    Edges::Right => widths.right = width,
                    Edges::Bottom => widths.bottom = width,
                    Edges::Left => widths.left = width,
                    // `physical()` never returns logical edges.
                    Edges::Start | Edges::End => {}
                }
                layer
            }
            Decl::Radius(corners, radius) => {
                let value = radius.resolve(theme);
                match corners.physical() {
                    Corners::All => layer.rounded(value),
                    Corners::Top => layer.rounded_t(value),
                    Corners::Bottom => layer.rounded_b(value),
                    Corners::Left => layer.rounded_l(value),
                    Corners::Right => layer.rounded_r(value),
                    Corners::TopLeft => layer.rounded_tl(value),
                    Corners::TopRight => layer.rounded_tr(value),
                    Corners::BottomLeft => layer.rounded_bl(value),
                    Corners::BottomRight => layer.rounded_br(value),
                    // `physical()` never returns logical corners.
                    Corners::Start
                    | Corners::End
                    | Corners::TopStart
                    | Corners::TopEnd
                    | Corners::BottomStart
                    | Corners::BottomEnd => layer,
                }
            }
            Decl::Shadow(shadow) => match shadow {
                SxShadow::None => layer.shadow(Vec::new()),
                SxShadow::Xs => layer.shadow(extra_small_shadow()),
                SxShadow::Sm => layer.shadow_sm(),
                SxShadow::Md => layer.shadow_md(),
                SxShadow::Lg => layer.shadow_lg(),
                SxShadow::Xl => layer.shadow_xl(),
                SxShadow::Ring => layer.shadow(focus_ring_shadow(colors.ring)),
            },
            Decl::Opacity(opacity) => layer.opacity(*opacity),
            Decl::Cursor(cursor) => layer.cursor(*cursor),
            Decl::Text(text) => match text {
                SxText::Theme => layer.text_size(theme.font_size),
                SxText::Xs => layer.text_xs(),
                SxText::Sm => layer.text_sm(),
                SxText::Base => layer.text_base(),
                SxText::Lg => layer.text_lg(),
                SxText::Xl => layer.text_xl(),
                SxText::Xl2 => layer.text_2xl(),
                SxText::Xl3 => layer.text_3xl(),
                SxText::Px(size) => layer.text_size(*size),
            },
            Decl::FontWeight(weight) => layer.font_weight(*weight),
            Decl::Font(SxFont::Sans) => layer.font_family(theme.font_family.clone()),
            Decl::Font(SxFont::Mono) => layer.font_family(theme.monospace_font_family.clone()),
            Decl::Font(SxFont::Named(name)) => layer.font_family(name.clone()),
            Decl::LineHeight(length) => layer.line_height(length.definite()),
            Decl::TextAlign(SxTextAlign::Left) => layer.text_left(),
            Decl::TextAlign(SxTextAlign::Center) => layer.text_center(),
            Decl::TextAlign(SxTextAlign::Right) => layer.text_right(),
            Decl::TextAlign(SxTextAlign::Start) => layer.text_start(),
            Decl::TextAlign(SxTextAlign::End) => layer.text_end(),
            Decl::NoWrap(true) => layer.whitespace_nowrap(),
            Decl::NoWrap(false) => layer.whitespace_normal(),
            Decl::Truncate => layer.truncate(),
            Decl::LineClamp(lines) => layer.line_clamp(*lines),
            Decl::Italic => layer.italic(),
            Decl::Underline => layer.underline(),
            Decl::LineThrough => layer.line_through(),
            Decl::BorderStyle(border_style) => {
                let mut layer = layer;
                layer.0.border_style = Some(*border_style);
                layer
            }
        }
    }
}

fn resolve(decls: &[Decl], theme: &Theme) -> StyleRefinement {
    decls
        .iter()
        .fold(Layer(StyleRefinement::default()), |layer, decl| {
            decl.apply(layer, theme)
        })
        .0
}

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
