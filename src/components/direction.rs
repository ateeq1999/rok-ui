//! Direction: left-to-right or right-to-left layout (shadcn/ui's `<DirectionProvider>`).
//!
//! In RTL, layouts mirror the way CSS `dir="rtl"` mirrors them:
//! - horizontal flex rows flow right to left (components and `styles!` rows);
//! - logical spacing and positions (`ps`, `pe`, `start`, `end`, `padding_start`
//!   in `styles!`) land on the reading side;
//! - text aligns to the right, and directional icons (chevrons, arrows) flip;
//! - floating surfaces align to the trigger's right edge, sheets swap sides,
//!   sliders and progress bars fill from the right.
//!
//! GPUI shapes each line of text left to right, so this does not reorder
//! characters inside mixed-direction text.
//!
//! ```ignore
//! set_text_direction(TextDirection::from_locale("ar-EG"), cx);   // whole app
//!
//! Direction::new(TextDirection::Rtl).child(Settings::new())      // a subtree
//! Direction::build(TextDirection::Rtl, || view! { div(sx = ROW.base) { .. } })
//! ```

use std::cell::Cell;

use gpui::{
    div, prelude::*, px, AnyElement, App, DefiniteLength, GlobalElementId, InspectorElementId,
    LayoutId, Length, Pixels, Window,
};

use crate::icon::IconName;

/// Reading direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDirection {
    /// Left to right (English, French).
    #[default]
    Ltr,
    /// Right to left (Arabic, Hebrew, Persian).
    Rtl,
}

impl TextDirection {
    /// Whether this is right to left.
    #[must_use]
    pub fn is_rtl(self) -> bool {
        self == TextDirection::Rtl
    }

    /// The direction of a BCP 47 locale or language code: `"ar"`, `"he-IL"`, `"fa_IR"`.
    #[must_use]
    pub fn from_locale(locale: &str) -> Self {
        const RTL_LANGUAGES: &[&str] = &[
            "ar", "arc", "ckb", "dv", "fa", "ha", "he", "iw", "khw", "ks", "ku", "ps", "sd", "ug",
            "ur", "yi",
        ];
        let language = locale
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if RTL_LANGUAGES.contains(&language.as_str()) {
            TextDirection::Rtl
        } else {
            TextDirection::Ltr
        }
    }
}

thread_local! {
    static CURRENT: Cell<TextDirection> = const { Cell::new(TextDirection::Ltr) };
}

/// The direction in effect right now (while rendering: the enclosing
/// [`Direction`], else the app direction).
pub fn current_direction() -> TextDirection {
    CURRENT.with(Cell::get)
}

/// `current_direction().is_rtl()`.
#[must_use]
pub fn is_rtl() -> bool {
    current_direction().is_rtl()
}

/// Run `body` with `direction` in effect, then restore the previous one.
pub fn with_direction<R>(direction: TextDirection, body: impl FnOnce() -> R) -> R {
    let previous = CURRENT.with(|current| current.replace(direction));
    let result = body();
    CURRENT.with(|current| current.set(previous));
    result
}

/// Set the direction for the whole app (outside any [`Direction`] element).
pub fn set_text_direction(direction: TextDirection, cx: &mut App) {
    CURRENT.with(|current| current.set(direction));
    cx.refresh_windows();
}

/// Read the direction while rendering: `cx.direction().is_rtl()`.
pub trait ActiveDirection {
    /// The text direction in effect.
    fn direction(&self) -> TextDirection;
}

impl ActiveDirection for App {
    fn direction(&self) -> TextDirection {
        current_direction()
    }
}

impl IconName {
    /// The icon pointing the other way: left and right chevrons and arrows swap.
    #[must_use]
    pub fn mirrored(self) -> IconName {
        match self {
            IconName::ChevronLeft => IconName::ChevronRight,
            IconName::ChevronRight => IconName::ChevronLeft,
            IconName::ChevronsLeft => IconName::ChevronsRight,
            IconName::ChevronsRight => IconName::ChevronsLeft,
            IconName::ArrowLeft => IconName::ArrowRight,
            IconName::ArrowRight => IconName::ArrowLeft,
            other => other,
        }
    }

    /// Mirrored in RTL, unchanged in LTR. Use it for "forward" / "back" icons.
    #[must_use]
    pub fn for_direction(self) -> IconName {
        if is_rtl() {
            self.mirrored()
        } else {
            self
        }
    }
}

/// Direction-aware styling for GPUI elements and rok-ui components. These read
/// the direction when called, so call them while rendering.
pub trait DirectionalStyled: Styled + Sized {
    /// `flex()` with the row flowing in the reading direction (right to left
    /// in RTL). A later `flex_col()` still makes it a column.
    #[must_use]
    fn flex_dir(self) -> Self {
        let element = self.flex();
        if is_rtl() {
            element.flex_row_reverse()
        } else {
            element
        }
    }

    /// A row that always flows left to right (codes, numbers, charts).
    #[must_use]
    fn flex_ltr(self) -> Self {
        self.flex().flex_row()
    }

    /// Padding on the starting side (left in LTR, right in RTL).
    #[must_use]
    fn ps(self, length: impl Into<DefiniteLength> + Clone) -> Self {
        if is_rtl() {
            self.pr(length)
        } else {
            self.pl(length)
        }
    }

    /// Padding on the ending side.
    #[must_use]
    fn pe(self, length: impl Into<DefiniteLength> + Clone) -> Self {
        if is_rtl() {
            self.pl(length)
        } else {
            self.pr(length)
        }
    }

    /// Margin on the starting side.
    #[must_use]
    fn ms(self, length: impl Into<Length> + Clone) -> Self {
        if is_rtl() {
            self.mr(length)
        } else {
            self.ml(length)
        }
    }

    /// Margin on the ending side.
    #[must_use]
    fn me(self, length: impl Into<Length> + Clone) -> Self {
        if is_rtl() {
            self.ml(length)
        } else {
            self.mr(length)
        }
    }

    /// Offset from the starting edge (absolute / relative positioning).
    #[must_use]
    fn inset_start(self, length: impl Into<Length> + Clone) -> Self {
        if is_rtl() {
            self.right(length)
        } else {
            self.left(length)
        }
    }

    /// Offset from the ending edge.
    #[must_use]
    fn inset_end(self, length: impl Into<Length> + Clone) -> Self {
        if is_rtl() {
            self.left(length)
        } else {
            self.right(length)
        }
    }

    /// 1px border on the starting side.
    #[must_use]
    fn border_s_1(self) -> Self {
        if is_rtl() {
            self.border_r_1()
        } else {
            self.border_l_1()
        }
    }

    /// 1px border on the ending side.
    #[must_use]
    fn border_e_1(self) -> Self {
        if is_rtl() {
            self.border_l_1()
        } else {
            self.border_r_1()
        }
    }

    /// Round the starting corners.
    #[must_use]
    fn rounded_s(self, radius: impl Into<gpui::AbsoluteLength> + Clone) -> Self {
        if is_rtl() {
            self.rounded_r(radius)
        } else {
            self.rounded_l(radius)
        }
    }

    /// Round the ending corners.
    #[must_use]
    fn rounded_e(self, radius: impl Into<gpui::AbsoluteLength> + Clone) -> Self {
        if is_rtl() {
            self.rounded_l(radius)
        } else {
            self.rounded_r(radius)
        }
    }

    /// Square off the starting corners.
    #[must_use]
    fn rounded_s_none(self) -> Self {
        self.rounded_s(px(0.))
    }

    /// Square off the ending corners.
    #[must_use]
    fn rounded_e_none(self) -> Self {
        self.rounded_e(px(0.))
    }

    /// Align text to the starting side.
    #[must_use]
    fn text_start(self) -> Self {
        if is_rtl() {
            self.text_right()
        } else {
            self.text_left()
        }
    }

    /// Align text to the ending side.
    #[must_use]
    fn text_end(self) -> Self {
        if is_rtl() {
            self.text_left()
        } else {
            self.text_right()
        }
    }
}

impl<E: Styled + Sized> DirectionalStyled for E {}

/// Applies a direction to everything rendered inside it.
///
/// Components and views inside render in this direction. Plain elements you
/// build in the same expression are built before the `Direction` is laid out;
/// build them with [`Direction::build`] so they see it too.
pub struct Direction {
    value: TextDirection,
    children: Vec<AnyElement>,
    content: Option<AnyElement>,
}

impl Direction {
    /// Render children in `direction`.
    #[must_use]
    pub fn new(direction: TextDirection) -> Self {
        Self {
            value: direction,
            children: Vec::new(),
            content: None,
        }
    }

    /// Build `content` with `direction` in effect (so `.sx()`, `flex_dir()` and
    /// friends mirror), and render it inside a `Direction`.
    pub fn build<E: IntoElement>(direction: TextDirection, content: impl FnOnce() -> E) -> Self {
        let element = with_direction(direction, || content().into_any_element());
        Self::new(direction).child(element)
    }
}

impl ParentElement for Direction {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl IntoElement for Direction {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Direction {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let is_rtl = self.value.is_rtl();
        let mut content = div()
            .flex()
            .flex_col()
            .when(is_rtl, |content| content.text_right().items_end())
            .children(std::mem::take(&mut self.children))
            .into_any_element();
        // Components render while their layout is requested, so the direction
        // is installed for this step (and for prepaint and paint, where lists
        // and portals render too).
        let layout_id = with_direction(self.value, || content.request_layout(window, cx));
        self.content = Some(content);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(content) = self.content.as_mut() {
            with_direction(self.value, || content.prepaint(window, cx));
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(content) = self.content.as_mut() {
            with_direction(self.value, || content.paint(window, cx));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_map_to_directions() {
        assert_eq!(TextDirection::from_locale("ar"), TextDirection::Rtl);
        assert_eq!(TextDirection::from_locale("he-IL"), TextDirection::Rtl);
        assert_eq!(TextDirection::from_locale("fa_IR"), TextDirection::Rtl);
        assert_eq!(TextDirection::from_locale("ur-PK"), TextDirection::Rtl);
        assert_eq!(TextDirection::from_locale("en-US"), TextDirection::Ltr);
        assert_eq!(TextDirection::from_locale("fr"), TextDirection::Ltr);
        assert_eq!(TextDirection::from_locale(""), TextDirection::Ltr);
    }

    #[test]
    fn with_direction_restores_the_previous_direction() {
        assert_eq!(current_direction(), TextDirection::Ltr);
        with_direction(TextDirection::Rtl, || {
            assert!(is_rtl());
            assert_eq!(
                IconName::ChevronRight.for_direction(),
                IconName::ChevronLeft
            );
            with_direction(TextDirection::Ltr, || assert!(!is_rtl()));
            assert!(is_rtl());
        });
        assert!(!is_rtl());
        assert_eq!(
            IconName::ChevronRight.for_direction(),
            IconName::ChevronRight
        );
    }
}
