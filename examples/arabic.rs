//! An Arabic, right-to-left settings screen set in the Cairo font.
//!
//! Run with `cargo run --example arabic --features font-cairo`. Add
//! `font-noto-sans-arabic` for a button that switches to Noto Sans Arabic.
//!
//! - `rok_ui::fonts::CAIRO` registers the bundled Cairo font (Google Fonts, OFL).
//! - `set_text_direction(TextDirection::Rtl, cx)` mirrors every component.
//! - Component text (labels, titles, options) is reordered for display
//!   automatically. Wrap text you render yourself in `BidiText`.

use rok_ui::prelude::*;

/// Shorthand for Arabic text rendered by this example itself.
fn ar(text: &'static str) -> BidiText {
    BidiText::new(text)
}

const INVOICES: [(&str, &str, &str, &str); 4] = [
    ("INV-001", "مدفوعة", "بطاقة ائتمان", "٢٥٠٫٠٠ ر.س"),
    ("INV-002", "قيد الانتظار", "تحويل بنكي", "١٥٠٫٠٠ ر.س"),
    ("INV-003", "غير مدفوعة", "Apple Pay", "٣٥٠٫٠٠ ر.س"),
    ("INV-004", "مدفوعة", "بطاقة ائتمان", "٤٥٠٫٠٠ ر.س"),
];

#[component]
fn ProfileCard(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let name = use_input_state("ar-name", window, cx, |state| {
        state.with_text("ليلى أحمد").with_placeholder("الاسم الكامل")
    });
    let email = use_input_state("ar-email", window, cx, |state| {
        state.with_placeholder("name@example.com")
    });
    let bio = use_textarea_state("ar-bio", window, cx, |state| {
        state.with_placeholder("اكتب نبذة قصيرة عنك…").with_text(
            "مصممة واجهات من الرياض، أعمل على تطبيقات سطح المكتب منذ 2019. \
                 أحب الخطوط العربية وتجربة المستخدم البسيطة.",
        )
    });
    let city = use_state(window, cx, || Some(SharedString::from("riyadh")));
    let plan = use_state(window, cx, || Some(SharedString::from("pro")));

    Card::new()
        .child(
            CardHeader::new()
                .child(CardTitle::new("الملف الشخصي"))
                .child(CardDescription::new(
                    "حدّث بياناتك الشخصية. ستظهر هذه المعلومات للأعضاء الآخرين في مساحة العمل.",
                )),
        )
        .child(
            CardContent::new().child(
                FieldGroup::new()
                    .child(
                        Field::new()
                            .child(FieldLabel::new("الاسم الكامل"))
                            .child(Input::new(&name)),
                    )
                    .child(
                        Field::new()
                            .child(FieldLabel::new("البريد الإلكتروني"))
                            .child(Input::new(&email).leading_icon(IconName::Mail))
                            .child(FieldDescription::new("لن نشارك بريدك مع أي جهة.")),
                    )
                    .child(
                        Field::new().child(FieldLabel::new("المدينة")).child(
                            Select::new("ar-city")
                                .placeholder("اختر مدينة")
                                .option("riyadh", "الرياض")
                                .option("cairo", "القاهرة")
                                .option("dubai", "دبي")
                                .option("casablanca", "الدار البيضاء")
                                .value(city.get(cx))
                                .on_change({
                                    let city = city.clone();
                                    move |value, _, cx| city.set(Some(value.clone()), cx)
                                }),
                        ),
                    )
                    .child(
                        Field::new()
                            .child(FieldLabel::new("نبذة"))
                            .child(Textarea::new(&bio)),
                    )
                    .child(
                        Field::new().child(FieldLabel::new("الخطة")).child(
                            RadioGroup::new("ar-plan")
                                .option_with_description(
                                    "free",
                                    "مجانية",
                                    "مشروع واحد وحتى ثلاثة أعضاء.",
                                )
                                .option_with_description(
                                    "pro",
                                    "احترافية",
                                    "مشاريع غير محدودة ودعم ذو أولوية.",
                                )
                                .value(plan.get(cx))
                                .on_change({
                                    let plan = plan.clone();
                                    move |value, _, cx| plan.set(Some(value.clone()), cx)
                                }),
                        ),
                    ),
            ),
        )
        .child(
            CardFooter::new()
                .child(Button::new("ar-save").label("حفظ التغييرات"))
                .child(Button::new("ar-cancel").outline().label("إلغاء")),
        )
}

#[component]
fn NotificationsCard(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let email = use_state(window, cx, || true);
    let push = use_state(window, cx, || false);
    let toggle = |state: &State<bool>| {
        let state = state.clone();
        move |checked: &bool, _: &mut Window, cx: &mut App| state.set(*checked, cx)
    };

    Card::new()
        .child(
            CardHeader::new()
                .child(CardTitle::new("الإشعارات"))
                .child(CardDescription::new("اختر طريقة تلقي التنبيهات.")),
        )
        .child(
            CardContent::new()
                .child(
                    Switch::new("ar-email-notifications")
                        .label("إشعارات البريد الإلكتروني")
                        .checked(email.get(cx))
                        .on_change(toggle(&email)),
                )
                .child(
                    Switch::new("ar-push-notifications")
                        .label("الإشعارات الفورية")
                        .checked(push.get(cx))
                        .on_change(toggle(&push)),
                )
                .child(
                    Alert::new("تنبيه")
                        .icon(IconName::Info)
                        .description("يمكنك تغيير هذه الإعدادات في أي وقت من صفحة الحساب."),
                ),
        )
}

#[component]
fn InvoicesCard() -> impl IntoElement {
    let status_badge = |status: &'static str| {
        let variant = match status {
            "مدفوعة" => BadgeVariant::Primary,
            "قيد الانتظار" => BadgeVariant::Secondary,
            _ => BadgeVariant::Destructive,
        };
        Badge::new(status).variant(variant)
    };

    Card::new()
        .child(
            CardHeader::new()
                .child(CardTitle::new("الفواتير"))
                .child(CardDescription::new("آخر أربع فواتير لحسابك.")),
        )
        .child(
            CardContent::new().child(
                Table::new()
                    .child(
                        TableHeader::new().child(
                            TableRow::new()
                                .child(TableHead::new("الفاتورة").w(px(110.)).flex_none())
                                .child(TableHead::new("الحالة"))
                                .child(TableHead::new("طريقة الدفع"))
                                .child(TableHead::new("المبلغ")),
                        ),
                    )
                    .child(TableBody::new().children(INVOICES.iter().map(
                        |(invoice, status, method, amount)| {
                            TableRow::new()
                                .child(TableCell::new().w(px(110.)).flex_none().child(*invoice))
                                .child(TableCell::new().child(status_badge(status)))
                                .child(TableCell::new().child(ar(method)))
                                .child(TableCell::new().child(ar(amount)))
                        },
                    ))),
            ),
        )
}

/// With `font-noto-sans-arabic` enabled, a button that switches between Cairo and
/// Noto Sans Arabic.
fn font_switch() -> Option<Button> {
    #[cfg(feature = "font-noto-sans-arabic")]
    {
        Some(
            Button::new("ar-font")
                .outline()
                .label("تبديل الخط")
                .on_click(|_, _, cx| {
                    let cairo = rok_ui::fonts::CAIRO.family();
                    let next = if Theme::global(cx).font_family == cairo {
                        rok_ui::fonts::NOTO_SANS_ARABIC.family()
                    } else {
                        cairo
                    };
                    Theme::set_font_family(next, cx);
                }),
        )
    }
    #[cfg(not(feature = "font-noto-sans-arabic"))]
    None
}

struct ArabicWindow {
    dialog_open: bool,
}

impl Render for ArabicWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open_dialog = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.dialog_open = true;
            cx.notify();
        });
        let close_dialog = cx.listener(|this, _: &(), _, cx| {
            this.dialog_open = false;
            cx.notify();
        });

        AppRoot::new().child(
            div()
                .id("arabic-page")
                .size_full()
                .overflow_y_scroll()
                .p(px(32.))
                .flex_dir()
                .flex_col()
                .gap(px(24.))
                .child(
                    div()
                        .flex_dir()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex_dir()
                                .flex_col()
                                .gap(px(4.))
                                .child(H2::new("إعدادات الحساب"))
                                .child(Muted::new("إدارة ملفك الشخصي وإشعاراتك وفواتيرك.")),
                        )
                        .child(
                            div().flex_dir().gap(px(8.)).children(font_switch()).child(
                                Button::new("ar-theme")
                                    .outline()
                                    .icon(IconName::Moon)
                                    .label("الوضع الداكن")
                                    .on_click(|_, _, cx| Theme::toggle_mode(cx)),
                            ),
                        ),
                )
                .child(P::new(
                    "واجهة روك مكتبة مكونات لتطبيقات سطح المكتب مبنية على GPUI، \
                     تدعم الكتابة من اليمين إلى اليسار والنصوص المختلطة مثل Rust 1.88 \
                     والأرقام ١٢٣ في السطر نفسه، مع التفاف الأسطر الطويلة بشكل صحيح.",
                ))
                .child(
                    div()
                        .flex_dir()
                        .items_start()
                        .gap(px(24.))
                        .child(div().flex_1().child(ProfileCard::new()))
                        .child(
                            div()
                                .flex_1()
                                .flex_dir()
                                .flex_col()
                                .gap(px(24.))
                                .child(NotificationsCard::new())
                                .child(InvoicesCard::new())
                                .child(
                                    Button::new("ar-delete")
                                        .destructive()
                                        .icon(IconName::Trash)
                                        .label("حذف الحساب")
                                        .on_click(open_dialog),
                                ),
                        ),
                )
                .child(
                    Dialog::new("ar-delete-dialog")
                        .open(self.dialog_open)
                        .title("هل أنت متأكد تماماً؟")
                        .description(
                            "لا يمكن التراجع عن هذا الإجراء. سيُحذف حسابك وجميع بياناتك نهائياً.",
                        )
                        .footer(Button::new("ar-dialog-cancel").outline().label("إلغاء"))
                        .footer(
                            Button::new("ar-dialog-confirm")
                                .destructive()
                                .label("نعم، احذف الحساب"),
                        )
                        .on_close(close_dialog),
                ),
        )
    }
}

fn main() {
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(|cx: &mut App| {
            rok_ui::init(cx);
            rok_ui::fonts::CAIRO
                .register(cx)
                .expect("the Cairo font is bundled");
            #[cfg(feature = "font-noto-sans-arabic")]
            rok_ui::fonts::NOTO_SANS_ARABIC
                .register(cx)
                .expect("Noto Sans Arabic is bundled");
            Theme::set_font_family(rok_ui::fonts::CAIRO.family(), cx);
            set_text_direction(TextDirection::Rtl, cx);

            let bounds = Bounds::centered(None, gpui::size(px(1180.), px(860.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("rok-ui بالعربية".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| ArabicWindow { dialog_open: false }),
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}
