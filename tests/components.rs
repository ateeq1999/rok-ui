//! Integration tests: the `#[component]` macro's generated API, the `State` hook
//! handle, and a full render of every component under every theme.

use rok_ui::prelude::*;

/// A component exercising every kind of macro parameter.
#[component]
fn Greeting(
    name: SharedString,
    #[prop(optional)] excited: bool,
    #[prop(optional)] title: Option<SharedString>,
    #[prop(optional)] on_wave: Option<EventHandler<ClickEvent>>,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    let punctuation = if excited { "!" } else { "." };
    div()
        .text_color(cx.theme().colors.foreground)
        .when_some(title, |greeting, title| greeting.child(title))
        .child(format!("Hello, {name}{punctuation}"))
        .when(on_wave.is_some(), |greeting| greeting.child("(waves)"))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

#[test]
fn component_macro_generates_a_builder_api() {
    let greeting = Greeting::new("Ada")
        .excited(true)
        .title("Welcome")
        .on_wave(|_, _, _| {})
        .child("Glad you are here")
        .w_full();

    assert_eq!(greeting.name.as_ref(), "Ada");
    assert!(greeting.excited);
    assert_eq!(
        greeting.title.as_ref().map(|title| title.as_ref()),
        Some("Welcome")
    );
    assert!(greeting.on_wave.is_some());
    assert_eq!(greeting.children.len(), 1);

    let plain_greeting = Greeting::new("Grace");
    assert!(!plain_greeting.excited);
    assert!(plain_greeting.title.is_none());
    assert!(plain_greeting.children.is_empty());
}

#[gpui::test]
fn state_handle_sets_and_updates(cx: &mut gpui::TestAppContext) {
    let counter_entity = cx.new(|_| 1_i32);
    let counter = State::from_entity(counter_entity);
    cx.update(|cx| {
        assert_eq!(counter.get(cx), 1);
        counter.set(5, cx);
        assert_eq!(counter.get(cx), 5);
        counter.update(cx, |value| *value *= 2);
        assert_eq!(*counter.read(cx), 10);
    });
}

struct EveryComponentView {
    name_input: Entity<InputState>,
}

impl Render for EveryComponentView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .child(Greeting::new("Ada").child("child"))
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(Button::new("primary").label("Primary"))
                    .child(
                        Button::new("outline")
                            .outline()
                            .icon(IconName::Plus)
                            .label("Add"),
                    )
                    .child(Button::new("loading").loading(true).label("Saving"))
                    .child(
                        Button::new("icon")
                            .ghost()
                            .icon_only(IconName::Bell)
                            .tooltip("Bell"),
                    ),
            )
            .child(Badge::new("New").variant(BadgeVariant::Outline))
            .child(
                Card::new()
                    .child(
                        CardHeader::new()
                            .child(CardTitle::new("Title"))
                            .child(CardDescription::new("Description")),
                    )
                    .child(
                        CardContent::new()
                            .child(Label::new("Name"))
                            .child(Input::new(&self.name_input).leading_icon(IconName::User)),
                    )
                    .child(CardFooter::new().child(Button::new("save").label("Save"))),
            )
            .child(Checkbox::new("checkbox").checked(true).label("Checked"))
            .child(Switch::new("switch").checked(false).label("Off"))
            .child(Tabs::new("tabs").tab("One").tab("Two").selected_index(1))
            .child(
                Alert::new("Careful")
                    .destructive()
                    .description("Something failed"),
            )
            .child(Progress::new(150_f32))
            .child(Skeleton::new("skeleton").h(px(12.)).w(px(80.)))
            .child(Avatar::new("AL"))
            .child(KeyboardShortcut::new("Ctrl K"))
            .child(Separator::new())
            .child(Spinner::new())
            .child(
                Dialog::new("dialog")
                    .open(true)
                    .title("Dialog")
                    .description("Open during the test")
                    .footer(Button::new("confirm").label("Confirm")),
            )
    }
}

#[gpui::test]
fn every_component_renders_in_every_theme(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    for preset in ThemePreset::ALL {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            cx.update(|cx| Theme::set_global(Theme::from_preset(preset, mode), cx));
            let (_view, window_context) = cx.add_window_view(|_, cx| EveryComponentView {
                name_input: cx.new(|cx| InputState::new(cx).with_placeholder("Name")),
            });
            window_context.run_until_parked();
            window_context.update(|window, cx| {
                assert_eq!(cx.theme().mode, mode);
                window.refresh();
            });
            window_context.run_until_parked();
        }
    }
}

/// Every component added for shadcn/ui parity, with floating surfaces forced open.
struct ShadcnParityView {
    notes: Entity<InputState>,
    prompt: Entity<InputState>,
    scroller: MessageScrollerState,
}

impl Render for ShadcnParityView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let today = CalendarDate::new(2026, 10, 2).unwrap();
        let menu = Menu::new()
            .label("My Account")
            .separator()
            .item(
                MenuItem::new("Profile")
                    .icon(IconName::User)
                    .shortcut("Shift Ctrl P"),
            )
            .item(MenuItem::new("Status bar").checked(true))
            .item(MenuItem::new("Top").radio(true))
            .item(MenuItem::new("Invite").submenu(Menu::new().item(MenuItem::new("Email"))))
            .separator()
            .item(MenuItem::new("Log out").destructive());

        AppRoot::new()
            .child(
                Accordion::new("accordion")
                    .item(AccordionItem::new("Is it accessible?").child("Yes."))
                    .item(AccordionItem::new("Is it styled?").child("Yes."))
                    .default_open([0]),
            )
            .child(AspectRatio::new(16_f32 / 9.).child(div().size_full()))
            .child(
                Attachment::new("attachment", "report.pdf")
                    .meta("PDF, 2.4 MB")
                    .state(AttachmentState::Uploading(Some(40.)))
                    .on_remove(|_, _, _| {}),
            )
            .child(
                Attachment::new("failed-attachment", "photo.png")
                    .state(AttachmentState::Failed("Upload failed".into()))
                    .on_retry(|_, _, _| {}),
            )
            .child(
                Breadcrumb::new("breadcrumb")
                    .link("Home", |_, _, _| {})
                    .ellipsis_menu(Menu::new().item(MenuItem::new("Docs")))
                    .page("Breadcrumb"),
            )
            .child(
                Message::new()
                    .avatar(Avatar::new("AL"))
                    .name("Ada")
                    .timestamp("10:42")
                    .child(
                        Bubble::new("bubble")
                            .reaction(BubbleReaction::new("+1", 2).reacted(true))
                            .collapse_after(px(40.))
                            .child("Hello"),
                    )
                    .footer("Read"),
            )
            .child(
                Message::new().align(BubbleAlign::End).child(
                    Bubble::new("own-bubble")
                        .variant(BubbleVariant::Primary)
                        .align(BubbleAlign::End)
                        .group_position(BubbleGroupPosition::Middle)
                        .child("Hi"),
                ),
            )
            .child(Marker::status("Ada is typing").busy(true))
            .child(Marker::note("Conversation renamed"))
            .child(
                Marker::row("Searched the web")
                    .icon(IconName::Globe)
                    .detail("4 results"),
            )
            .child(Marker::separator("Today"))
            .child(
                ButtonGroup::new()
                    .item(Button::new("archive").outline().label("Archive"))
                    .text("or")
                    .item(Button::new("report").outline().label("Report")),
            )
            .child(Calendar::new("calendar").today(today).selected(Some(today)))
            .child(
                Calendar::new("range-calendar")
                    .today(today)
                    .number_of_months(2)
                    .range(Some(DateRange {
                        start: today,
                        end: Some(today.add_days(5)),
                    })),
            )
            .child(
                Carousel::new("carousel")
                    .items_per_view(2)
                    .item(div().h(px(40.)).child("1"))
                    .item(div().h(px(40.)).child("2"))
                    .item(div().h(px(40.)).child("3")),
            )
            .child(
                Chart::new("bar-chart", ChartKind::Bar)
                    .categories(["Jan", "Feb", "Mar"])
                    .series(ChartSeries::new("Desktop", [186., 305., 237.]))
                    .series(ChartSeries::new("Mobile", [80., 200., 120.]))
                    .legend(true)
                    .y_axis(true),
            )
            .child(
                Chart::new("stacked-chart", ChartKind::Bar)
                    .stacked(true)
                    .categories(["Jan", "Feb"])
                    .series(ChartSeries::new("Desktop", [186., 305.]))
                    .series(ChartSeries::new("Mobile", [80., 200.])),
            )
            .child(
                Chart::new("area-chart", ChartKind::Area)
                    .categories(["Jan", "Feb", "Mar"])
                    .series(ChartSeries::new("Desktop", [186., 305., 237.])),
            )
            .child(
                Chart::new("line-chart", ChartKind::Line)
                    .categories(["Jan", "Feb", "Mar"])
                    .series(ChartSeries::new("Desktop", [186., 305., 237.])),
            )
            .child(
                Chart::new("donut-chart", ChartKind::Donut)
                    .categories(["Chrome", "Safari"])
                    .series(ChartSeries::new("Visitors", [275., 200.]))
                    .center_label("475", "Visitors"),
            )
            .child(
                Chart::new("pie-chart", ChartKind::Pie)
                    .categories(["Chrome"])
                    .series(ChartSeries::new("Visitors", [275.])),
            )
            .child(
                Collapsible::new("collapsible")
                    .default_open(true)
                    .trigger(Button::new("toggle").label("Toggle"))
                    .child("Content"),
            )
            .child(
                Combobox::new("combobox")
                    .option("next", "Next.js")
                    .value(Some("next")),
            )
            .child(
                Command::new("command")
                    .group(
                        "Suggestions",
                        [CommandItem::new("Calendar").icon(IconName::Calendar)],
                    )
                    .group("Settings", [CommandItem::new("Profile").shortcut("Ctrl P")]),
            )
            .child(
                ContextMenu::new("context-menu")
                    .menu(menu.clone())
                    .child("Right click"),
            )
            .child(
                DataTable::new("data-table")
                    .column(DataColumn::new("status", "Status"))
                    .column(DataColumn::new("amount", "Amount").sortable().align_end())
                    .rows([
                        vec!["success".into(), "$316.00".into()],
                        vec!["failed".into(), "$721.00".into()],
                    ])
                    .filter_column("status", "Filter status")
                    .selectable(true)
                    .row_actions(|_| Menu::new().item(MenuItem::new("Copy"))),
            )
            .child(
                DatePicker::new("date-picker")
                    .date(Some(today))
                    .preset("Today", today),
            )
            .child(
                Direction::new(TextDirection::Rtl)
                    .child(Breadcrumb::new("rtl-breadcrumb").text("Home").page("Page"))
                    .child(Pagination::new("rtl-pagination", 10).current_page(5)),
            )
            .child(
                DropdownMenu::new("dropdown")
                    .open(true)
                    .trigger(Button::new("dropdown-trigger").label("Open"))
                    .menu(menu.clone()),
            )
            .child(
                Empty::new()
                    .icon(IconName::Folder)
                    .title("No projects")
                    .bordered(true),
            )
            .child(
                FieldSet::new()
                    .child(FieldLegend::new("Payment"))
                    .child(
                        Field::new()
                            .invalid(true)
                            .child(FieldLabel::new("Card"))
                            .child(FieldError::new("Invalid")),
                    )
                    .child(FieldSeparator::new().text("Or")),
            )
            .child(
                HoverCard::new("hover-card").trigger(Button::new("hover").link().label("@nextjs")),
            )
            .child(
                InputGroup::new(&self.prompt)
                    .leading_text("https://")
                    .trailing(Kbd::new("Ctrl K")),
            )
            .child(
                InputGroup::new(&self.notes).block_end(Button::new("send").small().label("Send")),
            )
            .child(InputOtp::new("otp", 6).groups([3, 3]).value("123"))
            .child(
                Item::new("item")
                    .outline()
                    .icon(IconName::Settings)
                    .title("Two-factor authentication")
                    .description("Verify via email.")
                    .action(Button::new("enable").small().label("Enable")),
            )
            .child(KbdGroup::new(vec!["Ctrl".into(), "K".into()]))
            .child(
                Menubar::new("menubar")
                    .menu("File", Menu::new().item(MenuItem::new("New Tab")))
                    .menu("Edit", Menu::new().item(MenuItem::new("Undo"))),
            )
            .child(
                MessageScroller::new(&self.scroller, |index, _, _| {
                    div().child(format!("Message {index}")).into_any_element()
                })
                .h(px(120.)),
            )
            .child(
                NativeSelect::new("native-select")
                    .option("a", "Apple")
                    .value(Some("a")),
            )
            .child(
                NavigationMenu::new("navigation")
                    .panel(
                        "Getting started",
                        NavigationMenuLink::new("intro", "Introduction")
                            .description("Re-usable components."),
                    )
                    .link("Docs", |_, _, _| {}),
            )
            .child(Pagination::new("pagination", 10).current_page(5))
            .child(
                Popover::new("popover")
                    .open(true)
                    .trigger(Button::new("popover-trigger").label("Open"))
                    .child(Label::new("Width")),
            )
            .child(
                Questionnaire::new("questionnaire")
                    .question(Question::single("role", "Role?", ["Engineer", "Designer"]))
                    .question(Question::freeform("goal", "Goal?").skippable()),
            )
            .child(
                RadioGroup::new("radio")
                    .option("a", "A")
                    .option_with_description("b", "B", "Second")
                    .value(Some("a")),
            )
            .child(
                ResizablePanelGroup::new("resizable")
                    .with_handle(true)
                    .panel(ResizablePanel::new().default_size(30.).child("One"))
                    .panel(ResizablePanel::new().child("Two"))
                    .h(px(80.)),
            )
            .child(
                ScrollArea::new("scroll-area")
                    .h(px(80.))
                    .children((0..20).map(|index| div().child(format!("Tag {index}")))),
            )
            .child(
                Select::new("select")
                    .group_label("Fruits")
                    .option("apple", "Apple")
                    .separator()
                    .option("pear", "Pear")
                    .value(Some("apple")),
            )
            .child(
                Sidebar::new("sidebar")
                    .header("Acme")
                    .group(
                        SidebarGroup::new()
                            .label("Application")
                            .item(SidebarItem::new("Home").icon(IconName::Home).active(true))
                            .item(
                                SidebarItem::new("Settings")
                                    .icon(IconName::Settings)
                                    .default_open(true)
                                    .sub_item(SidebarItem::new("General")),
                            ),
                    )
                    .footer(SidebarTrigger::new("sidebar-trigger")),
            )
            .child(
                Sidebar::new("collapsed-sidebar").collapsed(true).group(
                    SidebarGroup::new().item(SidebarItem::new("Inbox").icon(IconName::Inbox)),
                ),
            )
            .child(Slider::new("slider").value(50.))
            .child(Slider::new("range-slider").range(25., 75.).step(5.))
            .child(
                Table::new()
                    .child(TableCaption::new("Invoices"))
                    .child(
                        TableHeader::new().child(
                            TableRow::new()
                                .child(TableHead::new("Invoice"))
                                .child(TableHead::new("Amount")),
                        ),
                    )
                    .child(
                        TableBody::new().child(
                            TableRow::new()
                                .selected(true)
                                .child(TableCell::new().child("INV001"))
                                .child(TableCell::new().child("$250.00")),
                        ),
                    ),
            )
            .child(Textarea::new(&self.notes))
            .child(Toggle::new("toggle").icon(IconName::Bold).pressed(true))
            .child(
                ToggleGroup::new("toggle-group")
                    .outline()
                    .multiple(true)
                    .icon_item("bold", IconName::Bold, "Bold")
                    .icon_item("italic", IconName::Italic, "Italic")
                    .value(["bold"]),
            )
            .child(H1::new("Heading"))
            .child(H2::new("Heading"))
            .child(P::new("Paragraph"))
            .child(Blockquote::new("Quote"))
            .child(List::new(vec!["One".into(), "Two".into()]).ordered(true))
            .child(InlineCode::new("code"))
            .child(Muted::new("muted"))
            .child(
                AlertDialog::new("alert-dialog")
                    .open(true)
                    .title("Are you sure?")
                    .description("This cannot be undone.")
                    .destructive(true),
            )
            .child(
                Sheet::new("sheet")
                    .open(true)
                    .side(SheetSide::Left)
                    .title("Edit profile")
                    .footer(Button::new("sheet-save").label("Save")),
            )
            .child(Drawer::new("drawer").open(true).title("Move goal"))
            .child(
                CommandDialog::new(
                    "command-dialog",
                    Command::new("palette").items([CommandItem::new("Search")]),
                )
                .open(true),
            )
    }
}

#[gpui::test]
fn shadcn_parity_components_render_in_every_theme(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    cx.update(|cx| {
        toast(
            cx,
            Toast::success("Saved")
                .description("Your changes were saved.")
                .action("Undo", |_, _| {}),
        );
        toast(cx, Toast::loading("Uploading"));
    });
    for preset in ThemePreset::ALL {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            cx.update(|cx| Theme::set_global(Theme::from_preset(preset, mode), cx));
            let (_view, window_context) = cx.add_window_view(|_, cx| ShadcnParityView {
                notes: cx.new(|cx| {
                    InputState::new(cx)
                        .with_multiline(true)
                        .with_text("First line\nSecond line that is long enough to wrap")
                }),
                prompt: cx.new(|cx| InputState::new(cx).with_placeholder("example.com")),
                scroller: MessageScrollerState::new(50),
            });
            window_context.run_until_parked();
            window_context.update(|window, _| window.refresh());
            window_context.run_until_parked();
        }
    }
}

/// Floating surfaces nested inside each other (a menu in a popover in a dialog).
/// GPUI panics on nested `deferred` elements, so inner layers must become portals.
struct NestedLayersView;

impl Render for NestedLayersView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(
            Dialog::new("outer-dialog")
                .open(true)
                .title("Dialog")
                .child(
                    Popover::new("inner-popover")
                        .open(true)
                        .trigger(Button::new("popover-trigger").label("Popover"))
                        .child(
                            DropdownMenu::new("innermost-menu")
                                .open(true)
                                .trigger(Button::new("menu-trigger").label("Menu"))
                                .menu(Menu::new().item(MenuItem::new("Item"))),
                        ),
                )
                .child(
                    ContextMenu::new("context-in-dialog")
                        .menu(Menu::new().item(MenuItem::new("Copy")))
                        .child("Area"),
                ),
        )
    }
}

#[gpui::test]
fn nested_floating_layers_render(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    cx.update(|cx| {
        toast(cx, Toast::new("A toast above everything"));
    });
    let (_view, window_context) = cx.add_window_view(|_, _| NestedLayersView);
    window_context.run_until_parked();
    window_context.update(|window, _| window.refresh());
    window_context.run_until_parked();
}
