//! Custom fonts: bundled Google Fonts and helpers to register your own.
//!
//! GPUI draws text with fonts the platform knows about plus any you register at
//! startup. Register a font, then make it the theme's UI font:
//!
//! ```ignore
//! Application::new().with_assets(rok_ui::Assets).run(|cx: &mut App| {
//!     rok_ui::init(cx);
//!     rok_ui::fonts::CAIRO.register(cx).expect("Cairo is bundled");
//!     Theme::set_font_family(rok_ui::fonts::CAIRO.family(), cx);
//!     // ...open windows
//! });
//! ```
//!
//! **Bundled families** (Cargo features, off by default; each adds its files to your binary):
//!
//! | Feature | Family | Scripts | Weights |
//! |---|---|---|---|
//! | `font-cairo` | [Cairo](https://fonts.google.com/specimen/Cairo) | Arabic, Latin | 400–700 |
//! | `font-inter` | [Inter](https://fonts.google.com/specimen/Inter) | Latin, Greek, Cyrillic | 400–700 |
//! | `font-noto-sans-arabic` | [Noto Sans Arabic](https://fonts.google.com/specimen/Noto+Sans+Arabic) | Arabic, Persian, Urdu | 400–700 |
//!
//! All are licensed under the SIL Open Font License 1.1, whose text ships in
//! [`FontFamily::license`]. If you redistribute the fonts outside your app binary,
//! include that license with them.
//!
//! **Any other Google Font:** download static weights with
//! `scripts/fetch-google-font.sh "<Family>" 400,700 <dir>` from the rok-ui repository,
//! then register the files with [`register_font_files`] or embed them with
//! `include_bytes!` and [`register_fonts`].

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Mutex, OnceLock},
};

use gpui::{App, SharedString};

/// Register font files (TrueType or OpenType bytes) with GPUI's text system.
///
/// Afterwards their family names work anywhere a font family is accepted:
/// `Theme::set_font_family`, `.font_family(..)` or `font_family: "Cairo"` in `styles!`.
///
/// # Errors
///
/// Fails when GPUI's text system rejects a font (not TrueType or OpenType data).
pub fn register_fonts(
    cx: &App,
    fonts: impl IntoIterator<Item = Cow<'static, [u8]>>,
) -> gpui::Result<()> {
    let fonts: Vec<Cow<'static, [u8]>> = fonts.into_iter().collect();
    for font in &fonts {
        record_presentation_coverage(font);
    }
    cx.text_system().add_fonts(fonts)
}

/// Per registered family, the Arabic presentation-form characters its files do not
/// map. Right-to-left text on Windows is drawn with these forms (see
/// [`crate::bidi`]), so it avoids the missing ones.
fn presentation_gaps() -> &'static Mutex<HashMap<String, HashSet<char>>> {
    static GAPS: OnceLock<Mutex<HashMap<String, HashSet<char>>>> = OnceLock::new();
    GAPS.get_or_init(Default::default)
}

/// Note which presentation forms `font` lacks, under its family name. Fonts that
/// cannot be parsed are skipped; GPUI reports them when registering.
fn record_presentation_coverage(font: &[u8]) {
    let Ok(face) = ttf_parser::Face::parse(font, 0) else {
        return;
    };
    let name = |id: u16| {
        face.names()
            .into_iter()
            .find(|name| name.name_id == id && name.is_unicode())
            .and_then(|name| name.to_string())
    };
    // The typographic family groups every weight; older fonts only have name ID 1.
    let Some(family) = name(16).or_else(|| name(1)) else {
        return;
    };
    let missing: HashSet<char> = crate::bidi::presentation_forms()
        .into_iter()
        .filter(|form| face.glyph_index(*form).is_none())
        .collect();
    let mut gaps = presentation_gaps()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    gaps.entry(family).or_default().extend(missing);
}

/// Whether the font family `family` lacks the presentation form `form`. Families
/// registered through this module are known; installed fonts are looked up once
/// on Windows, the only platform that draws these forms. Unknown families are
/// assumed to have every form.
pub(crate) fn lacks_presentation_form(family: &str, form: char) -> bool {
    let mut gaps = presentation_gaps()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !gaps.contains_key(family) {
        let missing = installed_presentation_gaps(family).unwrap_or_default();
        gaps.insert(family.to_string(), missing);
    }
    gaps.get(family)
        .is_some_and(|missing| missing.contains(&form))
}

/// The presentation forms the installed font family `family` lacks, from
/// DirectWrite. `None` when no such family is installed.
#[cfg(windows)]
fn installed_presentation_gaps(family: &str) -> Option<HashSet<char>> {
    use windows::{
        core::HSTRING,
        Win32::Graphics::DirectWrite::{
            DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
            DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
        },
    };

    // SAFETY: plain DirectWrite queries on interfaces this function owns; every
    // out-pointer is a live local.
    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).ok()?;
        let mut collection = None;
        factory
            .GetSystemFontCollection(&mut collection, false)
            .ok()?;
        let collection = collection?;
        let (mut index, mut exists) = (0, Default::default());
        collection
            .FindFamilyName(&HSTRING::from(family), &mut index, &mut exists)
            .ok()?;
        if !exists.as_bool() {
            return None;
        }
        let font = collection
            .GetFontFamily(index)
            .ok()?
            .GetFirstMatchingFont(
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
            )
            .ok()?;
        Some(
            crate::bidi::presentation_forms()
                .into_iter()
                .filter(|form| {
                    !font
                        .HasCharacter(*form as u32)
                        .map(|has| has.as_bool())
                        .unwrap_or(true)
                })
                .collect(),
        )
    }
}

#[cfg(not(windows))]
fn installed_presentation_gaps(_family: &str) -> Option<HashSet<char>> {
    None
}

/// Read font files from disk and register them, for example fonts downloaded next to
/// your app.
///
/// # Errors
///
/// Fails on the first file that cannot be read, or as [`register_fonts`] does.
pub fn register_font_files(
    cx: &App,
    paths: impl IntoIterator<Item = impl AsRef<Path>>,
) -> gpui::Result<()> {
    let mut fonts = Vec::new();
    for path in paths {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("reading font {}: {error}", path.display()),
            )
        })?;
        fonts.push(Cow::Owned(bytes));
    }
    register_fonts(cx, fonts)
}

/// A font family whose files are compiled into the binary.
#[derive(Clone, Copy, Debug)]
pub struct FontFamily {
    family: &'static str,
    files: &'static [&'static [u8]],
    license: &'static str,
}

impl FontFamily {
    /// A family from embedded files, for your own bundled fonts:
    ///
    /// ```ignore
    /// const BRAND: FontFamily = FontFamily::new(
    ///     "Brand Sans",
    ///     &[include_bytes!("../fonts/BrandSans-400.ttf"), include_bytes!("../fonts/BrandSans-700.ttf")],
    ///     include_str!("../fonts/LICENSE.txt"),
    /// );
    /// ```
    #[must_use]
    pub const fn new(
        family: &'static str,
        files: &'static [&'static [u8]],
        license: &'static str,
    ) -> Self {
        Self {
            family,
            files,
            license,
        }
    }

    /// The family name to pass to `Theme::set_font_family` or `.font_family(..)`.
    #[must_use]
    pub fn family(&self) -> SharedString {
        SharedString::new_static(self.family)
    }

    /// The font license text, to show in an about or licenses screen.
    #[must_use]
    pub fn license(&self) -> &'static str {
        self.license
    }

    /// Register every weight of this family. Registering twice is harmless.
    ///
    /// # Errors
    ///
    /// As [`register_fonts`].
    pub fn register(&self, cx: &App) -> gpui::Result<()> {
        register_fonts(cx, self.files.iter().map(|file| Cow::Borrowed(*file)))
    }
}

/// [Cairo](https://fonts.google.com/specimen/Cairo): a contemporary Arabic and Latin
/// family, weights 400, 500, 600 and 700. SIL Open Font License 1.1.
#[cfg(feature = "font-cairo")]
pub const CAIRO: FontFamily = FontFamily::new(
    "Cairo",
    &[
        include_bytes!("../assets/fonts/cairo/Cairo-400.ttf"),
        include_bytes!("../assets/fonts/cairo/Cairo-500.ttf"),
        include_bytes!("../assets/fonts/cairo/Cairo-600.ttf"),
        include_bytes!("../assets/fonts/cairo/Cairo-700.ttf"),
    ],
    include_str!("../assets/fonts/cairo/LICENSE.txt"),
);

/// [Inter](https://fonts.google.com/specimen/Inter): a UI family for Latin, Greek and
/// Cyrillic, weights 400, 500, 600 and 700. SIL Open Font License 1.1.
#[cfg(feature = "font-inter")]
pub const INTER: FontFamily = FontFamily::new(
    "Inter",
    &[
        include_bytes!("../assets/fonts/inter/Inter-400.ttf"),
        include_bytes!("../assets/fonts/inter/Inter-500.ttf"),
        include_bytes!("../assets/fonts/inter/Inter-600.ttf"),
        include_bytes!("../assets/fonts/inter/Inter-700.ttf"),
    ],
    include_str!("../assets/fonts/inter/LICENSE.txt"),
);

/// [Noto Sans Arabic](https://fonts.google.com/specimen/Noto+Sans+Arabic): an Arabic,
/// Persian and Urdu family, weights 400, 500, 600 and 700, with every Arabic
/// presentation form mapped. SIL Open Font License 1.1.
#[cfg(feature = "font-noto-sans-arabic")]
pub const NOTO_SANS_ARABIC: FontFamily = FontFamily::new(
    "Noto Sans Arabic",
    &[
        include_bytes!("../assets/fonts/noto-sans-arabic/NotoSansArabic-400.ttf"),
        include_bytes!("../assets/fonts/noto-sans-arabic/NotoSansArabic-500.ttf"),
        include_bytes!("../assets/fonts/noto-sans-arabic/NotoSansArabic-600.ttf"),
        include_bytes!("../assets/fonts/noto-sans-arabic/NotoSansArabic-700.ttf"),
    ],
    include_str!("../assets/fonts/noto-sans-arabic/LICENSE.txt"),
);

/// Record that `family` lacks `forms`, as if a font missing them had been registered.
#[cfg(test)]
pub(crate) fn mark_missing_presentation_forms(family: &str, forms: impl IntoIterator<Item = char>) {
    presentation_gaps()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(family.to_string())
        .or_default()
        .extend(forms);
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn installed_fonts_report_their_presentation_forms() {
        // Segoe UI ships with Windows and covers Arabic.
        assert_eq!(
            installed_presentation_gaps("Segoe UI").map(|gaps| gaps.len()),
            Some(0)
        );
        // Consolas has no Arabic letters at all.
        if let Some(gaps) = installed_presentation_gaps("Consolas") {
            assert!(gaps.contains(&'\u{FEAF}'));
            assert!(lacks_presentation_form("Consolas", '\u{FEAF}'));
        }
        assert_eq!(installed_presentation_gaps("No Such Font Family"), None);
        assert!(!lacks_presentation_form("No Such Font Family", '\u{FEAF}'));
    }
}
