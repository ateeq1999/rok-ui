//! Item: a row with media, title, description and actions, for lists and settings.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, ClickEvent, CursorStyle, ElementId, FontWeight,
    SharedString, StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
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

    pub fn variant(mut self, variant: ItemVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Shorthand for `.variant(ItemVariant::Outline)`.
    pub fn outline(self) -> Self {
        self.variant(ItemVariant::Outline)
    }

    /// Shorthand for `.variant(ItemVariant::Muted)`.
    pub fn muted(self) -> Self {
        self.variant(ItemVariant::Muted)
    }

    /// Tighter padding.
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// An icon in a bordered tile on the left.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Any element on the left, such as an [`super::Avatar`] or an image.
    pub fn media(mut self, media: impl IntoElement) -> Self {
        self.media = Some(media.into_any_element());
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

    /// An element on the right, such as a button or a badge.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    /// Make the whole row clickable.
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

impl RenderOnce for Item {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // The component's own interaction styles, merged with the caller's `sx` in one
        // call (GPUI allows a single hover / focus style per element).
        let own_states = if self.on_click.is_some() {
            crate::sx::Sx::new().hover(|state| state.bg(crate::sx::ColorToken::Accent.alpha(0.5)))
        } else {
            crate::sx::Sx::new()
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (padding_x, padding_y, gap) = if self.small {
            (px(16.), px(12.), px(10.))
        } else {
            (px(16.), px(16.), px(16.))
        };
        div()
            .id(self.id)
            .flex_dir()
            .items_center()
            .gap(gap)
            .w_full()
            .px(padding_x)
            .py(padding_y)
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(gpui::transparent_black())
            .text_sm()
            .map(|item| match self.variant {
                ItemVariant::Default => item,
                ItemVariant::Outline => item.border_color(colors.border),
                ItemVariant::Muted => item.bg(colors.muted.opacity(0.5)),
            })
            .when_some(self.on_click, |item, handler| {
                item.cursor(CursorStyle::PointingHand)
                    .on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when_some(self.icon, |item, icon| {
                item.child(
                    div()
                        .flex_none()
                        .size(px(32.))
                        .flex_dir()
                        .items_center()
                        .justify_center()
                        .rounded(theme.radius_small())
                        .border_1()
                        .border_color(colors.border)
                        .bg(colors.muted)
                        .child(Icon::new(icon).size(px(16.))),
                )
            })
            .when_some(self.media, |item, media| {
                item.child(div().flex_none().child(media))
            })
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .flex_1()
                    .gap(px(4.))
                    .min_w_0()
                    .when_some(self.title, |content, title| {
                        content.child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .line_height(px(20.))
                                .child(title),
                        )
                    })
                    .when_some(self.description, |content, description| {
                        content.child(
                            div()
                                .text_color(colors.muted_foreground)
                                .line_clamp(2)
                                .child(description),
                        )
                    })
                    .children(self.children),
            )
            .when(!self.actions.is_empty(), |item| {
                item.child(
                    div()
                        .flex_dir()
                        .items_center()
                        .gap(px(8.))
                        .children(self.actions),
                )
            })
            .sx((&own_states, &self.sx))
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let border = cx.theme().colors.border;
        let count = self.items.len();
        let mut children = Vec::with_capacity(count * 2);
        for (index, item) in self.items.into_iter().enumerate() {
            children.push(item);
            if index + 1 < count {
                children.push(div().h(px(1.)).mx(px(16.)).bg(border).into_any_element());
            }
        }
        div()
            .flex_dir()
            .flex_col()
            .w_full()
            .children(children)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}
