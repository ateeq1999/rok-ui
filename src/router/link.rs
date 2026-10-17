//! `Link`: a clickable element that navigates.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, ElementId, SharedString, StyleRefinement, Window};

use super::{
    is_active, navigate, preload, preload_once_visible, replace, with_window, Location, Route,
    Search,
};
use crate::{
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

styles! {
    LINK = {
        root: {
            cursor: pointer,
            color: primary,
            border: 1,
            border_color: transparent,
            radius: sm,
            hover: { underline: true },
            focus: { border_color: ring },
        },
        active: { font: semibold },
    }
}

/// A clickable element that navigates to a path. Styled as a text link by
/// default; restyle with `.sx(..)` or wrap any element (a card, a row).
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    to: SharedString,
    replace: bool,
    preload: bool,
    preload_visible: bool,
    exact: bool,
    children: Vec<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Link);

impl Link {
    /// A link to `to`. `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, to: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            to: to.into(),
            replace: false,
            preload: false,
            preload_visible: false,
            exact: false,
            children: Vec::new(),
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// A link to a typed route. Its element id is the route's path; use
    /// [`Link::new`] when two links to the same path are siblings.
    ///
    /// ```
    /// # use rok_ui::{prelude::*, typed_route};
    /// typed_route! { pub struct NoteRoute = "/notes/:id" { pub id: u64 } }
    /// let link = Link::to(&NoteRoute { id: 3 }).child("Open note 3");
    /// # let _ = link;
    /// ```
    pub fn to<R: Route>(route: &R) -> Self {
        let href = route.href();
        Self::new(ElementId::Name(href.clone()), href)
    }

    /// Add typed search params to the link's path.
    #[must_use]
    pub fn search<S: Search>(mut self, search: &S) -> Self {
        let location = Location::parse(&self.to);
        self.to = Location::build(location.path(), &search.to_query()).into();
        self
    }

    /// Run the target route's loaders when the pointer enters the link, so its data is
    /// loading before the click (TanStack Router's `preload: "intent"`). Default: off.
    #[must_use]
    pub fn preload(mut self, preload: bool) -> Self {
        self.preload = preload;
        self
    }

    /// Run the target route's loaders as soon as the link is on screen (TanStack Router's
    /// `preload: "viewport"`), for links the user is likely to follow, such as the next page
    /// of a list. Each link preloads once per location the user visits. Default: off.
    #[must_use]
    pub fn preload_visible(mut self, preload: bool) -> Self {
        self.preload_visible = preload;
        self
    }

    /// Replace the current history entry instead of adding one.
    #[must_use]
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// Only count as active on exactly this path, not on paths below it.
    #[must_use]
    pub fn exact(mut self, exact: bool) -> Self {
        self.exact = exact;
        self
    }
}

impl ParentElement for Link {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Link {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let active = is_active(&self.to, self.exact, cx);
        let (to, replace_entry) = (self.to, self.replace);
        let preload_target = self.preload.then(|| to.clone());
        let visible_target = self.preload_visible.then(|| to.clone());
        let element = div()
            .id(self.id)
            .tab_index(0)
            .when_some(visible_target, |element, target| {
                // A marker the size of the link notices when the link is drawn on screen.
                element.relative().child(
                    gpui::canvas(
                        move |bounds, window, cx| {
                            if window.content_mask().bounds.intersects(&bounds) {
                                preload_once_visible(target, window, cx);
                            }
                        },
                        |_, (), _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
            })
            .when_some(preload_target, |element, target| {
                element.on_hover(move |hovered, _, cx| {
                    if *hovered {
                        preload(&target, cx);
                    }
                })
            })
            .sx((&LINK.root, active.then_some(&LINK.active), &self.sx))
            .children(self.children)
            .apply_style_overrides(&self.style_overrides);
        crate::components::interaction::on_activate(
            element,
            Rc::new(move |window: &mut Window, cx: &mut App| {
                with_window(window.window_handle(), || {
                    if replace_entry {
                        replace(to.clone(), cx);
                    } else {
                        navigate(to.clone(), cx);
                    }
                });
            }),
        )
    }
}
