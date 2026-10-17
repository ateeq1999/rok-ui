//! Layout that depends on the space available: `LayoutBuilder` and `WindowSizeClass`.

use std::{cell::Cell, rc::Rc};

use gpui::{
    canvas, div, prelude::*, px, AnyElement, App, ElementId, Pixels, Size, StyleRefinement, Window,
};

use crate::{
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

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
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # fn two_panes() -> AnyElement { div().into_any_element() }
/// # fn one_pane() -> AnyElement { div().into_any_element() }
/// let products = LayoutBuilder::new("products", |constraints, _, _| {
///     if constraints.max_width > px(720.) { two_panes() } else { one_pane() }
/// });
/// # }
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
}
