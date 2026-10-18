//! Where generated files go. The generator plans every file at its path in a single-crate
//! app (`src/data/..`, `src/features/<f>/bloc/..`); a [`Layout`] moves them, and rewrites
//! their paths, for an app split into crates (`cargo rok-ui new --layout workspace`):
//!
//! | Single crate | Workspace |
//! |---|---|
//! | `src/data.rs`, `src/data/..` | `crates/<app>-data/src/lib.rs`, `crates/<app>-data/src/..` |
//! | `src/features/<f>/bloc.rs`, `../bloc/..` | `crates/<app>-<f>/src/lib.rs`, `crates/<app>-<f>/src/..` |
//! | `tests/features/<f>/..` | `crates/<app>-<f>/tests/..` |
//! | anything else | `crates/<app>/..` |
//!
//! The app crate aliases the moved modules (`pub use <app>_data as data;` in `lib.rs`,
//! `pub use <app>_<f> as bloc;` in `features/<f>.rs`), so views, routes and `app.rs` keep
//! their `crate::data::..` and `crate::features::<f>::bloc::..` paths.

use std::{fs, path::Path};

/// How the app at the generator's root is laid out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    /// One crate: files go where the generator plans them.
    #[default]
    Single,
    /// A data crate, a crate per feature's business logic, and the app crate.
    Workspace(Workspace),
}

/// The names of a workspace app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    /// The app crate's package name: `my-app`.
    package: String,
    /// Its name in Rust paths: `my_app`.
    ident: String,
}

/// What a single-crate path is in a workspace.
enum Place<'a> {
    /// In the data crate, at this path below `src/`.
    Data(&'a str),
    /// In a feature crate's `src/`.
    Feature { feature: &'a str, rest: &'a str },
    /// In a feature crate's `tests/`.
    FeatureTest { feature: &'a str, rest: &'a str },
    /// A test barrel the workspace does not need (each test file is its own crate).
    Dropped,
    /// Already a workspace path (`crates/..`).
    Workspace,
    /// In the app crate.
    App,
}

fn place(path: &str) -> Place<'_> {
    if path.starts_with("crates/") {
        return Place::Workspace;
    }
    if path == "src/data.rs" {
        return Place::Data("lib.rs");
    }
    if let Some(rest) = path.strip_prefix("src/data/") {
        return Place::Data(rest);
    }
    if let Some(rest) = path.strip_prefix("src/features/") {
        if let Some((feature, rest)) = rest.split_once('/') {
            if rest == "bloc.rs" {
                return Place::Feature {
                    feature,
                    rest: "lib.rs",
                };
            }
            if let Some(rest) = rest.strip_prefix("bloc/") {
                return Place::Feature { feature, rest };
            }
        }
    }
    if let Some(rest) = path.strip_prefix("tests/features/") {
        return match rest.split_once('/') {
            Some((feature, rest)) => Place::FeatureTest { feature, rest },
            None => Place::Dropped,
        };
    }
    Place::App
}

impl Layout {
    /// The layout `rok-ui.toml` at `root` names (`layout = "workspace"` with `app = ".."`),
    /// or a single crate.
    ///
    /// # Errors
    ///
    /// When `rok-ui.toml` names the workspace layout without the app's name.
    pub fn detect(root: &Path) -> Result<Self, String> {
        let Ok(settings) = fs::read_to_string(root.join("rok-ui.toml")) else {
            return Ok(Self::Single);
        };
        let value = |key: &str| {
            settings.lines().find_map(|line| {
                let (name, value) = line.split_once('=')?;
                (name.trim() == key).then(|| value.trim().trim_matches('"').to_string())
            })
        };
        if value("layout").as_deref() != Some("workspace") {
            return Ok(Self::Single);
        }
        let package = value("app")
            .filter(|app| !app.is_empty())
            .ok_or("rok-ui.toml: `layout = \"workspace\"` needs `app = \"<app crate>\"`")?;
        Ok(Self::workspace(&package))
    }

    /// The workspace layout for the app crate `package`.
    pub fn workspace(package: &str) -> Self {
        Self::Workspace(Workspace {
            package: package.to_string(),
            ident: package.replace('-', "_"),
        })
    }

    /// The app crate's name in Rust paths, for a workspace.
    pub fn crate_name(&self) -> Option<&str> {
        match self {
            Self::Single => None,
            Self::Workspace(workspace) => Some(&workspace.ident),
        }
    }

    /// Where the single-crate `path` goes, or `None` when the layout has no such file.
    pub fn physical(&self, path: &str) -> Option<String> {
        let Self::Workspace(workspace) = self else {
            return Some(path.to_string());
        };
        let package = &workspace.package;
        Some(match place(path) {
            Place::Data(rest) => format!("crates/{package}-data/src/{rest}"),
            Place::Feature { feature, rest } => {
                format!("{}/src/{rest}", workspace.feature_dir(feature))
            }
            Place::FeatureTest { feature, rest } => {
                format!("{}/tests/{rest}", workspace.feature_dir(feature))
            }
            Place::Dropped => return None,
            Place::Workspace => path.to_string(),
            Place::App => format!("crates/{package}/{path}"),
        })
    }

    /// `text`, planned for the single-crate `path`, with its paths rewritten for where
    /// [`physical`](Self::physical) puts it.
    pub fn to_physical(&self, path: &str, text: &str) -> String {
        let Self::Workspace(workspace) = self else {
            return text.to_string();
        };
        let ident = &workspace.ident;
        match place(path) {
            Place::Data(_) => text
                .replace("crate::data::", "crate::")
                .replace("rok_ui::bloc::", "rok_ui_bloc::"),
            Place::Feature { .. } => text
                .replace("crate::data::", &format!("{ident}_data::"))
                .replace("rok_ui::bloc::", "rok_ui_bloc::"),
            Place::FeatureTest { feature, .. } => text
                .replace(
                    &format!("{ident}::features::{feature}::bloc::"),
                    &format!("{ident}_{feature}::"),
                )
                .replace(&format!("{ident}::data::"), &format!("{ident}_data::"))
                .replace("rok_ui::bloc::", "rok_ui_bloc::"),
            Place::App => workspace.alias(path, text, false),
            Place::Dropped | Place::Workspace => text.to_string(),
        }
    }

    /// The inverse of [`to_physical`](Self::to_physical) for the app crate's barrels, so
    /// the generator reads `pub use <app>_data as data;` as the `pub mod data;` it plans.
    pub fn to_logical(&self, path: &str, text: &str) -> String {
        let Self::Workspace(workspace) = self else {
            return text.to_string();
        };
        if matches!(place(path), Place::App) {
            workspace.alias(path, text, true)
        } else {
            text.to_string()
        }
    }

    /// The features whose business logic `paths` (single-crate paths) write.
    fn features<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        let mut features: Vec<&str> = paths
            .into_iter()
            .filter_map(|path| match place(path) {
                Place::Feature { feature, .. } => Some(feature),
                _ => None,
            })
            .collect();
        features.sort_unstable();
        features.dedup();
        features
    }

    /// The manifest and `clippy.toml` of each feature crate `paths` write into, as
    /// workspace paths with their text.
    pub fn crate_files<'a>(
        &self,
        paths: impl IntoIterator<Item = &'a str>,
    ) -> Vec<(String, String)> {
        let Self::Workspace(workspace) = self else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for feature in Self::features(paths) {
            let directory = workspace.feature_dir(feature);
            out.push((
                format!("{directory}/Cargo.toml"),
                format!(
                    "[package]\nname = \"{}\"\nversion.workspace = true\nedition.workspace = true\npublish = false\n\n# The business logic of `{feature}`: no GPUI, by construction.\n[dependencies]\n{}-data.workspace = true\nrok-ui-bloc.workspace = true\n\n[lints]\nworkspace = true\n",
                    workspace.feature_package(feature),
                    workspace.package
                ),
            ));
            out.push((
                format!("{directory}/clippy.toml"),
                "# This crate has no GPUI to misuse; what is left is blocs holding blocs.\ndisallowed-types = [\n    { path = \"rok_ui_bloc::BlocHandle\", reason = \"a bloc never holds another bloc: share a repository, or listen in the view\" },\n]\n".to_string(),
            ));
        }
        out
    }

    /// The dependency lines the app crate's `Cargo.toml` gets for the feature crates
    /// `paths` write into, at the `# rok-ui:features` marker.
    pub fn dependency_lines<'a>(&self, paths: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let Self::Workspace(workspace) = self else {
            return Vec::new();
        };
        Self::features(paths)
            .into_iter()
            .map(|feature| {
                let package = workspace.feature_package(feature);
                format!("{package} = {{ path = \"../{package}\" }}")
            })
            .collect()
    }

    /// Why the plan cannot be written in this layout, if it cannot.
    pub fn check<'a>(
        &self,
        paths: impl IntoIterator<Item = &'a str>,
        is_http: bool,
    ) -> Result<(), String> {
        if *self == Self::Single {
            return Ok(());
        }
        if is_http {
            return Err(
                "the workspace layout does not generate HTTP providers yet: their \
                        errors (`ApiError`) would put rok-ui in the feature crates"
                    .into(),
            );
        }
        if Self::features(paths).contains(&"data") {
            return Err("a feature called `data` would clash with the data crate".into());
        }
        Ok(())
    }
}

impl Workspace {
    fn feature_package(&self, feature: &str) -> String {
        format!("{}-{}", self.package, feature.replace('_', "-"))
    }

    fn feature_dir(&self, feature: &str) -> String {
        format!("crates/{}", self.feature_package(feature))
    }

    fn data_alias(&self) -> String {
        format!("pub use {}_data as data;", self.ident)
    }

    fn feature_alias(&self, feature: &str) -> String {
        format!("pub use {}_{feature} as bloc;", self.ident)
    }

    /// The app crate file `path` with its moved module declared as an alias (or, `back`,
    /// the alias read as the declaration).
    fn alias(&self, path: &str, text: &str, back: bool) -> String {
        let (declaration, alias) = if path == "src/lib.rs" {
            ("pub mod data;".to_string(), self.data_alias())
        } else if let Some(feature) = feature_barrel(path) {
            ("pub mod bloc;".to_string(), self.feature_alias(feature))
        } else {
            return text.to_string();
        };
        let (from, to) = if back {
            (&alias, &declaration)
        } else {
            (&declaration, &alias)
        };
        replace_line(text, from, to).unwrap_or_else(|| text.to_string())
    }
}

/// `src/features/<f>.rs`: the feature's barrel, which declares its `bloc`.
fn feature_barrel(path: &str) -> Option<&str> {
    path.strip_prefix("src/features/")?
        .strip_suffix(".rs")
        .filter(|feature| !feature.contains('/'))
}

/// `text` with the line that trims to `from` replaced by `to` (at its indentation), or
/// `None` when it has no such line.
fn replace_line(text: &str, from: &str, to: &str) -> Option<String> {
    if !text.lines().any(|line| line.trim() == from) {
        return None;
    }
    let mut out: String = text
        .lines()
        .map(|line| {
            if line.trim() == from {
                let indentation = &line[..line.len() - line.trim_start().len()];
                format!("{indentation}{to}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_crate_paths_move_into_crates() {
        let layout = Layout::workspace("my-app");
        for (from, to) in [
            ("src/data.rs", Some("crates/my-app-data/src/lib.rs")),
            (
                "src/data/models/note.rs",
                Some("crates/my-app-data/src/models/note.rs"),
            ),
            (
                "src/features/user_profile/bloc.rs",
                Some("crates/my-app-user-profile/src/lib.rs"),
            ),
            (
                "src/features/notes/bloc/notes_bloc.rs",
                Some("crates/my-app-notes/src/notes_bloc.rs"),
            ),
            (
                "tests/features/notes/notes_bloc_test.rs",
                Some("crates/my-app-notes/tests/notes_bloc_test.rs"),
            ),
            ("tests/features/main.rs", None),
            ("tests/features/notes.rs", None),
            (
                "src/features/notes/view/notes_page.rs",
                Some("crates/my-app/src/features/notes/view/notes_page.rs"),
            ),
            ("Cargo.toml", Some("crates/my-app/Cargo.toml")),
            (
                "crates/my-app-notes/Cargo.toml",
                Some("crates/my-app-notes/Cargo.toml"),
            ),
        ] {
            assert_eq!(layout.physical(from).as_deref(), to, "{from}");
        }
        assert_eq!(
            Layout::Single.physical("src/data.rs").as_deref(),
            Some("src/data.rs")
        );
    }

    #[test]
    fn paths_in_moved_files_are_rewritten() {
        let layout = Layout::workspace("my-app");
        assert_eq!(
            layout.to_physical(
                "src/data/repositories/notes_repository.rs",
                "use crate::data::models::note::Note;\nuse rok_ui::bloc::BoxFuture;\n"
            ),
            "use crate::models::note::Note;\nuse rok_ui_bloc::BoxFuture;\n"
        );
        assert_eq!(
            layout.to_physical(
                "src/features/notes/bloc/notes_state.rs",
                "use crate::data::models::note::Note;\n"
            ),
            "use my_app_data::models::note::Note;\n"
        );
        assert_eq!(
            layout.to_physical(
                "tests/features/notes/notes_bloc_test.rs",
                "use my_app::data::error::DataError;\nuse my_app::features::notes::bloc::notes_bloc::NotesBloc;\nuse rok_ui::bloc::test;\n"
            ),
            "use my_app_data::error::DataError;\nuse my_app_notes::notes_bloc::NotesBloc;\nuse rok_ui_bloc::test;\n"
        );
    }

    #[test]
    fn app_barrels_alias_the_moved_crates_both_ways() {
        let layout = Layout::workspace("my-app");
        let logical = "//! Notes.\n\npub mod bloc;\npub mod view;\n";
        let physical = "//! Notes.\n\npub use my_app_notes as bloc;\npub mod view;\n";
        assert_eq!(
            layout.to_physical("src/features/notes.rs", logical),
            physical
        );
        assert_eq!(
            layout.to_logical("src/features/notes.rs", physical),
            logical
        );
        let lib = "pub mod app;\npub mod data;\n";
        let aliased = "pub mod app;\npub use my_app_data as data;\n";
        assert_eq!(layout.to_physical("src/lib.rs", lib), aliased);
        assert_eq!(layout.to_logical("src/lib.rs", aliased), lib);
    }

    #[test]
    fn feature_crates_get_a_manifest_and_a_dependency() {
        let layout = Layout::workspace("my-app");
        let paths = [
            "src/features/notes/bloc/notes_bloc.rs",
            "src/features/notes/bloc/notes_state.rs",
            "src/data/models/note.rs",
        ];
        let files = layout.crate_files(paths);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "crates/my-app-notes/Cargo.toml");
        assert!(files[0].1.contains("name = \"my-app-notes\""));
        assert!(files[0].1.contains("my-app-data.workspace = true"));
        assert!(!files[0].1.contains("rok-ui."));
        assert_eq!(
            layout.dependency_lines(paths),
            ["my-app-notes = { path = \"../my-app-notes\" }"]
        );
        assert_eq!(Layout::Single.crate_files(paths).len(), 0);
        assert!(layout.check(paths, true).is_err());
        assert!(layout
            .check(["src/features/data/bloc/data_bloc.rs"], false)
            .is_err());
        assert!(layout.check(paths, false).is_ok());
    }

    #[test]
    fn the_layout_comes_from_rok_ui_toml() {
        let root = std::env::temp_dir().join(format!("rok-ui-layout-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        assert_eq!(Layout::detect(&root), Ok(Layout::Single));
        fs::write(root.join("rok-ui.toml"), "routes = \"src/routes\"\n").unwrap();
        assert_eq!(Layout::detect(&root), Ok(Layout::Single));
        fs::write(
            root.join("rok-ui.toml"),
            "layout = \"workspace\"\napp = \"my-app\"\n",
        )
        .unwrap();
        assert_eq!(Layout::detect(&root), Ok(Layout::workspace("my-app")));
        fs::write(root.join("rok-ui.toml"), "layout = \"workspace\"\n").unwrap();
        assert!(Layout::detect(&root).is_err());
        fs::remove_dir_all(&root).ok();
    }
}
