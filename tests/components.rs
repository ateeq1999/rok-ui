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
        .when_some(title, gpui::ParentElement::child)
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
        greeting.title.as_ref().map(std::convert::AsRef::as_ref),
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
                    .on_remove(|(), _, _| {}),
            )
            .child(
                Attachment::new("failed-attachment", "photo.png")
                    .state(AttachmentState::Failed("Upload failed".into()))
                    .on_retry(|(), _, _| {}),
            )
            .child(
                Breadcrumb::new("breadcrumb")
                    .link("Home", |(), _, _| {})
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
                    .link("Docs", |(), _, _| {}),
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
            .child(
                SizedBox::new(px(800.), px(400.)).child(
                    AdaptiveScaffold::new("adaptive")
                        .app_bar(AppBar::new().title("Photos").center_title(true))
                        .destination(NavigationDestination::new(IconName::Home, "Home"))
                        .destination(
                            NavigationDestination::new(IconName::Heart, "Favorites").badge(""),
                        )
                        .floating_action_button(
                            FloatingActionButton::new("adaptive-fab", IconName::Plus).label("Add"),
                        )
                        .child(
                            Stack::new()
                                .alignment(Alignment::Center)
                                .child(SizedBox::square(px(40.)))
                                .positioned(Positioned::fill().child("Overlay")),
                        ),
                ),
            )
            .child(
                SizedBox::new(px(600.), px(300.)).child(
                    Scaffold::new("scaffold")
                        .app_bar(
                            AppBar::new()
                                .title("Mail")
                                .bottom(Tabs::new("mail-tabs").tab("All")),
                        )
                        .drawer(
                            NavigationDrawer::new("mail-drawer")
                                .section("Mail")
                                .divider(),
                        )
                        .end_drawer("Filters")
                        .navigation(
                            NavigationRail::new("rail")
                                .extended(true)
                                .destination(NavigationDestination::new(IconName::Inbox, "Inbox")),
                        )
                        .bottom_navigation_bar(
                            NavigationBar::new("bottom-nav")
                                .label_behavior(NavigationLabelBehavior::OnlyShowSelected)
                                .destination(NavigationDestination::new(IconName::Inbox, "Inbox"))
                                .destination(
                                    NavigationDestination::new(IconName::Send, "Sent")
                                        .disabled(true),
                                ),
                        )
                        .bottom_sheet("Draft saved")
                        .footer_button(Button::new("scaffold-send").label("Send"))
                        .fab_location(FabLocation::CenterFloat)
                        .child(
                            Column::new()
                                .main_axis_size(MainAxisSize::Min)
                                .child(Padding::symmetric(px(8.), px(4.)).child("Padded"))
                                .child(Center::new().child("Centered"))
                                .child(Aligned::new(Alignment::BottomEnd).child("Aligned"))
                                .child(
                                    Row::new()
                                        .child(Flexible::new().child("Flexible"))
                                        .child(Expanded::new().flex(2.).child("Expanded")),
                                )
                                .child(Wrap::new().spacing(px(4.)).children(["a", "b", "c"]))
                                .child(
                                    GridView::extent("scaffold-grid", px(100.))
                                        .children(["1", "2", "3"]),
                                )
                                .child(LayoutBuilder::new(
                                    "scaffold-layout",
                                    |constraints, _, _| {
                                        format!("{:?}", constraints.size_class()).into_any_element()
                                    },
                                )),
                        ),
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

#[gpui::test]
fn shadcn_parity_components_render_right_to_left(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    cx.update(|cx| set_text_direction(TextDirection::Rtl, cx));
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        cx.update(|cx| Theme::set_global(Theme::from_preset(ThemePreset::Rok, mode), cx));
        let (_view, window_context) = cx.add_window_view(|_, cx| ShadcnParityView {
            notes: cx.new(|cx| {
                InputState::new(cx)
                    .with_multiline(true)
                    .with_text("سطر أول\nسطر ثان")
            }),
            prompt: cx.new(|cx| InputState::new(cx).with_placeholder("example.com")),
            scroller: MessageScrollerState::new(50),
        });
        window_context.run_until_parked();
        window_context.update(|window, _| window.refresh());
        window_context.run_until_parked();
    }
    cx.update(|cx| set_text_direction(TextDirection::Ltr, cx));
}

/// Records the laid-out width of a `ButtonGroup` and of its three buttons on their own.
struct ButtonGroupWidthView(std::rc::Rc<std::cell::Cell<(f32, f32)>>);

impl Render for ButtonGroupWidthView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let widths = self.0.clone();
        let probe = move |slot: usize| {
            let widths = widths.clone();
            gpui::canvas(
                move |bounds, _, _| {
                    let (group, row) = widths.get();
                    let width = f32::from(bounds.size.width);
                    widths.set(if slot == 0 {
                        (width, row)
                    } else {
                        (group, width)
                    });
                },
                |_, (), _, _| {},
            )
            .absolute()
            .size_full()
        };
        let buttons = |prefix: &'static str| {
            ["Archive", "Report", "Snooze"]
                .map(|label| Button::new((prefix, label.len())).outline().label(label))
        };
        let [archive, report, snooze] = buttons("group");
        let [plain_archive, plain_report, plain_snooze] = buttons("plain");
        div()
            .w(px(600.))
            .flex()
            .flex_wrap()
            .gap(px(8.))
            .child(
                div()
                    .relative()
                    .child(probe(0))
                    .child(ButtonGroup::new().item(archive).item(report).item(snooze)),
            )
            .child(
                div().relative().child(probe(1)).child(
                    div()
                        .flex()
                        .child(plain_archive)
                        .child(plain_report)
                        .child(plain_snooze),
                ),
            )
    }
}

#[gpui::test]
fn button_group_is_as_wide_as_its_items(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let widths = std::rc::Rc::new(std::cell::Cell::new((0., 0.)));
    let view_widths = widths.clone();
    let (_view, window_context) =
        cx.add_window_view(move |_, _| ButtonGroupWidthView(view_widths.clone()));
    window_context.run_until_parked();
    let (group, row) = widths.get();
    // Joined items share a border, so the group is at most a few pixels narrower.
    assert!(row > 200., "buttons measured {row}px");
    assert!(
        (row - group).abs() <= 3.,
        "group {group}px vs buttons {row}px"
    );
}

use gpui::Focusable as _;

/// An input outside a dialog and two inside it.
struct FocusTrapView {
    outside: Entity<InputState>,
    first: Entity<InputState>,
    second: Entity<InputState>,
}

impl Render for FocusTrapView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(Input::new(&self.outside)).child(
            Dialog::new("trap")
                .open(true)
                .title("Edit")
                .child(Input::new(&self.first))
                .child(Input::new(&self.second)),
        )
    }
}

#[gpui::test]
fn dialog_focuses_its_first_field_and_traps_tab(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (view, window_context) = cx.add_window_view(|_, cx| FocusTrapView {
        outside: cx.new(InputState::new),
        first: cx.new(InputState::new),
        second: cx.new(InputState::new),
    });
    window_context.run_until_parked();
    window_context.update(|window, _| window.refresh());
    window_context.run_until_parked();

    let focused = |window_context: &mut gpui::VisualTestContext,
                   pick: fn(&FocusTrapView) -> &Entity<InputState>| {
        let view = view.clone();
        window_context.update(move |window, cx| {
            let state = pick(view.read(cx)).clone();
            state.read(cx).focus_handle(cx).is_focused(window)
        })
    };

    assert!(
        focused(window_context, |view| &view.first),
        "opening the dialog focuses its first field"
    );
    // Tab cycles through the dialog's own focusable elements and never escapes.
    for _ in 0..8 {
        window_context.simulate_keystrokes("tab");
        window_context.run_until_parked();
        assert!(!focused(window_context, |view| &view.outside));
    }
    for _ in 0..8 {
        window_context.simulate_keystrokes("shift-tab");
        window_context.run_until_parked();
        assert!(!focused(window_context, |view| &view.outside));
    }
    // From the first field, Shift-Tab wraps to the end and Tab wraps back to it.
    window_context.update({
        let view = view.clone();
        move |window, cx| {
            let first = view.read(cx).first.read(cx).focus_handle(cx);
            window.focus(&first);
        }
    });
    window_context.simulate_keystrokes("shift-tab");
    window_context.run_until_parked();
    assert!(!focused(window_context, |view| &view.first));
    window_context.simulate_keystrokes("tab");
    window_context.run_until_parked();
    assert!(
        focused(window_context, |view| &view.first),
        "Tab from the last element wraps to the first"
    );
}

/// A popover whose content holds a second popover. Records each open change as
/// `(popover, open)`.
struct NestedPopoverView(std::rc::Rc<std::cell::RefCell<Vec<(&'static str, bool)>>>);

impl Render for NestedPopoverView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let record = |name: &'static str| {
            let log = self.0.clone();
            move |open: &bool, _: &mut Window, _: &mut App| log.borrow_mut().push((name, *open))
        };
        AppRoot::new().p(px(40.)).child(
            Popover::new("outer")
                .on_open_change(record("outer"))
                .trigger(
                    div()
                        .debug_selector(|| "outer-trigger".into())
                        .child(Button::new("outer-button").label("Outer")),
                )
                .child(
                    Popover::new("inner")
                        .on_open_change(record("inner"))
                        .trigger(
                            div()
                                .debug_selector(|| "inner-trigger".into())
                                .child(Button::new("inner-button").label("Inner")),
                        )
                        .child(
                            div()
                                .debug_selector(|| "inner-body".into())
                                .h(px(60.))
                                .child("Inner content"),
                        ),
                ),
        )
    }
}

#[gpui::test]
fn clicking_a_nested_popover_keeps_its_parent_open(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let view_log = log.clone();
    let (_view, window_context) =
        cx.add_window_view(move |_, _| NestedPopoverView(view_log.clone()));
    window_context.run_until_parked();

    let mut click = |selector: &'static str| {
        let bounds = window_context
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} is not rendered"));
        window_context.simulate_click(bounds.center(), gpui::Modifiers::none());
        window_context.run_until_parked();
    };
    click("outer-trigger");
    click("inner-trigger");
    assert_eq!(*log.borrow(), [("outer", true), ("inner", true)]);

    // Pressing inside the inner popover is outside the outer one, but must not close it.
    click("inner-body");
    assert_eq!(
        *log.borrow(),
        [("outer", true), ("inner", true)],
        "a press inside a nested popover closes nothing"
    );
}

struct FocusVisibleView;

impl Render for FocusVisibleView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .child(Button::new("first").label("First"))
            .child(Button::new("second").label("Second"))
    }
}

#[gpui::test]
fn focus_rings_follow_the_last_input_device(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (_view, window_context) = cx.add_window_view(|_, _| FocusVisibleView);
    window_context.run_until_parked();

    window_context.simulate_keystrokes("tab");
    window_context.run_until_parked();
    assert!(
        rok_ui::sx::focus_visible(),
        "keyboard input shows focus rings"
    );

    window_context.simulate_click(gpui::point(px(10.), px(10.)), gpui::Modifiers::none());
    window_context.run_until_parked();
    assert!(!rok_ui::sx::focus_visible(), "a click hides them");

    rok_ui::sx::set_focus_ring_mode(rok_ui::sx::FocusRingMode::Always);
    assert!(rok_ui::sx::focus_visible(), "Always shows them regardless");
    rok_ui::sx::set_focus_ring_mode(rok_ui::sx::FocusRingMode::KeyboardOnly);
}

/// A row and a grid of three labeled boxes, right to left.
struct RtlLayoutView;

impl Render for RtlLayoutView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let cell = |name: &'static str| div().debug_selector(move || name.into()).size(px(40.));
        AppRoot::new().child(
            Direction::new(TextDirection::Rtl).child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .w(px(600.))
                    .child(
                        Row::new()
                            .spacing(px(8.))
                            .child(cell("row-first"))
                            .child(Spacer::new())
                            .child(cell("row-last")),
                    )
                    .child(GridView::count(3).children(["grid-0", "grid-1", "grid-2"].map(cell)))
                    .child(cell("column-start")),
            ),
        )
    }
}

#[gpui::test]
fn layout_widgets_follow_the_reading_direction(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (_view, window_context) = cx.add_window_view(|_, _| RtlLayoutView);
    window_context.run_until_parked();
    let mut left = |selector: &'static str| {
        window_context
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} is not rendered"))
            .left()
    };
    // The row starts on the right and the spacer pushes its last child to the left.
    assert!(left("row-first") > left("row-last"));
    assert_eq!(left("row-first") - left("row-last"), px(560.));
    // Grid cells fill from the right.
    assert!(left("grid-0") > left("grid-1"));
    assert!(left("grid-1") > left("grid-2"));
    // A column's cross-axis start is the right edge.
    assert_eq!(left("column-start"), left("row-first"));
}

/// A scaffold whose start drawer is controlled by the view.
struct DrawerShellView {
    drawer_open: bool,
    page: usize,
    log: std::rc::Rc<std::cell::RefCell<Vec<bool>>>,
}

impl Render for DrawerShellView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let log = self.log.clone();
        AppRoot::new().child(
            Scaffold::new("shell")
                .app_bar(AppBar::new().title("Mail"))
                .drawer(
                    NavigationDrawer::new("drawer")
                        .destination(NavigationDestination::new(IconName::Inbox, "Inbox"))
                        .destination(NavigationDestination::new(IconName::Send, "Sent"))
                        .selected_index(self.page)
                        .on_change(cx.listener(|view, index: &usize, _, cx| {
                            view.page = *index;
                            cx.notify();
                        })),
                )
                .drawer_open(self.drawer_open)
                .on_drawer_change(cx.listener(move |view, open: &bool, _, cx| {
                    log.borrow_mut().push(*open);
                    view.drawer_open = *open;
                    cx.notify();
                }))
                .child("Messages"),
        )
    }
}

#[gpui::test]
fn scaffold_drawer_opens_from_the_menu_and_closes_on_navigation(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    // Drawers slide in on real time; without motion they open in one frame.
    rok_ui::motion::set_reduced_motion(true);
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let view_log = log.clone();
    let (view, window_context) = cx.add_window_view(move |_, _| DrawerShellView {
        drawer_open: false,
        page: 0,
        log: view_log.clone(),
    });
    window_context.run_until_parked();

    // The app bar's implied menu button: 8 px padding plus half of a 36 px button.
    window_context.simulate_click(gpui::point(px(26.), px(32.)), gpui::Modifiers::none());
    window_context.run_until_parked();
    assert_eq!(*log.borrow(), [true], "the menu button opens the drawer");

    // Pick the second destination: rows are 48 px under 12 px of padding, 2 px apart.
    window_context.simulate_click(gpui::point(px(100.), px(86.)), gpui::Modifiers::none());
    window_context.run_until_parked();
    assert_eq!(
        *log.borrow(),
        [true, false],
        "picking a destination closes it"
    );
    assert_eq!(view.read_with(window_context, |view, _| view.page), 1);
    rok_ui::motion::set_reduced_motion(false);
}
