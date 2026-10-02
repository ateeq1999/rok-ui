//! Accordion: stacked headings that each reveal a section of content.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, FontWeight, SharedString,
    StyleRefinement, Window,
};

use super::{interaction::on_activate, overlay::child_id};
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// One heading and its panel.
pub struct AccordionItem {
    title: SharedString,
    children: Vec<AnyElement>,
    disabled: bool,
}

impl AccordionItem {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for AccordionItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Uncontrolled by default (`default_open` sets the first state); pass
/// `open_items` + `on_change` to control it. One item open at a time unless
/// `.multiple(true)`.
///
/// ```ignore
/// Accordion::new("faq")
///     .item(AccordionItem::new("Is it accessible?").child("Yes. It adheres to WAI-ARIA."))
///     .item(AccordionItem::new("Is it styled?").child("Yes. It comes with default styles."))
///     .default_open([0])
/// ```
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    items: Vec<AccordionItem>,
    multiple: bool,
    default_open: Vec<usize>,
    open_items: Option<Vec<usize>>,
    on_change: Option<EventHandler<Vec<usize>>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Accordion);

impl Accordion {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            multiple: false,
            default_open: Vec::new(),
            open_items: None,
            on_change: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    /// Allow several items open at once.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Items open on first render (uncontrolled mode).
    pub fn default_open(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.default_open = indices.into_iter().collect();
        self
    }

    /// Indices of the open items (controlled mode).
    pub fn open_items(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.open_items = Some(indices.into_iter().collect());
        self
    }

    /// Receives the indices of the open items after a toggle.
    pub fn on_change(
        mut self,
        handler: impl Fn(&Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_open = self.default_open.clone();
        let internal = use_keyed_state(child_id(&self.id, "open"), window, cx, || default_open);
        let open_items = self.open_items.clone().unwrap_or_else(|| internal.get(cx));
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let item_count = self.items.len();
        let multiple = self.multiple;

        let items = self.items.into_iter().enumerate().map(|(index, item)| {
            let is_open = open_items.contains(&index);
            let toggle = {
                let open_items = open_items.clone();
                let internal = internal.clone();
                let on_change = self.on_change.clone();
                Rc::new(move |window: &mut Window, cx: &mut App| {
                    let mut next = if multiple {
                        open_items.clone()
                    } else {
                        Vec::new()
                    };
                    if is_open {
                        next.retain(|open_index| *open_index != index);
                    } else {
                        next.push(index);
                    }
                    internal.set(next.clone(), cx);
                    if let Some(handler) = on_change.as_ref() {
                        handler(&next, window, cx);
                    }
                })
            };
            let ring_color = colors.ring;
            let trigger = div()
                .id(("accordion-trigger", index))
                .flex()
                .items_start()
                .justify_between()
                .gap(px(16.))
                .py(px(16.))
                .rounded(theme.radius_medium())
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(div().flex_1().child(item.title))
                .child(
                    Icon::new(if is_open {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .size(px(16.))
                    .color(colors.muted_foreground)
                    .mt(px(2.)),
                );
            let trigger = if item.disabled {
                trigger.opacity(0.5)
            } else {
                on_activate(
                    trigger
                        .tab_index(0)
                        .cursor(CursorStyle::PointingHand)
                        .hover(|style| style.underline())
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .focus(move |style| style.border_color(ring_color)),
                    toggle,
                )
            };
            div()
                .flex()
                .flex_col()
                .when(index + 1 < item_count, |item_row| {
                    item_row.border_b_1().border_color(colors.border)
                })
                .child(trigger)
                .when(is_open, |item_row| {
                    item_row.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .pb(px(16.))
                            .text_sm()
                            .children(item.children),
                    )
                })
        });

        div()
            .id(self.id)
            .flex()
            .flex_col()
            .w_full()
            .children(items)
            .apply_style_overrides(&self.style_overrides)
    }
}
