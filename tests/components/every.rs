//! Every component renders under every theme.

use rok_ui::prelude::*;

use super::macros::Greeting;

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
