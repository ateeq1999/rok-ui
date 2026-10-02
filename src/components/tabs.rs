//! Tabs: a segmented list of tab triggers. You render the active panel yourself,
//! the way a controlled React component works.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, CursorStyle, ElementId, FontWeight, SharedString, StyleRefinement,
    Window,
};

use super::direction::DirectionalStyled;
use super::focus_ring_shadow;
use crate::sx::SxStyled;
use crate::{hooks::EventHandler, styles::ApplyStyleOverrides, theme::ActiveTheme};

/// ```ignore
/// let selected_tab = use_state(window, cx, || 0usize);
/// let selected = selected_tab.get(cx);
/// div()
///     .child(Tabs::new("settings-tabs")
///         .tab("Account").tab("Password")
///         .selected_index(selected)
///         .on_change(move |index, _, cx| selected_tab.set(*index, cx)))
///     .child(if selected == 0 { account_panel() } else { password_panel() })
/// ```
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    tab_labels: Vec<SharedString>,
    selected_index: usize,
    on_change: Option<EventHandler<usize>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Tabs);

impl Tabs {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tab_labels: Vec::new(),
            selected_index: 0,
            on_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add a tab trigger.
    pub fn tab(mut self, label: impl Into<SharedString>) -> Self {
        self.tab_labels.push(label.into());
        self
    }

    pub fn selected_index(mut self, selected_index: usize) -> Self {
        self.selected_index = selected_index;
        self
    }

    /// Receives the index of the tab the user picked.
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let trigger_radius = theme.radius_medium();
        let ring_color = colors.ring;
        let selected_index = self.selected_index;
        let on_change = self.on_change;

        let triggers = self
            .tab_labels
            .into_iter()
            .enumerate()
            .map(|(tab_index, label)| {
                let is_selected = tab_index == selected_index;
                let on_change = on_change.clone();
                div()
                    .id(tab_index)
                    .flex_dir()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .h_full()
                    .px(px(8.))
                    .rounded(trigger_radius)
                    .border_1()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .tab_index(0)
                    .cursor(CursorStyle::PointingHand)
                    .focus(move |style| style.shadow(focus_ring_shadow(ring_color)))
                    .map(|trigger| {
                        if is_selected {
                            trigger
                                .bg(colors.background)
                                .border_color(colors.border)
                                .text_color(colors.foreground)
                                .shadow(super::extra_small_shadow())
                        } else {
                            trigger
                                .border_color(gpui::transparent_black())
                                .text_color(colors.muted_foreground)
                                .hover(|style| style.text_color(colors.foreground))
                        }
                    })
                    .when_some(on_change, |trigger, handler| {
                        trigger.on_click(move |_, window, cx| handler(&tab_index, window, cx))
                    })
                    .child(label)
            });

        div()
            .id(self.id)
            .flex_dir()
            .items_center()
            .h(px(36.))
            .p(px(3.))
            .rounded(theme.radius_large())
            .bg(colors.muted)
            .text_color(colors.muted_foreground)
            .children(triggers)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}
