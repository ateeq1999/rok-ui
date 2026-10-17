//! Layering children: `Stack` and `Positioned`.

use gpui::{div, prelude::*, px, AnyElement, App, Pixels, StyleRefinement, Window};

use super::Alignment;
use crate::components::direction::DirectionalStyled;
use crate::{
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

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
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let online_dot = div();
/// let avatar = Stack::new()
///     .child(Avatar::new("LA"))
///     .positioned(Positioned::new().bottom(px(0.)).end(px(0.)).child(online_dot));
/// # }
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
            .when_some(
                self.end,
                crate::components::direction::DirectionalStyled::inset_end,
            )
            .when_some(self.width, gpui::Styled::w)
            .when_some(self.height, gpui::Styled::h)
            .children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}
