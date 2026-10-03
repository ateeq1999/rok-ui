//! Combobox: a [`super::Select`] with a search field, built from a popover and a
//! [`super::Command`] list.

use std::rc::Rc;

use gpui::{div, prelude::*, px, App, ElementId, SharedString, StyleRefinement, Window};

use super::{
    command::{render_command, CommandGroup, CommandItem},
    input::use_input_state,
    interaction::Callback,
    overlay::{
        child_id, dismissable, floating, measure_width, popover_surface, trigger_wrapper,
        use_open_state, Align, Side,
    },
};
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles,
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// Controlled like [`super::Select`]: pass `value`, update it in `on_change`.
/// Picking the current value again clears it, like shadcn/ui's example.
///
/// ```no_run
/// # use rok_ui::prelude::*;
/// # fn example(window: &mut Window, cx: &mut App) {
/// let framework = use_state(window, cx, || None::<SharedString>);
/// let picker = Combobox::new("framework")
///     .placeholder("Select framework...")
///     .search_placeholder("Search framework...")
///     .option("next", "Next.js")
///     .option("svelte", "SvelteKit")
///     .value(framework.get(cx))
///     .on_change(move |value, _, cx| framework.set(value.clone(), cx));
/// # }
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
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Combobox);

impl Combobox {
    /// Create the component. `id` must be unique among its siblings; it keys the component's state.
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
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add an option: `value` is what `on_change` receives, `label` is what is shown.
    #[must_use]
    pub fn option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.options.push((value.into(), label.into()));
        self
    }

    /// The selected value (controlled).
    #[must_use]
    pub fn value(mut self, value: Option<impl Into<SharedString>>) -> Self {
        self.value = value.map(Into::into);
        self
    }

    /// Text shown while nothing is entered or selected.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Placeholder of the search field in the popover.
    #[must_use]
    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.search_placeholder = placeholder.into();
        self
    }

    /// Text shown when nothing matches the search.
    #[must_use]
    pub fn empty_text(mut self, empty_text: impl Into<SharedString>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    /// Disable it: it ignores input and renders muted.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Receives the new value, or `None` when the current one was picked again.
    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

styles! {
    COMBOBOX = {
        trigger: {
            position: relative,
            display: flex,
            align: center,
            justify: between,
            gap: 2,
            height: 9,
            width: 50,
            padding_x: 3,
            radius: md,
            border: 1,
            border_color: input,
            background: background,
            text: sm,
            font: medium,
            shadow: xs,
        },
        interactive: {
            cursor: pointer,
            hover: { background: accent },
            focus: { border_color: ring, shadow: ring },
        },
        inert: { opacity: 0.5 },
        value: { truncate: true },
        placeholder: { truncate: true, color: muted_foreground },
        panel: { overflow: hidden },
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
        let muted_foreground = cx.theme().colors.muted_foreground;

        let selected_label = self.value.as_ref().and_then(|value| {
            self.options
                .iter()
                .find(|(option_value, _)| option_value == value)
                .map(|(_, label)| label.clone())
        });

        let trigger = div()
            .id("combobox-trigger")
            // The caller's `sx` is merged into the same call: GPUI allows a single
            // hover / focus style per element.
            .sx((
                &COMBOBOX.trigger,
                if self.disabled {
                    &COMBOBOX.inert
                } else {
                    &COMBOBOX.interactive
                },
                &self.sx,
            ))
            .when(!self.disabled, |trigger| trigger.tab_index(0))
            .child(match selected_label {
                Some(label) => div()
                    .sx(&COMBOBOX.value)
                    .child(crate::components::bidi_text::text(label)),
                None => div()
                    .sx(&COMBOBOX.placeholder)
                    .child(crate::components::bidi_text::text(self.placeholder)),
            })
            .child(
                Icon::new(IconName::ChevronsUpDown)
                    .size(px(16.))
                    .color(muted_foreground),
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
                .on_select(move |(), window, cx| {
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
        let panel = popover_surface()
            .sx(&COMBOBOX.panel)
            .w(trigger_width.get().max(px(200.)))
            .child(command);
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(Side::Bottom, Align::Start, panel, cx))
    }
}
