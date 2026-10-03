//! Sheet and Drawer: panels that slide in from an edge of the window.

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, Div, ElementId, SharedString, Window};

use super::{
    button::Button,
    interaction::{modal_presence, render_modal, Callback, ModalPlacement},
};
use crate::sx::SxStyled;
use crate::{hooks::EventHandler, icon::IconName, styles};

/// The edge a [`Sheet`] is attached to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SheetSide {
    /// Slides down from the top edge.
    Top,
    /// Slides in from the right edge.
    #[default]
    Right,
    /// Slides up from the bottom edge.
    Bottom,
    /// Slides in from the left edge.
    Left,
}

styles! {
    SHEET = {
        panel: {
            position: relative,
            display: flex,
            direction: column,
            gap: 4,
            background: background,
            border_color: border,
            shadow: lg,
        },
        // Physical sides: `Left` and `Right` are already swapped for RTL.
        side(SheetSide): {
            Top: { width: full, border_bottom: 1 },
            Right: { height: full, border_left: 1 },
            Bottom: { width: full, border_top: 1 },
            Left: { height: full, border_right: 1 },
        },
        header: { display: flex, direction: column, gap: 1.5, padding: 4 },
        header_centered: { align: center },
        title: { text: base, font: semibold },
        description: { text: sm, color: muted_foreground },
        body: { display: flex, direction: column, flex: 1, gap: 4, padding_x: 4 },
        footer: { display: flex, direction: column, gap: 2, padding: 4 },
        close_slot: { position: absolute, top: 3, inset_end: 3 },
        close: { width: 7, height: 7 },
    }
}

styles! {
    DRAWER = {
        panel: {
            display: flex,
            direction: column,
            align: center,
            width: full,
            radius_top: lg,
            border_top: 1,
            border_color: border,
            background: background,
            shadow: lg,
        },
        handle: { margin_top: 4, height: 2, width: 25, radius: full, background: muted },
        body: { display: flex, direction: column, width: full, max_width: 96 },
    }
}

/// Header, body and footer shared by [`Sheet`] and [`Drawer`].
struct PanelContent {
    title: Option<SharedString>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    footer: Vec<AnyElement>,
}

impl PanelContent {
    fn new() -> Self {
        Self {
            title: None,
            description: None,
            children: Vec::new(),
            footer: Vec::new(),
        }
    }

    fn render_into(self, panel: Div, centered_header: bool) -> Div {
        let has_header = self.title.is_some() || self.description.is_some();
        panel
            .when(has_header, |panel| {
                panel.child(
                    div()
                        .sx((
                            &SHEET.header,
                            centered_header.then_some(&SHEET.header_centered),
                        ))
                        .when_some(self.title, |header, title| {
                            header.child(
                                div()
                                    .sx(&SHEET.title)
                                    .child(crate::components::bidi_text::text(title)),
                            )
                        })
                        .when_some(self.description, |header, description| {
                            header.child(
                                div()
                                    .sx(&SHEET.description)
                                    .child(crate::components::bidi_text::text(description)),
                            )
                        }),
                )
            })
            .child(
                div()
                    .sx(&SHEET.body)
                    .id("sheet-body")
                    .overflow_y_scroll()
                    .children(self.children),
            )
            .when(!self.footer.is_empty(), |panel| {
                panel.child(div().sx(&SHEET.footer).children(self.footer))
            })
    }
}

fn to_callback(handler: Option<EventHandler<()>>) -> Callback {
    Rc::new(move |window, cx| {
        if let Some(handler) = handler.as_ref() {
            handler(&(), window, cx);
        }
    })
}

/// A dialog attached to an edge of the window (shadcn/ui's `<Sheet>`), for
/// content that complements the main screen. Controlled like [`super::Dialog`].
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let sheet_open = true;
/// # let form = div();
/// # fn set_sheet_open(_: bool, _: &mut App) {}
/// let sheet = Sheet::new("edit-profile")
///     .open(sheet_open)
///     .side(SheetSide::Right)
///     .title("Edit profile")
///     .description("Make changes to your profile here.")
///     .child(form)
///     .footer(Button::new("save").label("Save changes"))
///     .on_close(move |_, _, cx| set_sheet_open(false, cx));
/// # }
/// ```
#[derive(IntoElement)]
pub struct Sheet {
    id: ElementId,
    open: bool,
    side: SheetSide,
    content: PanelContent,
    on_close: Option<EventHandler<()>>,
}

impl Sheet {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            side: SheetSide::Right,
            content: PanelContent::new(),
            on_close: None,
        }
    }

    /// Whether it is open (controlled).
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Which window edge the sheet slides in from.
    #[must_use]
    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    /// The title.
    #[must_use]
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.content.title = Some(title.into());
        self
    }

    /// Secondary text below the title.
    #[must_use]
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.content.description = Some(description.into());
        self
    }

    /// Add an element to the bottom actions area.
    #[must_use]
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.content.footer.push(element.into_any_element());
        self
    }

    /// Called on Escape, on a backdrop click and on the close button.
    #[must_use]
    pub fn on_close(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.content.children.extend(elements);
    }
}

impl RenderOnce for Sheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Left and right are the starting and ending edges: they swap in RTL.
        let side = match (self.side, super::direction::is_rtl()) {
            (SheetSide::Left, true) => SheetSide::Right,
            (SheetSide::Right, true) => SheetSide::Left,
            (side, _) => side,
        };
        let presence = modal_presence(&self.id, self.open, window, cx);
        if !presence.is_mounted() {
            return div().into_any_element();
        }
        let close = to_callback(self.on_close);
        let close_from_button = close.clone();
        let viewport_size = window.viewport_size();

        let panel = div().sx((&SHEET.panel, SHEET.side(side))).when(
            matches!(side, SheetSide::Left | SheetSide::Right),
            |panel| panel.w(px(384.).min(viewport_size.width * 0.75)),
        );
        let panel = self.content.render_into(panel, false).child(
            div().sx(&SHEET.close_slot).child(
                Button::new("sheet-close")
                    .ghost()
                    .small()
                    .icon_only(IconName::Close)
                    .sx(&SHEET.close)
                    .tooltip("Close")
                    .on_click(move |_, window, cx| close_from_button(window, cx)),
            ),
        );

        let placement = match side {
            SheetSide::Top => ModalPlacement::Top,
            SheetSide::Right => ModalPlacement::Right,
            SheetSide::Bottom => ModalPlacement::Bottom,
            SheetSide::Left => ModalPlacement::Left,
        };
        render_modal(
            self.id,
            placement,
            presence.progress(),
            panel,
            Some(close.clone()),
            Some(close),
            window,
            cx,
        )
    }
}

/// A bottom sheet with a grab handle (shadcn/ui's `<Drawer>`), good for short
/// focused tasks. Controlled like [`super::Dialog`].
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// # let drawer_open = true;
/// # let goal_picker = div();
/// # fn set_drawer_open(_: bool, _: &mut App) {}
/// let drawer = Drawer::new("move-goal")
///     .open(drawer_open)
///     .title("Move Goal")
///     .description("Set your daily activity goal.")
///     .child(goal_picker)
///     .footer(Button::new("submit").label("Submit"))
///     .on_close(move |_, _, cx| set_drawer_open(false, cx));
/// # }
/// ```
#[derive(IntoElement)]
pub struct Drawer {
    id: ElementId,
    open: bool,
    content: PanelContent,
    on_close: Option<EventHandler<()>>,
}

impl Drawer {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            content: PanelContent::new(),
            on_close: None,
        }
    }

    /// Whether it is open (controlled).
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The title.
    #[must_use]
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.content.title = Some(title.into());
        self
    }

    /// Secondary text below the title.
    #[must_use]
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.content.description = Some(description.into());
        self
    }

    /// Content pinned to the bottom of the drawer.
    #[must_use]
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.content.footer.push(element.into_any_element());
        self
    }

    /// Called on Escape and on a backdrop click.
    #[must_use]
    pub fn on_close(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Drawer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.content.children.extend(elements);
    }
}

impl RenderOnce for Drawer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let presence = modal_presence(&self.id, self.open, window, cx);
        if !presence.is_mounted() {
            return div().into_any_element();
        }
        let close = to_callback(self.on_close);
        let viewport_size = window.viewport_size();

        let panel = div()
            .sx(&DRAWER.panel)
            .max_h(viewport_size.height * 0.8)
            .child(div().sx(&DRAWER.handle));
        let body = self.content.render_into(div().sx(&DRAWER.body), true);

        render_modal(
            self.id,
            ModalPlacement::Bottom,
            presence.progress(),
            panel.child(body),
            Some(close.clone()),
            Some(close),
            window,
            cx,
        )
    }
}
