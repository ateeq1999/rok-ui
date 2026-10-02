//! Combobox: a [`super::Select`] with a search field, built from a popover and a
//! [`super::Command`] list.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, CursorStyle, ElementId, FontWeight, SharedString, StyleRefinement,
    Window,
};

use super::direction::DirectionalStyled;
use super::{
    command::{render_command, CommandGroup, CommandItem},
    extra_small_shadow, focus_ring_shadow,
    input::use_input_state,
    interaction::Callback,
    overlay::{
        child_id, dismissable, floating, measure_width, popover_surface, trigger_wrapper,
        use_open_state, Align, Side,
    },
};
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Controlled like [`super::Select`]: pass `value`, update it in `on_change`.
/// Picking the current value again clears it, like shadcn/ui's example.
///
/// ```ignore
/// Combobox::new("framework")
///     .placeholder("Select framework…")
///     .search_placeholder("Search framework…")
///     .option("next", "Next.js")
///     .option("svelte", "SvelteKit")
///     .value(framework.get(cx))
///     .on_change(move |value, _, cx| framework.set(value.clone(), cx))
/// ```
#[derive(IntoElement)]
pub struct Combobox {
    id: ElementId,
    options: Vec<(SharedString, SharedString)>,
    value: Option<SharedString>,
    placeholder: SharedString,
    search_placeholder: SharedString,
    empty_text: SharedString,
    disabled: bool,
    on_change: Option<EventHandler<Option<SharedString>>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Combobox);

impl Combobox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            value: None,
            placeholder: "Select…".into(),
            search_placeholder: "Search…".into(),
            empty_text: "No results found.".into(),
            disabled: false,
            on_change: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add an option: `value` is what `on_change` receives, `label` is what is shown.
    pub fn option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.options.push((value.into(), label.into()));
        self
    }

    pub fn value(mut self, value: Option<impl Into<SharedString>>) -> Self {
        self.value = value.map(Into::into);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.search_placeholder = placeholder.into();
        self
    }

    /// Text shown when nothing matches the search.
    pub fn empty_text(mut self, empty_text: impl Into<SharedString>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Receives the new value, or `None` when the current one was picked again.
    pub fn on_change(
        mut self,
        handler: impl Fn(&Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Combobox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state = use_open_state(&self.id, None, None, window, cx);
        let is_open = open_state.is_open(cx);
        let trigger_width = open_state.trigger_width(cx);
        let search_placeholder = self.search_placeholder.clone();
        let search = use_input_state(child_id(&self.id, "search"), window, cx, |state| {
            state.with_placeholder(search_placeholder)
        });
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let radius = theme.radius_medium();
        let ring_color = colors.ring;

        let selected_label = self.value.as_ref().and_then(|value| {
            self.options
                .iter()
                .find(|(option_value, _)| option_value == value)
                .map(|(_, label)| label.clone())
        });

        let trigger = div()
            .id("combobox-trigger")
            .relative()
            .flex_dir()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .h(px(36.))
            .w(px(200.))
            .px(px(12.))
            .rounded(radius)
            .border_1()
            .border_color(colors.input)
            .bg(colors.background)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .shadow(extra_small_shadow())
            .when(!self.disabled, |trigger| {
                trigger
                    .tab_index(0)
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(colors.accent))
                    .focus(move |style| {
                        style
                            .border_color(ring_color)
                            .shadow(focus_ring_shadow(ring_color))
                    })
            })
            .when(self.disabled, |trigger| trigger.opacity(0.5))
            .child(match selected_label {
                Some(label) => div().truncate().child(label),
                None => div()
                    .truncate()
                    .text_color(colors.muted_foreground)
                    .child(self.placeholder),
            })
            .child(
                Icon::new(IconName::ChevronsUpDown)
                    .size(px(16.))
                    .color(colors.muted_foreground),
            )
            .child(measure_width(trigger_width.clone()))
            .apply_style_overrides(&self.style_overrides);

        let wrapper =
            trigger_wrapper(self.id.clone(), &open_state, self.disabled, cx).child(trigger);
        if !is_open {
            return wrapper;
        }

        let items = self.options.iter().map(|(value, label)| {
            let is_selected = self.value.as_ref() == Some(value);
            let on_change = self.on_change.clone();
            let next_value = (!is_selected).then(|| value.clone());
            CommandItem::new(label.clone())
                .keywords([value.clone()])
                .checked(is_selected)
                .on_select(move |_, window, cx| {
                    if let Some(handler) = on_change.as_ref() {
                        handler(&next_value, window, cx);
                    }
                })
        });
        let groups = [CommandGroup::without_heading(items.collect())];

        let close_state = open_state.clone();
        let clear_search = search.clone();
        let after_select: Callback = Rc::new(move |window, cx| {
            clear_search.update(cx, |state, cx| state.set_text("", cx));
            close_state.set_open(false, window, cx);
        });

        // Typing goes to the search field, so hand focus from the panel to it.
        open_state.focus_if_requested(window, cx);
        let search_focus = search.read(cx).focus_handle_ref().clone();
        if open_state.focus_handle(cx).is_focused(window) {
            window.defer(cx, move |window, _| window.focus(&search_focus));
        }

        let command = render_command(
            &self.id,
            &search,
            &groups,
            self.empty_text,
            Some(after_select),
            window,
            cx,
        );
        let panel = popover_surface(cx.theme())
            .w(trigger_width.get().max(px(200.)))
            .overflow_hidden()
            .child(command);
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(Side::Bottom, Align::Start, panel, cx))
    }
}
