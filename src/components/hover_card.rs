//! HoverCard: a preview card shown while the pointer rests on a trigger.

use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, App, ElementId, StyleRefinement, Task, Window};

use super::overlay::{child_id, floating, popover_surface, Align, Side};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, State},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

struct HoverMemory {
    over_trigger: bool,
    over_card: bool,
    open: bool,
    pending: Option<Task<()>>,
}

/// Opens after the pointer rests on the trigger for `open_delay`, and stays
/// open while the pointer is over the trigger or the card.
///
/// ```ignore
/// HoverCard::new("nextjs")
///     .trigger(Button::new("handle").link().label("@nextjs"))
///     .child(Avatar::new("N"))
///     .child(div().child("The React Framework – created and maintained by @vercel."))
/// ```
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    trigger: Option<AnyElement>,
    children: Vec<AnyElement>,
    side: Side,
    align: Align,
    open_delay: Duration,
    close_delay: Duration,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(HoverCard);

impl HoverCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            children: Vec::new(),
            side: Side::Bottom,
            align: Align::Start,
            open_delay: Duration::from_millis(500),
            close_delay: Duration::from_millis(200),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// How long the pointer must rest before the card opens. Defaults to 500ms.
    pub fn open_delay(mut self, delay: Duration) -> Self {
        self.open_delay = delay;
        self
    }

    /// Grace period before closing, so the pointer can travel to the card.
    pub fn close_delay(mut self, delay: Duration) -> Self {
        self.close_delay = delay;
        self
    }
}

impl ParentElement for HoverCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Re-evaluate after a hover change: open or close once `delay` has passed and
/// the pointer is still (or no longer) over the trigger or card.
fn schedule(memory: &State<HoverMemory>, delay: Duration, cx: &mut App) {
    let should_open = {
        let memory = memory.read(cx);
        memory.over_trigger || memory.over_card
    };
    if should_open == memory.read(cx).open {
        memory.update(cx, |memory| memory.pending = None);
        return;
    }
    let entity = memory.entity().clone();
    let task = cx.spawn(async move |cx| {
        cx.background_executor().timer(delay).await;
        entity
            .update(cx, |memory, cx| {
                memory.open = memory.over_trigger || memory.over_card;
                memory.pending = None;
                cx.notify();
            })
            .ok();
    });
    memory.update(cx, |memory| memory.pending = Some(task));
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let memory: State<HoverMemory> =
            use_keyed_state(child_id(&self.id, "hover"), window, cx, || HoverMemory {
                over_trigger: false,
                over_card: false,
                open: false,
                pending: None,
            });
        let is_open = memory.read(cx).open;
        let open_delay = self.open_delay;
        let close_delay = self.close_delay;
        let trigger_memory = memory.clone();

        let wrapper = div()
            .id(self.id.clone())
            .relative()
            .on_hover(move |hovered, _, cx| {
                trigger_memory.update(cx, |memory| memory.over_trigger = *hovered);
                schedule(
                    &trigger_memory,
                    if *hovered { open_delay } else { close_delay },
                    cx,
                );
            })
            .children(self.trigger);
        if !is_open {
            return wrapper;
        }

        let card_memory = memory.clone();
        let card = popover_surface(cx.theme())
            .id(child_id(&self.id, "card"))
            .occlude()
            .w(px(256.))
            .p(px(16.))
            .gap(px(8.))
            .on_hover(move |hovered, _, cx| {
                card_memory.update(cx, |memory| memory.over_card = *hovered);
                schedule(&card_memory, close_delay, cx);
            })
            .children(self.children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides);
        wrapper.child(floating(self.side, self.align, card, cx))
    }
}
