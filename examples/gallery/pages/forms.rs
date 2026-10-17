//! The forms page: inputs, choices and a sign-up form.

use rok_ui::prelude::*;

use super::{column, demo, row};

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
