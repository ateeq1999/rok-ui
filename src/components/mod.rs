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

pub mod accordion;
pub mod alert;
pub mod alert_dialog;
pub mod app_root;
pub mod aspect_ratio;
pub mod attachment;
pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod bubble;
pub mod button;
pub mod button_group;
pub mod calendar;
pub mod card;
pub mod carousel;
pub mod chart;
pub mod checkbox;
pub mod collapsible;
pub mod combobox;
pub mod command;
pub mod data_table;
pub mod date_picker;
pub mod dialog;
pub mod direction;
pub mod empty;
pub mod field;
pub mod hover_card;
pub mod input;
pub mod input_group;
pub mod input_otp;
pub(crate) mod interaction;
pub mod item;
pub mod keyboard_shortcut;
pub mod label;
pub(crate) mod layer;
pub mod marker;
pub mod menu;
pub mod menubar;
pub mod message;
pub mod message_scroller;
pub(crate) mod overlay;
pub mod pagination;
pub mod popover;
pub mod progress;
pub mod questionnaire;
pub mod radio_group;
pub mod resizable;
pub mod scroll_area;
pub mod select;
pub mod separator;
pub mod sheet;
pub mod sidebar;
pub mod skeleton;
pub mod slider;
pub mod spinner;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod toast;
pub mod toggle;
pub mod tooltip;
pub mod typography;

pub use accordion::{Accordion, AccordionItem};
pub use alert::{Alert, AlertVariant};
pub use alert_dialog::AlertDialog;
pub use app_root::AppRoot;
pub use aspect_ratio::AspectRatio;
pub use attachment::{icon_for_file_name, Attachment, AttachmentState};
pub use avatar::Avatar;
pub use badge::{Badge, BadgeVariant};
pub use breadcrumb::Breadcrumb;
pub use bubble::{Bubble, BubbleAlign, BubbleGroupPosition, BubbleReaction, BubbleVariant};
pub use button::{Button, ButtonSize, ButtonVariant, IconPosition};
pub use button_group::{ButtonGroup, ButtonGroupOrientation, ButtonGroupText};
pub use calendar::{
    days_in_month, is_leap_year, month_grid, month_name, Calendar, CalendarDate, DateRange,
};
pub use card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
pub use carousel::{Carousel, CarouselOrientation};
pub use chart::{chart_color, chart_palette, nice_ceiling, Chart, ChartKind, ChartSeries};
pub use checkbox::Checkbox;
pub use collapsible::Collapsible;
pub use combobox::Combobox;
pub use command::{Command, CommandDialog, CommandItem};
pub use data_table::{
    compare_cells, visible_rows, ColumnAlign, DataColumn, DataTable, SortDirection,
};
pub use date_picker::DatePicker;
pub use dialog::Dialog;
pub use direction::{set_text_direction, ActiveDirection, Direction, TextDirection};
pub use empty::Empty;
pub use field::{
    Field, FieldContent, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldLegend,
    FieldOrientation, FieldSeparator, FieldSet, FieldTitle,
};
pub use hover_card::HoverCard;
pub use input::{use_input_state, use_textarea_state, Input, InputEvent, InputState, Textarea};
pub use input_group::InputGroup;
pub use input_otp::{InputOtp, OtpPattern};
pub use item::{Item, ItemGroup, ItemVariant};
pub use keyboard_shortcut::{Kbd, KbdGroup, KeyboardShortcut};
pub use label::Label;
pub use marker::Marker;
pub use menu::{ContextMenu, DropdownMenu, Menu, MenuItem};
pub use menubar::{Menubar, NavigationMenu, NavigationMenuLink};
pub use message::Message;
pub use message_scroller::{MessageScroller, MessageScrollerState};
pub use overlay::{Align, Side};
pub use pagination::{page_slots, PageSlot, Pagination};
pub use popover::Popover;
pub use progress::Progress;
pub use questionnaire::{Answer, Question, Questionnaire, QuestionnaireAnswer};
pub use radio_group::RadioGroup;
pub use resizable::{ResizableDirection, ResizablePanel, ResizablePanelGroup};
pub use scroll_area::{ScrollArea, ScrollAxis};
pub use select::{NativeSelect, Select};
pub use separator::{Separator, SeparatorOrientation};
pub use sheet::{Drawer, Sheet, SheetSide};
pub use sidebar::{Sidebar, SidebarGroup, SidebarItem, SidebarSide, SidebarTrigger};
pub use skeleton::Skeleton;
pub use slider::Slider;
pub use spinner::Spinner;
pub use switch::Switch;
pub use table::{
    Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow,
};
pub use tabs::Tabs;
pub use toast::{dismiss_toast, toast, update_toast, Toast, ToastId, ToastVariant, Toaster};
pub use toggle::{Toggle, ToggleGroup, ToggleVariant};
pub use tooltip::Tooltip;
pub use typography::{Blockquote, InlineCode, Large, Lead, List, Muted, Small, H1, H2, H3, H4, P};

use gpui::{point, px, BoxShadow, Hsla};

/// shadcn/ui's focus ring (`ring-[3px] ring-ring/50`), drawn as a spread shadow.
/// GPUI skips shadows with zero blur, so a 1px blur plus 2px spread gives the 3px ring.
pub(crate) fn focus_ring_shadow(ring_color: Hsla) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: ring_color.opacity(0.5),
        offset: point(px(0.), px(0.)),
        blur_radius: px(1.),
        spread_radius: px(2.),
    }]
}

/// shadcn/ui's `shadow-xs`, used on outlined controls.
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
