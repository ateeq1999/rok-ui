//! Tooltip: shadcn/ui's dark pill that appears on hover.

use gpui::{div, prelude::*, px, AnyView, App, Context, SharedString, Window};

use crate::theme::ActiveTheme;

/// A text tooltip. Attach it to any interactive element:
///
/// ```ignore
/// div().id("help").tooltip(Tooltip::text("Opens the docs"))
/// Button::new("save").tooltip("Save")   // buttons have a shortcut
/// ```
pub struct Tooltip {
    text: SharedString,
}

impl Tooltip {
    /// A tooltip builder for `.tooltip(..)` that shows `text`.
    pub fn text(
        text: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let text = text.into();
        move |_, cx| {
            let text = text.clone();
            cx.new(|_| Tooltip { text }).into()
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        // Tooltips sit a little below and right of the pointer.
        div().pl(px(8.)).pt(px(10.)).child(
            div()
                .px(px(12.))
                .py(px(6.))
                .rounded(theme.radius_medium())
                .bg(theme.colors.foreground)
                .text_color(theme.colors.background)
                .font_family(theme.font_family.clone())
                .text_xs()
                .child(self.text.clone()),
        )
    }
}
