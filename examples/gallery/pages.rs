//! The gallery pages beyond the overview. Each page is a `#[component]` that
//! keeps its demo state in hooks, so the views stay small.

use std::time::Duration;

use rok_ui::prelude::*;

/// A titled demo block.
fn demo(title: &'static str, description: &'static str, content: impl IntoElement) -> Card {
    Card::new()
        .child(
            CardHeader::new()
                .child(CardTitle::new(title))
                .child(CardDescription::new(description)),
        )
        .child(CardContent::new().child(content))
}

fn column() -> Div {
    div().flex_dir().flex_col().flex_1().min_w_0().gap(px(16.))
}

fn row() -> Div {
    div().flex_dir().flex_wrap().items_center().gap(px(8.))
}

/// Text inputs, choices and pickers.
#[component]
pub fn FormsPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let url = use_input_state("forms-url", window, cx, |state| {
        state.with_placeholder("example.com")
    });
    let search = use_input_state("forms-search", window, cx, |state| {
        state.with_placeholder("Search…")
    });
    let card_name = use_input_state("forms-card-name", window, cx, |state| {
        state.with_placeholder("Ada Lovelace")
    });
    let card_number = use_input_state("forms-card-number", window, cx, |state| {
        state.with_placeholder("1234 5678 9012 3456")
    });
    let bio = use_textarea_state("forms-bio", window, cx, |state| {
        state.with_placeholder("Tell us a little bit about yourself")
    });
    let prompt = use_textarea_state("forms-prompt", window, cx, |state| {
        state.with_placeholder("Ask, search or chat…")
    });
    let code = use_state(window, cx, || SharedString::from("12"));
    let fruit = use_state(window, cx, || Some(SharedString::from("banana")));
    let country = use_state(window, cx, || None::<SharedString>);
    let framework = use_state(window, cx, || None::<SharedString>);
    let density = use_state(window, cx, || Some(SharedString::from("comfortable")));
    let volume = use_state(window, cx, || 50_f32);
    let price = use_state(window, cx, || (25_f32, 75_f32));
    let bold = use_state(window, cx, || true);
    let formatting = use_state(window, cx, || vec![SharedString::from("italic")]);
    let alignment = use_state(window, cx, || vec![SharedString::from("left")]);
    let due_date = use_state(window, cx, || Some(CalendarDate::today()));
    let trip = use_state(window, cx, || None::<DateRange>);
    let picked_day = use_state(window, cx, || Some(CalendarDate::today()));
    let card_number_invalid = !card_number.read(cx).text().is_empty()
        && card_number
            .read(cx)
            .text()
            .chars()
            .filter(char::is_ascii_digit)
            .count()
            < 12;

    let left = column()
        .child(demo(
            "Field",
            "Labels, controls, help text and errors.",
            FieldSet::new()
                .child(FieldLegend::new("Payment method"))
                .child(FieldDescription::new(
                    "All transactions are secure and encrypted.",
                ))
                .child(
                    FieldGroup::new()
                        .child(
                            Field::new()
                                .child(FieldLabel::new("Name on card"))
                                .child(Input::new(&card_name)),
                        )
                        .child(
                            Field::new()
                                .invalid(card_number_invalid)
                                .child(FieldLabel::new("Card number"))
                                .child(Input::new(&card_number).invalid(card_number_invalid))
                                .when(card_number_invalid, |field| {
                                    field.child(FieldError::new("Enter at least 12 digits."))
                                })
                                .when(!card_number_invalid, |field| {
                                    field.child(FieldDescription::new(
                                        "Typing fewer than 12 digits shows the error state.",
                                    ))
                                }),
                        )
                        .child(FieldSeparator::new().text("Or"))
                        .child(
                            Field::new()
                                .orientation(FieldOrientation::Horizontal)
                                .child(
                                    FieldContent::new()
                                        .child(FieldTitle::new("Save card"))
                                        .child(FieldDescription::new("Use it next time.")),
                                )
                                .child(Switch::new("save-card").checked(true)),
                        ),
                ),
        ))
        .child(demo(
            "Input Group",
            "Addons inside the field's border.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(12.))
                .child(
                    InputGroup::new(&url)
                        .leading_text("https://")
                        .trailing_text(".com"),
                )
                .child(
                    InputGroup::new(&search)
                        .leading_icon(IconName::Search)
                        .trailing(Kbd::new("Ctrl K")),
                )
                .child(
                    InputGroup::new(&prompt).block_end(
                        div()
                            .flex_dir()
                            .w_full()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                Button::new("prompt-attach")
                                    .ghost()
                                    .small()
                                    .icon_only(IconName::Paperclip)
                                    .tooltip("Attach"),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("prompt-send")
                                    .small()
                                    .icon_only(IconName::Send)
                                    .tooltip("Send"),
                            ),
                    ),
                ),
        ))
        .child(demo(
            "Textarea",
            "Grows with its content. Enter adds a line, Ctrl/Cmd-Enter submits.",
            Textarea::new(&bio),
        ))
        .child(demo(
            "Input OTP",
            "Type or paste a code.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(8.))
                .child(
                    InputOtp::new("otp", 6)
                        .groups([3, 3])
                        .value(code.get(cx))
                        .on_change({
                            let code = code.clone();
                            move |value, _, cx| code.set(value.clone(), cx)
                        })
                        .on_complete(|value, _, cx| {
                            toast(cx, Toast::success(format!("Code {value} entered")));
                        }),
                )
                .child(Muted::new(format!("Value: {}", code.get(cx)))),
        ));

    let right = column()
        .child(demo(
            "Select, Native Select and Combobox",
            "Pick one value from a list.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(12.))
                .child(
                    Select::new("fruit")
                        .w(px(200.))
                        .placeholder("Select a fruit")
                        .group_label("Fruits")
                        .option("apple", "Apple")
                        .option("banana", "Banana")
                        .option("blueberry", "Blueberry")
                        .disabled_option("grapes", "Grapes")
                        .separator()
                        .group_label("Vegetables")
                        .option("carrot", "Carrot")
                        .value(fruit.get(cx))
                        .on_change({
                            let fruit = fruit.clone();
                            move |value, _, cx| fruit.set(Some(value.clone()), cx)
                        }),
                )
                .child(
                    NativeSelect::new("country")
                        .placeholder("Select a country")
                        .option("nl", "Netherlands")
                        .option("ma", "Morocco")
                        .option("jp", "Japan")
                        .value(country.get(cx))
                        .on_change({
                            let country = country.clone();
                            move |value, _, cx| country.set(Some(value.clone()), cx)
                        }),
                )
                .child(
                    Combobox::new("framework")
                        .placeholder("Select framework…")
                        .search_placeholder("Search framework…")
                        .empty_text("No framework found.")
                        .option("next", "Next.js")
                        .option("svelte", "SvelteKit")
                        .option("nuxt", "Nuxt.js")
                        .option("remix", "Remix")
                        .option("astro", "Astro")
                        .value(framework.get(cx))
                        .on_change({
                            let framework = framework.clone();
                            move |value, _, cx| framework.set(value.clone(), cx)
                        }),
                ),
        ))
        .child(demo(
            "Radio Group, Slider and Toggles",
            "Choices and ranges.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(16.))
                .child(
                    RadioGroup::new("density")
                        .option("default", "Default")
                        .option_with_description(
                            "comfortable",
                            "Comfortable",
                            "More space between rows.",
                        )
                        .option("compact", "Compact")
                        .value(density.get(cx))
                        .on_change({
                            let density = density.clone();
                            move |value, _, cx| density.set(Some(value.clone()), cx)
                        }),
                )
                .child(Separator::new())
                .child(
                    div()
                        .flex_dir()
                        .justify_between()
                        .text_sm()
                        .child("Volume")
                        .child(Muted::new(format!("{}", volume.get(cx) as i32))),
                )
                .child(Slider::new("volume").value(volume.get(cx)).on_change({
                    let volume = volume.clone();
                    move |values, _, cx| volume.set(values[0], cx)
                }))
                .child(
                    div()
                        .flex_dir()
                        .justify_between()
                        .text_sm()
                        .child("Price range")
                        .child(Muted::new(format!(
                            "${} – ${}",
                            price.get(cx).0 as i32,
                            price.get(cx).1 as i32
                        ))),
                )
                .child(
                    Slider::new("price")
                        .range(price.get(cx).0, price.get(cx).1)
                        .step(5.)
                        .on_change({
                            let price = price.clone();
                            move |values, _, cx| price.set((values[0], values[1]), cx)
                        }),
                )
                .child(Separator::new())
                .child(
                    row()
                        .child(
                            Toggle::new("bold-toggle")
                                .icon(IconName::Bold)
                                .tooltip("Bold")
                                .pressed(bold.get(cx))
                                .on_change({
                                    let bold = bold.clone();
                                    move |pressed, _, cx| bold.set(*pressed, cx)
                                }),
                        )
                        .child(
                            ToggleGroup::new("formatting")
                                .multiple(true)
                                .icon_item("bold", IconName::Bold, "Bold")
                                .icon_item("italic", IconName::Italic, "Italic")
                                .icon_item("underline", IconName::Underline, "Underline")
                                .value(formatting.get(cx))
                                .on_change({
                                    let formatting = formatting.clone();
                                    move |values, _, cx| formatting.set(values.clone(), cx)
                                }),
                        )
                        .child(
                            ToggleGroup::new("alignment")
                                .outline()
                                .icon_item("left", IconName::AlignLeft, "Align left")
                                .icon_item("center", IconName::AlignCenter, "Align center")
                                .icon_item("right", IconName::AlignRight, "Align right")
                                .value(alignment.get(cx))
                                .on_change({
                                    let alignment = alignment.clone();
                                    move |values, _, cx| alignment.set(values.clone(), cx)
                                }),
                        ),
                ),
        ))
        .child(demo(
            "Date Picker and Calendar",
            "Single dates, ranges and presets.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(12.))
                .child(
                    DatePicker::new("due-date")
                        .date(due_date.get(cx))
                        .preset("Today", CalendarDate::today())
                        .preset("Tomorrow", CalendarDate::today().add_days(1))
                        .preset("In a week", CalendarDate::today().add_days(7))
                        .on_change({
                            let due_date = due_date.clone();
                            move |date, _, cx| due_date.set(Some(*date), cx)
                        }),
                )
                .child(
                    DatePicker::new("trip")
                        .range(trip.get(cx))
                        .number_of_months(2)
                        .on_range_change({
                            let trip = trip.clone();
                            move |range, _, cx| trip.set(Some(*range), cx)
                        }),
                )
                .child(
                    Calendar::new("inline-calendar")
                        .selected(picked_day.get(cx))
                        .on_select({
                            let picked_day = picked_day.clone();
                            move |date, _, cx| picked_day.set(Some(*date), cx)
                        })
                        .rounded_md()
                        .border_1()
                        .border_color(cx.theme().colors.border)
                        .w(px(260.)),
                ),
        ));

    div()
        .flex_dir()
        .items_start()
        .gap(px(24.))
        .child(left)
        .child(right)
}

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

/// Disclosure, navigation and layout helpers.
#[component]
pub fn LayoutPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let page = use_state(window, cx, || 5_usize);
    let colors = cx.theme().colors.clone();
    let radius = cx.theme().radius_medium();
    let slide = move |number: usize| {
        div()
            .h(px(140.))
            .flex_dir()
            .items_center()
            .justify_center()
            .rounded(radius)
            .border_1()
            .border_color(colors.border)
            .child(H2::new(number.to_string()).border_b_0().pb(px(0.)))
    };

    let left = column()
        .child(demo(
            "Accordion",
            "One section open at a time.",
            Accordion::new("faq")
                .item(
                    AccordionItem::new("Is it accessible?")
                        .child("Yes. Triggers are focusable and open with Enter or Space."),
                )
                .item(
                    AccordionItem::new("Is it styled?")
                        .child("Yes. It reads every color from the active theme."),
                )
                .item(
                    AccordionItem::new("Is it animated?")
                        .child("Not yet: sections open instantly."),
                )
                .default_open([0]),
        ))
        .child(demo(
            "Collapsible",
            "Show and hide a panel.",
            Collapsible::new("starred-repos")
                .trigger(
                    div()
                        .flex_dir()
                        .items_center()
                        .justify_between()
                        .child(Small::new("@peduarte starred 3 repositories"))
                        .child(
                            Button::new("collapsible-toggle")
                                .ghost()
                                .small()
                                .icon_only(IconName::ChevronsUpDown)
                                .tooltip("Toggle"),
                        ),
                )
                .always_visible(RepositoryRow::new("@radix-ui/primitives"))
                .child(RepositoryRow::new("@radix-ui/colors"))
                .child(RepositoryRow::new("@stitches/react")),
        ))
        .child(demo(
            "Breadcrumb and Pagination",
            "Where you are, and where you can go.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(16.))
                .child(
                    Breadcrumb::new("path")
                        .link("Home", |(), _, _| {})
                        .ellipsis_menu(
                            Menu::new()
                                .item(MenuItem::new("Documentation"))
                                .item(MenuItem::new("Themes"))
                                .item(MenuItem::new("GitHub")),
                        )
                        .link("Components", |(), _, _| {})
                        .page("Breadcrumb"),
                )
                .child(
                    Pagination::new("pager", 10)
                        .current_page(page.get(cx))
                        .on_change({
                            let page = page.clone();
                            move |next, _, cx| page.set(*next, cx)
                        }),
                ),
        ))
        .child(demo(
            "Direction",
            "A right-to-left subtree mirrors navigation layouts.",
            Direction::new(TextDirection::Rtl)
                .child(
                    Breadcrumb::new("rtl-path")
                        .text("Home")
                        .text("Components")
                        .page("Direction"),
                )
                .child(Pagination::new("rtl-pager", 5).current_page(2)),
        ))
        .child(demo(
            "Typography",
            "shadcn/ui's text styles.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(12.))
                .child(H1::new("The Joke Tax Chronicles"))
                .child(Lead::new(
                    "Once upon a time, in a far-off land, there was a very lazy king.",
                ))
                .child(H2::new("The King's Plan"))
                .child(P::new(
                    "The king thought long and hard, and finally came up with a brilliant plan.",
                ))
                .child(Blockquote::new(
                    "\"After all,\" he said, \"everyone enjoys a good joke, so it's only fair that they should pay for the privilege.\"",
                ))
                .child(H3::new("The Joke Tax"))
                .child(List::new(vec![
                    "1st level of puns: 5 gold coins".into(),
                    "2nd level of jokes: 10 gold coins".into(),
                    "3rd level of one-liners: 20 gold coins".into(),
                ]))
                .child(
                    row()
                        .child(div().text_sm().child("Install with"))
                        .child(InlineCode::new("cargo add rok-ui")),
                ),
        ));

    let right = column()
        .child(demo(
            "Resizable",
            "Drag the handles.",
            ResizablePanelGroup::new("resizable-demo")
                .with_handle(true)
                .h(px(220.))
                .rounded(cx.theme().radius_large())
                .border_1()
                .border_color(cx.theme().colors.border)
                .panel(
                    ResizablePanel::new()
                        .default_size(40.)
                        .min_size(20.)
                        .child(centered("One")),
                )
                .panel(ResizablePanel::new().child(
                    ResizablePanelGroup::new("resizable-nested").vertical()
                        .panel(ResizablePanel::new().default_size(30.).child(centered("Two")))
                        .panel(ResizablePanel::new().child(centered("Three"))),
                )),
        ))
        .child(demo(
            "Carousel",
            "Arrows, dots and the arrow keys.",
            div()
                .px(px(24.))
                .child(
                    Carousel::new("numbers")
                        .items_per_view(2)
                        .item(slide(1))
                        .item(slide(2))
                        .item(slide(3))
                        .item(slide(4))
                        .item(slide(5)),
                ),
        ))
        .child(demo(
            "Scroll Area and Aspect Ratio",
            "A themed scrollbar, and a box that keeps its shape.",
            div()
                .flex_dir()
                .gap(px(16.))
                .child(
                    ScrollArea::new("tags")
                        .h(px(200.))
                        .w(px(160.))
                        .rounded(cx.theme().radius_medium())
                        .border_1()
                        .border_color(cx.theme().colors.border)
                        .child(
                            div()
                                .flex_dir()
                                .flex_col()
                                .p(px(16.))
                                .gap(px(8.))
                                .child(Small::new("Tags"))
                                .children((1..=40).map(|version| {
                                    div()
                                        .text_sm()
                                        .child(format!("v1.2.0-beta.{version}"))
                                })),
                        ),
                )
                .child(
                    div().flex_1().child(
                        AspectRatio::new(16_f32 / 9.)
                            .rounded(cx.theme().radius_medium())
                            .bg(cx.theme().colors.muted)
                            .child(centered("16 : 9")),
                    ),
                ),
        ))
        .child(demo(
            "Item, Button Group and Kbd",
            "Rows, joined controls and key caps.",
            div()
                .flex_dir()
                .flex_col()
                .gap(px(12.))
                .child(
                    Item::new("two-factor")
                        .outline()
                        .icon(IconName::Settings)
                        .title("Two-factor authentication")
                        .description("Verify via email or phone number.")
                        .action(Button::new("enable-2fa").small().label("Enable")),
                )
                .child(
                    Item::new("profile-verified")
                        .muted()
                        .small()
                        .icon(IconName::CircleCheck)
                        .title("Your profile has been verified.")
                        .action(Icon::new(IconName::ChevronRight)),
                )
                .child(
                    row()
                        .child(
                            ButtonGroup::new()
                                .item(Button::new("group-archive").outline().label("Archive"))
                                .item(Button::new("group-report").outline().label("Report"))
                                .item(Button::new("group-snooze").outline().label("Snooze")),
                        )
                        .child(
                            ButtonGroup::new()
                                .text("https://")
                                .item(Button::new("group-copy").outline().icon_only(IconName::Copy)),
                        )
                        .child(KbdGroup::new(vec!["Ctrl".into(), "Shift".into(), "P".into()])),
                ),
        ))
        .child(demo(
            "Empty",
            "Nothing here yet.",
            Empty::new()
                .bordered(true)
                .icon(IconName::Folder)
                .title("No projects yet")
                .description("You haven't created any projects yet. Get started by creating your first project.")
                .child(Button::new("create-project").label("Create project"))
                .child(Button::new("import-project").outline().label("Import project")),
        ));

    div()
        .flex_dir()
        .items_start()
        .gap(px(24.))
        .child(left)
        .child(right)
}

/// A monospace row in the collapsible demo.
#[component]
fn RepositoryRow(name: SharedString, cx: &mut App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .px(px(16.))
        .py(px(8.))
        .rounded(theme.radius_medium())
        .border_1()
        .border_color(theme.colors.border)
        .font_family(theme.monospace_font_family.clone())
        .text_sm()
        .child(name)
}

fn centered(text: &'static str) -> impl IntoElement {
    div()
        .size_full()
        .flex_dir()
        .items_center()
        .justify_center()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
}

/// Tables and charts.
#[component]
pub fn DataPage() -> impl IntoElement {
    let payments: [(&str, &str, &str); 8] = [
        ("success", "ken99@example.com", "$316.00"),
        ("success", "abe45@example.com", "$242.00"),
        ("processing", "monserrat44@example.com", "$837.00"),
        ("success", "silas22@example.com", "$874.00"),
        ("failed", "carmella@example.com", "$721.00"),
        ("pending", "jason78@example.com", "$1,450.00"),
        ("success", "sarah23@example.com", "$90.00"),
        ("failed", "derek11@example.com", "$612.00"),
    ];
    let months = ["January", "February", "March", "April", "May", "June"];

    let invoices = [
        ("INV001", "Paid", "Credit Card", "$250.00"),
        ("INV002", "Pending", "PayPal", "$150.00"),
        ("INV003", "Unpaid", "Bank Transfer", "$350.00"),
        ("INV004", "Paid", "Credit Card", "$450.00"),
    ];

    div()
        .flex_dir()
        .flex_col()
        .gap(px(16.))
        .child(demo(
            "Data Table",
            "Filter by email, sort by amount, select rows, hide columns, page through results.",
            DataTable::new("payments")
                .column(DataColumn::new("status", "Status").width(px(120.)))
                .column(DataColumn::new("email", "Email").sortable())
                .column(
                    DataColumn::new("amount", "Amount")
                        .sortable()
                        .align_end()
                        .width(px(140.)),
                )
                .rows(payments.iter().map(|(status, email, amount)| {
                    vec![(*status).into(), (*email).into(), (*amount).into()]
                }))
                .filter_column("email", "Filter emails…")
                .selectable(true)
                .page_size(5)
                .row_actions(move |row| {
                    let email = payments[row].1;
                    Menu::new()
                        .label("Actions")
                        .item(MenuItem::new("Copy email").on_select(move |(), _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                email.to_string(),
                            ));
                            toast(cx, Toast::success("Copied to the clipboard"));
                        }))
                        .separator()
                        .item(MenuItem::new("View customer"))
                        .item(MenuItem::new("View payment details"))
                }),
        ))
        .child(
            div()
                .flex_dir()
                .items_start()
                .gap(px(16.))
                .child(
                    column().child(demo(
                        "Bar Chart",
                        "January – June 2024",
                        Chart::new("visitors-bar", ChartKind::Bar)
                            .categories(months)
                            .series(ChartSeries::new(
                                "Desktop",
                                [186., 305., 237., 73., 209., 214.],
                            ))
                            .series(ChartSeries::new(
                                "Mobile",
                                [80., 200., 120., 190., 130., 140.],
                            ))
                            .legend(true),
                    )),
                )
                .child(
                    column().child(demo(
                        "Area Chart",
                        "Showing total visitors for the last 6 months",
                        Chart::new("visitors-area", ChartKind::Area)
                            .categories(months)
                            .series(ChartSeries::new(
                                "Desktop",
                                [186., 305., 237., 73., 209., 214.],
                            ))
                            .series(ChartSeries::new(
                                "Mobile",
                                [80., 200., 120., 190., 130., 140.],
                            ))
                            .legend(true),
                    )),
                ),
        )
        .child(
            div()
                .flex_dir()
                .items_start()
                .gap(px(16.))
                .child(
                    column().child(demo(
                        "Line Chart",
                        "With a y-axis.",
                        Chart::new("visitors-line", ChartKind::Line)
                            .categories(months)
                            .series(ChartSeries::new(
                                "Desktop",
                                [186., 305., 237., 73., 209., 214.],
                            ))
                            .y_axis(true),
                    )),
                )
                .child(
                    column().child(demo(
                        "Donut Chart",
                        "Browser share",
                        Chart::new("browsers", ChartKind::Donut)
                            .categories(["Chrome", "Safari", "Firefox", "Edge", "Other"])
                            .series(ChartSeries::new("Visitors", [275., 200., 187., 173., 90.]))
                            .center_label("925", "Visitors")
                            .legend(true),
                    )),
                ),
        )
        .child(demo(
            "Table",
            "A list of your recent invoices.",
            Table::new()
                .child(
                    TableHeader::new().child(
                        TableRow::new()
                            .child(TableHead::new("Invoice").w(px(120.)).flex_none())
                            .child(TableHead::new("Status"))
                            .child(TableHead::new("Method"))
                            .child(TableHead::new("Amount").justify_end()),
                    ),
                )
                .child(TableBody::new().children(invoices.iter().map(
                    |(invoice, status, method, amount)| {
                        TableRow::new()
                            .child(
                                TableCell::new()
                                    .w(px(120.))
                                    .flex_none()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(*invoice),
                            )
                            .child(TableCell::new().child(*status))
                            .child(TableCell::new().child(*method))
                            .child(TableCell::new().flex_dir().justify_end().child(*amount))
                    },
                )))
                .child(
                    TableFooter::new().child(
                        TableRow::new()
                            .border_b_0()
                            .child(TableCell::new().child("Total"))
                            .child(TableCell::new().flex_dir().justify_end().child("$1,200.00")),
                    ),
                )
                .child(TableCaption::new("A list of your recent invoices.")),
        ))
}

/// One message in the chat demo.
#[derive(Clone)]
struct ChatEntry {
    from_me: bool,
    text: &'static str,
    kind: ChatKind,
}

#[derive(Clone, Copy, PartialEq)]
enum ChatKind {
    Text,
    Separator,
    Note,
    ToolCall,
    Attachments,
}

fn chat_entries() -> Vec<ChatEntry> {
    let entry = |from_me, text, kind| ChatEntry {
        from_me,
        text,
        kind,
    };
    let mut entries = vec![entry(false, "Yesterday", ChatKind::Separator)];
    for index in 0..12 {
        entries.push(entry(
            index % 2 == 0,
            if index % 2 == 0 {
                "Older message — scroll up from the latest to load more history."
            } else {
                "Loaded from history without moving your place."
            },
            ChatKind::Text,
        ));
    }
    entries.extend([
        entry(false, "Today", ChatKind::Separator),
        entry(false, "Ada joined the conversation", ChatKind::Note),
        entry(false, "Hi! Can you summarize the engine drawings?", ChatKind::Text),
        entry(true, "Sure, attaching them now.", ChatKind::Text),
        entry(true, "", ChatKind::Attachments),
        entry(false, "Searched the archive", ChatKind::ToolCall),
        entry(
            false,
            "The Analytical Engine has a store, a mill, and a reader for punched cards. The store holds numbers, the mill operates on them, and the cards describe the operations — which is what makes it programmable rather than merely a calculator. Expand this bubble to read the rest of this deliberately long reply about the design and its consequences.",
            ChatKind::Text,
        ),
    ]);
    entries
}

fn render_chat_entry(index: usize, entry: &ChatEntry) -> AnyElement {
    match entry.kind {
        ChatKind::Separator => Marker::separator(entry.text).into_any_element(),
        ChatKind::Note => Marker::note(entry.text)
            .icon(IconName::User)
            .into_any_element(),
        ChatKind::ToolCall => Marker::row(entry.text)
            .icon(IconName::Search)
            .detail("3 results")
            .into_any_element(),
        ChatKind::Attachments => Message::new()
            .align(BubbleAlign::End)
            .child(
                div()
                    .flex_dir()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        Attachment::new(("chat-attachment", index), "engine-drawings.pdf")
                            .meta("PDF, 12 pages, 4.1 MB"),
                    )
                    .child(
                        Attachment::new(("chat-upload", index), "mill-detail.png")
                            .state(AttachmentState::Uploading(Some(64.)))
                            .on_remove(|(), _, _| {}),
                    )
                    .child(
                        Attachment::new(("chat-failed", index), "store-notes.txt")
                            .state(AttachmentState::Failed("Upload failed".into()))
                            .on_retry(|(), _, _| {})
                            .on_remove(|(), _, _| {}),
                    ),
            )
            .into_any_element(),
        ChatKind::Text if entry.from_me => Message::new()
            .align(BubbleAlign::End)
            .child(
                Bubble::new(("chat-bubble", index))
                    .variant(BubbleVariant::Primary)
                    .align(BubbleAlign::End)
                    .child(entry.text),
            )
            .footer("Read")
            .into_any_element(),
        ChatKind::Text => Message::new()
            .avatar(Avatar::new("AL"))
            .name("Ada Lovelace")
            .timestamp("10:42")
            .child(
                Bubble::new(("chat-bubble", index))
                    .reaction(BubbleReaction::new("+1", 2).reacted(true))
                    .reaction(BubbleReaction::new("Wow", 1))
                    .collapse_after(px(60.))
                    .child(entry.text),
            )
            .into_any_element(),
    }
}

/// Conversation components and the questionnaire.
#[component]
pub fn ChatPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let entries = chat_entries();
    // The first 13 entries are "history", loaded when the reader reaches the top.
    let history_count = 13;
    let loaded_from = use_state(window, cx, move || history_count);
    let scroller = use_state(window, cx, move || {
        MessageScrollerState::new(chat_entries().len() - history_count)
    })
    .get(cx);
    let first_loaded = loaded_from.get(cx);
    let visible: Vec<ChatEntry> = entries[first_loaded..].to_vec();
    let visible_count = visible.len();
    let typing = use_state(window, cx, || true);
    let answers = use_state(window, cx, Vec::<QuestionnaireAnswer>::new);

    let load_history = {
        let loaded_from = loaded_from.clone();
        let scroller = scroller.clone();
        move |(): &(), _: &mut Window, cx: &mut App| {
            let first = loaded_from.get(cx);
            if first > 0 {
                scroller.prepend(first);
                loaded_from.set(0, cx);
            }
        }
    };
    let jump_scroller = scroller.clone();

    div()
        .flex_dir()
        .items_start()
        .gap(px(24.))
        .child(
            column().child(demo(
                "Message Scroller",
                "Messages, bubbles, markers and attachments. Scroll up to load history.",
                div()
                    .flex_dir()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        MessageScroller::new(&scroller, move |index, _, _| {
                            visible.get(index).map_or_else(
                                || div().into_any_element(),
                                |entry| render_chat_entry(index, entry),
                            )
                        })
                        .on_reach_top(load_history)
                        .h(px(460.))
                        .rounded(cx.theme().radius_large())
                        .border_1()
                        .border_color(cx.theme().colors.border),
                    )
                    .child(
                        row()
                            .child(if typing.get(cx) {
                                Marker::status("Ada is typing…")
                                    .busy(true)
                                    .into_any_element()
                            } else {
                                Marker::status("Ada is online")
                                    .dot_color(gpui::rgb(0x16A34A).into())
                                    .into_any_element()
                            })
                            .child(div().flex_1())
                            .child(
                                Button::new("toggle-typing")
                                    .ghost()
                                    .small()
                                    .label("Toggle status")
                                    .on_click({
                                        let typing = typing.clone();
                                        move |_, _, cx| {
                                            typing.update(cx, |typing| *typing = !*typing);
                                        }
                                    }),
                            )
                            .child(
                                Button::new("jump-first")
                                    .outline()
                                    .small()
                                    .label("Jump to first")
                                    .on_click(move |_, window, _| {
                                        jump_scroller.scroll_to_message(0);
                                        window.refresh();
                                    }),
                            ),
                    )
                    .child(Muted::new(format!("{visible_count} messages loaded"))),
            )),
        )
        .child(
            column()
                .child(
                    Questionnaire::new("onboarding")
                        .question(
                            Question::single(
                                "role",
                                "What best describes you?",
                                ["Engineer", "Designer", "Product manager", "Founder"],
                            )
                            .description("We'll tailor the examples to you."),
                        )
                        .question(Question::multiple(
                            "tools",
                            "Which tools do you use every day?",
                            ["Zed", "VS Code", "Figma", "Linear", "GitHub"],
                        ))
                        .question(
                            Question::freeform("goal", "What do you want to build with rok-ui?")
                                .placeholder("A desktop client for…")
                                .skippable(),
                        )
                        .on_complete({
                            let answers = answers.clone();
                            move |result, _, cx| {
                                answers.set(result.clone(), cx);
                                toast(cx, Toast::success("Thanks for your answers"));
                            }
                        }),
                )
                .when(!answers.read(cx).is_empty(), |column| {
                    column.child(Muted::new(format!(
                        "on_complete received {} answers",
                        answers.read(cx).len()
                    )))
                }),
        )
}
