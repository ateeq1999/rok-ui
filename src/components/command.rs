//! Command: a searchable list of actions (cmdk), plus [`CommandDialog`] for a
//! ⌘K-style palette. [`super::Combobox`] is built on the same list.

use std::rc::Rc;

use gpui::{
    div, point, prelude::*, px, AnyElement, App, Corner, CursorStyle, Div, ElementId, Entity,
    FontWeight, SharedString, StyleRefinement, Window,
};

use super::direction::DirectionalStyled;
use super::layer::layer_at;
use super::{
    input::{use_input_state, Input, InputState, Submit},
    interaction::Callback,
    overlay::child_id,
};
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

/// One selectable row in a [`Command`] list.
#[derive(Clone)]
pub struct CommandItem {
    label: SharedString,
    keywords: Vec<SharedString>,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    disabled: bool,
    checked: Option<bool>,
    on_select: Option<EventHandler<()>>,
}

impl CommandItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            keywords: Vec::new(),
            icon: None,
            shortcut: None,
            disabled: false,
            checked: None,
            on_select: None,
        }
    }

    /// Extra words the search matches besides the label.
    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show a check on the right (used by [`super::Combobox`] for the current value).
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        std::iter::once(&self.label)
            .chain(self.keywords.iter())
            .any(|text| fuzzy_match(&text.to_lowercase(), &query))
    }
}

/// Every query character appears in `text`, in order (cmdk-style loose matching).
fn fuzzy_match(text: &str, query: &str) -> bool {
    let mut text_characters = text.chars();
    query
        .chars()
        .filter(|character| !character.is_whitespace())
        .all(|query_character| text_characters.any(|character| character == query_character))
}

#[derive(Clone)]
pub(crate) struct CommandGroup {
    heading: Option<SharedString>,
    items: Vec<CommandItem>,
}

impl CommandGroup {
    #[cfg_attr(not(feature = "full"), allow(dead_code))]
    pub(crate) fn without_heading(items: Vec<CommandItem>) -> Self {
        Self {
            heading: None,
            items,
        }
    }
}

/// Highlighted row, reset whenever the query changes.
struct CommandHighlight {
    query: SharedString,
    index: usize,
}

/// Render the search field and the filtered list. `after_select` runs after an
/// item's own handler (Combobox uses it to close its popover).
pub(crate) fn render_command(
    id: &ElementId,
    search: &Entity<InputState>,
    groups: &[CommandGroup],
    empty_text: SharedString,
    after_select: Option<Callback>,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let query = search.read(cx).text().clone();
    let highlight = use_keyed_state(child_id(id, "highlight"), window, cx, || CommandHighlight {
        query: SharedString::default(),
        index: 0,
    });
    if highlight.read(cx).query != query {
        highlight.update(cx, |highlight| {
            highlight.query = query.clone();
            highlight.index = 0;
        });
    }
    let highlighted_index = highlight.read(cx).index;
    let theme = cx.theme();
    let colors = theme.colors.clone();
    let item_radius = theme.radius_small();

    // Flatten the visible items so keyboard navigation can step across groups.
    let mut visible_items: Vec<CommandItem> = Vec::new();
    let mut sections: Vec<AnyElement> = Vec::new();
    for group in groups {
        let matching: Vec<CommandItem> = group
            .items
            .iter()
            .filter(|item| item.matches(&query))
            .cloned()
            .collect();
        if matching.is_empty() {
            continue;
        }
        if !sections.is_empty() {
            sections.push(
                div()
                    .h(px(1.))
                    .mx(px(-4.))
                    .bg(colors.border)
                    .into_any_element(),
            );
        }
        let mut section = div().flex_dir().flex_col().py(px(4.));
        if let Some(heading) = group.heading.clone() {
            section = section.child(
                div()
                    .px(px(8.))
                    .py(px(6.))
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.muted_foreground)
                    .child(heading),
            );
        }
        for item in matching {
            let flat_index = visible_items.len();
            let is_highlighted = flat_index == highlighted_index && !item.disabled;
            let hover_highlight = highlight.clone();
            let after_select = after_select.clone();
            let selected_item = item.clone();
            section = section.child(
                div()
                    .id(("command-item", flat_index))
                    .flex_dir()
                    .items_center()
                    .gap(px(8.))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(item_radius)
                    .text_sm()
                    .when(is_highlighted, |row| {
                        row.bg(colors.accent).text_color(colors.accent_foreground)
                    })
                    .when(item.disabled, |row| row.opacity(0.5))
                    .when(!item.disabled, |row| {
                        row.cursor(CursorStyle::PointingHand)
                            .on_hover(move |hovered, _, cx| {
                                if *hovered {
                                    hover_highlight
                                        .update(cx, |highlight| highlight.index = flat_index);
                                }
                            })
                            .on_click(move |_, window, cx| {
                                select_item(&selected_item, after_select.as_ref(), window, cx)
                            })
                    })
                    .when_some(item.icon, |row, icon| {
                        row.child(Icon::new(icon).size(px(16.)).color(colors.muted_foreground))
                    })
                    .child(div().flex_1().child(item.label.clone()))
                    .when_some(item.shortcut.clone(), |row, shortcut| {
                        row.child(
                            div()
                                .text_xs()
                                .text_color(colors.muted_foreground)
                                .child(shortcut),
                        )
                    })
                    .when_some(item.checked, |row, checked| {
                        row.child(div().size(px(16.)).when(checked, |slot| {
                            slot.child(Icon::new(IconName::Check).size(px(16.)))
                        }))
                    }),
            );
            visible_items.push(item);
        }
        sections.push(section.into_any_element());
    }

    let list = div()
        .id(child_id(id, "list"))
        .flex_dir()
        .flex_col()
        .max_h(px(300.))
        .overflow_y_scroll()
        .px(px(4.))
        .when(visible_items.is_empty(), |list| {
            list.child(
                div()
                    .py(px(24.))
                    .text_center()
                    .text_sm()
                    .text_color(colors.muted_foreground)
                    .child(empty_text),
            )
        })
        .children(sections);

    let visible_items = Rc::new(visible_items);
    let key_highlight = highlight.clone();
    let key_items = visible_items.clone();
    let submit_after_select = after_select.clone();
    div()
        .flex_dir()
        .flex_col()
        .on_key_down(move |event, _, cx| {
            let count = key_items.len();
            if count == 0 {
                return;
            }
            let step = match event.keystroke.key.as_str() {
                "down" => 1,
                "up" => count - 1,
                _ => return,
            };
            cx.stop_propagation();
            key_highlight.update(cx, |highlight| {
                highlight.index = (highlight.index + step) % count
            });
        })
        // Enter is the input's Submit action; capture it before the input does.
        .capture_action(move |_: &Submit, window, cx| {
            let index = highlight.read(cx).index;
            if let Some(item) = visible_items.get(index).filter(|item| !item.disabled) {
                cx.stop_propagation();
                select_item(item, submit_after_select.as_ref(), window, cx);
            }
        })
        .child(
            Input::new(search)
                .leading_icon(IconName::Search)
                .h(px(40.))
                .border_0()
                .rounded_none()
                .without_focus_ring()
                .border_b_1()
                .border_color(colors.border),
        )
        .child(list.pb(px(4.)))
}

fn select_item(
    item: &CommandItem,
    after_select: Option<&Callback>,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(handler) = item.on_select.as_ref() {
        handler(&(), window, cx);
    }
    if let Some(after_select) = after_select {
        after_select(window, cx);
    }
}

/// shadcn/ui's `<Command>`: a search field over grouped items, filtered as you
/// type. Up / Down move the highlight and Enter runs the highlighted item.
///
/// ```ignore
/// Command::new("command")
///     .placeholder("Type a command or search…")
///     .group("Suggestions", [
///         CommandItem::new("Calendar").icon(IconName::Calendar),
///         CommandItem::new("Search emoji").icon(IconName::Smile),
///     ])
///     .group("Settings", [CommandItem::new("Profile").shortcut("⌘P")])
/// ```
#[derive(IntoElement)]
pub struct Command {
    id: ElementId,
    placeholder: SharedString,
    empty_text: SharedString,
    groups: Vec<CommandGroup>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Command);

impl Command {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            placeholder: "Type a command or search…".into(),
            empty_text: "No results found.".into(),
            groups: Vec::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Text shown when nothing matches the search.
    pub fn empty_text(mut self, empty_text: impl Into<SharedString>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    /// A headed group of items.
    pub fn group(
        mut self,
        heading: impl Into<SharedString>,
        items: impl IntoIterator<Item = CommandItem>,
    ) -> Self {
        self.groups.push(CommandGroup {
            heading: Some(heading.into()),
            items: items.into_iter().collect(),
        });
        self
    }

    /// Items without a group heading.
    pub fn items(mut self, items: impl IntoIterator<Item = CommandItem>) -> Self {
        self.groups.push(CommandGroup {
            heading: None,
            items: items.into_iter().collect(),
        });
        self
    }

    fn render_body(self, after_select: Option<Callback>, window: &mut Window, cx: &mut App) -> Div {
        let placeholder = self.placeholder.clone();
        let search = use_input_state(child_id(&self.id, "search"), window, cx, |state| {
            state.with_placeholder(placeholder)
        });
        let theme = cx.theme();
        let (radius, popover, popover_foreground, border) = (
            theme.radius_medium(),
            theme.colors.popover,
            theme.colors.popover_foreground,
            theme.colors.border,
        );
        render_command(
            &self.id,
            &search,
            &self.groups,
            self.empty_text,
            after_select,
            window,
            cx,
        )
        .w_full()
        .overflow_hidden()
        .rounded(radius)
        .border_1()
        .border_color(border)
        .bg(popover)
        .text_color(popover_foreground)
        .apply_style_overrides(&self.style_overrides)
    }
}

impl RenderOnce for Command {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_body(None, window, cx)
    }
}

/// A [`Command`] in a modal, the ⌘K palette. Controlled like [`super::Dialog`];
/// it closes on Escape, on a backdrop click and after an item runs.
///
/// ```ignore
/// CommandDialog::new("palette", Command::new("palette-command").group(..))
///     .open(palette_open)
///     .on_close(move |_, _, cx| set_palette_open(false, cx))
/// ```
#[derive(IntoElement)]
pub struct CommandDialog {
    id: ElementId,
    command: Command,
    open: bool,
    on_close: Option<EventHandler<()>>,
}

impl CommandDialog {
    pub fn new(id: impl Into<ElementId>, command: Command) -> Self {
        Self {
            id: id.into(),
            command,
            open: false,
            on_close: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CommandDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let presence =
            crate::components::interaction::modal_presence(&self.id, self.open, window, cx);
        if !presence.is_mounted() {
            return div().into_any_element();
        }
        let progress = presence.progress();
        let close: Callback = {
            let on_close = self.on_close.clone();
            Rc::new(move |window, cx| {
                if let Some(handler) = on_close.as_ref() {
                    handler(&(), window, cx);
                }
            })
        };

        // Focus the search field when the palette opens.
        let search_focus = window
            .use_keyed_state(child_id(&self.command.id, "search"), cx, |_, cx| {
                InputState::new(cx).with_placeholder(self.command.placeholder.clone())
            })
            .read(cx)
            .focus_handle_ref()
            .clone();
        // Only when it opens; see `render_modal` for why not on every frame.
        let (focus_container, focus_pending) = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| {
                (
                    cx.focus_handle(),
                    std::rc::Rc::new(std::cell::Cell::new(true)),
                )
            })
            .read(cx)
            .clone();
        if focus_pending.replace(false) {
            window.defer(cx, move |window, _| window.focus(&search_focus));
        }

        let theme = cx.theme();
        let (overlay, font_family, font_size) = (
            theme.colors.overlay,
            theme.font_family.clone(),
            theme.font_size,
        );
        let viewport_size = window.viewport_size();
        let escape_close = close.clone();
        let backdrop_close = close.clone();
        let panel = div()
            .track_focus(&focus_container)
            .w_full()
            .max_w(px(512.))
            .shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    escape_close(window, cx);
                }
            })
            .child(self.command.render_body(Some(close), window, cx));

        let scrim = div()
            .id(self.id)
            .occlude()
            .w(viewport_size.width)
            .h(viewport_size.height)
            .flex_dir()
            .flex_col()
            .items_center()
            .pt(viewport_size.height * 0.2)
            .px(px(16.))
            .bg(overlay.opacity(progress))
            .font_family(font_family)
            .text_size(font_size)
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                backdrop_close(window, cx)
            })
            .child(
                panel
                    .relative()
                    .top(px(8. * (1. - progress)))
                    .opacity(progress),
            );
        layer_at(point(px(0.), px(0.)), Corner::TopLeft, px(0.), scrim, 1, cx)
    }
}
