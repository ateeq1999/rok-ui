//! Design tokens and the active theme.
//!
//! The token names mirror shadcn/ui's CSS variables (`--background`, `--primary`,
//! `--muted-foreground`, …) so anyone who has themed shadcn/ui already knows them.
//! The theme lives in a GPUI [`Global`]; read it anywhere with [`ActiveTheme::theme`].

mod presets;

use gpui::{px, App, Global, Hsla, Pixels, SharedString, Window};

pub use presets::ThemePreset;

/// Light or dark appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeMode {
    /// Dark text on light surfaces.
    #[default]
    Light,
    /// Light text on dark surfaces.
    Dark,
}

impl ThemeMode {
    /// `true` for [`ThemeMode::Dark`].
    #[must_use]
    pub fn is_dark(self) -> bool {
        self == ThemeMode::Dark
    }

    /// The other mode.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }
}

/// Color tokens, one per shadcn/ui CSS variable.
///
/// Every `*_foreground` token is the text color meant to sit on the token it is
/// named after; each pair is checked for at least 4.5:1 contrast in the tests.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeColors {
    /// App background.
    pub background: Hsla,
    /// Default text.
    pub foreground: Hsla,
    /// Card surface.
    pub card: Hsla,
    /// Text on cards.
    pub card_foreground: Hsla,
    /// Floating surfaces: dialogs, tooltips' counterpart, menus.
    pub popover: Hsla,
    /// Text in popovers and menus.
    pub popover_foreground: Hsla,
    /// Main call to action.
    pub primary: Hsla,
    /// Text on the primary color.
    pub primary_foreground: Hsla,
    /// Low-emphasis filled surfaces.
    pub secondary: Hsla,
    /// Text on the secondary color.
    pub secondary_foreground: Hsla,
    /// Subdued backgrounds and secondary text.
    pub muted: Hsla,
    /// Secondary text: descriptions, placeholders.
    pub muted_foreground: Hsla,
    /// Hover and selected backgrounds.
    pub accent: Hsla,
    /// Text on the accent color.
    pub accent_foreground: Hsla,
    /// Destructive actions and errors.
    pub destructive: Hsla,
    /// Text on the destructive color.
    pub destructive_foreground: Hsla,
    /// Error text drawn directly on `background` or `card` (alerts, field errors).
    pub destructive_text: Hsla,
    /// Hairlines and dividers.
    pub border: Hsla,
    /// Form control borders.
    pub input: Hsla,
    /// Focus ring.
    pub ring: Hsla,
    /// Scrim behind modal dialogs.
    pub overlay: Hsla,
}

/// The complete theme: colors plus shape and type tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Human-readable name, for example "Neutral" or "Rok".
    pub name: SharedString,
    /// Light or dark.
    pub mode: ThemeMode,
    /// The color tokens.
    pub colors: ThemeColors,
    /// Base corner radius (shadcn's `--radius`). Components derive smaller and
    /// larger radii from it, like `rounded-md` = radius − 2px.
    pub radius: Pixels,
    /// UI font. `.SystemUIFont` resolves to the platform's UI font.
    pub font_family: SharedString,
    /// Code and keyboard-shortcut font.
    pub monospace_font_family: SharedString,
    /// Base text size (shadcn's `text-sm` is used for most controls).
    pub font_size: Pixels,
    /// Which preset produced this theme, so mode toggles keep the brand.
    pub preset: ThemePreset,
}

impl Global for Theme {}

impl Theme {
    /// Build a theme from a preset and a mode.
    #[must_use]
    pub fn from_preset(preset: ThemePreset, mode: ThemeMode) -> Self {
        presets::build_theme(preset, mode)
    }

    /// The theme currently installed in the app.
    ///
    /// Panics if [`crate::init`] was not called.
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Install `theme` and redraw every window.
    pub fn set_global(theme: Theme, cx: &mut App) {
        crate::sx::set_theme_snapshot(&theme);
        cx.set_global(theme);
        cx.refresh_windows();
    }

    /// Switch between light and dark, keeping the current preset and fonts.
    pub fn toggle_mode(cx: &mut App) {
        let current = Theme::global(cx);
        let next = current.rebuilt(current.preset, current.mode.toggled());
        Theme::set_global(next, cx);
    }

    /// Switch preset, keeping the current mode and fonts.
    pub fn change_preset(preset: ThemePreset, cx: &mut App) {
        let current = Theme::global(cx);
        let next = current.rebuilt(preset, current.mode);
        Theme::set_global(next, cx);
    }

    /// Use `family` for all UI text, for example a font registered with
    /// [`crate::fonts`]. Preset and mode changes keep it.
    ///
    /// ```ignore
    /// rok_ui::fonts::CAIRO.register(cx)?;
    /// Theme::set_font_family(rok_ui::fonts::CAIRO.family(), cx);
    /// ```
    pub fn set_font_family(family: impl Into<SharedString>, cx: &mut App) {
        let mut theme = Theme::global(cx).clone();
        theme.font_family = family.into();
        Theme::set_global(theme, cx);
    }

    /// The `preset` theme in `mode`, carrying over this theme's fonts.
    fn rebuilt(&self, preset: ThemePreset, mode: ThemeMode) -> Theme {
        Theme {
            font_family: self.font_family.clone(),
            monospace_font_family: self.monospace_font_family.clone(),
            font_size: self.font_size,
            ..Theme::from_preset(preset, mode)
        }
    }

    /// Follow the operating system's light or dark appearance.
    pub fn sync_with_system_appearance(window: &Window, cx: &mut App) {
        let mode = match window.appearance() {
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => ThemeMode::Dark,
            gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => {
                ThemeMode::Light
            }
        };
        let current = Theme::global(cx);
        let next = current.rebuilt(current.preset, mode);
        Theme::set_global(next, cx);
    }

    /// `rounded-sm`: radius − 4px, never below zero.
    #[must_use]
    pub fn radius_small(&self) -> Pixels {
        (self.radius - px(4.)).max(px(0.))
    }

    /// `rounded-md`: radius − 2px, never below zero.
    #[must_use]
    pub fn radius_medium(&self) -> Pixels {
        (self.radius - px(2.)).max(px(0.))
    }

    /// `rounded-lg`: the base radius.
    #[must_use]
    pub fn radius_large(&self) -> Pixels {
        self.radius
    }

    /// `rounded-xl`: radius + 4px (cards, dialogs).
    #[must_use]
    pub fn radius_extra_large(&self) -> Pixels {
        self.radius + px(4.)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::from_preset(ThemePreset::default(), ThemeMode::Light)
    }
}

/// Read the active theme from any GPUI context: `cx.theme().colors.primary`.
pub trait ActiveTheme {
    /// The active theme.
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

/// Install the default theme if none is set. Called by [`crate::init`].
pub fn init(cx: &mut App) {
    if !cx.has_global::<Theme>() {
        cx.set_global(Theme::default());
    }
    crate::sx::set_theme_snapshot(Theme::global(cx));
}
