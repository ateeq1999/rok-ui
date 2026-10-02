//! Sheet and Drawer: panels that slide in from an edge of the window.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, Div, ElementId, FontWeight, SharedString, Window,
};

use super::{
    button::Button,
    interaction::{render_modal, Callback, ModalPlacement},
};
use crate::{hooks::EventHandler, icon::IconName, theme::ActiveTheme};

/// The edge a [`Sheet`] is attached to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SheetSide {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
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

    fn render_into(self, panel: Div, centered_header: bool, cx: &App) -> Div {
        let muted_foreground = cx.theme().colors.muted_foreground;
        let has_header = self.title.is_some() || self.description.is_some();
        panel
            .when(has_header, |panel| {
                panel.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .p(px(16.))
                        .when(centered_header, |header| header.items_center())
                        .when_some(self.title, |header, title| {
                            header.child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                        })
                        .when_some(self.description, |header, description| {
                            header.child(
                                div()
                                    .text_sm()
                                    .text_color(muted_foreground)
                                    .child(description),
                            )
                        }),
                )
            })
            .child(
                div()
                    .id("sheet-body")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap(px(16.))
                    .px(px(16.))
                    .overflow_y_scroll()
                    .children(self.children),
            )
            .when(!self.footer.is_empty(), |panel| {
                panel.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .p(px(16.))
                        .children(self.footer),
                )
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
/// ```ignore
/// Sheet::new("edit-profile")
///     .open(sheet_open)
///     .side(SheetSide::Right)
///     .title("Edit profile")
///     .description("Make changes to your profile here.")
///     .child(form)
///     .footer(Button::new("save").label("Save changes"))
///     .on_close(move |_, _, cx| set_sheet_open(false, cx))
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
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            side: SheetSide::Right,
            content: PanelContent::new(),
            on_close: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.content.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.content.description = Some(description.into());
        self
    }

    /// Add an element to the bottom actions area.
    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.content.footer.push(element.into_any_element());
        self
    }

    /// Called on Escape, on a backdrop click and on the close button.
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
        if !self.open {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let close = to_callback(self.on_close);
        let close_from_button = close.clone();
        let viewport_size = window.viewport_size();

        let panel = div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(16.))
            .bg(colors.background)
            .border_color(colors.border)
            .shadow_lg()
            .map(|panel| match self.side {
                SheetSide::Right => panel
                    .h_full()
                    .w(px(384.).min(viewport_size.width * 0.75))
                    .border_l_1(),
                SheetSide::Left => panel
                    .h_full()
                    .w(px(384.).min(viewport_size.width * 0.75))
                    .border_r_1(),
                SheetSide::Top => panel.w_full().border_b_1(),
                SheetSide::Bottom => panel.w_full().border_t_1(),
            });
        let panel = self.content.render_into(panel, false, cx).child(
            div().absolute().top(px(12.)).right(px(12.)).child(
                Button::new("sheet-close")
                    .ghost()
                    .small()
                    .icon_only(IconName::Close)
                    .w(px(28.))
                    .h(px(28.))
                    .tooltip("Close")
                    .on_click(move |_, window, cx| close_from_button(window, cx)),
            ),
        );

        let placement = match self.side {
            SheetSide::Top => ModalPlacement::Top,
            SheetSide::Right => ModalPlacement::Right,
            SheetSide::Bottom => ModalPlacement::Bottom,
            SheetSide::Left => ModalPlacement::Left,
        };
        render_modal(
            self.id,
            placement,
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
/// ```ignore
/// Drawer::new("move-goal")
///     .open(drawer_open)
///     .title("Move Goal")
///     .description("Set your daily activity goal.")
///     .child(goal_picker)
///     .footer(Button::new("submit").label("Submit"))
///     .on_close(move |_, _, cx| set_drawer_open(false, cx))
/// ```
#[derive(IntoElement)]
pub struct Drawer {
    id: ElementId,
    open: bool,
    content: PanelContent,
    on_close: Option<EventHandler<()>>,
}

impl Drawer {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            content: PanelContent::new(),
            on_close: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.content.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.content.description = Some(description.into());
        self
    }

    pub fn footer(mut self, element: impl IntoElement) -> Self {
        self.content.footer.push(element.into_any_element());
        self
    }

    /// Called on Escape and on a backdrop click.
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
        if !self.open {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let close = to_callback(self.on_close);
        let viewport_size = window.viewport_size();
        let radius = theme.radius_large();

        let panel = div()
            .flex()
            .flex_col()
            .items_center()
            .w_full()
            .max_h(viewport_size.height * 0.8)
            .rounded_t(radius)
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.background)
            .shadow_lg()
            .child(
                div()
                    .mt(px(16.))
                    .h(px(8.))
                    .w(px(100.))
                    .rounded_full()
                    .bg(colors.muted),
            );
        let body =
            self.content
                .render_into(div().flex().flex_col().w_full().max_w(px(384.)), true, cx);

        render_modal(
            self.id,
            ModalPlacement::Bottom,
            panel.child(body),
            Some(close.clone()),
            Some(close),
            window,
            cx,
        )
    }
}
