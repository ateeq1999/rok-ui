//! The gallery's Motion page: keyframes, presets, easings, transitions and presence.

use rok_ui::prelude::*;

keyframes! {
    ENTER = {
        from: { opacity: 0, y: 4 },
        to: { opacity: 1, y: 0 },
    }

    TRAVEL = {
        from: { x: 0 },
        to: { x: 60 },
    }

    GLOW = {
        from: { background: primary, color: primary_foreground, radius: none },
        50%: { radius: xl },
        to: { background: accent, color: accent_foreground, radius: full },
    }
}

/// A titled demo block.
fn demo(title: &'static str, description: &'static str, content: impl IntoElement) -> Card {
    Card::new()
        .child(
            CardHeader::new()
                .child(CardTitle::new(title))
                .child(CardDescription::new(description)),
        )
        .child(CardContent::new().child(content))
}

#[component]
pub fn MotionPage(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let replay = use_state(window, cx, || 0_usize);
    let wide = use_state(window, cx, || false);
    let shown = use_state(window, cx, || true);
    let shakes = use_state(window, cx, || 0_usize);
    let generation = replay.get(cx) as u64;
    let width = use_transition(
        "motion-width",
        window,
        cx,
        if wide.get(cx) { 320. } else { 96. },
        Transition::spring(),
    );
    let presence = use_presence(
        "motion-panel",
        window,
        cx,
        shown.get(cx),
        Transition::ease_out(250),
    );
    let colors = cx.theme().colors.clone();
    let radius = cx.theme().radius_medium();

    let easings = [
        ("Linear", Easing::Linear),
        ("Ease in", Easing::EaseIn),
        ("Ease out", Easing::EaseOut),
        ("Ease in-out", Easing::EaseInOut),
        ("Spring", Easing::Spring { damping: 0.4 }),
        ("Steps (6)", Easing::Steps(6)),
    ];
    let easing_rows = easings
        .into_iter()
        .enumerate()
        .map(|(index, (label, easing))| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(div().w(px(96.)).text_sm().child(label))
                .child(
                    div()
                        .flex_1()
                        .h(px(16.))
                        .rounded_full()
                        .bg(colors.muted)
                        .child(
                            div()
                                .size(px(16.))
                                .rounded_full()
                                .bg(colors.primary)
                                .motion(
                                    ElementId::NamedInteger(
                                        format!("easing-{index}").into(),
                                        generation,
                                    ),
                                    Motion::new(&TRAVEL).duration_ms(1200).easing(easing),
                                ),
                        ),
                )
        });

    let keyframes_demo = view! {
        div(sx = style! { display: flex, direction: column, gap: 4 }) {
            Button("motion-replay", label = "Replay", icon = IconName::Refresh,
                   on_click = move |_, _, cx| replay.update(cx, |count| *count += 1)).outline()
            div(sx = style! { display: flex, align: center, gap: 4 }) {
                {Card::new()
                    .w(px(220.))
                    .child(CardHeader::new()
                        .child(CardTitle::new("Fade up"))
                        .child(CardDescription::new("opacity and y")))
                    .motion(
                        ElementId::NamedInteger("enter".into(), generation),
                        Motion::new(&ENTER).duration_ms(400),
                    )}
                {div()
                    .size(px(96.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .child("Glow")
                    .motion(
                        ElementId::NamedInteger("glow".into(), generation),
                        Motion::new(&GLOW)
                            .duration_ms(1400)
                            .easing(Easing::EaseInOut)
                            .iterations(2)
                            .alternate(),
                    )}
            }
        }
    };

    let presets_demo = div()
        .flex()
        .items_center()
        .gap(px(24.))
        .child(Badge::new("Live").motion("pulse", motion::pulse()))
        .child(
            Icon::new(IconName::ArrowDown)
                .size(px(24.))
                .motion("bounce", motion::bounce()),
        )
        .child(
            Button::new("shake")
                .destructive()
                .label("Shake me")
                .on_click({
                    let shakes = shakes.clone();
                    move |_, _, cx| shakes.update(cx, |count| *count += 1)
                })
                .motion(("shake", shakes.get(cx) as u64), motion::shake()),
        );

    let transition_demo = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            Button::new("toggle-width")
                .outline()
                .label(if wide.get(cx) { "Shrink" } else { "Grow" })
                .on_click({
                    let wide = wide.clone();
                    move |_, _, cx| wide.update(cx, |wide| *wide = !*wide)
                }),
        )
        .child(
            div()
                .h(px(40.))
                .w(px(width))
                .rounded(radius)
                .bg(colors.primary),
        );

    let presence_demo = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            Button::new("toggle-panel")
                .outline()
                .label(if shown.get(cx) { "Hide" } else { "Show" })
                .on_click({
                    let shown = shown.clone();
                    move |_, _, cx| shown.update(cx, |shown| *shown = !*shown)
                }),
        )
        .child(div().h(px(80.)).when(presence.is_mounted(), |slot| {
            slot.child(
                presence.apply(
                    Alert::new("Saved")
                        .icon(IconName::CircleCheck)
                        .description("Fades and slides with the presence progress."),
                    &ENTER,
                ),
            )
        }));

    div()
        .flex()
        .items_start()
        .gap(px(24.))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(16.))
                .child(demo(
                    "Keyframes",
                    "keyframes! with from, 50% and to frames; Replay changes the motion id.",
                    keyframes_demo,
                ))
                .child(demo(
                    "Easing",
                    "The same motion with each timing function.",
                    div().flex().flex_col().gap(px(12.)).children(easing_rows),
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(16.))
                .child(demo(
                    "Presets",
                    "pulse, bounce and shake, like the Tailwind animate utilities.",
                    presets_demo,
                ))
                .child(demo(
                    "Transition",
                    "use_transition springs a value toward its new target.",
                    transition_demo,
                ))
                .child(demo(
                    "Presence",
                    "use_presence keeps the panel mounted while it animates out.",
                    presence_demo,
                )),
        )
}
