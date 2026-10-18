//! The layout page: cards, tabs, layout widgets and navigation.

use rok_ui::prelude::*;

use super::{centered, column, demo, row, RepositoryRow};

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
