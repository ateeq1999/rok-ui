//! Every rok-ui component. Each one follows the same conventions:
//!
//! - Built with a constructor, configured with builder methods (`Button::new("save").outline()`).
//! - Controlled, like React: the component never owns its value. You pass `checked`,
//!   `selected_index` or `value` in and get changes back through `on_change`.
//!   Floating and disclosure components (popovers, menus, accordions) keep their
//!   open state per element id, like Radix, unless you pass `open` yourself.
//! - Implements GPUI's `Styled` where it makes sense, so `.w_full()` and friends
//!   override its defaults the way `className` does in shadcn/ui.
//! - Reads colors, radius and fonts from the active [`crate::theme::Theme`].

#[cfg(feature = "accordion")]
pub mod accordion;
#[cfg(feature = "alert")]
pub mod alert;
#[cfg(feature = "alert-dialog")]
pub mod alert_dialog;
pub mod app_root;
#[cfg(feature = "aspect-ratio")]
pub mod aspect_ratio;
#[cfg(feature = "attachment")]
pub mod attachment;
#[cfg(feature = "avatar")]
pub mod avatar;
#[cfg(feature = "badge")]
pub mod badge;
#[cfg(feature = "breadcrumb")]
pub mod breadcrumb;
#[cfg(feature = "bubble")]
pub mod bubble;
#[cfg(feature = "button")]
pub mod button;
#[cfg(feature = "button-group")]
pub mod button_group;
#[cfg(feature = "calendar")]
pub mod calendar;
#[cfg(feature = "card")]
pub mod card;
#[cfg(feature = "carousel")]
pub mod carousel;
#[cfg(feature = "chart")]
pub mod chart;
#[cfg(feature = "checkbox")]
pub mod checkbox;
#[cfg(feature = "collapsible")]
pub mod collapsible;
#[cfg(feature = "combobox")]
pub mod combobox;
#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "data-table")]
pub mod data_table;
#[cfg(feature = "date-picker")]
pub mod date_picker;
#[cfg(feature = "dialog")]
pub mod dialog;
pub mod direction;
#[cfg(feature = "empty")]
pub mod empty;
#[cfg(feature = "field")]
pub mod field;
#[cfg(feature = "hover-card")]
pub mod hover_card;
#[cfg(feature = "input")]
pub mod input;
#[cfg(feature = "input-group")]
pub mod input_group;
#[cfg(feature = "input-otp")]
pub mod input_otp;
pub(crate) mod interaction;
#[cfg(feature = "item")]
pub mod item;
#[cfg(feature = "keyboard-shortcut")]
pub mod keyboard_shortcut;
#[cfg(feature = "label")]
pub mod label;
pub(crate) mod layer;
#[cfg(feature = "marker")]
pub mod marker;
#[cfg(feature = "menu")]
pub mod menu;
#[cfg(feature = "menubar")]
pub mod menubar;
#[cfg(feature = "message")]
pub mod message;
#[cfg(feature = "message-scroller")]
pub mod message_scroller;
pub(crate) mod overlay;
#[cfg(feature = "pagination")]
pub mod pagination;
#[cfg(feature = "popover")]
pub mod popover;
#[cfg(feature = "progress")]
pub mod progress;
#[cfg(feature = "questionnaire")]
pub mod questionnaire;
#[cfg(feature = "radio-group")]
pub mod radio_group;
#[cfg(feature = "resizable")]
pub mod resizable;
#[cfg(feature = "scroll-area")]
pub mod scroll_area;
#[cfg(feature = "select")]
pub mod select;
#[cfg(feature = "separator")]
pub mod separator;
#[cfg(feature = "sheet")]
pub mod sheet;
#[cfg(feature = "sidebar")]
pub mod sidebar;
#[cfg(feature = "skeleton")]
pub mod skeleton;
#[cfg(feature = "slider")]
pub mod slider;
#[cfg(feature = "spinner")]
pub mod spinner;
#[cfg(feature = "switch")]
pub mod switch;
#[cfg(feature = "table")]
pub mod table;
#[cfg(feature = "tabs")]
pub mod tabs;
#[cfg(feature = "toast")]
pub mod toast;
#[cfg(feature = "toggle")]
pub mod toggle;
#[cfg(feature = "tooltip")]
pub mod tooltip;
#[cfg(feature = "typography")]
pub mod typography;

#[cfg(feature = "accordion")]
pub use accordion::{Accordion, AccordionItem};
#[cfg(feature = "alert")]
pub use alert::{Alert, AlertVariant};
#[cfg(feature = "alert-dialog")]
pub use alert_dialog::AlertDialog;
pub use app_root::AppRoot;
#[cfg(feature = "aspect-ratio")]
pub use aspect_ratio::AspectRatio;
#[cfg(feature = "attachment")]
pub use attachment::{icon_for_file_name, Attachment, AttachmentState};
#[cfg(feature = "avatar")]
pub use avatar::Avatar;
#[cfg(feature = "badge")]
pub use badge::{Badge, BadgeVariant};
#[cfg(feature = "breadcrumb")]
pub use breadcrumb::Breadcrumb;
#[cfg(feature = "bubble")]
pub use bubble::{Bubble, BubbleAlign, BubbleGroupPosition, BubbleReaction, BubbleVariant};
#[cfg(feature = "button")]
pub use button::{Button, ButtonSize, ButtonVariant, IconPosition};
#[cfg(feature = "button-group")]
pub use button_group::{ButtonGroup, ButtonGroupOrientation, ButtonGroupText};
#[cfg(feature = "calendar")]
pub use calendar::{
    days_in_month, is_leap_year, month_grid, month_name, Calendar, CalendarDate, DateRange,
};
#[cfg(feature = "card")]
pub use card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
#[cfg(feature = "carousel")]
pub use carousel::{Carousel, CarouselOrientation};
#[cfg(feature = "chart")]
pub use chart::{chart_color, chart_palette, nice_ceiling, Chart, ChartKind, ChartSeries};
#[cfg(feature = "checkbox")]
pub use checkbox::Checkbox;
#[cfg(feature = "collapsible")]
pub use collapsible::Collapsible;
#[cfg(feature = "combobox")]
pub use combobox::Combobox;
#[cfg(feature = "command")]
pub use command::{Command, CommandDialog, CommandItem};
#[cfg(feature = "data-table")]
pub use data_table::{
    compare_cells, visible_rows, ColumnAlign, DataColumn, DataTable, SortDirection,
};
#[cfg(feature = "date-picker")]
pub use date_picker::DatePicker;
#[cfg(feature = "dialog")]
pub use dialog::Dialog;
pub use direction::{
    current_direction, is_rtl, set_text_direction, with_direction, ActiveDirection, Direction,
    DirectionalStyled, TextDirection,
};
#[cfg(feature = "empty")]
pub use empty::Empty;
#[cfg(feature = "field")]
pub use field::{
    Field, FieldContent, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldLegend,
    FieldOrientation, FieldSeparator, FieldSet, FieldTitle,
};
#[cfg(feature = "hover-card")]
pub use hover_card::HoverCard;
#[cfg(feature = "input")]
pub use input::{use_input_state, use_textarea_state, Input, InputEvent, InputState, Textarea};
#[cfg(feature = "input-group")]
pub use input_group::InputGroup;
#[cfg(feature = "input-otp")]
pub use input_otp::{InputOtp, OtpPattern};
#[cfg(feature = "item")]
pub use item::{Item, ItemGroup, ItemVariant};
#[cfg(feature = "keyboard-shortcut")]
pub use keyboard_shortcut::{Kbd, KbdGroup, KeyboardShortcut};
#[cfg(feature = "label")]
pub use label::Label;
#[cfg(feature = "marker")]
pub use marker::Marker;
#[cfg(feature = "menu")]
pub use menu::{ContextMenu, DropdownMenu, Menu, MenuItem};
#[cfg(feature = "menubar")]
pub use menubar::{Menubar, NavigationMenu, NavigationMenuLink};
#[cfg(feature = "message")]
pub use message::Message;
#[cfg(feature = "message-scroller")]
pub use message_scroller::{MessageScroller, MessageScrollerState};
pub use overlay::{Align, Side};
#[cfg(feature = "pagination")]
pub use pagination::{page_slots, PageSlot, Pagination};
#[cfg(feature = "popover")]
pub use popover::Popover;
#[cfg(feature = "progress")]
pub use progress::Progress;
#[cfg(feature = "questionnaire")]
pub use questionnaire::{Answer, Question, Questionnaire, QuestionnaireAnswer};
#[cfg(feature = "radio-group")]
pub use radio_group::RadioGroup;
#[cfg(feature = "resizable")]
pub use resizable::{ResizableDirection, ResizablePanel, ResizablePanelGroup};
#[cfg(feature = "scroll-area")]
pub use scroll_area::{ScrollArea, ScrollAxis};
#[cfg(feature = "select")]
pub use select::{NativeSelect, Select};
#[cfg(feature = "separator")]
pub use separator::{Separator, SeparatorOrientation};
#[cfg(feature = "sheet")]
pub use sheet::{Drawer, Sheet, SheetSide};
#[cfg(feature = "sidebar")]
pub use sidebar::{Sidebar, SidebarGroup, SidebarItem, SidebarSide, SidebarTrigger};
#[cfg(feature = "skeleton")]
pub use skeleton::Skeleton;
#[cfg(feature = "slider")]
pub use slider::Slider;
#[cfg(feature = "spinner")]
pub use spinner::Spinner;
#[cfg(feature = "switch")]
pub use switch::Switch;
#[cfg(feature = "table")]
pub use table::{
    Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow,
};
#[cfg(feature = "tabs")]
pub use tabs::Tabs;
#[cfg(feature = "toast")]
pub use toast::{dismiss_toast, toast, update_toast, Toast, ToastId, ToastVariant, Toaster};
#[cfg(feature = "toggle")]
pub use toggle::{Toggle, ToggleGroup, ToggleVariant};
#[cfg(feature = "tooltip")]
pub use tooltip::Tooltip;
#[cfg(feature = "typography")]
pub use typography::{Blockquote, InlineCode, Large, Lead, List, Muted, Small, H1, H2, H3, H4, P};

use gpui::{point, px, BoxShadow, Hsla};

/// shadcn/ui's focus ring (`ring-[3px] ring-ring/50`), drawn as a spread shadow.
/// GPUI skips shadows with zero blur, so a 1px blur plus 2px spread gives the 3px ring.
#[cfg_attr(not(feature = "full"), allow(dead_code))]
pub(crate) fn focus_ring_shadow(ring_color: Hsla) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: ring_color.opacity(0.5),
        offset: point(px(0.), px(0.)),
        blur_radius: px(1.),
        spread_radius: px(2.),
    }]
}

/// shadcn/ui's `shadow-xs`, used on outlined controls.
#[cfg_attr(not(feature = "full"), allow(dead_code))]
pub(crate) fn extra_small_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: gpui::black().opacity(0.05),
        offset: point(px(0.), px(1.)),
        blur_radius: px(2.),
        spread_radius: px(0.),
    }]
}

/// The focus ring for transparent controls (inputs, select triggers). As a
/// shadow it would show through them, because GPUI paints shadows under the
/// element, so it is drawn as a 3px outline just outside the 1px border.
/// Add it as a child of a `relative()` control whose corner radius is `radius`.
#[cfg_attr(not(feature = "full"), allow(dead_code))]
pub(crate) fn focus_ring_outline(ring_color: Hsla, radius: gpui::Pixels) -> gpui::Div {
    use gpui::Styled;
    let width = px(3.);
    let offset = -(width + px(1.));
    let mut ring = gpui::div()
        .absolute()
        .top(offset)
        .left(offset)
        .right(offset)
        .bottom(offset)
        .rounded(radius + width + px(1.))
        .border_color(ring_color.opacity(0.5));
    let edge = Some(gpui::AbsoluteLength::from(width));
    ring.style().border_widths = gpui::EdgesRefinement {
        top: edge,
        right: edge,
        bottom: edge,
        left: edge,
    };
    ring
}
