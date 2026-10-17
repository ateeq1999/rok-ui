//! [`FloatingActionButton`]: the primary action of a screen.

use std::rc::Rc;

use gpui::{div, prelude::*, px, App, ElementId, SharedString, StyleRefinement, Window};

use crate::components::{interaction::on_activate, tooltip::Tooltip};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    sx::{Sx, SxStyled},
};

/// The size of a [`FloatingActionButton`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FabSize {
    /// 40 px.
    Small,
    /// 56 px.
    #[default]
    Regular,
    /// 96 px.
    Large,
}

impl FabSize {
    fn icon_size(self) -> f32 {
        match self {
            Self::Small => 20.,
            Self::Regular => 24.,
            Self::Large => 36.,
        }
    }
}

/// The colors of a [`FloatingActionButton`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FabVariant {
    /// Filled with the primary color.
    #[default]
    Primary,
    /// Filled with the secondary color.
    Secondary,
    /// The card color with a primary-colored icon.
    Surface,
}

styles! {
    FAB = {
        root: {
            display: flex,
            direction: row,
            align: center,
            justify: center,
            gap: 3,
            flex: none,
            shadow: lg,
            cursor: pointer,
            text: sm,
            font: semibold,
            border: 1,
            border_color: transparent,
            hover: { opacity: 0.9 },
            focus: { border_color: ring },
        },
        size(FabSize): {
            Small: { height: 10, min_width: 10, radius: lg },
            Regular: { height: 14, min_width: 14, radius: xl },
            Large: { height: 24, min_width: 24, radius: xl },
        },
        variant(FabVariant): {
            Primary: { background: primary, color: primary_foreground },
            Secondary: { background: secondary, color: secondary_foreground },
            Surface: { background: card, color: primary, border_color: border },
        },
        extended: { padding_x: 4 },
    }
}

/// The screen's primary action, floating above the content (Material's FAB).
/// With a label it becomes an extended FAB.
#[derive(IntoElement)]
pub struct FloatingActionButton {
    id: ElementId,
    icon: IconName,
    label: Option<SharedString>,
    show_label: bool,
    size: FabSize,
    variant: FabVariant,
    tooltip: Option<SharedString>,
    on_click: Option<EventHandler<()>>,
    sx: Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(FloatingActionButton);

impl FloatingActionButton {
    /// A floating action button showing `icon`. `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            icon,
            label: None,
            show_label: true,
            size: FabSize::Regular,
            variant: FabVariant::Primary,
            tooltip: None,
            on_click: None,
            sx: Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Text beside the icon (an extended FAB). It becomes the tooltip when hidden.
    #[must_use]
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show or hide the label, for example to collapse while scrolling.
    #[must_use]
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    /// The size.
    #[must_use]
    pub fn size(mut self, size: FabSize) -> Self {
        self.size = size;
        self
    }

    /// The visual variant.
    #[must_use]
    pub fn variant(mut self, variant: FabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// A tooltip shown on hover.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Called when clicked or activated with the keyboard.
    #[must_use]
    pub fn on_click(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FloatingActionButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let label = self.label.clone().filter(|_| self.show_label);
        let tooltip = self
            .tooltip
            .or_else(|| self.label.clone().filter(|_| label.is_none()));
        let on_click = self.on_click;
        let button = div()
            .id(self.id)
            .tab_index(0)
            .sx((
                &FAB.root,
                FAB.size(self.size),
                FAB.variant(self.variant),
                label.is_some().then_some(&FAB.extended),
                &self.sx,
            ))
            .child(Icon::new(self.icon).size(px(self.size.icon_size())))
            .when_some(label, |button, label| {
                button.child(crate::components::bidi_text::text(label))
            })
            .when_some(tooltip, |button, tooltip| {
                button.tooltip(Tooltip::text(tooltip))
            })
            .apply_style_overrides(&self.style_overrides);
        on_activate(
            button,
            Rc::new(move |window, cx| {
                if let Some(handler) = on_click.as_ref() {
                    handler(&(), window, cx);
                }
            }),
        )
    }
}
