//! The overlays page: dialogs, sheets, popovers, menus and toasts.

use std::time::Duration;

use rok_ui::prelude::*;

use super::{column, demo, row};

/// Popovers, menus, modals and toasts.
#[component]
pub fn OverlaysPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let width = use_input_state("overlay-width", window, cx, |state| state.with_text("100%"));
    let show_status_bar = use_state(window, cx, || true);
    let panel_position = use_state(window, cx, || SharedString::from("bottom"));
    let alert_open = use_state(window, cx, || false);
    let sheet_open = use_state(window, cx, || false);
    let sheet_side = use_state(window, cx, || SheetSide::Right);
    let drawer_open = use_state(window, cx, || false);
    let palette_open = use_state(window, cx, || false);
    let goal = use_state(window, cx, || 350_i32);

    let account_menu = {
        let show_status_bar = show_status_bar.clone();
        let panel_position = panel_position.clone();
        let position_item = |value: &'static str, label: &'static str| {
            let panel_position = panel_position.clone();
            MenuItem::new(label)
                .radio(panel_position.get(cx).as_ref() == value)
                .on_select(move |(), _, cx| panel_position.set(value.into(), cx))
        };
        Menu::new()
            .label("My Account")
            .separator()
            .item(
                MenuItem::new("Profile")
                    .icon(IconName::User)
                    .shortcut("Shift Ctrl P"),
            )
            .item(
                MenuItem::new("Settings")
                    .icon(IconName::Settings)
                    .shortcut("Ctrl S"),
            )
            .item(
                MenuItem::new("Invite users").icon(IconName::Mail).submenu(
                    Menu::new()
                        .item(MenuItem::new("Email"))
                        .item(MenuItem::new("Message"))
                        .separator()
                        .item(MenuItem::new("More…")),
                ),
            )
            .separator()
            .label("Appearance")
            .item(
                MenuItem::new("Status bar")
                    .checked(show_status_bar.get(cx))
                    .on_select(move |(), _, cx| {
                        show_status_bar.update(cx, |shown| *shown = !*shown);
                    }),
            )
            .item(position_item("top", "Panel at top"))
            .item(position_item("bottom", "Panel at bottom"))
            .separator()
            .item(MenuItem::new("API").disabled(true))
            .item(
                MenuItem::new("Log out")
                    .icon(IconName::LogOut)
                    .destructive()
                    .on_select(|(), _, cx| {
                        toast(cx, Toast::info("Logged out"));
                    }),
            )
    };

    let toasts = row()
        .child(
            Button::new("toast-default")
                .outline()
                .label("Default")
                .on_click(|_, _, cx| {
                    toast(
                        cx,
                        Toast::new("Event has been created")
                            .description("Sunday, December 03, 2023 at 9:00 AM")
                            .action("Undo", |_, cx| {
                                toast(cx, Toast::info("Undone"));
                            }),
                    );
                }),
        )
        .child(
            Button::new("toast-success")
                .outline()
                .label("Success")
                .on_click(|_, _, cx| {
                    toast(cx, Toast::success("Profile saved"));
                }),
        )
        .child(
            Button::new("toast-error")
                .outline()
                .label("Error")
                .on_click(|_, _, cx| {
                    toast(
                        cx,
                        Toast::error("Upload failed").description("The file is larger than 10 MB."),
                    );
                }),
        )
        .child(
            Button::new("toast-loading")
                .outline()
                .label("Loading → done")
                .on_click(|_, _, cx| {
                    let id = toast(cx, Toast::loading("Deploying…"));
                    cx.spawn(async move |cx| {
                        cx.background_executor().timer(Duration::from_secs(2)).await;
                        cx.update(|cx| update_toast(id, Toast::success("Deployed"), cx))
                            .ok();
                    })
                    .detach();
                }),
        );

    let left = column()
        .child(demo(
            "Popover and Hover Card",
            "Click or rest the pointer on a trigger.",
            row()
                .child(
                    Popover::new("dimensions")
                        .trigger(Button::new("open-popover").outline().label("Open popover"))
                        .child(
                            div()
                                .flex_dir()
                                .flex_col()
                                .gap(px(4.))
                                .child(Large::new("Dimensions"))
                                .child(Muted::new("Set the dimensions for the layer.")),
                        )
                        .child(
                            Field::new()
                                .orientation(FieldOrientation::Horizontal)
                                .child(FieldLabel::new("Width").w(px(64.)))
                                .child(Input::new(&width)),
                        ),
                )
                .child(
                    HoverCard::new("nextjs")
                        .trigger(Button::new("hover-trigger").link().label("@nextjs"))
                        .child(
                            div().flex_dir().gap(px(12.)).child(Avatar::new("N")).child(
                                div()
                                    .flex_dir()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(Small::new("@nextjs"))
                                    .child(div().text_sm().child(
                                        "The React Framework – created and maintained by @vercel.",
                                    ))
                                    .child(Muted::new("Joined December 2021")),
                            ),
                        ),
                ),
        ))
        .child(demo(
            "Dropdown Menu and Context Menu",
            "Icons, shortcuts, checkbox and radio items, submenus. Try the arrow keys.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(12.))
                .child(
                    DropdownMenu::new("account-menu")
                        .trigger(Button::new("account-trigger").outline().label("Open menu"))
                        .menu(account_menu.clone()),
                )
                .child(
                    ContextMenu::new("canvas-menu").menu(account_menu).child(
                        div()
                            .flex_dir()
                            .items_center()
                            .justify_center()
                            .h(px(120.))
                            .rounded(cx.theme().radius_medium())
                            .border_1()
                            .border_dashed()
                            .border_color(cx.theme().colors.border)
                            .text_sm()
                            .child("Right click here"),
                    ),
                ),
        ))
        .child(demo(
            "Menubar and Navigation Menu",
            "Desktop menus and website navigation.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(16.))
                .child(
                    Menubar::new("app-menubar")
                        .menu(
                            "File",
                            Menu::new()
                                .item(MenuItem::new("New Tab").shortcut("Ctrl T"))
                                .item(MenuItem::new("New Window").shortcut("Ctrl N"))
                                .separator()
                                .item(MenuItem::new("Print…").shortcut("Ctrl P")),
                        )
                        .menu(
                            "Edit",
                            Menu::new()
                                .item(MenuItem::new("Undo").shortcut("Ctrl Z"))
                                .item(MenuItem::new("Redo").shortcut("Ctrl Y"))
                                .separator()
                                .item(
                                    MenuItem::new("Find").submenu(
                                        Menu::new()
                                            .item(MenuItem::new("Search the web"))
                                            .item(MenuItem::new("Find…")),
                                    ),
                                ),
                        )
                        .menu(
                            "View",
                            Menu::new()
                                .item(MenuItem::new("Always show bookmarks").checked(true))
                                .item(MenuItem::new("Always show full URLs").checked(false)),
                        ),
                )
                .child(
                    NavigationMenu::new("site-navigation")
                        .panel(
                            "Getting started",
                            div()
                                .flex_dir()
                                .flex_col()
                                .w(px(320.))
                                .child(
                                    NavigationMenuLink::new("nav-intro", "Introduction")
                                        .description("Re-usable components built with GPUI."),
                                )
                                .child(
                                    NavigationMenuLink::new("nav-install", "Installation")
                                        .description("Add rok-ui to your Cargo.toml."),
                                ),
                        )
                        .panel(
                            "Components",
                            div()
                                .flex_dir()
                                .flex_col()
                                .w(px(320.))
                                .child(
                                    NavigationMenuLink::new("nav-alert-dialog", "Alert Dialog")
                                        .description("A modal that expects a response."),
                                )
                                .child(
                                    NavigationMenuLink::new("nav-hover-card", "Hover Card")
                                        .description("Preview content behind a link."),
                                ),
                        )
                        .link("Docs", |(), _, cx| {
                            toast(cx, Toast::info("Docs clicked"));
                        }),
                ),
        ));

    let set = |state: &State<bool>| {
        let state = state.clone();
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| state.set(true, cx)
    };
    let close = |state: &State<bool>| {
        let state = state.clone();
        move |(): &(), _: &mut Window, cx: &mut App| state.set(false, cx)
    };
    let open_sheet = |side: SheetSide| {
        let sheet_open = sheet_open.clone();
        let sheet_side = sheet_side.clone();
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            sheet_side.set(side, cx);
            sheet_open.set(true, cx);
        }
    };

    let right = column()
        .child(demo(
            "Modals",
            "Alert Dialog, Sheet, Drawer and the Ctrl-K command palette.",
            row()
                .child(
                    Button::new("open-alert")
                        .destructive()
                        .label("Delete account")
                        .on_click(set(&alert_open)),
                )
                .child(
                    Button::new("open-sheet-right")
                        .outline()
                        .label("Sheet (right)")
                        .on_click(open_sheet(SheetSide::Right)),
                )
                .child(
                    Button::new("open-sheet-left")
                        .outline()
                        .label("Sheet (left)")
                        .on_click(open_sheet(SheetSide::Left)),
                )
                .child(
                    Button::new("open-drawer")
                        .outline()
                        .label("Drawer")
                        .on_click(set(&drawer_open)),
                )
                .child(
                    Button::new("open-palette")
                        .outline()
                        .icon(IconName::Search)
                        .label("Command palette")
                        .on_click(set(&palette_open)),
                ),
        ))
        .child(demo("Toast", "Sonner-style notifications.", toasts))
        .child(demo(
            "Command",
            "Inline, filtered as you type.",
            Command::new("inline-command")
                .placeholder("Type a command or search…")
                .group(
                    "Suggestions",
                    [
                        CommandItem::new("Calendar").icon(IconName::Calendar),
                        CommandItem::new("Search emoji").icon(IconName::Smile),
                        CommandItem::new("Calculator").disabled(true),
                    ],
                )
                .group(
                    "Settings",
                    [
                        CommandItem::new("Profile")
                            .icon(IconName::User)
                            .shortcut("Ctrl P"),
                        CommandItem::new("Mail")
                            .icon(IconName::Mail)
                            .shortcut("Ctrl B"),
                        CommandItem::new("Settings")
                            .icon(IconName::Settings)
                            .shortcut("Ctrl S"),
                    ],
                ),
        ));

    let goal_value = goal.get(cx);
    // The modals render nothing while closed; keep them out of the column row
    // so they do not add gaps.
    div()
        .child(
            div()
                .flex_dir()
                .items_start()
                .gap(px(24.))
                .child(left)
                .child(right),
        )
        .child(
            AlertDialog::new("delete-account")
                .open(alert_open.get(cx))
                .title("Are you absolutely sure?")
                .description(
                    "This action cannot be undone. This will permanently delete your account.",
                )
                .action_label("Delete account")
                .destructive(true)
                .on_cancel(close(&alert_open))
                .on_action({
                    let alert_open = alert_open.clone();
                    move |(), _, cx| {
                        alert_open.set(false, cx);
                        toast(cx, Toast::success("Account deleted (not really)"));
                    }
                }),
        )
        .child(
            Sheet::new("edit-profile-sheet")
                .open(sheet_open.get(cx))
                .side(sheet_side.get(cx))
                .title("Edit profile")
                .description("Make changes to your profile here. Click save when you're done.")
                .child(
                    Field::new()
                        .child(FieldLabel::new("Width"))
                        .child(Input::new(&width)),
                )
                .footer(Button::new("sheet-save").label("Save changes").on_click({
                    let sheet_open = sheet_open.clone();
                    move |_, _, cx| sheet_open.set(false, cx)
                }))
                .on_close(close(&sheet_open)),
        )
        .child(
            Drawer::new("goal-drawer")
                .open(drawer_open.get(cx))
                .title("Move Goal")
                .description("Set your daily activity goal.")
                .child(
                    div()
                        .flex_dir()
                        .items_center()
                        .justify_between()
                        .child(
                            Button::new("goal-minus")
                                .outline()
                                .icon_only(IconName::Minus)
                                .rounded_full()
                                .on_click({
                                    let goal = goal.clone();
                                    move |_, _, cx| goal.update(cx, |goal| *goal -= 10)
                                }),
                        )
                        .child(
                            div()
                                .flex_dir()
                                .flex_col()
                                .items_center()
                                .child(H1::new(goal_value.to_string()))
                                .child(Muted::new("CALORIES/DAY")),
                        )
                        .child(
                            Button::new("goal-plus")
                                .outline()
                                .icon_only(IconName::Plus)
                                .rounded_full()
                                .on_click({
                                    let goal = goal.clone();
                                    move |_, _, cx| goal.update(cx, |goal| *goal += 10)
                                }),
                        ),
                )
                .footer(Button::new("goal-submit").label("Submit").on_click({
                    let drawer_open = drawer_open.clone();
                    move |_, _, cx| drawer_open.set(false, cx)
                }))
                .footer(
                    Button::new("goal-cancel")
                        .outline()
                        .label("Cancel")
                        .on_click({
                            let drawer_open = drawer_open.clone();
                            move |_, _, cx| drawer_open.set(false, cx)
                        }),
                )
                .on_close(close(&drawer_open)),
        )
        .child(
            CommandDialog::new(
                "palette",
                Command::new("palette-command")
                    .group(
                        "Pages",
                        [
                            CommandItem::new("Home").icon(IconName::Home),
                            CommandItem::new("Inbox").icon(IconName::Inbox),
                        ],
                    )
                    .group(
                        "Actions",
                        [CommandItem::new("Toggle theme")
                            .icon(IconName::Moon)
                            .on_select(|(), _, cx| Theme::toggle_mode(cx))],
                    ),
            )
            .open(palette_open.get(cx))
            .on_close(close(&palette_open)),
        )
}
