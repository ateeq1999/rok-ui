//! The overview page's cards: buttons, data display, settings, alerts and the dialog.

use rok_ui::prelude::*;

use super::{form_field, Gallery};

impl Gallery {
    pub(super) fn buttons_card() -> impl IntoElement {
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
                            .flex_dir()
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
                            .flex_dir()
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
                            .flex_dir()
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
                                    .icon(IconName::ArrowRight.for_direction())
                                    .icon_position(IconPosition::End),
                            )
                            .child(Button::new("loading").loading(true).label("Please wait"))
                            .child(Button::new("disabled").disabled(true).label("Disabled")),
                    ),
            )
    }

    pub(super) fn display_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .flex_dir()
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
                            .flex_dir()
                            .items_center()
                            .gap(px(12.))
                            .child(Avatar::new("AL"))
                            .child(
                                div()
                                    .flex_dir()
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
                                    .flex_dir()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(KeyboardShortcut::new("Ctrl"))
                                    .child(KeyboardShortcut::new("K")),
                            ),
                    )
                    .child(Separator::new())
                    .child(
                        div()
                            .flex_dir()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_dir()
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
                            .flex_dir()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                Skeleton::new("skeleton-avatar")
                                    .size(px(40.))
                                    .rounded_full(),
                            )
                            .child(
                                div()
                                    .flex_dir()
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

    pub(super) fn settings_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_foreground = cx.theme().colors.muted_foreground;
        let tab_panel = match self.selected_tab_index {
            0 => div()
                .flex_dir()
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
                .flex_dir()
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

    pub(super) fn alerts_column() -> impl IntoElement {
        div()
            .flex_dir()
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

    pub(super) fn dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .on_close(cx.listener(|gallery, (): &(), _, cx| {
                gallery.dialog_open = false;
                cx.notify();
            }))
    }
}
