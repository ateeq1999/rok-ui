//! Toast: short messages that appear in a corner and dismiss themselves
//! (shadcn/ui's Sonner).
//!
//! ```ignore
//! toast(cx, Toast::new("Event has been created")
//!     .description("Sunday, December 03, 2023 at 9:00 AM")
//!     .action("Undo", |_, cx| undo(cx)));
//! toast(cx, Toast::success("Profile saved"));
//! ```
//!
//! [`super::AppRoot`] draws the toasts; without it, render a [`Toaster`] yourself.

use std::{rc::Rc, time::Duration};

use gpui::{div, point, prelude::*, px, App, Corner, FontWeight, Global, SharedString, Window};

use super::layer::layer_at;
use super::{button::Button, spinner::Spinner};
use crate::{
    icon::{Icon, IconName},
    motion::{presets, Easing, MotionExt, MotionSide},
    theme::ActiveTheme,
};

/// Identifies a toast, for [`dismiss_toast`] and [`update_toast`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

/// Icon and accent of a toast.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToastVariant {
    #[default]
    Default,
    Success,
    Info,
    Warning,
    Error,
    /// A spinner; stays until dismissed or updated.
    Loading,
}

type ToastAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// The content of one toast.
#[derive(Clone)]
pub struct Toast {
    title: SharedString,
    description: Option<SharedString>,
    variant: ToastVariant,
    action: Option<(SharedString, ToastAction)>,
    duration: Option<Duration>,
}

impl Toast {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            variant: ToastVariant::Default,
            action: None,
            duration: Some(Duration::from_secs(4)),
        }
    }

    pub fn success(title: impl Into<SharedString>) -> Self {
        Self::new(title).variant(ToastVariant::Success)
    }

    pub fn info(title: impl Into<SharedString>) -> Self {
        Self::new(title).variant(ToastVariant::Info)
    }

    pub fn warning(title: impl Into<SharedString>) -> Self {
        Self::new(title).variant(ToastVariant::Warning)
    }

    pub fn error(title: impl Into<SharedString>) -> Self {
        Self::new(title).variant(ToastVariant::Error)
    }

    /// A toast with a spinner that stays until you update or dismiss it.
    pub fn loading(title: impl Into<SharedString>) -> Self {
        Self::new(title).variant(ToastVariant::Loading).persistent()
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    /// A button on the toast; clicking it runs `handler` and dismisses the toast.
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.action = Some((label.into(), Rc::new(handler)));
        self
    }

    /// How long it stays. Defaults to 4 seconds.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Stay until dismissed.
    pub fn persistent(mut self) -> Self {
        self.duration = None;
        self
    }
}

#[derive(Default)]
struct ToastStore {
    toasts: Vec<(ToastId, Toast)>,
    next_id: u64,
}

impl Global for ToastStore {}

/// Show a toast. Returns its id so it can be updated or dismissed early.
pub fn toast(cx: &mut App, toast: Toast) -> ToastId {
    let store = cx.default_global::<ToastStore>();
    store.next_id += 1;
    let id = ToastId(store.next_id);
    let duration = toast.duration;
    store.toasts.push((id, toast));
    // Keep the stack short, like Sonner's visible limit.
    if store.toasts.len() > 5 {
        store.toasts.remove(0);
    }
    schedule_dismissal(id, duration, cx);
    cx.refresh_windows();
    id
}

/// Replace a toast's content (turn a loading toast into a success, say).
pub fn update_toast(id: ToastId, toast: Toast, cx: &mut App) {
    let duration = toast.duration;
    let store = cx.default_global::<ToastStore>();
    if let Some(entry) = store
        .toasts
        .iter_mut()
        .find(|(toast_id, _)| *toast_id == id)
    {
        entry.1 = toast;
        schedule_dismissal(id, duration, cx);
        cx.refresh_windows();
    }
}

/// Remove a toast now.
pub fn dismiss_toast(id: ToastId, cx: &mut App) {
    let store = cx.default_global::<ToastStore>();
    let count = store.toasts.len();
    store.toasts.retain(|(toast_id, _)| *toast_id != id);
    if store.toasts.len() != count {
        cx.refresh_windows();
    }
}

fn schedule_dismissal(id: ToastId, duration: Option<Duration>, cx: &mut App) {
    let Some(duration) = duration else {
        return;
    };
    cx.spawn(async move |cx| {
        cx.background_executor().timer(duration).await;
        cx.update(|cx| dismiss_toast(id, cx)).ok();
    })
    .detach();
}

/// Draws the active toasts in the bottom-right corner of the window.
#[derive(IntoElement, Default)]
pub struct Toaster;

impl Toaster {
    pub fn new() -> Self {
        Self
    }
}

impl RenderOnce for Toaster {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let toasts: Vec<(ToastId, Toast)> = cx
            .try_global::<ToastStore>()
            .map(|store| store.toasts.clone())
            .unwrap_or_default();
        if toasts.is_empty() {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let radius = theme.radius_large();
        let viewport_size = window.viewport_size();
        let width = px(356.).min(viewport_size.width - px(32.));

        let cards = toasts.into_iter().map(|(id, toast)| {
            let icon = match toast.variant {
                ToastVariant::Default => None,
                ToastVariant::Success => Some(Icon::new(IconName::CircleCheck).into_any_element()),
                ToastVariant::Info => Some(Icon::new(IconName::Info).into_any_element()),
                ToastVariant::Warning => {
                    Some(Icon::new(IconName::TriangleAlert).into_any_element())
                }
                ToastVariant::Error => Some(
                    Icon::new(IconName::CircleX)
                        .color(colors.destructive_text)
                        .into_any_element(),
                ),
                ToastVariant::Loading => Some(Spinner::new().into_any_element()),
            };
            let action = toast.action.clone().map(|(label, handler)| {
                Button::new(("toast-action", id.0))
                    .small()
                    .label(label)
                    .on_click(move |_, window, cx| {
                        handler(window, cx);
                        dismiss_toast(id, cx);
                    })
            });
            div()
                .id(("toast", id.0))
                .occlude()
                .relative()
                .flex()
                .items_center()
                .gap(px(10.))
                .w(width)
                .p(px(16.))
                .pr(px(32.))
                .rounded(radius)
                .border_1()
                .border_color(colors.border)
                .bg(colors.popover)
                .text_color(colors.popover_foreground)
                .shadow_lg()
                .children(icon)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(toast.title.clone()),
                        )
                        .when_some(toast.description.clone(), |text, description| {
                            text.child(
                                div()
                                    .text_sm()
                                    .text_color(colors.muted_foreground)
                                    .child(description),
                            )
                        }),
                )
                .children(action)
                .child(
                    div().absolute().top(px(6.)).right(px(6.)).child(
                        Button::new(("toast-close", id.0))
                            .ghost()
                            .icon_only(IconName::Close)
                            .w(px(20.))
                            .h(px(20.))
                            .tooltip("Dismiss")
                            .on_click(move |_, _, cx| dismiss_toast(id, cx)),
                    ),
                )
                .motion(
                    ("toast-enter", id.0),
                    presets::slide_in(MotionSide::Bottom, 4.)
                        .duration_ms(220)
                        .easing(Easing::Spring { damping: 0.7 }),
                )
        });

        let stack = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .font_family(theme.font_family.clone())
            .text_size(theme.font_size)
            .children(cards);
        let corner = point(
            viewport_size.width - px(16.),
            viewport_size.height - px(16.),
        );
        layer_at(corner, Corner::BottomRight, px(0.), stack, 2, cx)
    }
}
