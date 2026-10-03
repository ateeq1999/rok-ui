//! Accordion: stacked headings that each reveal a section of content.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ElementId, SharedString, StyleRefinement, Window,
};

use super::{interaction::on_activate, overlay::child_id};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::{Icon, IconName},
    styles,
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
    /// Create it with its title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
            disabled: false,
        }
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
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
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let faq = Accordion::new("faq")
///     .item(AccordionItem::new("Is it accessible?").child("Yes. It adheres to WAI-ARIA."))
///     .item(AccordionItem::new("Is it styled?").child("Yes. It comes with default styles."))
///     .default_open([0]);
/// # }
/// ```
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    items: Vec<AccordionItem>,
    multiple: bool,
    default_open: Vec<usize>,
    open_items: Option<Vec<usize>>,
    on_change: Option<EventHandler<Vec<usize>>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Accordion);

impl Accordion {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            multiple: false,
            default_open: Vec::new(),
            open_items: None,
            on_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add a section.
    #[must_use]
    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    /// Allow several items open at once.
    #[must_use]
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Items open on first render (uncontrolled mode).
    #[must_use]
    pub fn default_open(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.default_open = indices.into_iter().collect();
        self
    }

    /// Indices of the open items (controlled mode).
    #[must_use]
    pub fn open_items(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.open_items = Some(indices.into_iter().collect());
        self
    }

    /// Receives the indices of the open items after a toggle.
    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

styles! {
    ACCORDION = {
        root: { display: flex, direction: column, width: full },
        item: { display: flex, direction: column },
        divided: { border_bottom: 1, border_color: border },
        trigger: {
            display: flex,
            align: start,
            justify: between,
            gap: 4,
            padding_y: 4,
            radius: md,
            text: sm,
            font: medium,
        },
        trigger_interactive: {
            cursor: pointer,
            border: 1,
            border_color: transparent,
            hover: { underline: true },
            focus: { border_color: ring },
        },
        trigger_inert: { opacity: 0.5 },
        title: { flex: 1 },
        panel: { display: flex, direction: column, gap: 2, padding_bottom: 4, text: sm },
    }
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_open = self.default_open.clone();
        let internal = use_keyed_state(child_id(&self.id, "open"), window, cx, || default_open);
        let open_items = self.open_items.clone().unwrap_or_else(|| internal.get(cx));
        let muted_foreground = cx.theme().colors.muted_foreground;
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
            let trigger = div()
                .id(("accordion-trigger", index))
                .sx((
                    &ACCORDION.trigger,
                    if item.disabled {
                        &ACCORDION.trigger_inert
                    } else {
                        &ACCORDION.trigger_interactive
                    },
                ))
                .child(
                    div()
                        .sx(&ACCORDION.title)
                        .child(crate::components::bidi_text::text(item.title)),
                )
                .child(
                    Icon::new(if is_open {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .size(px(16.))
                    .color(muted_foreground)
                    .mt(px(2.)),
                );
            let trigger = if item.disabled {
                trigger
            } else {
                on_activate(trigger.tab_index(0), toggle)
            };
            div()
                .sx((
                    &ACCORDION.item,
                    (index + 1 < item_count).then_some(&ACCORDION.divided),
                ))
                .child(trigger)
                .when(is_open, |item_row| {
                    item_row.child(div().sx(&ACCORDION.panel).children(item.children))
                })
        });

        div()
            .id(self.id)
            .sx((&ACCORDION.root, &self.sx))
            .children(items)
            .apply_style_overrides(&self.style_overrides)
    }
}
