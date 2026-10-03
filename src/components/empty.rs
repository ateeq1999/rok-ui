//! Empty: the placeholder shown when there is nothing to display yet.

use gpui::{div, prelude::*, px, AnyElement, App, SharedString, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// ```ignore
/// Empty::new()
///     .icon(IconName::Folder)
///     .title("No projects yet")
///     .description("You haven't created any projects yet.")
///     .child(Button::new("create").label("Create project"))
/// ```
#[derive(IntoElement)]
pub struct Empty {
    icon: Option<IconName>,
    media: Option<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    bordered: bool,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Empty);

impl Empty {
    /// An empty state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            icon: None,
            media: None,
            title: None,
            description: None,
            children: Vec::new(),
            bordered: false,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// An icon in a muted tile above the title.
    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Any element above the title instead of an icon, such as an avatar.
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

    /// Dashed outline around the whole area.
    #[must_use]
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
}

impl Default for Empty {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Empty {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

styles! {
    EMPTY = {
        root: {
            display: flex,
            direction: column,
            align: center,
            justify: center,
            gap: 6,
            width: full,
            padding: 12,
            radius: lg,
        },
        bordered: { border: 1, border_style: dashed, border_color: border },
        header: { display: flex, direction: column, align: center, gap: 2, max_width: 96 },
        icon_tile: {
            margin_bottom: 2,
            size: 10,
            display: flex,
            align: center,
            justify: center,
            radius: lg,
            background: muted,
        },
        media: { margin_bottom: 2 },
        title: { text: lg, font: medium },
        description: { text: sm, text_align: center, color: muted_foreground },
        actions: { display: flex, align: center, gap: 2 },
    }
}

impl RenderOnce for Empty {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let foreground = cx.theme().colors.foreground;
        div()
            .sx((
                &EMPTY.root,
                self.bordered.then_some(&EMPTY.bordered),
                &self.sx,
            ))
            .child(
                div()
                    .sx(&EMPTY.header)
                    .when_some(self.icon, |header, icon| {
                        header.child(
                            div()
                                .sx(&EMPTY.icon_tile)
                                .child(Icon::new(icon).size(px(24.)).color(foreground)),
                        )
                    })
                    .when_some(self.media, |header, media| {
                        header.child(div().sx(&EMPTY.media).child(media))
                    })
                    .when_some(self.title, |header, title| {
                        header.child(
                            div()
                                .sx(&EMPTY.title)
                                .child(crate::components::bidi_text::text(title)),
                        )
                    })
                    .when_some(self.description, |header, description| {
                        header.child(
                            div()
                                .sx(&EMPTY.description)
                                .child(crate::components::bidi_text::text(description)),
                        )
                    }),
            )
            .when(!self.children.is_empty(), |empty| {
                empty.child(div().sx(&EMPTY.actions).children(self.children))
            })
            .apply_style_overrides(&self.style_overrides)
    }
}
