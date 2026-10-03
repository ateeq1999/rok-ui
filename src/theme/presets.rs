//! Built-in theme presets. To rebrand, add a preset here (or build a [`Theme`]
//! by hand and pass it to [`Theme::set_global`]).

use gpui::{px, rgb, Hsla, SharedString};

use super::{Theme, ThemeColors, ThemeMode};

/// The built-in color palettes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemePreset {
    /// shadcn/ui's "neutral" base color, with contrast fixes for muted text and
    /// destructive buttons so every pair reaches 4.5:1.
    Neutral,
    /// The rok brand: warm stone neutrals, an ember accent and crisp 4px corners.
    #[default]
    Rok,
}

impl ThemePreset {
    /// Every preset, for theme pickers.
    pub const ALL: [ThemePreset; 2] = [ThemePreset::Rok, ThemePreset::Neutral];

    /// Display name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            ThemePreset::Neutral => "Neutral",
            ThemePreset::Rok => "Rok",
        }
    }
}

/// `0xRRGGBB` to [`Hsla`].
fn hex(value: u32) -> Hsla {
    rgb(value).into()
}

/// `0xRRGGBB` plus alpha to [`Hsla`].
fn hex_with_alpha(value: u32, alpha: f32) -> Hsla {
    let mut color: Hsla = rgb(value).into();
    color.a = alpha;
    color
}

pub(super) fn build_theme(preset: ThemePreset, mode: ThemeMode) -> Theme {
    let (colors, radius) = match (preset, mode) {
        (ThemePreset::Neutral, ThemeMode::Light) => (neutral_light_colors(), px(8.)),
        (ThemePreset::Neutral, ThemeMode::Dark) => (neutral_dark_colors(), px(8.)),
        (ThemePreset::Rok, ThemeMode::Light) => (rok_light_colors(), px(4.)),
        (ThemePreset::Rok, ThemeMode::Dark) => (rok_dark_colors(), px(4.)),
    };
    Theme {
        name: SharedString::new_static(preset.label()),
        mode,
        colors,
        radius,
        font_family: ".SystemUIFont".into(),
        monospace_font_family: default_monospace_font().into(),
        font_size: px(14.),
        preset,
    }
}

/// "monospace" is a fontconfig alias; DirectWrite and CoreText need a real family name.
fn default_monospace_font() -> &'static str {
    if cfg!(target_os = "windows") {
        "Consolas"
    } else if cfg!(target_os = "macos") {
        "Menlo"
    } else {
        "monospace"
    }
}

fn neutral_light_colors() -> ThemeColors {
    ThemeColors {
        background: hex(0xFFFFFF),
        foreground: hex(0x0A0A0A),
        card: hex(0xFFFFFF),
        card_foreground: hex(0x0A0A0A),
        popover: hex(0xFFFFFF),
        popover_foreground: hex(0x0A0A0A),
        primary: hex(0x171717),
        primary_foreground: hex(0xFAFAFA),
        secondary: hex(0xF5F5F5),
        secondary_foreground: hex(0x171717),
        muted: hex(0xF5F5F5),
        muted_foreground: hex(0x666666),
        accent: hex(0xF5F5F5),
        accent_foreground: hex(0x171717),
        destructive: hex(0xDC2626),
        destructive_foreground: hex(0xFAFAFA),
        destructive_text: hex(0xDC2626),
        border: hex(0xE5E5E5),
        input: hex(0xE5E5E5),
        ring: hex(0xA3A3A3),
        overlay: hex_with_alpha(0x000000, 0.5),
    }
}

fn neutral_dark_colors() -> ThemeColors {
    ThemeColors {
        background: hex(0x0A0A0A),
        foreground: hex(0xFAFAFA),
        card: hex(0x171717),
        card_foreground: hex(0xFAFAFA),
        popover: hex(0x171717),
        popover_foreground: hex(0xFAFAFA),
        primary: hex(0xE5E5E5),
        primary_foreground: hex(0x171717),
        secondary: hex(0x262626),
        secondary_foreground: hex(0xFAFAFA),
        muted: hex(0x262626),
        muted_foreground: hex(0xA3A3A3),
        accent: hex(0x262626),
        accent_foreground: hex(0xFAFAFA),
        destructive: hex(0xB91C1C),
        destructive_foreground: hex(0xFAFAFA),
        destructive_text: hex(0xF87171),
        border: hex(0x2E2E2E),
        input: hex(0x3A3A3A),
        ring: hex(0x737373),
        overlay: hex_with_alpha(0x000000, 0.7),
    }
}

fn rok_light_colors() -> ThemeColors {
    ThemeColors {
        background: hex(0xFAFAF9),
        foreground: hex(0x1C1917),
        card: hex(0xFFFFFF),
        card_foreground: hex(0x1C1917),
        popover: hex(0xFFFFFF),
        popover_foreground: hex(0x1C1917),
        primary: hex(0xB4400F),
        primary_foreground: hex(0xFFFFFF),
        secondary: hex(0xF0EEEC),
        secondary_foreground: hex(0x1C1917),
        muted: hex(0xF5F5F4),
        muted_foreground: hex(0x57534E),
        accent: hex(0xFBF1EC),
        accent_foreground: hex(0x1C1917),
        destructive: hex(0xDC2626),
        destructive_foreground: hex(0xFFFFFF),
        destructive_text: hex(0xDC2626),
        border: hex(0xE7E5E4),
        input: hex(0xD6D3D1),
        ring: hex(0xC8501E),
        overlay: hex_with_alpha(0x1C1917, 0.5),
    }
}

fn rok_dark_colors() -> ThemeColors {
    ThemeColors {
        background: hex(0x141210),
        foreground: hex(0xFAFAF9),
        card: hex(0x1C1917),
        card_foreground: hex(0xFAFAF9),
        popover: hex(0x1C1917),
        popover_foreground: hex(0xFAFAF9),
        primary: hex(0xEA7A45),
        primary_foreground: hex(0x1C1917),
        secondary: hex(0x292524),
        secondary_foreground: hex(0xFAFAF9),
        muted: hex(0x292524),
        muted_foreground: hex(0xA8A29E),
        accent: hex(0x3A2A22),
        accent_foreground: hex(0xFAFAF9),
        destructive: hex(0xB91C1C),
        destructive_foreground: hex(0xFAFAF9),
        destructive_text: hex(0xF87171),
        border: hex(0x2E2A27),
        input: hex(0x44403C),
        ring: hex(0xEA7A45),
        overlay: hex_with_alpha(0x000000, 0.7),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relative_luminance(color: Hsla) -> f32 {
        let rgba = color.to_rgb();
        let linearize = |channel: f32| {
            if channel <= 0.03928 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linearize(rgba.r) + 0.7152 * linearize(rgba.g) + 0.0722 * linearize(rgba.b)
    }

    fn contrast_ratio(first: Hsla, second: Hsla) -> f32 {
        let (first, second) = (relative_luminance(first), relative_luminance(second));
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }

    #[test]
    fn every_text_pair_reaches_four_point_five_to_one() {
        for preset in ThemePreset::ALL {
            for mode in [ThemeMode::Light, ThemeMode::Dark] {
                let colors = build_theme(preset, mode).colors;
                let pairs = [
                    (
                        "foreground/background",
                        colors.foreground,
                        colors.background,
                    ),
                    ("card", colors.card_foreground, colors.card),
                    ("popover", colors.popover_foreground, colors.popover),
                    ("primary", colors.primary_foreground, colors.primary),
                    ("secondary", colors.secondary_foreground, colors.secondary),
                    ("muted", colors.muted_foreground, colors.muted),
                    (
                        "muted on background",
                        colors.muted_foreground,
                        colors.background,
                    ),
                    ("muted on card", colors.muted_foreground, colors.card),
                    ("accent", colors.accent_foreground, colors.accent),
                    (
                        "destructive",
                        colors.destructive_foreground,
                        colors.destructive,
                    ),
                    (
                        "destructive text on background",
                        colors.destructive_text,
                        colors.background,
                    ),
                    (
                        "destructive text on card",
                        colors.destructive_text,
                        colors.card,
                    ),
                ];
                for (pair_name, text, surface) in pairs {
                    let ratio = contrast_ratio(text, surface);
                    assert!(
                        ratio >= 4.5,
                        "{preset:?} {mode:?} {pair_name}: contrast {ratio:.2} is below 4.5"
                    );
                }
            }
        }
    }
}
