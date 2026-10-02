//! AlertDialog: a modal that interrupts the user and waits for an answer.

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, ElementId, FontWeight, SharedString, Window};

use super::{
    button::Button,
    interaction::{render_modal, Callback, ModalPlacement},
};
use crate::{hooks::EventHandler, theme::ActiveTheme};

/// Unlike [`super::Dialog`], a click on the backdrop does not dismiss it: the
/// user must pick Cancel (or press Escape, which counts as Cancel) or the action.
///
/// ```ignore
/// AlertDialog::new("delete-account")
///     .open(confirm_open)
///     .title("Are you absolutely sure?")
///     .description("This action cannot be undone.")
///     .action_label("Delete account")
///     .destructive(true)
///     .on_action(move |_, _, cx| delete_account(cx))
///     .on_cancel(move |_, _, cx| set_confirm_open(false, cx))
/// ```
#[derive(IntoElement)]
pub struct AlertDialog {
    id: ElementId,
    open: bool,
    title: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    cancel_label: SharedString,
    action_label: SharedString,
    destructive: bool,
    on_cancel: Option<EventHandler<()>>,
    on_action: Option<EventHandler<()>>,
}

impl AlertDialog {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            title: SharedString::default(),
            description: None,
            children: Vec::new(),
            cancel_label: "Cancel".into(),
            action_label: "Continue".into(),
            destructive: false,
            on_cancel: None,
            on_action: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = label.into();
        self
    }

    pub fn action_label(mut self, label: impl Into<SharedString>) -> Self {
        self.action_label = label.into();
        self
    }

    /// Style the action button as destructive.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    /// Called on Cancel and on Escape. Close the dialog here.
    pub fn on_cancel(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    /// Called on the action button. Close the dialog here too.
    pub fn on_action(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_action = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for AlertDialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

fn to_callback(handler: Option<EventHandler<()>>) -> Callback {
    Rc::new(move |window, cx| {
        if let Some(handler) = handler.as_ref() {
            handler(&(), window, cx);
        }
    })
}

impl RenderOnce for AlertDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let cancel = to_callback(self.on_cancel);
        let action = to_callback(self.on_action);
        let cancel_from_button = cancel.clone();

        let panel = div()
            .flex()
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
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title),
                    )
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
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("alert-dialog-cancel")
                            .outline()
                            .label(self.cancel_label)
                            .on_click(move |_, window, cx| cancel_from_button(window, cx)),
                    )
                    .child(
                        Button::new("alert-dialog-action")
                            .when(self.destructive, |button| button.destructive())
                            .label(self.action_label)
                            .on_click(move |_, window, cx| action(window, cx)),
                    ),
            );

        render_modal(
            self.id,
            ModalPlacement::Center,
            panel,
            Some(cancel),
            None,
            window,
            cx,
        )
    }
}
