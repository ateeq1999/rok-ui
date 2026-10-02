//! StyleX-style styling: style objects defined once, merged at use, last wins.
//!
//! ```ignore
//! styles! {
//!     pub CARD = {
//!         base: { display: flex, direction: column, gap: 6, padding: 6, radius: xl,
//!                 border: 1, border_color: border, background: card,
//!                 hover: { border_color: ring } },
//!         compact: { padding: 3 },
//!     }
//! }
//!
//! div().sx((&CARD.base, compact.then_some(&CARD.compact)))
//! div().sx(sx![CARD.base, compact => CARD.compact])
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

use std::{cell::RefCell, rc::Rc};

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
pub(crate) fn set_theme_snapshot(theme: &Theme) {
    THEME_SNAPSHOT.with(|snapshot| *snapshot.borrow_mut() = Some(Rc::new(theme.clone())));
}

/// The theme `.sx()` resolves against (the default theme before `init`).
pub fn current_theme() -> Rc<Theme> {
    THEME_SNAPSHOT.with(|snapshot| {
        snapshot
            .borrow_mut()
            .get_or_insert_with(|| Rc::new(Theme::default()))
            .clone()
    })
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
            $($variant,)*
        }

        impl ColorToken {
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
    pub fn alpha(self, alpha: f32) -> SxColor {
        SxColor::Token(self, alpha)
    }
}

/// A theme token (with opacity) or a fixed color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SxColor {
    Token(ColorToken, f32),
    Value(Hsla),
}

impl SxColor {
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
    None,
    /// `rounded-sm`: radius − 4px.
    Sm,
    /// `rounded-md`: radius − 2px.
    Md,
    /// `rounded-lg`: the theme radius.
    Lg,
    /// `rounded-xl`: radius + 4px.
    Xl,
    Full,
    Length(SxLength),
}

impl SxRadius {
    pub(crate) fn resolve(self, theme: &Theme) -> AbsoluteLength {
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
    None,
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
    /// The 3px focus ring in the theme's ring color.
    Ring,
}

/// Text sizes matching Tailwind's scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SxText {
    Xs,
    Sm,
    Base,
    Lg,
    Xl,
    Xl2,
    Xl3,
    Px(Pixels),
}

/// Font family from the theme.
#[derive(Clone, Debug, PartialEq)]
pub enum SxFont {
    /// `theme.font_family`.
    Sans,
    /// `theme.monospace_font_family`.
    Mono,
    Named(SharedString),
}

/// Which edges a padding, margin, inset or border applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edges {
    All,
    X,
    Y,
    Top,
    Right,
    Bottom,
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
    All,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    /// The starting side's corners (left in LTR).
    Start,
    End,
    TopStart,
    TopEnd,
    BottomStart,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxDisplay {
    Flex,
    Block,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxAlign {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxJustify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SxTextAlign {
    Left,
    Center,
    Right,
    Start,
    End,
}

// ---------------------------------------------------------------------------
// Declarations.

/// One style declaration. You rarely build these by hand: use the [`Sx`]
/// builder methods or the `styles!` / `style!` macros.
#[derive(Clone, Debug, PartialEq)]
pub enum Decl {
    Display(SxDisplay),
    Direction(SxDirection),
    Wrap(bool),
    Flex(SxFlex),
    Grow(f32),
    Shrink(f32),
    Basis(SxLength),
    Align(SxAlign),
    AlignSelf(SxAlign),
    Justify(SxJustify),
    Gap(SxLength),
    GapX(SxLength),
    GapY(SxLength),
    Absolute(bool),
    Inset(Edges, SxLength),
    OverflowHidden(Edges),
    Width(SxLength),
    Height(SxLength),
    MinWidth(SxLength),
    MaxWidth(SxLength),
    MinHeight(SxLength),
    MaxHeight(SxLength),
    AspectRatio(f32),
    Padding(Edges, SxLength),
    Margin(Edges, SxLength),
    Background(SxColor),
    TextColor(SxColor),
    BorderColor(SxColor),
    BorderWidth(Edges, Pixels),
    Radius(Corners, SxRadius),
    Shadow(SxShadow),
    Opacity(f32),
    Cursor(CursorStyle),
    Text(SxText),
    FontWeight(FontWeight),
    Font(SxFont),
    LineHeight(SxLength),
    TextAlign(SxTextAlign),
    NoWrap(bool),
    Truncate,
    LineClamp(usize),
    Italic,
    Underline,
    LineThrough,
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
                |layer, value| layer.p(value),
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
                |layer, value| layer.m(value),
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
            pub fn $name(mut self, $($arg: $ty),*) -> Self {
                self.base.push($decl);
                self
            }
        )*
    };
}

impl Sx {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a declaration (later declarations win).
    pub fn decl(mut self, decl: Decl) -> Self {
        self.base.push(decl);
        self
    }

    /// Styles while the pointer is over the element.
    pub fn hover(mut self, styles: impl FnOnce(Sx) -> Sx) -> Self {
        let state = styles(Sx::new());
        self.hover.extend(state.base);
        self
    }

    /// Styles while the element has keyboard focus (focusable elements only).
    pub fn focus(mut self, styles: impl FnOnce(Sx) -> Sx) -> Self {
        let state = styles(Sx::new());
        self.focus.extend(state.base);
        self
    }

    /// Styles while the element is pressed (elements with an id only).
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

    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
            && self.hover.is_empty()
            && self.focus.is_empty()
            && self.active.is_empty()
    }

    /// The resting-state declarations resolved against `theme`.
    pub fn resolve_base(&self, theme: &Theme) -> StyleRefinement {
        resolve(&self.base, theme)
    }

    decl_methods! {
        flex() => Decl::Display(SxDisplay::Flex);
        block() => Decl::Display(SxDisplay::Block);
        hidden() => Decl::Display(SxDisplay::Hidden);
        direction(direction: SxDirection) => Decl::Direction(direction);
        flex_row() => Decl::Direction(SxDirection::Row);
        flex_col() => Decl::Direction(SxDirection::Column);
        wrap(wrap: bool) => Decl::Wrap(wrap);
        flex_1() => Decl::Flex(SxFlex::One);
        flex_auto() => Decl::Flex(SxFlex::Auto);
        flex_none() => Decl::Flex(SxFlex::None);
        grow(grow: f32) => Decl::Grow(grow);
        shrink(shrink: f32) => Decl::Shrink(shrink);
        basis(length: impl Into<SxLength>) => Decl::Basis(length.into());
        align(align: SxAlign) => Decl::Align(align);
        align_self(align: SxAlign) => Decl::AlignSelf(align);
        justify(justify: SxJustify) => Decl::Justify(justify);
        gap(length: impl Into<SxLength>) => Decl::Gap(length.into());
        gap_x(length: impl Into<SxLength>) => Decl::GapX(length.into());
        gap_y(length: impl Into<SxLength>) => Decl::GapY(length.into());
        relative() => Decl::Absolute(false);
        absolute() => Decl::Absolute(true);
        inset(edges: Edges, length: impl Into<SxLength>) => Decl::Inset(edges, length.into());
        overflow_hidden() => Decl::OverflowHidden(Edges::All);
        w(length: impl Into<SxLength>) => Decl::Width(length.into());
        h(length: impl Into<SxLength>) => Decl::Height(length.into());
        min_w(length: impl Into<SxLength>) => Decl::MinWidth(length.into());
        max_w(length: impl Into<SxLength>) => Decl::MaxWidth(length.into());
        min_h(length: impl Into<SxLength>) => Decl::MinHeight(length.into());
        max_h(length: impl Into<SxLength>) => Decl::MaxHeight(length.into());
        aspect_ratio(ratio: f32) => Decl::AspectRatio(ratio);
        p(length: impl Into<SxLength>) => Decl::Padding(Edges::All, length.into());
        px(length: impl Into<SxLength>) => Decl::Padding(Edges::X, length.into());
        py(length: impl Into<SxLength>) => Decl::Padding(Edges::Y, length.into());
        padding(edges: Edges, length: impl Into<SxLength>) => Decl::Padding(edges, length.into());
        m(length: impl Into<SxLength>) => Decl::Margin(Edges::All, length.into());
        margin(edges: Edges, length: impl Into<SxLength>) => Decl::Margin(edges, length.into());
        bg(color: impl Into<SxColor>) => Decl::Background(color.into());
        text_color(color: impl Into<SxColor>) => Decl::TextColor(color.into());
        border_color(color: impl Into<SxColor>) => Decl::BorderColor(color.into());
        /// Border width on every side, in pixels.
        border(width: f32) => Decl::BorderWidth(Edges::All, gpui::px(width));
        border_on(edges: Edges, width: f32) => Decl::BorderWidth(edges, gpui::px(width));
        rounded(radius: impl Into<SxRadius>) => Decl::Radius(Corners::All, radius.into());
        rounded_on(corners: Corners, radius: impl Into<SxRadius>) => Decl::Radius(corners, radius.into());
        shadow(shadow: SxShadow) => Decl::Shadow(shadow);
        opacity(opacity: f32) => Decl::Opacity(opacity);
        cursor(cursor: CursorStyle) => Decl::Cursor(cursor);
        text(size: SxText) => Decl::Text(size);
        font_weight(weight: FontWeight) => Decl::FontWeight(weight);
        font(font: SxFont) => Decl::Font(font);
        line_height(length: impl Into<SxLength>) => Decl::LineHeight(length.into());
        text_align(align: SxTextAlign) => Decl::TextAlign(align);
        nowrap() => Decl::NoWrap(true);
        truncate() => Decl::Truncate;
        line_clamp(lines: usize) => Decl::LineClamp(lines);
        italic() => Decl::Italic;
        underline() => Decl::Underline;
        line_through() => Decl::LineThrough;
    }
}

impl Sx {
    /// `rounded(SxRadius::Full)`.
    pub fn rounded_full(self) -> Self {
        self.rounded(SxRadius::Full)
    }
}

// ---------------------------------------------------------------------------
// Arguments to `.sx()`.

/// Anything `.sx()` accepts: a style, an optional style, an owned style,
/// arrays and tuples of those (merged left to right).
pub trait SxArg {
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
/// component's own styles (StyleX's `xstyle` prop).
pub trait SxStyled: Sized {
    /// Receive a merged style.
    fn apply_sx(self, sx: Sx) -> Self;

    /// Merge `styles` left to right (last wins) and apply them.
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
    if !sx.focus.is_empty() {
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
