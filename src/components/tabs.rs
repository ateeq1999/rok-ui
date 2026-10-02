//! Tabs: a segmented list of tab triggers. You render the active panel yourself,
//! the way a controlled React component works.

use std::rc::Rc;

use gpui::{div, prelude::*, App, ElementId, SharedString, StyleRefinement, Window};

use crate::sx::SxStyled;
use crate::{hooks::EventHandler, styles, styles::ApplyStyleOverrides};

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

styles! {
    TABS = {
        list: {
            display: flex,
            align: center,
            height: 9,
            padding: 0.75,
            radius: lg,
            background: muted,
            color: muted_foreground,
        },
        trigger: {
            display: flex,
            flex: 1,
            align: center,
            justify: center,
            height: full,
            padding_x: 2,
            radius: md,
            border: 1,
            text: sm,
            font: medium,
            whitespace: nowrap,
            cursor: pointer,
            focus: { shadow: ring },
        },
        selected: {
            background: background,
            border_color: border,
            color: foreground,
            shadow: xs,
        },
        unselected: {
            border_color: transparent,
            color: muted_foreground,
            hover: { color: foreground },
        },
    }
}

impl RenderOnce for Tabs {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let selected_index = self.selected_index;
        let on_change = self.on_change;

        let triggers = self
            .tab_labels
            .into_iter()
            .enumerate()
            .map(|(tab_index, label)| {
                let on_change = on_change.clone();
                div()
                    .id(tab_index)
                    .tab_index(0)
                    .sx((
                        &TABS.trigger,
                        if tab_index == selected_index {
                            &TABS.selected
                        } else {
                            &TABS.unselected
                        },
                    ))
                    .when_some(on_change, |trigger, handler| {
                        trigger.on_click(move |_, window, cx| handler(&tab_index, window, cx))
                    })
                    .child(crate::components::bidi_text::text(label))
            });

        div()
            .id(self.id)
            .sx((&TABS.list, &self.sx))
            .children(triggers)
            .apply_style_overrides(&self.style_overrides)
    }
}
