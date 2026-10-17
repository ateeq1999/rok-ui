//! [`AdaptiveScaffold`]: a scaffold whose navigation moves between a bottom bar, a rail and
//! an extended rail as the window grows.

use std::rc::Rc;

use gpui::{prelude::*, AnyElement, App, ElementId, StyleRefinement, Window};

use super::{AppBar, FloatingActionButton, Scaffold};
use crate::components::{
    layout_widgets::{LayoutBuilder, WindowSizeClass},
    navigation::{NavigationBar, NavigationDestination, NavigationRail},
    overlay::child_id,
};
use crate::{
    hooks::EventHandler,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// A [`Scaffold`] whose navigation adapts to the width it has, following Material
/// 3: a bottom [`NavigationBar`] on compact widths, a [`NavigationRail`] from
/// medium widths and an extended rail with labels from large widths. The
/// floating action button moves into the rail with it.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let page = 0;
/// # fn set_page(_: usize, _: &mut App) {}
/// # let page_content = div();
/// let app = AdaptiveScaffold::new("app")
///     .app_bar(AppBar::new().title("Photos"))
///     .destination(NavigationDestination::new(IconName::Home, "Home"))
///     .destination(NavigationDestination::new(IconName::Image, "Albums"))
///     .destination(NavigationDestination::new(IconName::Heart, "Favorites"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx))
///     .floating_action_button(FloatingActionButton::new("add", IconName::Plus).label("Add"))
///     .child(page_content);
/// # }
/// ```
#[derive(IntoElement)]
pub struct AdaptiveScaffold {
    id: ElementId,
    app_bar: Option<AppBar>,
    destinations: Vec<NavigationDestination>,
    selected_index: usize,
    on_change: Option<EventHandler<usize>>,
    floating_action_button: Option<FloatingActionButton>,
    rail_from: WindowSizeClass,
    extended_from: WindowSizeClass,
    body: Vec<AnyElement>,
    body_scrollable: bool,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(AdaptiveScaffold);

impl AdaptiveScaffold {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            app_bar: None,
            destinations: Vec::new(),
            selected_index: 0,
            on_change: None,
            floating_action_button: None,
            rail_from: WindowSizeClass::Medium,
            extended_from: WindowSizeClass::Large,
            body: Vec::new(),
            body_scrollable: true,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The bar at the top.
    #[must_use]
    pub fn app_bar(mut self, app_bar: AppBar) -> Self {
        self.app_bar = Some(app_bar);
        self
    }

    /// Add a destination.
    #[must_use]
    pub fn destination(mut self, destination: NavigationDestination) -> Self {
        self.destinations.push(destination);
        self
    }

    /// Add several destinations.
    #[must_use]
    pub fn destinations(
        mut self,
        destinations: impl IntoIterator<Item = NavigationDestination>,
    ) -> Self {
        self.destinations.extend(destinations);
        self
    }

    /// The selected destination.
    #[must_use]
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// Called with the new value.
    #[must_use]
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// A floating action button over the body.
    #[must_use]
    pub fn floating_action_button(mut self, button: FloatingActionButton) -> Self {
        self.floating_action_button = Some(button);
        self
    }

    /// The smallest size class that shows a rail instead of a bottom bar.
    /// Default: `Medium` (600 px).
    #[must_use]
    pub fn rail_from(mut self, size_class: WindowSizeClass) -> Self {
        self.rail_from = size_class;
        self
    }

    /// The smallest size class whose rail shows labels beside the icons.
    /// Default: `Large` (1200 px).
    #[must_use]
    pub fn extended_from(mut self, size_class: WindowSizeClass) -> Self {
        self.extended_from = size_class;
        self
    }

    /// Whether the body scrolls vertically. Default: true.
    #[must_use]
    pub fn body_scrollable(mut self, scrollable: bool) -> Self {
        self.body_scrollable = scrollable;
        self
    }
}

impl ParentElement for AdaptiveScaffold {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for AdaptiveScaffold {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            app_bar,
            destinations,
            selected_index,
            on_change,
            floating_action_button,
            rail_from,
            extended_from,
            body,
            body_scrollable,
            sx,
            style_overrides,
        } = self;
        let scaffold_id = child_id(&id, "scaffold");
        let navigation_id = child_id(&id, "navigation");
        LayoutBuilder::new(id, move |constraints, _, _| {
            let size_class = constraints.size_class();
            let forward = move |index: &usize, window: &mut Window, cx: &mut App| {
                if let Some(handler) = on_change.as_ref() {
                    handler(index, window, cx);
                }
            };
            let mut scaffold = Scaffold::new(scaffold_id)
                .body_scrollable(body_scrollable)
                .children(body);
            if let Some(app_bar) = app_bar {
                scaffold = scaffold.app_bar(app_bar);
            }
            if size_class < rail_from {
                scaffold = scaffold.bottom_navigation_bar(
                    NavigationBar::new(navigation_id)
                        .destinations(destinations)
                        .selected_index(selected_index)
                        .on_change(forward),
                );
                if let Some(fab) = floating_action_button {
                    scaffold = scaffold.floating_action_button(fab);
                }
            } else {
                let extended = size_class >= extended_from;
                scaffold = scaffold.navigation(
                    NavigationRail::new(navigation_id)
                        .extended(extended)
                        .when_some(floating_action_button, |rail, fab| {
                            rail.leading(fab.show_label(extended))
                        })
                        .destinations(destinations)
                        .selected_index(selected_index)
                        .on_change(forward),
                );
            }
            scaffold.into_any_element()
        })
        .size_full()
        .sx(sx)
        .apply_style_overrides(&style_overrides)
    }
}
