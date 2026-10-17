//! `Row`, `Column` and what sizes their children: `Expanded`, `Flexible`, `Spacer`.

use gpui::{
    div, prelude::*, px, relative, AlignItems, AnyElement, App, Div, JustifyContent, Pixels,
    StyleRefinement, Window,
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
    pub(super) fn justify_content(self) -> JustifyContent {
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
    pub(super) fn align_items(self, vertical: bool) -> AlignItems {
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
    pub(super) fn new() -> Self {
        Self {
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Center,
            main_axis_size: MainAxisSize::Max,
            spacing: px(0.),
            children: Vec::new(),
        }
    }

    pub(super) fn render(self, vertical: bool) -> Div {
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
pub(super) fn fill_box(children: Vec<AnyElement>) -> Div {
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
