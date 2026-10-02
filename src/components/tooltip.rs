//! Tooltip: shadcn/ui's dark pill that appears on hover.

use gpui::{div, prelude::*, AnyView, App, Context, SharedString, Window};

use crate::styles;
use crate::sx::SxStyled;

styles! {
    TOOLTIP = {
        // Tooltips sit a little below and right of the pointer.
        offset: { padding_left: 2, padding_top: 2.5 },
        pill: {
            padding_x: 3,
            padding_y: 1.5,
            radius: md,
            background: foreground,
            color: background,
            font_family: sans,
            text: xs,
        },
    }
}

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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().sx(&TOOLTIP.offset).child(
            div()
                .sx(&TOOLTIP.pill)
                .child(crate::components::bidi_text::text(self.text.clone())),
        )
    }
}
