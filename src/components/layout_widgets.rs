//! Flutter-style layout widgets: `Row`, `Column`, `Expanded`, `Stack`, `Padding` and
//! friends, as typed wrappers over flexbox and grid.
//!
//! ```ignore
//! Column::new()
//!     .cross_axis_alignment(CrossAxisAlignment::Stretch)
//!     .spacing(px(12.))
//!     .child(AppTitle::new())
//!     .child(
//!         Row::new()
//!             .main_axis_alignment(MainAxisAlignment::SpaceBetween)
//!             .child(Expanded::new().child(search_field))
//!             .child(SizedBox::width(px(8.)))
//!             .child(Button::new("filter").label("Filter")),
//!     )
//!     .child(Expanded::new().child(results))
//! ```
//!
//! "Start" and "end" follow the reading direction like Flutter's
//! `AlignmentDirectional`: a `Row` flows right to left in RTL, and
//! `CrossAxisAlignment::Start` in a `Column` hugs the right edge.
//!
//! Sizing follows CSS flexbox, which differs from Flutter's constraints in one
//! place: a `Column` inside a `Column` fills the parent's height by default
//! (`MainAxisSize::Max`). Give it `MainAxisSize::Min` or wrap it in `Expanded`.

use std::{cell::Cell, rc::Rc};

use gpui::{
    canvas, div, prelude::*, px, relative, AlignItems, AnyElement, App, Div, ElementId,
    JustifyContent, Pixels, Size, StyleRefinement, Window,
};

use crate::components::direction::{is_rtl, DirectionalStyled};
use crate::{
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// How children are placed along a `Row`'s or `Column`'s main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MainAxisAlignment {
    /// At the start: the reading-direction start for a `Row`, the top for a `Column`.
    #[default]
    Start,
    /// At the end of the main axis.
    End,
    /// Centered on the main axis.
    Center,
    /// Free space between children, none before the first or after the last.
    SpaceBetween,
    /// Equal space around each child (half a gap at the ends).
    SpaceAround,
    /// Equal space between children and at both ends.
    SpaceEvenly,
}

impl MainAxisAlignment {
    fn justify_content(self) -> JustifyContent {
        // Flex-start and flex-end follow `row-reverse`, which `Row` uses in RTL.
        match self {
            Self::Start => JustifyContent::FlexStart,
            Self::End => JustifyContent::FlexEnd,
            Self::Center => JustifyContent::Center,
            Self::SpaceBetween => JustifyContent::SpaceBetween,
            Self::SpaceAround => JustifyContent::SpaceAround,
            Self::SpaceEvenly => JustifyContent::SpaceEvenly,
        }
    }
}

/// How children are placed across a `Row`'s or `Column`'s main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CrossAxisAlignment {
    /// The top of a `Row`, the reading-direction start of a `Column`.
    Start,
    /// The bottom of a `Row`, the reading-direction end of a `Column`.
    End,
    /// Centered on the cross axis.
    #[default]
    Center,
    /// Children fill the cross axis.
    Stretch,
    /// Text baselines line up (rows only).
    Baseline,
}

impl CrossAxisAlignment {
    fn align_items(self, vertical: bool) -> AlignItems {
        // A column's cross axis is horizontal, where "start" depends on direction.
        let mirrored = vertical && is_rtl();
        match self {
            Self::Start if mirrored => AlignItems::FlexEnd,
            Self::End if mirrored => AlignItems::FlexStart,
            Self::Start => AlignItems::FlexStart,
            Self::End => AlignItems::FlexEnd,
            Self::Center => AlignItems::Center,
            Self::Stretch => AlignItems::Stretch,
            Self::Baseline => AlignItems::Baseline,
        }
    }
}

/// How much main-axis space a `Row` or `Column` takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MainAxisSize {
    /// As little as its children need.
    Min,
    /// All of its parent's width (`Row`) or height (`Column`).
    #[default]
    Max,
}

/// Settings shared by [`Row`] and [`Column`].
struct FlexProps {
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_size: MainAxisSize,
    spacing: Pixels,
    children: Vec<AnyElement>,
}

impl FlexProps {
    fn new() -> Self {
        Self {
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Center,
            main_axis_size: MainAxisSize::Max,
            spacing: px(0.),
            children: Vec::new(),
        }
    }

    fn render(self, vertical: bool) -> Div {
        let mut flex = div().flex_dir().gap(self.spacing);
        if vertical {
            flex = flex.flex_col();
        }
        flex.style().justify_content = Some(self.main_axis_alignment.justify_content());
        flex.style().align_items = Some(self.cross_axis_alignment.align_items(vertical));
        match (self.main_axis_size, vertical) {
            (MainAxisSize::Max, false) => flex = flex.w_full(),
            (MainAxisSize::Max, true) => flex = flex.h_full(),
            (MainAxisSize::Min, _) => {}
        }
        flex.children(self.children)
    }
}

macro_rules! flex_widget {
    ($(#[$doc:meta])* $name:ident, vertical: $vertical:expr) => {
        $(#[$doc])*
        #[derive(IntoElement)]
        pub struct $name {
            props: FlexProps,
            sx: Sx,
            style_overrides: StyleRefinement,
        }

        crate::implement_style_overrides!($name);

        impl $name {
            #[must_use]
            /// An empty widget; add children with `.child(..)`.
            pub fn new() -> Self {
                Self {
                    props: FlexProps::new(),
                    sx: Sx::new(),
                    style_overrides: StyleRefinement::default(),
                }
            }

            /// Placement along the main axis. Default: `Start`.
            #[must_use]
            pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
                self.props.main_axis_alignment = alignment;
                self
            }

            /// Placement across the main axis. Default: `Center`.
            #[must_use]
            pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
                self.props.cross_axis_alignment = alignment;
                self
            }

            /// Default: `Max`, filling the parent along the main axis.
            #[must_use]
            pub fn main_axis_size(mut self, size: MainAxisSize) -> Self {
                self.props.main_axis_size = size;
                self
            }

            /// Space between consecutive children (Flutter's `spacing`).
            #[must_use]
            pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
                self.props.spacing = spacing.into();
                self
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.props.children.extend(elements);
            }
        }

        impl RenderOnce for $name {
            fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
                self.props
                    .render($vertical)
                    .sx(&self.sx)
                    .apply_style_overrides(&self.style_overrides)
            }
        }
    };
}

flex_widget!(
    /// Children side by side in the reading direction (right to left in RTL).
    Row,
    vertical: false
);

flex_widget!(
    /// Children stacked top to bottom.
    Column,
    vertical: true
);

/// A container for one or more children that a widget lays out as a single box:
/// each child gets an equal share of the box's height and its full width.
fn fill_box(children: Vec<AnyElement>) -> Div {
    let rows = children.len().max(1) as u16;
    div().grid().grid_cols(1).grid_rows(rows).children(children)
}

/// Makes its children fill the free space along a `Row`'s or `Column`'s main axis,
/// shared with other `Expanded` children by their `flex` factors.
#[derive(IntoElement)]
pub struct Expanded {
    flex: f32,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Expanded);

impl Expanded {
    /// Fill the remaining main-axis space of a `Row` or `Column`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flex: 1.,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// This child's share of the free space relative to its siblings. Default: 1.
    #[must_use]
    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }
}

impl Default for Expanded {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Expanded {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Expanded {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut expanded = fill_box(self.children).min_w(px(0.)).min_h(px(0.));
        let style = expanded.style();
        style.flex_grow = Some(self.flex);
        style.flex_shrink = Some(1.);
        style.flex_basis = Some(relative(0.).into());
        expanded
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Like [`Expanded`], but its children keep their natural size within the space
/// it may grow into (Flutter's `FlexFit.loose`).
#[derive(IntoElement)]
pub struct Flexible {
    flex: f32,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Flexible);

impl Flexible {
    /// Take a share of the remaining main-axis space, shrinking to the child if smaller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flex: 1.,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// This child's share of the free space relative to its siblings. Default: 1.
    #[must_use]
    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }
}

impl Default for Flexible {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Flexible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Flexible {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut flexible = div()
            .flex_dir()
            .flex_col()
            .items_start()
            .min_w(px(0.))
            .min_h(px(0.))
            .children(self.children);
        let style = flexible.style();
        style.flex_grow = Some(self.flex);
        style.flex_shrink = Some(1.);
        flexible
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Empty space that takes a share of the free space in a `Row` or `Column`.
#[derive(IntoElement)]
pub struct Spacer {
    flex: f32,
}

impl Spacer {
    /// Empty space that fills the remaining main axis.
    #[must_use]
    pub fn new() -> Self {
        Self { flex: 1. }
    }

    /// This spacer's share of the free space relative to its siblings. Default: 1.
    #[must_use]
    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Spacer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut spacer = div();
        let style = spacer.style();
        style.flex_grow = Some(self.flex);
        style.flex_shrink = Some(1.);
        style.flex_basis = Some(relative(0.).into());
        spacer
    }
}

/// A point in a box, in reading-direction terms (Flutter's `AlignmentDirectional`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// Top, reading-direction start.
    #[default]
    TopStart,
    /// Top, centered horizontally.
    TopCenter,
    /// Top, reading-direction end.
    TopEnd,
    /// Centered vertically, at the start.
    CenterStart,
    /// Centered on both axes.
    Center,
    /// Centered vertically, at the end.
    CenterEnd,
    /// Bottom, reading-direction start.
    BottomStart,
    /// Bottom, centered horizontally.
    BottomCenter,
    /// Bottom, reading-direction end.
    BottomEnd,
}

impl Alignment {
    /// Lay out `element` (a flex row in the reading direction) so its children sit
    /// at this point.
    fn apply(self, element: Div) -> Div {
        use Alignment::{
            BottomCenter, BottomEnd, BottomStart, Center, CenterEnd, CenterStart, TopCenter,
            TopEnd, TopStart,
        };
        let horizontal = match self {
            TopStart | CenterStart | BottomStart => JustifyContent::FlexStart,
            TopCenter | Center | BottomCenter => JustifyContent::Center,
            TopEnd | CenterEnd | BottomEnd => JustifyContent::FlexEnd,
        };
        let vertical = match self {
            TopStart | TopCenter | TopEnd => AlignItems::FlexStart,
            CenterStart | Center | CenterEnd => AlignItems::Center,
            BottomStart | BottomCenter | BottomEnd => AlignItems::FlexEnd,
        };
        let mut element = element.flex_dir();
        element.style().justify_content = Some(horizontal);
        element.style().align_items = Some(vertical);
        element
    }
}

/// Fills its parent and places its children at an [`Alignment`].
#[derive(IntoElement)]
pub struct Aligned {
    alignment: Alignment,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Aligned);

impl Aligned {
    /// Place the child at `alignment` inside the available space.
    #[must_use]
    pub fn new(alignment: Alignment) -> Self {
        Self {
            alignment,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl ParentElement for Aligned {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Aligned {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.alignment
            .apply(div().size_full())
            .children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Fills its parent and centers its children.
#[derive(IntoElement)]
pub struct Center {
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Center);

impl Center {
    /// Center the child on both axes.
    #[must_use]
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl Default for Center {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Center {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Center {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        Alignment::Center
            .apply(div().size_full())
            .children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Space on each side of a box, with `start` and `end` in the reading direction
/// (Flutter's `EdgeInsetsDirectional`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    /// Space above.
    pub top: Pixels,
    /// Space at the reading-direction end.
    pub end: Pixels,
    /// Space below.
    pub bottom: Pixels,
    /// Space at the reading-direction start.
    pub start: Pixels,
}

impl EdgeInsets {
    /// No space on any side.
    #[must_use]
    pub fn zero() -> Self {
        Self::default()
    }

    /// The same space on every side.
    pub fn all(value: impl Into<Pixels>) -> Self {
        let value = value.into();
        Self {
            top: value,
            end: value,
            bottom: value,
            start: value,
        }
    }

    /// `horizontal` on the start and end, `vertical` on the top and bottom.
    pub fn symmetric(horizontal: impl Into<Pixels>, vertical: impl Into<Pixels>) -> Self {
        let (horizontal, vertical) = (horizontal.into(), vertical.into());
        Self {
            top: vertical,
            end: horizontal,
            bottom: vertical,
            start: horizontal,
        }
    }

    /// Set the space above.
    #[must_use]
    pub fn top(mut self, value: impl Into<Pixels>) -> Self {
        self.top = value.into();
        self
    }

    /// Set the space at the reading-direction end.
    #[must_use]
    pub fn end(mut self, value: impl Into<Pixels>) -> Self {
        self.end = value.into();
        self
    }

    /// Set the space below.
    #[must_use]
    pub fn bottom(mut self, value: impl Into<Pixels>) -> Self {
        self.bottom = value.into();
        self
    }

    /// Set the space at the reading-direction start.
    #[must_use]
    pub fn start(mut self, value: impl Into<Pixels>) -> Self {
        self.start = value.into();
        self
    }
}

/// Insets its children by [`EdgeInsets`]. Children stack top to bottom and fill
/// the width.
#[derive(IntoElement)]
pub struct Padding {
    insets: EdgeInsets,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Padding);

impl Padding {
    /// Pad the child by `insets`.
    #[must_use]
    pub fn new(insets: EdgeInsets) -> Self {
        Self {
            insets,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The same padding on every side.
    pub fn all(value: impl Into<Pixels>) -> Self {
        Self::new(EdgeInsets::all(value))
    }

    /// `horizontal` padding on the start and end, `vertical` on the top and bottom.
    pub fn symmetric(horizontal: impl Into<Pixels>, vertical: impl Into<Pixels>) -> Self {
        Self::new(EdgeInsets::symmetric(horizontal, vertical))
    }
}

impl ParentElement for Padding {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Padding {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let insets = self.insets;
        div()
            .flex()
            .flex_col()
            .pt(insets.top)
            .pb(insets.bottom)
            .ps(insets.start)
            .pe(insets.end)
            .children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// A box of a fixed size: a gap in a `Row` or `Column`, or a fixed frame its
/// children fill.
#[derive(IntoElement)]
pub struct SizedBox {
    width: Option<Pixels>,
    height: Option<Pixels>,
    expand: bool,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(SizedBox);

impl SizedBox {
    fn sized(width: Option<Pixels>, height: Option<Pixels>, expand: bool) -> Self {
        Self {
            width,
            height,
            expand,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A box of exactly `width` by `height`.
    pub fn new(width: impl Into<Pixels>, height: impl Into<Pixels>) -> Self {
        Self::sized(Some(width.into()), Some(height.into()), false)
    }

    /// Only a width: a horizontal gap.
    pub fn width(width: impl Into<Pixels>) -> Self {
        Self::sized(Some(width.into()), None, false)
    }

    /// Only a height: a vertical gap.
    pub fn height(height: impl Into<Pixels>) -> Self {
        Self::sized(None, Some(height.into()), false)
    }

    /// A square box with sides of `side`.
    pub fn square(side: impl Into<Pixels>) -> Self {
        let side = side.into();
        Self::sized(Some(side), Some(side), false)
    }

    /// As large as the parent allows.
    #[must_use]
    pub fn expand() -> Self {
        Self::sized(None, None, true)
    }

    /// Zero by zero.
    #[must_use]
    pub fn shrink() -> Self {
        Self::sized(Some(px(0.)), Some(px(0.)), false)
    }
}

impl ParentElement for SizedBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SizedBox {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        fill_box(self.children)
            .flex_none()
            .when(self.expand, gpui::Styled::size_full)
            .when_some(self.width, gpui::Styled::w)
            .when_some(self.height, gpui::Styled::h)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// How a [`Stack`] sizes itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StackFit {
    /// As large as its largest child.
    #[default]
    Loose,
    /// As large as its parent.
    Expand,
}

/// Children layered on top of each other, later ones in front. Children added
/// with `.child(..)` are placed at the stack's [`Alignment`] and size it;
/// [`Positioned`] children added with `.positioned(..)` are pinned to its edges.
///
/// ```ignore
/// Stack::new()
///     .child(Avatar::new("LA"))
///     .positioned(Positioned::new().bottom(px(0.)).end(px(0.)).child(online_dot))
/// ```
#[derive(IntoElement)]
pub struct Stack {
    alignment: Alignment,
    fit: StackFit,
    children: Vec<AnyElement>,
    positioned: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Stack);

impl Stack {
    /// Children layered on top of each other.
    #[must_use]
    pub fn new() -> Self {
        Self {
            alignment: Alignment::TopStart,
            fit: StackFit::Loose,
            children: Vec::new(),
            positioned: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Where non-positioned children sit. Default: `TopStart`.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// How children without a position are sized.
    #[must_use]
    pub fn fit(mut self, fit: StackFit) -> Self {
        self.fit = fit;
        self
    }

    /// A child pinned to the stack's edges.
    #[must_use]
    pub fn positioned(mut self, positioned: Positioned) -> Self {
        self.positioned.push(positioned.into_any_element());
        self
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Stack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Stack {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let alignment = self.alignment;
        // Every child shares the single grid cell, so the cell (and the stack)
        // grows to the largest one; each child is then aligned inside it.
        let layers = self.children.into_iter().map(move |child| {
            alignment
                .apply(div().col_start(1).row_start(1))
                .child(child)
        });
        div()
            .relative()
            .grid()
            .grid_cols(1)
            .grid_rows(1)
            .when(self.fit == StackFit::Expand, gpui::Styled::size_full)
            .children(layers)
            .children(self.positioned)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// A child pinned to the edges of its parent, usually a [`Stack`]. `start` and
/// `end` follow the reading direction. It must be a direct child of the element
/// it is positioned in.
#[derive(IntoElement)]
pub struct Positioned {
    top: Option<Pixels>,
    end: Option<Pixels>,
    bottom: Option<Pixels>,
    start: Option<Pixels>,
    width: Option<Pixels>,
    height: Option<Pixels>,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Positioned);

impl Positioned {
    /// A child of a `Stack` placed by its edges.
    #[must_use]
    pub fn new() -> Self {
        Self {
            top: None,
            end: None,
            bottom: None,
            start: None,
            width: None,
            height: None,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Pinned to all four edges, covering the parent.
    #[must_use]
    pub fn fill() -> Self {
        Self::new()
            .top(px(0.))
            .end(px(0.))
            .bottom(px(0.))
            .start(px(0.))
    }

    /// Distance from the top of the stack.
    #[must_use]
    pub fn top(mut self, top: impl Into<Pixels>) -> Self {
        self.top = Some(top.into());
        self
    }

    /// Distance from the reading-direction end.
    #[must_use]
    pub fn end(mut self, end: impl Into<Pixels>) -> Self {
        self.end = Some(end.into());
        self
    }

    /// Distance from the bottom of the stack.
    #[must_use]
    pub fn bottom(mut self, bottom: impl Into<Pixels>) -> Self {
        self.bottom = Some(bottom.into());
        self
    }

    /// Distance from the reading-direction start.
    #[must_use]
    pub fn start(mut self, start: impl Into<Pixels>) -> Self {
        self.start = Some(start.into());
        self
    }

    /// A fixed width.
    #[must_use]
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// A fixed height.
    #[must_use]
    pub fn height(mut self, height: impl Into<Pixels>) -> Self {
        self.height = Some(height.into());
        self
    }
}

impl Default for Positioned {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Positioned {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Positioned {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .absolute()
            .flex()
            .flex_col()
            .when_some(self.top, gpui::Styled::top)
            .when_some(self.bottom, gpui::Styled::bottom)
            .when_some(self.start, |positioned, start| {
                positioned.inset_start(start)
            })
            .when_some(self.end, super::direction::DirectionalStyled::inset_end)
            .when_some(self.width, gpui::Styled::w)
            .when_some(self.height, gpui::Styled::h)
            .children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Children in runs that wrap onto new lines when a run is full, like words in a
/// paragraph (chips, tags, buttons).
#[derive(IntoElement)]
pub struct Wrap {
    alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    spacing: Pixels,
    run_spacing: Pixels,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Wrap);

impl Wrap {
    /// Children that wrap onto new lines.
    #[must_use]
    pub fn new() -> Self {
        Self {
            alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Start,
            spacing: px(0.),
            run_spacing: px(0.),
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Placement of children within a run. Default: `Start`.
    #[must_use]
    pub fn alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Placement of children of different heights within a run. Default: `Start`.
    #[must_use]
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }

    /// Space between children in a run.
    #[must_use]
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into();
        self
    }

    /// Space between runs.
    #[must_use]
    pub fn run_spacing(mut self, run_spacing: impl Into<Pixels>) -> Self {
        self.run_spacing = run_spacing.into();
        self
    }
}

impl Default for Wrap {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Wrap {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Wrap {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut wrap = div()
            .flex_dir()
            .flex_wrap()
            .w_full()
            .gap_x(self.spacing)
            .gap_y(self.run_spacing);
        wrap.style().justify_content = Some(self.alignment.justify_content());
        wrap.style().align_items = Some(self.cross_axis_alignment.align_items(false));
        wrap.children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// How a [`GridView`] picks its column count.
#[derive(Clone, Copy, Debug, PartialEq)]
enum GridColumns {
    Count(usize),
    MaxExtent(Pixels),
}

/// Children in a grid of equal-width columns, filled in reading order (right to
/// left in RTL). Wrap it in a scroll container for long grids.
///
/// ```ignore
/// GridView::count(3).spacing(px(12.)).children(photos)
/// GridView::extent("products", px(220.)).spacing(px(16.)).children(cards)
/// ```
#[derive(IntoElement)]
pub struct GridView {
    id: Option<ElementId>,
    columns: GridColumns,
    main_axis_spacing: Pixels,
    cross_axis_spacing: Pixels,
    row_height: Option<Pixels>,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(GridView);

impl GridView {
    fn with_columns(id: Option<ElementId>, columns: GridColumns) -> Self {
        Self {
            id,
            columns,
            main_axis_spacing: px(0.),
            cross_axis_spacing: px(0.),
            row_height: None,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A fixed number of columns (Flutter's `GridView.count`).
    #[must_use]
    pub fn count(columns: usize) -> Self {
        Self::with_columns(None, GridColumns::Count(columns.max(1)))
    }

    /// As many columns as fit with each at most `max_extent` wide (Flutter's
    /// `GridView.extent`). The column count follows the grid's own width.
    pub fn extent(id: impl Into<ElementId>, max_extent: impl Into<Pixels>) -> Self {
        Self::with_columns(Some(id.into()), GridColumns::MaxExtent(max_extent.into()))
    }

    /// Space between rows and between columns.
    #[must_use]
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        let spacing = spacing.into();
        self.main_axis_spacing = spacing;
        self.cross_axis_spacing = spacing;
        self
    }

    /// Space between rows.
    #[must_use]
    pub fn main_axis_spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.main_axis_spacing = spacing.into();
        self
    }

    /// Space between columns.
    #[must_use]
    pub fn cross_axis_spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.cross_axis_spacing = spacing.into();
        self
    }

    /// A fixed height for every row. Rows otherwise fit their tallest child.
    #[must_use]
    pub fn row_height(mut self, height: impl Into<Pixels>) -> Self {
        self.row_height = Some(height.into());
        self
    }
}

impl ParentElement for GridView {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// The number of columns at most `max_extent` wide that fill `width`.
fn columns_for_extent(width: Pixels, max_extent: Pixels, spacing: Pixels) -> usize {
    let per_column = f32::from(max_extent + spacing).max(1.);
    (f32::from(width + spacing) / per_column).ceil().max(1.) as usize
}

fn render_grid(
    columns: usize,
    children: Vec<AnyElement>,
    main_axis_spacing: Pixels,
    cross_axis_spacing: Pixels,
    row_height: Option<Pixels>,
) -> Div {
    let rtl = is_rtl();
    let cells = children.into_iter().enumerate().map(move |(index, child)| {
        let (row, column) = (index / columns, index % columns);
        let column = if rtl { columns - 1 - column } else { column };
        div()
            .flex()
            .flex_col()
            .min_w(px(0.))
            .col_start(column as i16 + 1)
            .row_start(row as i16 + 1)
            .when_some(row_height, gpui::Styled::h)
            .child(child)
    });
    div()
        .grid()
        .grid_cols(columns as u16)
        .w_full()
        .gap_x(cross_axis_spacing)
        .gap_y(main_axis_spacing)
        .children(cells)
}

impl RenderOnce for GridView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            columns,
            main_axis_spacing,
            cross_axis_spacing,
            row_height,
            children,
            sx,
            style_overrides,
        } = self;
        match (columns, id) {
            (GridColumns::MaxExtent(max_extent), Some(id)) => {
                LayoutBuilder::new(id, move |constraints, _, _| {
                    let columns =
                        columns_for_extent(constraints.max_width, max_extent, cross_axis_spacing);
                    render_grid(
                        columns,
                        children,
                        main_axis_spacing,
                        cross_axis_spacing,
                        row_height,
                    )
                    .into_any_element()
                })
                .sx(sx)
                .apply_style_overrides(&style_overrides)
                .into_any_element()
            }
            (columns, _) => {
                let columns = match columns {
                    GridColumns::Count(count) => count,
                    GridColumns::MaxExtent(_) => 1,
                };
                render_grid(
                    columns,
                    children,
                    main_axis_spacing,
                    cross_axis_spacing,
                    row_height,
                )
                .sx(&sx)
                .apply_style_overrides(&style_overrides)
                .into_any_element()
            }
        }
    }
}

/// The space a [`LayoutBuilder`] has, measured from its last layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxConstraints {
    /// The available width.
    pub max_width: Pixels,
    /// The available height.
    pub max_height: Pixels,
}

impl BoxConstraints {
    /// The Material size class for this width.
    #[must_use]
    pub fn size_class(&self) -> WindowSizeClass {
        WindowSizeClass::from_width(self.max_width)
    }
}

type LayoutBuilderFn = Box<dyn FnOnce(BoxConstraints, &mut Window, &mut App) -> AnyElement>;

/// Builds its content from the size it is given, for layouts that change with
/// the available width (Flutter's `LayoutBuilder`).
///
/// It fills its parent's width, measures itself after each layout and rebuilds
/// when the size changes, so content can depend on `max_width` freely. Content
/// that depends on `max_height` should not change the builder's own height; give
/// it a definite height (`.h_full()`, `.size_full()`) in that case. The first frame
/// uses the window's size.
///
/// ```ignore
/// LayoutBuilder::new("products", |constraints, _, _| {
///     if constraints.max_width > px(720.) { two_panes() } else { one_pane() }
/// })
/// ```
#[derive(IntoElement)]
pub struct LayoutBuilder {
    id: ElementId,
    builder: LayoutBuilderFn,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(LayoutBuilder);

impl LayoutBuilder {
    /// Build content from the space available: `builder` gets the [`BoxConstraints`] measured on the last layout.
    pub fn new(
        id: impl Into<ElementId>,
        builder: impl FnOnce(BoxConstraints, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            builder: Box::new(builder),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for LayoutBuilder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let measured = window
            .use_keyed_state(self.id.clone(), cx, |_, _| {
                Rc::new(Cell::new(None::<Size<Pixels>>))
            })
            .read(cx)
            .clone();
        let size = measured.get().unwrap_or_else(|| window.viewport_size());
        let constraints = BoxConstraints {
            max_width: size.width,
            max_height: size.height,
        };
        let content = (self.builder)(constraints, window, cx);
        let measure = canvas(
            move |bounds, window, cx| {
                let changed = measured.get().is_none_or(|last| {
                    (last.width - bounds.size.width).abs() > px(0.5)
                        || (last.height - bounds.size.height).abs() > px(0.5)
                });
                if changed {
                    measured.set(Some(bounds.size));
                    // Windows ignore refresh requests while they draw, so rebuild
                    // on the next turn of the event loop.
                    let handle = window.window_handle();
                    cx.defer(move |cx| {
                        handle.update(cx, |_, window, _| window.refresh()).ok();
                    });
                }
            },
            |_, (), _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .child(content)
            .child(measure)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Material 3 window size classes, by width. Compare them with `<` and `>=`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindowSizeClass {
    /// Under 600 px: phones, narrow windows.
    Compact,
    /// 600 to 839 px.
    Medium,
    /// 840 to 1199 px.
    Expanded,
    /// 1200 to 1599 px.
    Large,
    /// 1600 px and over.
    ExtraLarge,
}

impl WindowSizeClass {
    /// The Material size class for a window or pane `width` wide.
    #[must_use]
    pub fn from_width(width: Pixels) -> Self {
        let width = f32::from(width);
        if width < 600. {
            Self::Compact
        } else if width < 840. {
            Self::Medium
        } else if width < 1200. {
            Self::Expanded
        } else if width < 1600. {
            Self::Large
        } else {
            Self::ExtraLarge
        }
    }

    /// The class of the window's current width.
    #[must_use]
    pub fn of(window: &Window) -> Self {
        Self::from_width(window.viewport_size().width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_classes_follow_material_breakpoints() {
        assert_eq!(
            WindowSizeClass::from_width(px(599.)),
            WindowSizeClass::Compact
        );
        assert_eq!(
            WindowSizeClass::from_width(px(600.)),
            WindowSizeClass::Medium
        );
        assert_eq!(
            WindowSizeClass::from_width(px(840.)),
            WindowSizeClass::Expanded
        );
        assert_eq!(
            WindowSizeClass::from_width(px(1200.)),
            WindowSizeClass::Large
        );
        assert_eq!(
            WindowSizeClass::from_width(px(1600.)),
            WindowSizeClass::ExtraLarge
        );
        assert!(WindowSizeClass::Medium >= WindowSizeClass::Compact);
    }

    #[test]
    fn extent_grids_fit_as_many_columns_as_the_width_allows() {
        // 3 columns of 200 with 2 gaps of 10 = 620.
        assert_eq!(columns_for_extent(px(620.), px(200.), px(10.)), 3);
        // A little wider and the columns would exceed 200, so a fourth is added.
        assert_eq!(columns_for_extent(px(640.), px(200.), px(10.)), 4);
        assert_eq!(columns_for_extent(px(50.), px(200.), px(10.)), 1);
    }
}
