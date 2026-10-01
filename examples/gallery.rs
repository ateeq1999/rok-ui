//! Every rok-ui component on one screen, with live theme switching.
//!
//! Run with `cargo run --example gallery`.
//! Options: `--dark`, `--preset neutral`, `--open-dialog`.

use rok_ui::prelude::*;

struct GalleryOptions {
    start_in_dark_mode: bool,
    preset: ThemePreset,
    open_dialog_on_start: bool,
}

fn parse_gallery_options() -> GalleryOptions {
    let arguments: Vec<String> = std::env::args().collect();
    let preset = match arguments
        .iter()
        .position(|argument| argument == "--preset")
        .and_then(|index| arguments.get(index + 1))
        .map(String::as_str)
    {
        Some("neutral") => ThemePreset::Neutral,
        _ => ThemePreset::Rok,
    };
    GalleryOptions {
        start_in_dark_mode: arguments.iter().any(|argument| argument == "--dark"),
        preset,
        open_dialog_on_start: arguments.iter().any(|argument| argument == "--open-dialog"),
    }
}

/// The section heading used throughout the gallery.
#[component]
fn SectionTitle(text: SharedString, cx: &mut App) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(cx.theme().colors.muted_foreground)
        .child(text.to_uppercase())
}

/// A sign-up form that owns its own state through hooks, React style.
#[component]
fn CreateAccountCard(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let name_input = use_input_state("create-account-name", window, cx, |state| {
        state.with_placeholder("Ada Lovelace")
    });
    let email_input = use_input_state("create-account-email", window, cx, |state| {
        state.with_placeholder("ada@example.com")
    });
    let password_input = use_input_state("create-account-password", window, cx, |state| {
        state
            .with_placeholder("At least 12 characters")
            .with_masked_text(true)
    });
    let terms_accepted = use_state(window, cx, || false);
    let submitting = use_state(window, cx, || false);
    let is_submitting = submitting.get(cx);

    Card::new()
        .child(
            CardHeader::new()
                .child(CardTitle::new("Create an account"))
                .child(CardDescription::new("Enter your details to get started.")),
        )
        .child(
            CardContent::new()
                .child(form_field("Name", Input::new(&name_input)))
                .child(form_field(
                    "Email",
                    Input::new(&email_input).leading_icon(IconName::Mail),
                ))
                .child(form_field("Password", Input::new(&password_input)))
                .child(
                    Checkbox::new("terms")
                        .checked(terms_accepted.get(cx))
                        .label("I accept the terms and conditions")
                        .on_change({
                            let terms_accepted = terms_accepted.clone();
                            move |checked, _, cx| terms_accepted.set(*checked, cx)
                        }),
                ),
        )
        .child(
            CardFooter::new()
                .justify_end()
                .child(Button::new("cancel-account").outline().label("Cancel"))
                .child(
                    Button::new("create-account")
                        .label(if is_submitting {
                            "Creating…"
                        } else {
                            "Create account"
                        })
                        .loading(is_submitting)
                        .disabled(!terms_accepted.get(cx))
                        .on_click(move |_, _, cx| submitting.set(true, cx)),
                ),
        )
}

fn form_field(label: &'static str, control: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(Label::new(label))
        .child(control)
}

struct Gallery {
    dialog_open: bool,
    selected_tab_index: usize,
    notifications_enabled: bool,
    marketing_emails_enabled: bool,
    upload_progress: f32,
    display_name_input: Entity<InputState>,
    current_password_input: Entity<InputState>,
}

impl Gallery {
    fn new(options: &GalleryOptions, cx: &mut Context<Self>) -> Self {
        Self {
            dialog_open: options.open_dialog_on_start,
            selected_tab_index: 0,
            notifications_enabled: true,
            marketing_emails_enabled: false,
            upload_progress: 64.,
            display_name_input: cx.new(|cx| InputState::new(cx).with_text("Ada Lovelace")),
            current_password_input: cx.new(|cx| {
                InputState::new(cx)
                    .with_placeholder("Current password")
                    .with_masked_text(true)
            }),
        }
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let current_preset = theme.preset;
        let is_dark = theme.mode.is_dark();
        let colors = theme.colors.clone();
        let selected_preset_index = ThemePreset::ALL
            .iter()
            .position(|preset| *preset == current_preset)
            .unwrap_or(0);

        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .h(px(64.))
            .px(px(32.))
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.background)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(rok_logo_mark(colors.primary, colors.primary_foreground))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child("rok-ui"),
                    )
                    .child(Badge::new("v0.1").variant(BadgeVariant::Secondary)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        Tabs::new("theme-preset")
                            .w(px(180.))
                            .tab("Rok")
                            .tab("Neutral")
                            .selected_index(selected_preset_index)
                            .on_change(|index, _, cx| {
                                Theme::change_preset(ThemePreset::ALL[*index], cx)
                            }),
                    )
                    .child(
                        Button::new("toggle-theme")
                            .outline()
                            .icon_only(if is_dark {
                                IconName::Sun
                            } else {
                                IconName::Moon
                            })
                            .tooltip(if is_dark {
                                "Switch to light"
                            } else {
                                "Switch to dark"
                            })
                            .on_click(|_, _, cx| Theme::toggle_mode(cx)),
                    ),
            )
    }

    fn buttons_card(&self) -> impl IntoElement {
        Card::new()
            .child(
                CardHeader::new()
                    .child(CardTitle::new("Button"))
                    .child(CardDescription::new(
                        "Six variants, four sizes, icons and states.",
                    )),
            )
            .child(
                CardContent::new()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(8.))
                            .child(Button::new("primary").label("Primary"))
                            .child(Button::new("secondary").secondary().label("Secondary"))
                            .child(Button::new("outline").outline().label("Outline"))
                            .child(Button::new("ghost").ghost().label("Ghost"))
                            .child(Button::new("destructive").destructive().label("Delete"))
                            .child(Button::new("link").link().label("Link")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .child(Button::new("small").small().label("Small"))
                            .child(Button::new("medium").label("Medium"))
                            .child(Button::new("large").large().label("Large"))
                            .child(
                                Button::new("icon")
                                    .outline()
                                    .icon_only(IconName::Bell)
                                    .tooltip("Notifications"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                Button::new("with-icon")
                                    .icon(IconName::Mail)
                                    .label("Login with email"),
                            )
                            .child(
                                Button::new("continue")
                                    .secondary()
                                    .label("Continue")
                                    .icon(IconName::ArrowRight)
                                    .icon_position(IconPosition::End),
                            )
                            .child(Button::new("loading").loading(true).label("Please wait"))
                            .child(Button::new("disabled").disabled(true).label("Disabled")),
                    ),
            )
    }

    fn display_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Card::new()
            .child(
                CardHeader::new()
                    .child(CardTitle::new("Display"))
                    .child(CardDescription::new(
                        "Badges, avatars, shortcuts and loading states.",
                    )),
            )
            .child(
                CardContent::new()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(8.))
                            .child(Badge::new("Primary"))
                            .child(Badge::new("Secondary").variant(BadgeVariant::Secondary))
                            .child(Badge::new("Outline").variant(BadgeVariant::Outline))
                            .child(
                                Badge::new("Failed")
                                    .variant(BadgeVariant::Destructive)
                                    .icon(IconName::TriangleAlert),
                            )
                            .child(
                                Badge::new("Verified")
                                    .variant(BadgeVariant::Outline)
                                    .icon(IconName::CircleCheck),
                            ),
                    )
                    .child(Separator::new())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(Avatar::new("AL"))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Ada Lovelace"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().colors.muted_foreground)
                                            .child("ada@example.com"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(KeyboardShortcut::new("Ctrl"))
                                    .child(KeyboardShortcut::new("K")),
                            ),
                    )
                    .child(Separator::new())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_sm()
                                    .child("Uploading assets")
                                    .child(
                                        div()
                                            .text_color(cx.theme().colors.muted_foreground)
                                            .child(format!("{}%", self.upload_progress as i32)),
                                    ),
                            )
                            .child(Progress::new(self.upload_progress)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                Skeleton::new("skeleton-avatar")
                                    .size(px(40.))
                                    .rounded_full(),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.))
                                    .child(
                                        Skeleton::new("skeleton-line-one").h(px(14.)).w(px(220.)),
                                    )
                                    .child(
                                        Skeleton::new("skeleton-line-two").h(px(14.)).w(px(160.)),
                                    ),
                            )
                            .child(Spinner::new()),
                    ),
            )
    }

    fn settings_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_foreground = cx.theme().colors.muted_foreground;
        let tab_panel = match self.selected_tab_index {
            0 => div()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(form_field(
                    "Display name",
                    Input::new(&self.display_name_input),
                ))
                .child(
                    Switch::new("notifications")
                        .checked(self.notifications_enabled)
                        .label("Push notifications")
                        .on_change(cx.listener(|gallery, checked: &bool, _, cx| {
                            gallery.notifications_enabled = *checked;
                            cx.notify();
                        })),
                )
                .child(
                    Switch::new("marketing")
                        .checked(self.marketing_emails_enabled)
                        .label("Marketing emails")
                        .on_change(cx.listener(|gallery, checked: &bool, _, cx| {
                            gallery.marketing_emails_enabled = *checked;
                            cx.notify();
                        })),
                )
                .child(
                    Switch::new("locked")
                        .checked(true)
                        .disabled(true)
                        .label("Security alerts (required)"),
                ),
            _ => div()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(12.))
                .child(
                    div()
                        .text_sm()
                        .text_color(muted_foreground)
                        .child("Changing your password signs you out on every other device."),
                )
                .child(
                    Button::new("open-dialog")
                        .outline()
                        .label("Change password…")
                        .on_click(cx.listener(|gallery, _, _, cx| {
                            gallery.dialog_open = true;
                            cx.notify();
                        })),
                ),
        };

        Card::new()
            .child(
                CardHeader::new()
                    .child(CardTitle::new("Settings"))
                    .child(CardDescription::new(
                        "Tabs, switches and a dialog, driven by view state.",
                    )),
            )
            .child(
                CardContent::new()
                    .child(
                        Tabs::new("settings-tabs")
                            .tab("Account")
                            .tab("Security")
                            .selected_index(self.selected_tab_index)
                            .on_change(cx.listener(|gallery, index: &usize, _, cx| {
                                gallery.selected_tab_index = *index;
                                cx.notify();
                            })),
                    )
                    .child(tab_panel),
            )
    }

    fn alerts_column(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                Alert::new("Heads up!")
                    .icon(IconName::Info)
                    .description("Components read every color from the active theme."),
            )
            .child(
                Alert::new("Your session has expired")
                    .destructive()
                    .icon(IconName::TriangleAlert)
                    .description("Please sign in again to continue."),
            )
    }

    fn dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Dialog::new("change-password-dialog")
            .open(self.dialog_open)
            .title("Change password")
            .description("You will be signed out on every other device.")
            .child(form_field(
                "Current password",
                Input::new(&self.current_password_input),
            ))
            .footer(
                Button::new("dialog-cancel")
                    .outline()
                    .label("Cancel")
                    .on_click(cx.listener(|gallery, _, _, cx| {
                        gallery.dialog_open = false;
                        cx.notify();
                    })),
            )
            .footer(
                Button::new("dialog-confirm")
                    .label("Update password")
                    .on_click(cx.listener(|gallery, _, _, cx| {
                        gallery.dialog_open = false;
                        cx.notify();
                    })),
            )
            .on_close(cx.listener(|gallery, _: &(), _, cx| {
                gallery.dialog_open = false;
                cx.notify();
            }))
    }
}

/// The rok mark: three offset bars, built from divs so it needs no image.
fn rok_logo_mark(background: Hsla, bar_color: Hsla) -> impl IntoElement {
    let bar = |width: f32, left_offset: f32, opacity: f32| {
        div()
            .h(px(4.))
            .w(px(width))
            .ml(px(left_offset))
            .bg(bar_color)
            .opacity(opacity)
    };
    div()
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.))
        .size(px(32.))
        .pl(px(7.))
        .bg(background)
        .child(bar(18., 0., 1.0))
        .child(bar(14., 4., 0.8))
        .child(bar(18., 0., 0.6))
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .child(self.header(cx))
            .child(
                div()
                    .id("gallery-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .flex()
                            .gap(px(24.))
                            .p(px(32.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(16.))
                                    .child(SectionTitle::new("Actions"))
                                    .child(self.buttons_card())
                                    .child(SectionTitle::new("Feedback"))
                                    .child(self.alerts_column())
                                    .child(SectionTitle::new("Data display"))
                                    .child(self.display_card(cx)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(16.))
                                    .child(SectionTitle::new("Forms"))
                                    .child(CreateAccountCard::new())
                                    .child(SectionTitle::new("Navigation"))
                                    .child(self.settings_card(cx)),
                            ),
                    ),
            )
            .child(self.dialog(cx))
    }
}

fn main() {
    env_logger::init();
    let options = parse_gallery_options();
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(move |cx: &mut App| {
            rok_ui::init(cx);
            let mode = if options.start_in_dark_mode {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            };
            Theme::set_global(Theme::from_preset(options.preset, mode), cx);

            let bounds = Bounds::centered(None, gpui::size(px(1200.), px(900.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("rok-ui gallery".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| Gallery::new(&options, cx)),
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}
