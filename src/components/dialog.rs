//! Dialog: a modal window over a dimmed backdrop.

use std::rc::Rc;

use gpui::{
    div, point, prelude::*, px, AnyElement, App, Corner, ElementId, FontWeight, SharedString,
    Window,
};

use super::direction::DirectionalStyled;
use super::layer::layer_at;
use crate::{components::button::Button, hooks::EventHandler, icon::IconName, theme::ActiveTheme};

/// A controlled modal. Render it anywhere in your view; it draws on top of everything.
/// It closes on Escape, on backdrop click and on its close button, by calling `on_close`.
///
/// ```ignore
/// let dialog_open = use_state(window, cx, || false);
/// Dialog::new("edit-profile")
///     .open(dialog_open.get(cx))
///     .title("Edit profile")
///     .description("Make changes to your profile here.")
///     .child(Input::new(&name_input))
///     .footer(Button::new("save").label("Save changes"))
///     .on_close({ let dialog_open = dialog_open.clone(); move |_, _, cx| dialog_open.set(false, cx) })
/// ```
#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    open: bool,
    title: Option<SharedString>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    footer: Vec<AnyElement>,
    on_close: Option<EventHandler<()>>,
}

impl Dialog {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            title: None,
            description: None,
            children: Vec::new(),
            footer: Vec::new(),
            on_close: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an element to the right-aligned actions row.
    pub fn footer(mut self, footer_element: impl IntoElement) -> Self {
        self.footer.push(footer_element.into_any_element());
        self
    }

    /// Called when the user dismisses the dialog.
    pub fn on_close(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Dialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let presence =
            crate::components::interaction::modal_presence(&self.id, self.open, window, cx);
        if !presence.is_mounted() {
            return div().into_any_element();
        }
        let progress = presence.progress();

        // Focus moves into the dialog when it opens so Escape and Tab work inside it.
        // Only once: grabbing it every frame would fight a modal stacked on top.
        let (dialog_focus_handle, focus_pending) = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| {
                (cx.focus_handle(), Rc::new(std::cell::Cell::new(true)))
            })
            .read(cx)
            .clone();
        if focus_pending.replace(false) {
            let focus_handle_to_focus = dialog_focus_handle.clone();
            window.defer(cx, move |window, _| window.focus(&focus_handle_to_focus));
        }

        let theme = cx.theme();
        let colors = theme.colors.clone();
        let viewport_size = window.viewport_size();
        let on_close = self.on_close;

        let close = {
            let on_close = on_close.clone();
            move |window: &mut Window, cx: &mut App| {
                if let Some(handler) = on_close.as_ref() {
                    handler(&(), window, cx);
                }
            }
        };
        let close_from_backdrop = close.clone();
        let close_from_escape = close.clone();
        let close_from_button = close;

        let panel = div()
            .id("dialog-panel")
            .track_focus(&dialog_focus_handle)
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    close_from_escape(window, cx);
                }
            })
            // Clicks inside the panel must not reach the backdrop.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .relative()
            .flex_dir()
            .flex_col()
            .gap(px(16.))
            .w_full()
            .max_w(px(512.))
            .p(px(24.))
            .rounded(theme.radius_large())
            .border_1()
            .border_color(colors.border)
            .bg(colors.popover)
            .text_color(colors.popover_foreground)
            .shadow_lg()
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .gap(px(8.))
                    .pe(px(24.))
                    .when_some(self.title, |header, title| {
                        header.child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .line_height(px(20.))
                                .child(title),
                        )
                    })
                    .when_some(self.description, |header, description| {
                        header.child(
                            div()
                                .text_sm()
                                .text_color(colors.muted_foreground)
                                .child(description),
                        )
                    }),
            )
            .children(self.children)
            .when(!self.footer.is_empty(), |panel| {
                panel.child(
                    div()
                        .flex_dir()
                        .justify_end()
                        .gap(px(8.))
                        .children(self.footer),
                )
            })
            .child(
                div().absolute().top(px(12.)).inset_end(px(12.)).child(
                    Button::new("dialog-close")
                        .ghost()
                        .small()
                        .icon_only(IconName::Close)
                        .w(px(28.))
                        .h(px(28.))
                        .tooltip("Close")
                        .on_click(move |_, window, cx| close_from_button(window, cx)),
                ),
            );

        let scrim = div()
            .id(self.id)
            .occlude()
            .w(viewport_size.width)
            .h(viewport_size.height)
            .flex_dir()
            .items_center()
            .justify_center()
            .p(px(16.))
            .bg(colors.overlay.opacity(progress))
            .font_family(theme.font_family.clone())
            .text_size(theme.font_size)
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                close_from_backdrop(window, cx)
            })
            .child(
                panel
                    .relative()
                    .top(px(8. * (1. - progress)))
                    .opacity(progress),
            );
        layer_at(point(px(0.), px(0.)), Corner::TopLeft, px(0.), scrim, 1, cx)
    }
}
