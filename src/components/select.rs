//! Select and NativeSelect: pick one value from a list opened by a button.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, App, CursorStyle, ElementId, SharedString, StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::{
    focus_ring_outline,
    menu::{close_handler, render_menu_panel, Menu, MenuItem},
    overlay::{dismissable, floating, measure_width, trigger_wrapper, use_open_state, Align, Side},
};
use crate::sx::SxStyled;
use crate::{
    hooks::EventHandler,
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

#[derive(Clone)]
enum SelectEntry {
    Option {
        value: SharedString,
        label: SharedString,
        disabled: bool,
    },
    Label(SharedString),
    Separator,
}

/// shadcn/ui's `<Select>`: a button showing the chosen option that opens a list.
/// Controlled: pass `value`, update it in `on_change`.
///
/// ```ignore
/// Select::new("fruit")
///     .placeholder("Select a fruit")
///     .group_label("Fruits")
///     .option("apple", "Apple")
///     .option("banana", "Banana")
///     .value(fruit.get(cx))
///     .on_change(move |value, _, cx| fruit.set(Some(value.clone()), cx))
/// ```
#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    entries: Vec<SelectEntry>,
    value: Option<SharedString>,
    placeholder: SharedString,
    disabled: bool,
    invalid: bool,
    small: bool,
    native: bool,
    on_change: Option<EventHandler<SharedString>>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Select);

impl Select {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            value: None,
            placeholder: "Select…".into(),
            disabled: false,
            invalid: false,
            small: false,
            native: false,
            on_change: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Add an option: `value` is what `on_change` receives, `label` is what is shown.
    pub fn option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.entries.push(SelectEntry::Option {
            value: value.into(),
            label: label.into(),
            disabled: false,
        });
        self
    }

    /// Add an option that cannot be picked.
    pub fn disabled_option(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.entries.push(SelectEntry::Option {
            value: value.into(),
            label: label.into(),
            disabled: true,
        });
        self
    }

    /// A heading above the options that follow it.
    pub fn group_label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(SelectEntry::Label(label.into()));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(SelectEntry::Separator);
        self
    }

    /// The selected option's value, or `None` to show the placeholder.
    pub fn value(mut self, value: Option<impl Into<SharedString>>) -> Self {
        self.value = value.map(Into::into);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Red border, for validation errors.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// 32px tall instead of 36px.
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// Receives the chosen option's value.
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    fn selected_label(&self) -> Option<SharedString> {
        let value = self.value.as_ref()?;
        self.entries.iter().find_map(|entry| match entry {
            SelectEntry::Option {
                value: option_value,
                label,
                ..
            } if option_value == value => Some(label.clone()),
            _ => None,
        })
    }

    fn build_menu(&self) -> Menu {
        let mut menu = Menu::new();
        for entry in self.entries.iter().cloned() {
            menu = match entry {
                SelectEntry::Option {
                    value,
                    label,
                    disabled,
                } => {
                    let is_selected = self.value.as_ref() == Some(&value);
                    let on_change = self.on_change.clone();
                    let item = MenuItem::new(label).disabled(disabled);
                    let item = item.select_option(is_selected);
                    menu.item(item.on_select(move |_, window, cx| {
                        if let Some(handler) = on_change.as_ref() {
                            handler(&value, window, cx);
                        }
                    }))
                }
                SelectEntry::Label(label) => menu.label(label),
                SelectEntry::Separator => menu.separator(),
            };
        }
        menu
    }
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_state = use_open_state(&self.id, None, None, window, cx);
        let is_open = open_state.is_open(cx);
        let trigger_width = open_state.trigger_width(cx);
        let theme = cx.theme();
        let radius = theme.radius_medium();
        let colors = theme.colors.clone();
        let ring_color = if self.invalid {
            colors.destructive_text
        } else {
            colors.ring
        };
        let selected_label = self.selected_label();
        let menu = self.build_menu();

        let trigger = div()
            .id("select-trigger")
            .flex_dir()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .h(if self.small { px(32.) } else { px(36.) })
            .px(px(12.))
            .rounded(theme.radius_medium())
            .border_1()
            .border_color(if self.invalid {
                colors.destructive_text
            } else if is_open {
                colors.ring
            } else {
                colors.input
            })
            .text_sm()
            .whitespace_nowrap()
            // No shadows, like `Input`: GPUI paints them under the transparent trigger.
            .relative()
            .when(is_open, |trigger| {
                trigger.child(focus_ring_outline(ring_color, radius))
            })
            .when(self.native, |trigger| trigger.w_full())
            .when(!self.disabled, |trigger| {
                trigger
                    .tab_index(0)
                    .cursor(CursorStyle::PointingHand)
                    .focus(move |style| style.border_color(ring_color))
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
                Icon::new(if self.native {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronsUpDown
                })
                .size(px(16.))
                .color(colors.muted_foreground),
            )
            .child(measure_width(trigger_width.clone()));

        let wrapper = trigger_wrapper(self.id.clone(), &open_state, self.disabled, cx)
            .flex_dir()
            .flex_col()
            .when(self.native, |wrapper| wrapper.w_full())
            .child(trigger)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides);
        if !is_open {
            return wrapper;
        }
        let panel = render_menu_panel(
            self.id,
            menu,
            close_handler(&open_state),
            trigger_width.get().max(px(128.)),
            true,
            window,
            cx,
        );
        let panel = dismissable(panel, &open_state, window, cx);
        wrapper.child(floating(Side::Bottom, Align::Start, panel, cx))
    }
}

/// shadcn/ui's `<NativeSelect>`. GPUI has no platform select control, so this is
/// a full-width [`Select`] styled like a native one: full width with a down chevron.
pub struct NativeSelect;

impl NativeSelect {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(id: impl Into<ElementId>) -> Select {
        let mut select = Select::new(id);
        select.native = true;
        select
    }
}
