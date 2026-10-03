//! StyleX-style styling (`styles!`, `style!`, `sx![]`, `.sx()`) and the
//! children syntaxes (`children![]`, `view!`).

use rok_ui::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Calm,
    Loud,
}

styles! {
    pub PANEL = {
        base: {
            display: flex,
            direction: column,
            gap: 6,
            padding: 6,
            padding_x: 4,
            radius: xl,
            border: 1,
            border_color: border,
            background: card,
            color: card_foreground,
            shadow: xs,
            hover: { border_color: ring },
        },
        compact: { padding: 3, gap: 3 },
        tone(Tone): {
            Calm: { background: muted },
            Loud: { background: primary/90, color: primary_foreground, font: semibold },
        },
    }

    ROW = {
        base: { display: flex, align: center, justify: between, gap: 2, width: full },
        text: { text: sm, line_height: 5, whitespace: nowrap, truncate: true, text: 2xl },
        sizes: { width: 50%, max_width: px(320.), min_height: auto, aspect_ratio: 1.5 },
        position: { position: absolute, top: 0, left: -2, inset: 1 },
        misc: {
            cursor: pointer, opacity: 0.5, grow: 1, shrink: 0, flex: 1, wrap: wrap,
            radius_top_left: md, border_bottom: 2, margin_y: auto, font_family: mono,
            text_align: center, italic: true, underline: false,
        },
    }
}

/// A component taking StyleX-style overrides through `#[sx]`.
#[component]
fn Panel(
    title: SharedString,
    #[prop(optional)] tone: Option<Tone>,
    #[children] children: Vec<AnyElement>,
    #[sx] sx: Sx,
) -> impl IntoElement {
    div()
        .sx((&PANEL.base, tone.map(|tone| PANEL.tone(tone)), &sx))
        .child(title)
        .children(children)
}

#[test]
fn styles_resolve_against_the_theme() {
    let theme = Theme::from_preset(ThemePreset::Neutral, ThemeMode::Light);
    let base = PANEL.base.resolve_base(&theme);
    assert_eq!(base.background, Some(theme.colors.card.into()));
    // `gap: 6` is 24px on the 4px scale.
    assert_eq!(
        base.gap.width,
        Some(gpui::DefiniteLength::Absolute(
            gpui::AbsoluteLength::Pixels(px(24.))
        ))
    );

    // Later styles win: the loud tone replaces the card background.
    let merged = rok_ui::sx::merge((&PANEL.base, PANEL.tone(Tone::Loud)));
    assert_eq!(
        merged.resolve_base(&theme).background,
        Some(theme.colors.primary.opacity(0.9).into())
    );

    // `sx![]` with conditions.
    let compact = true;
    let conditional = sx![PANEL.base, compact => PANEL.compact, false => PANEL.tone(Tone::Calm)];
    assert_eq!(
        conditional.resolve_base(&theme).padding.top,
        Some(gpui::DefiniteLength::Absolute(
            gpui::AbsoluteLength::Pixels(px(12.))
        ))
    );

    // Inline one-off values.
    let width = 120.;
    let inline = style! { width: {px(width)}, background: transparent };
    assert_eq!(
        inline.resolve_base(&theme).size.width,
        Some(gpui::Length::Definite(px(120.).into()))
    );
}

#[test]
fn children_macro_handles_mixed_types_and_control_flow() {
    let loading = false;
    let maybe: Option<&str> = Some("present");
    let items = ["a", "b", "c"];
    let status = 2;
    let user_name = String::from("ليلى");
    let greeting = SharedString::from("Hello");
    let built: Vec<AnyElement> = children![
        "text",
        "مرحبا",
        Badge::new("badge"),
        user_name,
        greeting.clone(),
        if loading { Spinner::new() } else { Badge::new("ready") },
        if let Some(text) = maybe { div().child(text) },
        for item in items => div().child(item),
        for item in items { div().child(item), Badge::new(item) },
        match status {
            1 => "one",
            2 => { Badge::new("two"), "and more" },
            _ => Spinner::new(),
        },
    ];
    // 2 + 2 + 1 + 1 + 1 + 3 + 6 + 2
    assert_eq!(built.len(), 18);
}

struct StyledView {
    compact: bool,
    projects: Vec<&'static str>,
}

impl Render for StyledView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let compact = self.compact;
        let error: Option<&str> = Some("Network unreachable");
        let projects = self.projects.clone();
        AppRoot::new().child(view! {
            Panel("Projects", tone = Tone::Calm, sx = [PANEL.compact, compact => ROW.sizes]) {
                Card {
                    CardHeader {
                        CardTitle("Create project")
                        CardDescription("Deploy your new project in one click.")
                    }
                }
                if let Some(error) = error {
                    Alert("Deploy failed", description = error).destructive()
                }
                for project in projects {
                    Item(project, title = project)
                }
                match compact {
                    true => "compact",
                    false => Badge("roomy"),
                }
                div(sx = ROW.base) {
                    "Raw text"
                    {Button::new("deploy").label("Deploy")}
                    Button("cancel", label = "Cancel").outline()
                }
                div(sx = [ROW.text, ROW.position, ROW.misc]) { "styled" }
                div { "مرحبا بك في rok-ui" }
                div { {format!("{} مشاريع", 3)} {SharedString::from("rok-ui")} }
            }
        })
    }
}

#[gpui::test]
fn styled_views_render_in_every_theme(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    for preset in ThemePreset::ALL {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            cx.update(|cx| Theme::set_global(Theme::from_preset(preset, mode), cx));
            let (_view, window_context) = cx.add_window_view(|_, _| StyledView {
                compact: mode == ThemeMode::Dark,
                projects: vec!["alpha", "beta"],
            });
            window_context.run_until_parked();
        }
    }
}

styles! {
    OVERRIDE = {
        loud: { background: accent, padding_x: 3, radius: full, hover: { background: primary/20 } },
        wide: { width: full, margin_top: 2 },
    }
}

/// Every kind of component accepts `.sx(..)`: structs, `#[component]`s with and
/// without `#[style]`, form controls and icons.
struct OverrideView {
    name: Entity<InputState>,
    notes: Entity<InputState>,
}

impl Render for OverrideView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .child(Badge::new("New").sx(&OVERRIDE.loud))
            .child(Alert::new("Heads up").sx((&OVERRIDE.wide, &OVERRIDE.loud)))
            .child(Input::new(&self.name).sx(&OVERRIDE.wide))
            .child(Textarea::new(&self.notes).sx(&OVERRIDE.wide))
            .child(Select::new("select").option("a", "A").sx(&OVERRIDE.wide))
            .child(Item::new("item").title("Item").sx(&OVERRIDE.loud))
            .child(Checkbox::new("check").label("Check").sx(&OVERRIDE.loud))
            .child(Switch::new("switch").sx(&OVERRIDE.loud))
            .child(Tabs::new("tabs").tab("One").sx(&OVERRIDE.wide))
            .child(Label::new("Label").sx(&OVERRIDE.loud))
            .child(Separator::new().sx(&OVERRIDE.wide))
            .child(Kbd::new("K").sx(&OVERRIDE.loud))
            .child(InlineCode::new("code").sx(&OVERRIDE.loud))
            .child(FieldError::new("Required").sx(&OVERRIDE.wide))
            .child(
                Table::new().sx(&OVERRIDE.wide).child(
                    TableBody::new().sx(&OVERRIDE.loud).child(
                        TableRow::new().child(TableCell::new().sx(&OVERRIDE.loud).child("x")),
                    ),
                ),
            )
            .child(Icon::new(IconName::Bell).sx(&OVERRIDE.loud))
            .child(Skeleton::new("skeleton").sx(&OVERRIDE.wide))
            .child(Progress::new(40_f32).sx(&OVERRIDE.wide))
            .child(Slider::new("slider").value(10.).sx(&OVERRIDE.wide))
            .child(Toggle::new("toggle").label("B").sx(&OVERRIDE.loud))
            .child(Pagination::new("pages", 3).sx(&OVERRIDE.wide))
    }
}

#[gpui::test]
fn every_kind_of_component_takes_sx_overrides(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    for preset in ThemePreset::ALL {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            cx.update(|cx| Theme::set_global(Theme::from_preset(preset, mode), cx));
            let (_view, window_context) = cx.add_window_view(|_, cx| OverrideView {
                name: cx.new(InputState::new),
                notes: cx.new(|cx| InputState::new(cx).with_multiline(true)),
            });
            window_context.run_until_parked();
        }
    }
}

styles! {
    STATES = {
        all: { hover: { background: accent }, focus: { border_color: ring } },
    }
}

/// Hover and focus overrides on every component. GPUI panics if an element gets
/// two hover (or focus) styles, so components must merge their own with these.
struct StatesView {
    input: Entity<InputState>,
    scroller: MessageScrollerState,
}

impl Render for StatesView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let all = &STATES.all;
        let today = CalendarDate::new(2026, 10, 2).unwrap();
        let menu = || Menu::new().item(MenuItem::new("Item"));
        AppRoot::new()
            .sx(all)
            .child(
                Accordion::new("accordion")
                    .item(AccordionItem::new("A"))
                    .sx(all),
            )
            .child(Alert::new("Alert").sx(all))
            .child(AspectRatio::new(2_f32).sx(all))
            .child(
                Attachment::new("attachment", "a.pdf")
                    .on_open(|(), _, _| {})
                    .sx(all),
            )
            .child(Badge::new("Badge").sx(all))
            .child(Breadcrumb::new("crumbs").text("Home").page("Page").sx(all))
            .child(Bubble::new("bubble").child("Hi").sx(all))
            .child(Button::new("button").label("Button").sx(all))
            .child(
                ButtonGroup::new()
                    .item(Button::new("b1").label("1"))
                    .sx(all),
            )
            .child(Calendar::new("calendar").today(today).sx(all))
            .child(Card::new().sx(all).child(CardHeader::new().sx(all)))
            .child(Carousel::new("carousel").item("1").sx(all))
            .child(Chart::new("chart", ChartKind::Bar).sx(all))
            .child(Checkbox::new("checkbox").sx(all))
            .child(Collapsible::new("collapsible").sx(all))
            .child(Combobox::new("combobox").option("a", "A").sx(all))
            .child(
                Command::new("command")
                    .items([CommandItem::new("A")])
                    .sx(all),
            )
            .child(ContextMenu::new("context").menu(menu()).child("x").sx(all))
            .child(
                DataTable::new("table")
                    .column(DataColumn::new("a", "A"))
                    .sx(all),
            )
            .child(DatePicker::new("date").sx(all))
            .child(
                DropdownMenu::new("dropdown")
                    .trigger("Open")
                    .menu(menu())
                    .sx(all),
            )
            .child(Empty::new().title("Empty").sx(all))
            .child(Field::new().sx(all).child(FieldLabel::new("Field").sx(all)))
            .child(HoverCard::new("hover").trigger("Hover").sx(all))
            .child(Input::new(&self.input).sx(all))
            .child(InputGroup::new(&self.input).sx(all))
            .child(InputOtp::new("otp", 4).sx(all))
            .child(Item::new("item").on_click(|_, _, _| {}).sx(all))
            .child(Marker::note("Note").sx(all))
            .child(Menubar::new("menubar").menu("File", menu()).sx(all))
            .child(Message::new().child("Hi").sx(all))
            .child(MessageScroller::new(&self.scroller, |_, _, _| div().into_any_element()).sx(all))
            .child(
                NavigationMenu::new("nav")
                    .link("Docs", |(), _, _| {})
                    .sx(all),
            )
            .child(Pagination::new("pages", 3).sx(all))
            .child(Popover::new("popover").trigger("Open").sx(all))
            .child(Progress::new(10_f32).sx(all))
            .child(
                Questionnaire::new("q")
                    .question(Question::freeform("a", "A"))
                    .sx(all),
            )
            .child(RadioGroup::new("radio").option("a", "A").sx(all))
            .child(
                ResizablePanelGroup::new("resize")
                    .panel(ResizablePanel::new())
                    .sx(all),
            )
            .child(ScrollArea::new("scroll").sx(all))
            .child(Select::new("select").option("a", "A").sx(all))
            .child(Sidebar::new("sidebar").sx(all))
            .child(Skeleton::new("skeleton").sx(all))
            .child(Slider::new("slider").sx(all))
            .child(Switch::new("switch").sx(all))
            .child(Tabs::new("tabs").tab("A").sx(all))
            .child(Toggle::new("toggle").label("T").sx(all))
            .child(ToggleGroup::new("toggles").item("a", "A").sx(all))
            .child(H1::new("Title").sx(all))
            .child(Muted::new("Muted").sx(all))
            .child(Avatar::new("AL").sx(all))
            .child(Spinner::new().sx(all))
            .child(Icon::new(IconName::Bell).sx(all))
    }
}

#[gpui::test]
fn hover_and_focus_overrides_merge_on_every_component(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (_view, window_context) = cx.add_window_view(|_, cx| StatesView {
        input: cx.new(InputState::new),
        scroller: MessageScrollerState::new(3),
    });
    window_context.run_until_parked();
}
