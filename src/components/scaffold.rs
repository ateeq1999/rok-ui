//! App shells in the style of Flutter's Material widgets: a [`Scaffold`] with an
//! [`AppBar`], drawers, bottom navigation and a [`FloatingActionButton`], and an
//! [`AdaptiveScaffold`] that moves its navigation between a bottom bar, a rail and
//! an extended rail as the window grows.
//!
//! ```ignore
//! Scaffold::new("mail")
//!     .app_bar(AppBar::new().title("Inbox").action(search_button))
//!     .drawer(
//!         NavigationDrawer::new("mail-drawer")
//!             .destinations(destinations)
//!             .selected_index(page)
//!             .on_change(move |index, _, cx| set_page(*index, cx)),
//!     )
//!     .floating_action_button(
//!         FloatingActionButton::new("compose", IconName::Pencil).label("Compose"),
//!     )
//!     .child(message_list)
//! ```

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ElementId, SharedString, StyleRefinement, Window,
};

use super::{
    button::Button,
    direction::{is_rtl, DirectionalStyled},
    interaction::{modal_presence, on_activate, render_modal, Callback, ModalPlacement},
    layout_widgets::{LayoutBuilder, WindowSizeClass},
    navigation::{DrawerScope, NavigationBar, NavigationDestination, NavigationRail},
    overlay::child_id,
    tooltip::Tooltip,
};
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

styles! {
    APP_BAR = {
        root: {
            display: flex,
            direction: column,
            flex: none,
            width: full,
            background: background,
            border_bottom: 1,
            border_color: border,
        },
        row: { display: flex, direction: row, align: center, gap: 1, height: 16, padding_x: 2 },
        title: { flex: 1, min_width: 0, padding_x: 2, text: lg, font: semibold, truncate: true },
        title_centered: { flex: none, max_width: 120 },
        side: { display: flex, direction: row, align: center, gap: 1, flex: none },
        side_centered: { flex: 1, min_width: 0 },
        actions_centered: { justify: end },
    }
}

/// The bar across the top of a [`Scaffold`]: a leading button, a title and actions,
/// with an optional `bottom` row such as tabs.
#[derive(IntoElement)]
pub struct AppBar {
    leading: Option<AnyElement>,
    title: Option<AnyElement>,
    actions: Vec<AnyElement>,
    center_title: bool,
    bottom: Option<AnyElement>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(AppBar);

impl AppBar {
    /// An empty `AppBar`; add content with the builder methods.
    #[must_use]
    pub fn new() -> Self {
        Self {
            leading: None,
            title: None,
            actions: Vec::new(),
            center_title: false,
            bottom: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Before the title. Inside a [`Scaffold`] with a drawer it defaults to a menu
    /// button that opens the drawer.
    #[must_use]
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_any_element());
        self
    }

    /// The title.
    #[must_use]
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(super::bidi_text::text(title));
        self
    }

    /// A custom title, such as a search field or a logo.
    #[must_use]
    pub fn title_element(mut self, element: impl IntoElement) -> Self {
        self.title = Some(element.into_any_element());
        self
    }

    /// An element at the end of the bar, usually a ghost icon button.
    #[must_use]
    pub fn action(mut self, element: impl IntoElement) -> Self {
        self.actions.push(element.into_any_element());
        self
    }

    /// Center the title between the leading element and the actions.
    #[must_use]
    pub fn center_title(mut self, center: bool) -> Self {
        self.center_title = center;
        self
    }

    /// A second row under the bar, such as [`super::Tabs`].
    #[must_use]
    pub fn bottom(mut self, element: impl IntoElement) -> Self {
        self.bottom = Some(element.into_any_element());
        self
    }
}

impl Default for AppBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for AppBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let centered = self.center_title;
        let leading = div()
            .sx((&APP_BAR.side, centered.then_some(&APP_BAR.side_centered)))
            .children(self.leading);
        let title = div()
            .sx((&APP_BAR.title, centered.then_some(&APP_BAR.title_centered)))
            .children(self.title);
        let actions = div()
            .sx((
                &APP_BAR.side,
                centered.then_some(&APP_BAR.side_centered),
                centered.then_some(&APP_BAR.actions_centered),
            ))
            .children(self.actions);
        div()
            .sx((&APP_BAR.root, &self.sx))
            .child(
                div()
                    .sx(&APP_BAR.row)
                    .child(leading)
                    .child(title)
                    .child(actions),
            )
            .children(self.bottom)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// The size of a [`FloatingActionButton`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FabSize {
    /// 40 px.
    Small,
    /// 56 px.
    #[default]
    Regular,
    /// 96 px.
    Large,
}

impl FabSize {
    fn icon_size(self) -> f32 {
        match self {
            Self::Small => 20.,
            Self::Regular => 24.,
            Self::Large => 36.,
        }
    }
}

/// The colors of a [`FloatingActionButton`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FabVariant {
    /// Filled with the primary color.
    #[default]
    Primary,
    /// Filled with the secondary color.
    Secondary,
    /// The card color with a primary-colored icon.
    Surface,
}

styles! {
    FAB = {
        root: {
            display: flex,
            direction: row,
            align: center,
            justify: center,
            gap: 3,
            flex: none,
            shadow: lg,
            cursor: pointer,
            text: sm,
            font: semibold,
            border: 1,
            border_color: transparent,
            hover: { opacity: 0.9 },
            focus: { border_color: ring },
        },
        size(FabSize): {
            Small: { height: 10, min_width: 10, radius: lg },
            Regular: { height: 14, min_width: 14, radius: xl },
            Large: { height: 24, min_width: 24, radius: xl },
        },
        variant(FabVariant): {
            Primary: { background: primary, color: primary_foreground },
            Secondary: { background: secondary, color: secondary_foreground },
            Surface: { background: card, color: primary, border_color: border },
        },
        extended: { padding_x: 4 },
    }
}

/// The screen's primary action, floating above the content (Material's FAB).
/// With a label it becomes an extended FAB.
#[derive(IntoElement)]
pub struct FloatingActionButton {
    id: ElementId,
    icon: IconName,
    label: Option<SharedString>,
    show_label: bool,
    size: FabSize,
    variant: FabVariant,
    tooltip: Option<SharedString>,
    on_click: Option<EventHandler<()>>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(FloatingActionButton);

impl FloatingActionButton {
    /// A floating action button showing `icon`. `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            icon,
            label: None,
            show_label: true,
            size: FabSize::Regular,
            variant: FabVariant::Primary,
            tooltip: None,
            on_click: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Text beside the icon (an extended FAB). It becomes the tooltip when hidden.
    #[must_use]
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show or hide the label, for example to collapse while scrolling.
    #[must_use]
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    /// The size.
    #[must_use]
    pub fn size(mut self, size: FabSize) -> Self {
        self.size = size;
        self
    }

    /// The visual variant.
    #[must_use]
    pub fn variant(mut self, variant: FabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// A tooltip shown on hover.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Called when clicked or activated with the keyboard.
    #[must_use]
    pub fn on_click(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FloatingActionButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let label = self.label.clone().filter(|_| self.show_label);
        let tooltip = self
            .tooltip
            .or_else(|| self.label.clone().filter(|_| label.is_none()));
        let on_click = self.on_click;
        let button = div()
            .id(self.id)
            .tab_index(0)
            .sx((
                &FAB.root,
                FAB.size(self.size),
                FAB.variant(self.variant),
                label.is_some().then_some(&FAB.extended),
                &self.sx,
            ))
            .child(Icon::new(self.icon).size(px(self.size.icon_size())))
            .when_some(label, |button, label| {
                button.child(super::bidi_text::text(label))
            })
            .when_some(tooltip, |button, tooltip| {
                button.tooltip(Tooltip::text(tooltip))
            })
            .apply_style_overrides(&self.style_overrides);
        on_activate(
            button,
            Rc::new(move |window, cx| {
                if let Some(handler) = on_click.as_ref() {
                    handler(&(), window, cx);
                }
            }),
        )
    }
}

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

        let body = div()
            .id(child_id(&id, "body"))
            .sx(&SCAFFOLD.body)
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

/// A [`Scaffold`] whose navigation adapts to the width it has, following Material
/// 3: a bottom [`NavigationBar`] on compact widths, a [`NavigationRail`] from
/// medium widths and an extended rail with labels from large widths. The
/// floating action button moves into the rail with it.
///
/// ```ignore
/// AdaptiveScaffold::new("app")
///     .app_bar(AppBar::new().title("Photos"))
///     .destination(NavigationDestination::new(IconName::Home, "Home"))
///     .destination(NavigationDestination::new(IconName::Image, "Albums"))
///     .destination(NavigationDestination::new(IconName::Heart, "Favorites"))
///     .selected_index(page)
///     .on_change(move |index, _, cx| set_page(*index, cx))
///     .floating_action_button(FloatingActionButton::new("add", IconName::Plus).label("Add"))
///     .child(page_content)
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
