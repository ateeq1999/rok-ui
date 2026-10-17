//! App shells in the style of Flutter's Material widgets: a [`Scaffold`] with an
//! [`AppBar`], drawers, bottom navigation and a [`FloatingActionButton`], and an
//! [`AdaptiveScaffold`] that moves its navigation between a bottom bar, a rail and
//! an extended rail as the window grows.
//!
//! ```no_run
//! # use rok_ui::prelude::*;
//! # fn example(window: &mut Window, cx: &mut App) {
//! # let search_button = Button::new("search").ghost().icon_only(IconName::Search);
//! # let destinations: Vec<NavigationDestination> = Vec::new();
//! # let page = 0;
//! # fn set_page(_: usize, _: &mut App) {}
//! # let message_list = div();
//! let mail = Scaffold::new("mail")
//!     .app_bar(AppBar::new().title("Inbox").action(search_button))
//!     .drawer(
//!         NavigationDrawer::new("mail-drawer")
//!             .destinations(destinations)
//!             .selected_index(page)
//!             .on_change(move |index, _, cx| set_page(*index, cx)),
//!     )
//!     .floating_action_button(FloatingActionButton::new("compose", IconName::Pencil).label("Compose"))
//!     .child(message_list);
//! # }
//! ```

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, ElementId, StyleRefinement, Window};

use super::{
    button::Button,
    direction::{is_rtl, DirectionalStyled},
    interaction::{modal_presence, render_modal, Callback, ModalPlacement},
    navigation::DrawerScope,
    overlay::child_id,
};
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::IconName,
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

mod adaptive;
mod app_bar;
mod fab;

pub use adaptive::AdaptiveScaffold;
pub use app_bar::AppBar;
pub use fab::{FabSize, FabVariant, FloatingActionButton};

/// Where a [`Scaffold`] places its floating action button: above the bottom of
/// the body, 16 px from the edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FabLocation {
    /// At the reading-direction end.
    #[default]
    EndFloat,
    /// Centered horizontally.
    CenterFloat,
    /// At the reading-direction start.
    StartFloat,
}

styles! {
    SCAFFOLD = {
        root: {
            position: relative,
            display: flex,
            direction: column,
            size: full,
            overflow: hidden,
            background: background,
            color: foreground,
        },
        middle: { position: relative, display: flex, direction: row, flex: 1, min_height: 0 },
        body: { display: flex, direction: column, flex: 1, min_width: 0, min_height: 0 },
        scroll_content: { display: flex, direction: column, flex: none, width: full, min_height: full },
        bottom_sheet: {
            flex: none,
            border_top: 1,
            border_color: border,
            background: card,
            color: card_foreground,
        },
        footer: {
            display: flex,
            direction: row,
            justify: end,
            gap: 2,
            flex: none,
            padding: 2,
            border_top: 1,
            border_color: border,
        },
        drawer_panel: {
            display: flex,
            direction: column,
            height: full,
            background: background,
            border_color: border,
            shadow: lg,
        },
    }
}

/// Opens (`true`) or closes (`false`) a drawer.
type SetOpen = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// The open state of one of a scaffold's drawers, controlled or kept per id.
struct DrawerControl {
    open: bool,
    set_open: SetOpen,
}

/// A whole screen: an app bar on top, the body, and optional drawers, side
/// navigation, bottom sheet, footer buttons, bottom navigation bar and floating
/// action button, like Flutter's `Scaffold`.
///
/// Drawers open from the app bar's menu button (added automatically when the app
/// bar has no leading element) and keep their open state per scaffold id. Pass
/// `drawer_open` and `on_drawer_change` to control the start drawer yourself.
#[derive(IntoElement)]
pub struct Scaffold {
    id: ElementId,
    app_bar: Option<AppBar>,
    body: Vec<AnyElement>,
    body_scrollable: bool,
    #[cfg(feature = "router")]
    restore_scroll: bool,
    drawer: Option<AnyElement>,
    end_drawer: Option<AnyElement>,
    drawer_open: Option<bool>,
    on_drawer_change: Option<EventHandler<bool>>,
    navigation: Option<AnyElement>,
    bottom_navigation_bar: Option<AnyElement>,
    bottom_sheet: Option<AnyElement>,
    footer: Vec<AnyElement>,
    floating_action_button: Option<AnyElement>,
    fab_location: FabLocation,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Scaffold);

impl Scaffold {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            app_bar: None,
            body: Vec::new(),
            body_scrollable: true,
            #[cfg(feature = "router")]
            restore_scroll: false,
            drawer: None,
            end_drawer: None,
            drawer_open: None,
            on_drawer_change: None,
            navigation: None,
            bottom_navigation_bar: None,
            bottom_sheet: None,
            footer: Vec::new(),
            floating_action_button: None,
            fab_location: FabLocation::EndFloat,
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

    /// Whether the body scrolls vertically. Default: true. Turn it off when the
    /// body manages its own scrolling or fills the space with `Expanded`.
    #[must_use]
    pub fn body_scrollable(mut self, scrollable: bool) -> Self {
        self.body_scrollable = scrollable;
        self
    }

    /// Remember the body's scroll position per history entry (feature `router`): going back
    /// or forward to a page returns the body to where it was, and a new page starts at the top.
    #[cfg(feature = "router")]
    #[must_use]
    pub fn restore_scroll(mut self, restore: bool) -> Self {
        self.restore_scroll = restore;
        self
    }

    /// A panel that slides in from the start edge, usually a
    /// [`super::NavigationDrawer`].
    #[must_use]
    pub fn drawer(mut self, element: impl IntoElement) -> Self {
        self.drawer = Some(element.into_any_element());
        self
    }

    /// A panel that slides in from the end edge, such as filters or details.
    #[must_use]
    pub fn end_drawer(mut self, element: impl IntoElement) -> Self {
        self.end_drawer = Some(element.into_any_element());
        self
    }

    /// Control whether the start drawer is open.
    #[must_use]
    pub fn drawer_open(mut self, open: bool) -> Self {
        self.drawer_open = Some(open);
        self
    }

    /// Called when the start drawer asks to open or close.
    #[must_use]
    pub fn on_drawer_change(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drawer_change = Some(Rc::new(handler));
        self
    }

    /// Permanent navigation along the start edge, usually a
    /// [`super::NavigationRail`].
    #[must_use]
    pub fn navigation(mut self, element: impl IntoElement) -> Self {
        self.navigation = Some(element.into_any_element());
        self
    }

    /// Along the bottom of the window, usually a [`super::NavigationBar`].
    #[must_use]
    pub fn bottom_navigation_bar(mut self, element: impl IntoElement) -> Self {
        self.bottom_navigation_bar = Some(element.into_any_element());
        self
    }

    /// A panel that stays under the body, above the bottom navigation bar.
    #[must_use]
    pub fn bottom_sheet(mut self, element: impl IntoElement) -> Self {
        self.bottom_sheet = Some(element.into_any_element());
        self
    }

    /// A button in the row under the body (Flutter's `persistentFooterButtons`).
    #[must_use]
    pub fn footer_button(mut self, element: impl IntoElement) -> Self {
        self.footer.push(element.into_any_element());
        self
    }

    /// A floating action button over the body.
    #[must_use]
    pub fn floating_action_button(mut self, button: FloatingActionButton) -> Self {
        self.floating_action_button = Some(button.into_any_element());
        self
    }

    /// Where the floating action button sits.
    #[must_use]
    pub fn fab_location(mut self, location: FabLocation) -> Self {
        self.fab_location = location;
        self
    }
}

impl ParentElement for Scaffold {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

fn drawer_control(
    id: ElementId,
    controlled: Option<bool>,
    on_change: Option<EventHandler<bool>>,
    window: &mut Window,
    cx: &mut App,
) -> DrawerControl {
    let state = use_keyed_state(id, window, cx, || false);
    let open = controlled.unwrap_or_else(|| state.get(cx));
    DrawerControl {
        open,
        set_open: Rc::new(move |open, window, cx| {
            if controlled.is_none() {
                state.set(open, cx);
            }
            if let Some(handler) = on_change.as_ref() {
                handler(&open, window, cx);
            }
        }),
    }
}

/// A drawer panel on the top layer, sliding in from `edge` ("start" or "end").
fn render_drawer(
    id: ElementId,
    at_end: bool,
    content: AnyElement,
    control: &DrawerControl,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let presence = modal_presence(&id, control.open, window, cx);
    if !presence.is_mounted() {
        return div().into_any_element();
    }
    let set_open = control.set_open.clone();
    let close: Callback = Rc::new(move |window, cx| set_open(false, window, cx));
    // Physical edge: the start is on the right in RTL.
    let on_right = at_end != is_rtl();
    let width = px(320.).min(window.viewport_size().width * 0.85);
    let panel = div()
        .sx(&SCAFFOLD.drawer_panel)
        .w(width)
        .map(|panel| {
            if on_right {
                panel.border_l_1()
            } else {
                panel.border_r_1()
            }
        })
        .id(child_id(&id, "panel"))
        .overflow_y_scroll()
        .child(DrawerScope {
            closer: close.clone(),
            child: content,
        });
    let placement = if on_right {
        ModalPlacement::Right
    } else {
        ModalPlacement::Left
    };
    render_modal(
        id,
        placement,
        presence.progress(),
        div().h_full().child(panel),
        Some(close.clone()),
        Some(close),
        window,
        cx,
    )
}

impl RenderOnce for Scaffold {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let start_drawer = self.drawer.is_some().then(|| {
            drawer_control(
                child_id(&id, "drawer-open"),
                self.drawer_open,
                self.on_drawer_change.clone(),
                window,
                cx,
            )
        });
        let end_drawer = self
            .end_drawer
            .is_some()
            .then(|| drawer_control(child_id(&id, "end-drawer-open"), None, None, window, cx));

        let app_bar = self.app_bar.map(|mut app_bar| {
            if let (None, Some(control)) = (&app_bar.leading, &start_drawer) {
                let set_open = control.set_open.clone();
                app_bar.leading = Some(
                    Button::new(child_id(&id, "open-drawer"))
                        .ghost()
                        .icon_only(IconName::Menu)
                        .tooltip("Open navigation menu")
                        .on_click(move |_, window, cx| set_open(true, window, cx))
                        .into_any_element(),
                );
            }
            if let (true, Some(control)) = (app_bar.actions.is_empty(), &end_drawer) {
                let set_open = control.set_open.clone();
                app_bar.actions.push(
                    Button::new(child_id(&id, "open-end-drawer"))
                        .ghost()
                        .icon_only(IconName::Menu)
                        .tooltip("Open panel")
                        .on_click(move |_, window, cx| set_open(true, window, cx))
                        .into_any_element(),
                );
            }
            app_bar
        });

        #[cfg(feature = "router")]
        let restored = (self.body_scrollable && self.restore_scroll).then(|| {
            let memory =
                crate::hooks::use_keyed_state(child_id(&id, "body-scroll"), window, cx, || {
                    (gpui::ScrollHandle::new(), None::<usize>)
                });
            let (handle, last) = memory.read(cx).clone();
            let area = child_id(&id, "body").to_string().into();
            let entry = crate::router::restore_scroll(&area, &handle, last, window, cx);
            memory.update(cx, |memory| memory.1 = Some(entry));
            handle
        });
        #[cfg(not(feature = "router"))]
        let restored: Option<gpui::ScrollHandle> = None;
        let body = div()
            .id(child_id(&id, "body"))
            .sx(&SCAFFOLD.body)
            .when_some(restored, |body, handle| body.track_scroll(&handle))
            .map(|body| {
                if self.body_scrollable {
                    // Content keeps its height and scrolls instead of shrinking to fit.
                    body.overflow_y_scroll()
                        .child(div().sx(&SCAFFOLD.scroll_content).children(self.body))
                } else {
                    body.children(self.body)
                }
            });

        let fab = self.floating_action_button.map(|fab| {
            let slot = div().absolute().bottom(px(16.)).flex();
            match self.fab_location {
                FabLocation::EndFloat => slot.inset_end(px(16.)).child(fab),
                FabLocation::StartFloat => slot.inset_start(px(16.)).child(fab),
                FabLocation::CenterFloat => slot.left_0().right_0().justify_center().child(fab),
            }
        });

        let drawers: Vec<AnyElement> = [
            self.drawer.zip(start_drawer).map(|(content, control)| {
                render_drawer(
                    child_id(&id, "drawer"),
                    false,
                    content,
                    &control,
                    window,
                    cx,
                )
            }),
            self.end_drawer.zip(end_drawer).map(|(content, control)| {
                render_drawer(
                    child_id(&id, "end-drawer"),
                    true,
                    content,
                    &control,
                    window,
                    cx,
                )
            }),
        ]
        .into_iter()
        .flatten()
        .collect();

        div()
            .id(id)
            .sx((&SCAFFOLD.root, &self.sx))
            .children(app_bar)
            .child(
                div()
                    .sx(&SCAFFOLD.middle)
                    .children(self.navigation)
                    .child(body)
                    .children(fab),
            )
            .when_some(self.bottom_sheet, |scaffold, sheet| {
                scaffold.child(div().sx(&SCAFFOLD.bottom_sheet).child(sheet))
            })
            .when(!self.footer.is_empty(), |scaffold| {
                scaffold.child(div().sx(&SCAFFOLD.footer).children(self.footer))
            })
            .children(self.bottom_navigation_bar)
            .children(drawers)
            .apply_style_overrides(&self.style_overrides)
    }
}
