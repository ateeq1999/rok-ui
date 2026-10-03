//! Item: a row with media, title, description and actions, for lists and settings.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ClickEvent, ElementId, SharedString, StyleRefinement,
    Window,
};

use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
};

/// Visual style of an [`Item`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemVariant {
    /// No border or fill.
    #[default]
    Default,
    /// Bordered.
    Outline,
    /// Muted fill.
    Muted,
}

/// ```ignore
/// Item::new("security")
///     .variant(ItemVariant::Outline)
///     .icon(IconName::Settings)
///     .title("Two-factor authentication")
///     .description("Verify via email or phone number.")
///     .action(Button::new("enable").small().label("Enable"))
/// ```
#[derive(IntoElement)]
pub struct Item {
    id: ElementId,
    variant: ItemVariant,
    small: bool,
    icon: Option<IconName>,
    media: Option<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    actions: Vec<AnyElement>,
    children: Vec<AnyElement>,
    on_click: Option<EventHandler<ClickEvent>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Item);

impl Item {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: ItemVariant::Default,
            small: false,
            icon: None,
            media: None,
            title: None,
            description: None,
            actions: Vec::new(),
            children: Vec::new(),
            on_click: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// The visual variant.
    #[must_use]
    pub fn variant(mut self, variant: ItemVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Shorthand for `.variant(ItemVariant::Outline)`.
    #[must_use]
    pub fn outline(self) -> Self {
        self.variant(ItemVariant::Outline)
    }

    /// Shorthand for `.variant(ItemVariant::Muted)`.
    #[must_use]
    pub fn muted(self) -> Self {
        self.variant(ItemVariant::Muted)
    }

    /// Tighter padding.
    #[must_use]
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// An icon in a bordered tile on the left.
    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Any element on the left, such as an [`super::Avatar`] or an image.
    #[must_use]
    pub fn media(mut self, media: impl IntoElement) -> Self {
        self.media = Some(media.into_any_element());
        self
    }

    /// The title.
    #[must_use]
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Secondary text below the title.
    #[must_use]
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// An element on the right, such as a button or a badge.
    #[must_use]
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    /// Make the whole row clickable.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for Item {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

styles! {
    ITEM = {
        root: {
            display: flex,
            align: center,
            gap: 4,
            width: full,
            padding_x: 4,
            padding_y: 4,
            radius: md,
            border: 1,
            border_color: transparent,
            text: sm,
        },
        small: { gap: 2.5, padding_y: 3 },
        variant(ItemVariant): {
            Default: {},
            Outline: { border_color: border },
            Muted: { background: muted/50 },
        },
        clickable: { cursor: pointer, hover: { background: accent/50 } },
        icon: {
            flex: none,
            size: 8,
            display: flex,
            align: center,
            justify: center,
            radius: sm,
            border: 1,
            border_color: border,
            background: muted,
        },
        media: { flex: none },
        content: { display: flex, direction: column, flex: 1, gap: 1, min_width: 0 },
        title: { font: medium, line_height: 5 },
        description: { color: muted_foreground, line_clamp: 2 },
        actions: { display: flex, align: center, gap: 2 },
        group: { display: flex, direction: column, width: full },
        divider: { height: 0.25, margin_x: 4, background: border },
    }
}

impl RenderOnce for Item {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            // The caller's `sx` is merged into the same call: GPUI allows a single
            // hover / focus style per element.
            .sx((
                &ITEM.root,
                self.small.then_some(&ITEM.small),
                ITEM.variant(self.variant),
                self.on_click.is_some().then_some(&ITEM.clickable),
                &self.sx,
            ))
            .when_some(self.on_click, |item, handler| {
                item.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when_some(self.icon, |item, icon| {
                item.child(div().sx(&ITEM.icon).child(Icon::new(icon).size(px(16.))))
            })
            .when_some(self.media, |item, media| {
                item.child(div().sx(&ITEM.media).child(media))
            })
            .child(
                div()
                    .sx(&ITEM.content)
                    .when_some(self.title, |content, title| {
                        content.child(
                            div()
                                .sx(&ITEM.title)
                                .child(crate::components::bidi_text::text(title)),
                        )
                    })
                    .when_some(self.description, |content, description| {
                        content.child(
                            div()
                                .sx(&ITEM.description)
                                .child(crate::components::bidi_text::text(description)),
                        )
                    })
                    .children(self.children),
            )
            .when(!self.actions.is_empty(), |item| {
                item.child(div().sx(&ITEM.actions).children(self.actions))
            })
            .apply_style_overrides(&self.style_overrides)
    }
}

/// A list of [`Item`]s separated by hairlines.
#[derive(IntoElement)]
pub struct ItemGroup {
    items: Vec<AnyElement>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(ItemGroup);

impl ItemGroup {
    /// A list of items.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl Default for ItemGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for ItemGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.items.extend(elements);
    }
}

impl RenderOnce for ItemGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let count = self.items.len();
        let mut children = Vec::with_capacity(count * 2);
        for (index, item) in self.items.into_iter().enumerate() {
            children.push(item);
            if index + 1 < count {
                children.push(div().sx(&ITEM.divider).into_any_element());
            }
        }
        div()
            .sx((&ITEM.group, &self.sx))
            .children(children)
            .apply_style_overrides(&self.style_overrides)
    }
}
