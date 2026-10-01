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
            .child(Progress::new(150.))
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
