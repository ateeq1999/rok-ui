//! Built-in stroke icons (24px grid, 2px stroke, round caps, in the Lucide style)
//! and the asset source that serves them to GPUI.

use std::borrow::Cow;

use gpui::{
    prelude::*, px, svg, App, AssetSource, Hsla, Pixels, SharedString, StyleRefinement, Window,
};

use crate::sx::SxStyled;
use crate::{styles::ApplyStyleOverrides, theme::ActiveTheme};

macro_rules! define_icons {
    ($($variant:ident => $file_name:literal),* $(,)?) => {
        /// Every icon shipped with rok-ui.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum IconName {
            $($variant,)*
        }

        impl IconName {
            /// Every icon, for galleries and tests.
            pub const ALL: &'static [IconName] = &[$(IconName::$variant,)*];

            /// Asset path, as passed to `svg().path(..)`.
            pub fn asset_path(self) -> &'static str {
                match self {
                    $(IconName::$variant => concat!("icons/", $file_name, ".svg"),)*
                }
            }

            fn svg_bytes(self) -> &'static [u8] {
                match self {
                    $(IconName::$variant => include_bytes!(concat!("../assets/icons/", $file_name, ".svg")),)*
                }
            }
        }
    };
}

define_icons! {
    AlignCenter => "align-center",
    AlignLeft => "align-left",
    AlignRight => "align-right",
    ArrowDown => "arrow-down",
    ArrowLeft => "arrow-left",
    ArrowRight => "arrow-right",
    ArrowUp => "arrow-up",
    ArrowUpDown => "arrow-up-down",
    Bell => "bell",
    Bold => "bold",
    Calendar => "calendar",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronLeft => "chevron-left",
    ChevronRight => "chevron-right",
    ChevronUp => "chevron-up",
    ChevronsLeft => "chevrons-left",
    ChevronsRight => "chevrons-right",
    ChevronsUpDown => "chevrons-up-down",
    Circle => "circle",
    CircleAlert => "circle-alert",
    CircleCheck => "circle-check",
    CircleX => "circle-x",
    Close => "close",
    Copy => "copy",
    Dot => "dot",
    Download => "download",
    Ellipsis => "ellipsis",
    ExternalLink => "external-link",
    Eye => "eye",
    File => "file",
    FileText => "file-text",
    Filter => "filter",
    Folder => "folder",
    Globe => "globe",
    GripVertical => "grip-vertical",
    Heart => "heart",
    Home => "home",
    Image => "image",
    Inbox => "inbox",
    Info => "info",
    Italic => "italic",
    Link => "link",
    Loader => "loader",
    LogOut => "log-out",
    Mail => "mail",
    Menu => "menu",
    Minus => "minus",
    Moon => "moon",
    PanelLeft => "panel-left",
    Paperclip => "paperclip",
    Pencil => "pencil",
    Plus => "plus",
    Refresh => "refresh",
    Search => "search",
    Send => "send",
    Settings => "settings",
    Smile => "smile",
    Sparkles => "sparkles",
    Star => "star",
    Sun => "sun",
    Trash => "trash",
    TriangleAlert => "triangle-alert",
    Underline => "underline",
    Upload => "upload",
    User => "user",
}

/// An icon. Defaults to 16px in the theme's foreground color.
#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    size: Pixels,
    color: Option<Hsla>,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Icon);

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: px(16.),
            color: None,
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }

    /// Width and height in pixels.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// Stroke color. Icons do not inherit text color in GPUI, so components pass it explicitly.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().colors.foreground);
        svg()
            .path(self.name.asset_path())
            .flex_none()
            .size(self.size)
            .text_color(color)
            .sx(&self.sx)
            .apply_style_overrides(&self.style_overrides)
    }
}

/// Serves rok-ui's icons. Pass it to `Application::new().with_assets(..)`.
///
/// If your app has its own assets, wrap them: `Assets::with_fallback(MyAssets)`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Assets;

impl Assets {
    /// Serve rok-ui's icons first, then anything else from `application_assets`.
    pub fn with_fallback<Fallback: AssetSource>(
        application_assets: Fallback,
    ) -> AssetsWithFallback<Fallback> {
        AssetsWithFallback {
            fallback: application_assets,
        }
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(IconName::ALL
            .iter()
            .find(|icon| icon.asset_path() == path)
            .map(|icon| Cow::Borrowed(icon.svg_bytes())))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(IconName::ALL
            .iter()
            .map(|icon| icon.asset_path())
            .filter(|asset_path| asset_path.starts_with(path))
            .map(SharedString::new_static)
            .collect())
    }
}

/// rok-ui's icons layered over the application's own asset source.
pub struct AssetsWithFallback<Fallback> {
    fallback: Fallback,
}

impl<Fallback: AssetSource> AssetSource for AssetsWithFallback<Fallback> {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        match Assets.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => self.fallback.load(path),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut asset_paths = Assets.list(path)?;
        asset_paths.extend(self.fallback.list(path)?);
        Ok(asset_paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_served_and_is_svg() {
        for icon in IconName::ALL {
            let bytes = Assets
                .load(icon.asset_path())
                .unwrap()
                .unwrap_or_else(|| panic!("{icon:?} is not served"));
            let text = std::str::from_utf8(&bytes).unwrap();
            assert!(text.starts_with("<svg"), "{icon:?} is not an svg");
        }
        assert!(Assets.load("icons/does-not-exist.svg").unwrap().is_none());
    }
}
