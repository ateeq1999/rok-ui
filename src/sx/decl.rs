//! Declarations ([`Decl`]) and how they apply to a GPUI style.

use gpui::{
    AbsoluteLength, AlignItems, AlignSelf, CursorStyle, FontWeight, JustifyContent, Pixels,
    StyleRefinement, Styled,
};

#[cfg(doc)]
use super::Sx;
use super::{
    Corners, Edges, SxAlign, SxColor, SxDirection, SxDisplay, SxFlex, SxFont, SxJustify, SxLength,
    SxRadius, SxShadow, SxText, SxTextAlign,
};
use crate::{
    components::direction::DirectionalStyled,
    components::{extra_small_shadow, focus_ring_shadow},
    theme::Theme,
};

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
pub(super) struct Layer(StyleRefinement);

impl Styled for Layer {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.0
    }
}

pub(super) fn apply_edges<L>(
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
    pub(super) fn apply(&self, layer: Layer, theme: &Theme) -> Layer {
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

pub(super) fn resolve(decls: &[Decl], theme: &Theme) -> StyleRefinement {
    decls
        .iter()
        .fold(Layer(StyleRefinement::default()), |layer, decl| {
            decl.apply(layer, theme)
        })
        .0
}
