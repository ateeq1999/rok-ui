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
//!
//! Both are licensed under the SIL Open Font License 1.1, whose text ships in
//! [`FontFamily::license`]. If you redistribute the fonts outside your app binary,
//! include that license with them.
//!
//! **Any other Google Font:** download static weights with
//! `scripts/fetch-google-font.sh "<Family>" 400,700 <dir>` from the rok-ui repository,
//! then register the files with [`register_font_files`] or embed them with
//! `include_bytes!` and [`register_fonts`].

use std::{borrow::Cow, path::Path};

use gpui::{App, SharedString};

/// Register font files (TrueType or OpenType bytes) with GPUI's text system.
///
/// Afterwards their family names work anywhere a font family is accepted:
/// `Theme::set_font_family`, `.font_family(..)` or `font_family: "Cairo"` in `styles!`.
pub fn register_fonts(
    cx: &App,
    fonts: impl IntoIterator<Item = Cow<'static, [u8]>>,
) -> gpui::Result<()> {
    cx.text_system().add_fonts(fonts.into_iter().collect())
}

/// Read font files from disk and register them, for example fonts downloaded next to
/// your app. Fails on the first file that cannot be read.
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
    pub fn family(&self) -> SharedString {
        SharedString::new_static(self.family)
    }

    /// The font license text, to show in an about or licenses screen.
    pub fn license(&self) -> &'static str {
        self.license
    }

    /// Register every weight of this family. Registering twice is harmless.
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
