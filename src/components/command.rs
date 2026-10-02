//! Command: a searchable list of actions (cmdk), plus [`CommandDialog`] for a
//! ⌘K-style palette. [`super::Combobox`] is built on the same list.

use std::rc::Rc;

use gpui::{
    div, point, prelude::*, px, AnyElement, App, Corner, Div, ElementId, Entity, SharedString,
    StyleRefinement, Window,
};

use super::layer::layer_at;
use super::{
    input::{use_input_state, Input, InputState, Submit},
    interaction::Callback,
    overlay::child_id,
};
use crate::sx::SxStyled;
use crate::{
    hooks::{use_keyed_state, EventHandler},
    icon::{Icon, IconName},
    styles,
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

styles! {
    COMMAND = {
        root: { display: flex, direction: column },
        // The search field sits flush on top, divided from the list by its bottom border.
        search: { height: 10, border: 0, radius: none, border_bottom: 1, border_color: border },
        list: {
            display: flex,
            direction: column,
            max_height: 75,
            padding_x: 1,
            padding_bottom: 1,
        },
        empty: { padding_y: 6, text_align: center, text: sm, color: muted_foreground },
        separator: { height: 0.25, margin_x: -1, background: border },
        group: { display: flex, direction: column, padding_y: 1 },
        heading: {
            padding_x: 2,
            padding_y: 1.5,
            text: xs,
            font: medium,
            color: muted_foreground,
        },
        item: {
            display: flex,
            align: center,
            gap: 2,
            padding_x: 2,
            padding_y: 1.5,
            radius: sm,
            text: sm,
        },
        highlighted: { background: accent, color: accent_foreground },
        interactive: { cursor: pointer },
        inert: { opacity: 0.5 },
        item_label: { flex: 1 },
        shortcut: { text: xs, color: muted_foreground },
        check_slot: { size: 4 },
        frame: {
            width: full,
            overflow: hidden,
            radius: md,
            border: 1,
            border_color: border,
            background: popover,
            color: popover_foreground,
        },
        // Layers lay out apart from the window root, so text alignment is set here.
        dialog_scrim: {
            display: flex,
            direction: column,
            align: center,
            padding_x: 4,
            font_family: sans,
            text_align: start,
        },
        dialog_panel: { width: full, max_width: 128, shadow: lg },
    }
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
    let muted_foreground = cx.theme().colors.muted_foreground;

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
            sections.push(div().sx(&COMMAND.separator).into_any_element());
        }
        let mut section = div().sx(&COMMAND.group);
        if let Some(heading) = group.heading.clone() {
            section = section.child(div().sx(&COMMAND.heading).child(heading));
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
                    .sx((
                        &COMMAND.item,
                        is_highlighted.then_some(&COMMAND.highlighted),
                        if item.disabled {
                            &COMMAND.inert
                        } else {
                            &COMMAND.interactive
                        },
                    ))
                    .when(!item.disabled, |row| {
                        row.on_hover(move |hovered, _, cx| {
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
                        row.child(Icon::new(icon).size(px(16.)).color(muted_foreground))
                    })
                    .child(div().sx(&COMMAND.item_label).child(item.label.clone()))
                    .when_some(item.shortcut.clone(), |row, shortcut| {
                        row.child(div().sx(&COMMAND.shortcut).child(shortcut))
                    })
                    .when_some(item.checked, |row, checked| {
                        row.child(div().sx(&COMMAND.check_slot).when(checked, |slot| {
                            slot.child(Icon::new(IconName::Check).size(px(16.)))
                        }))
                    }),
            );
            visible_items.push(item);
        }
        sections.push(section.into_any_element());
    }

    let list = div()
        .sx(&COMMAND.list)
        .id(child_id(id, "list"))
        .overflow_y_scroll()
        .when(visible_items.is_empty(), |list| {
            list.child(div().sx(&COMMAND.empty).child(empty_text))
        })
        .children(sections);

    let visible_items = Rc::new(visible_items);
    let key_highlight = highlight.clone();
    let key_items = visible_items.clone();
    let submit_after_select = after_select.clone();
    div()
        .sx(&COMMAND.root)
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
                .without_focus_ring()
                .sx(&COMMAND.search),
        )
        .child(list)
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
    sx: crate::sx::Sx,
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
            sx: crate::sx::Sx::new(),
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
        render_command(
            &self.id,
            &search,
            &self.groups,
            self.empty_text,
            after_select,
            window,
            cx,
        )
        .sx((&COMMAND.frame, &self.sx))
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
        let (overlay, font_size) = (theme.colors.overlay, theme.font_size);
        let viewport_size = window.viewport_size();
        let escape_close = close.clone();
        let backdrop_close = close.clone();
        let panel = div()
            .track_focus(&focus_container)
            .sx(&COMMAND.dialog_panel)
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
            .sx(&COMMAND.dialog_scrim)
            .pt(viewport_size.height * 0.2)
            .bg(overlay.opacity(progress))
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
