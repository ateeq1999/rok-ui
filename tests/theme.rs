//! A theme keeps its colours across a mode switch: a preset's, and a
//! custom theme's.

use gpui::{Hsla, TestAppContext};
use rok_ui::prelude::*;

/// A custom theme: the Neutral pair with a brand primary in each mode.
fn forest() -> Theme {
    let mut pair = ThemePreset::Neutral.pair();
    pair.light.primary = gpui::rgb(0x2F5D50).into();
    pair.dark.primary = gpui::rgb(0x7FB8A6).into();
    Theme::custom("Forest", pair)
}

fn primary(cx: &mut TestAppContext) -> Hsla {
    cx.update(|cx| cx.theme().colors.primary)
}

#[gpui::test]
fn a_custom_theme_survives_toggle_mode(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let theme = forest();
    let (light, dark) = (theme.pair.light.primary, theme.pair.dark.primary);
    cx.update(|cx| Theme::set_global(theme, cx));
    assert_eq!(primary(cx), light);

    cx.update(Theme::toggle_mode);
    cx.update(|cx| {
        let theme = cx.theme();
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.name.as_ref(), "Forest");
        assert_eq!(theme.preset, None);
    });
    assert_eq!(primary(cx), dark);

    cx.update(Theme::toggle_mode);
    assert_eq!(primary(cx), light);

    cx.update(|cx| Theme::set_mode(ThemeMode::Dark, cx));
    assert_eq!(primary(cx), dark);
}

#[gpui::test]
fn a_custom_theme_survives_the_system_appearance(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let theme = forest();
    let expected = theme.pair.clone();
    cx.update(|cx| Theme::set_global(theme, cx));
    let window = cx.add_empty_window();
    window.update(|window, cx| {
        let mode = match window.appearance() {
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => ThemeMode::Dark,
            _ => ThemeMode::Light,
        };
        Theme::sync_with_system_appearance(window, cx);
        let theme = cx.theme();
        assert_eq!(theme.mode, mode);
        assert_eq!(theme.name.as_ref(), "Forest");
        assert_eq!(&theme.colors, expected.colors(mode));
    });
}

/// A preset switches mode as it always did, and `change_preset` moves a
/// custom theme onto the preset's colours and radius.
#[gpui::test]
fn presets_behave_as_before(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    for preset in ThemePreset::ALL {
        cx.update(|cx| Theme::set_global(Theme::from_preset(preset, ThemeMode::Light), cx));
        cx.update(Theme::toggle_mode);
        cx.update(|cx| {
            let expected = Theme::from_preset(preset, ThemeMode::Dark);
            assert_eq!(cx.theme(), &expected, "{preset:?}");
        });
    }

    cx.update(|cx| Theme::set_global(forest(), cx));
    cx.update(|cx| Theme::change_preset(ThemePreset::Rok, cx));
    cx.update(|cx| {
        let theme = cx.theme();
        assert_eq!(theme.preset, Some(ThemePreset::Rok));
        assert_eq!(theme.colors, ThemePreset::Rok.pair().light);
        assert_eq!(theme.radius, ThemePreset::Rok.radius());
    });
}
