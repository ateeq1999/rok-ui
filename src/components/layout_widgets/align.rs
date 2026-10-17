//! Placing and sizing one box: `Aligned`, `Center`, `Padding`, `SizedBox`.

use gpui::{
    div, prelude::*, px, AlignItems, AnyElement, App, Div, JustifyContent, Pixels, StyleRefinement,
    Window,
};

use super::fill_box;
use crate::components::direction::DirectionalStyled;
use crate::{
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

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
    pub(super) fn apply(self, element: Div) -> Div {
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
    pub(super) fn sized(width: Option<Pixels>, height: Option<Pixels>, expand: bool) -> Self {
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
