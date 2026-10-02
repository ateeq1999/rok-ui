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
    let built: Vec<AnyElement> = children![
        "text",
        Badge::new("badge"),
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
    // 1 + 1 + 1 + 1 + 3 + 6 + 2
    assert_eq!(built.len(), 15);
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
