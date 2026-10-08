//! Project templates for `cargo rok-ui new`.

/// A file of a template: its path in the new project and its text, with `{{name}}`,
/// `{{crate_name}}`, `{{rok_ui}}` and `{{rok_ui_build}}` placeholders.
pub struct TemplateFile {
    pub path: &'static str,
    pub text: &'static str,
}

macro_rules! file {
    ($path:literal, $source:literal) => {
        TemplateFile {
            path: $path,
            text: include_str!(concat!("../templates/", $source)),
        }
    };
}

const MINIMAL: &[TemplateFile] = &[
    file!("Cargo.toml", "minimal/Cargo.toml.tmpl"),
    file!("src/main.rs", "minimal/src/main.rs"),
];

const FULL_SHARED: &[TemplateFile] = &[
    file!("Cargo.toml", "full/Cargo.toml.tmpl"),
    file!("build.rs", "full/build.rs"),
    file!("rok-ui.toml", "full/rok-ui.toml"),
    file!("src/lib.rs", "full/src/lib.rs"),
    file!("src/components.rs", "full/src/components.rs"),
    file!("src/components/ui.rs", "full/src/components/ui.rs"),
    file!("src/features.rs", "full/src/features.rs"),
    file!("src/routes/__root.rs", "full/src/routes/__root.rs"),
    file!(
        "src/routes/__not_found.rs",
        "full/src/routes/__not_found.rs"
    ),
    file!("src/routes/index.rs", "full/src/routes/index.rs"),
    file!("src/routes/notes.rs", "full/src/routes/notes.rs"),
    file!(
        "src/routes/notes/index.rs",
        "full/src/routes/notes/index.rs"
    ),
    file!("src/routes/notes/$id.rs", "full/src/routes/notes/$id.rs"),
    file!("tests/routes.rs", "full/tests/routes.rs"),
];

const FULL_ONLY: &[TemplateFile] = &[
    file!("src/main.rs", "full/src/main.rs"),
    file!("src/features/notes.rs", "full/src/features/notes.rs"),
];

const DB_ONLY: &[TemplateFile] = &[
    file!("src/main.rs", "db/main.rs"),
    file!("src/features/notes.rs", "db/notes.rs"),
    file!(
        "migrations/0001_create_notes.sql",
        "db/migrations/0001_create_notes.sql"
    ),
];

const BLOC: &[TemplateFile] = &[
    file!("Cargo.toml", "bloc/Cargo.toml.tmpl"),
    file!("build.rs", "bloc/build.rs"),
    file!("clippy.toml", "bloc/clippy.toml"),
    file!("rok-ui.toml", "bloc/rok-ui.toml"),
    file!("src/main.rs", "bloc/src/main.rs"),
    file!("src/lib.rs", "bloc/src/lib.rs"),
    file!("src/app.rs", "bloc/src/app.rs"),
    file!("src/data.rs", "bloc/src/data.rs"),
    file!("src/features.rs", "bloc/src/features.rs"),
    file!("src/shared.rs", "bloc/src/shared.rs"),
    file!("src/shared/widgets.rs", "bloc/src/shared/widgets.rs"),
    file!("src/routes/__root.rs", "bloc/src/routes/__root.rs"),
    file!(
        "src/routes/__not_found.rs",
        "bloc/src/routes/__not_found.rs"
    ),
    file!("src/routes/index.rs", "bloc/src/routes/index.rs"),
];

/// The feature the `bloc` template generates into a new app.
pub const BLOC_FEATURE: &str = include_str!("../fixtures/notes.json");

/// A project template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Template {
    /// One window with a counter.
    Minimal,
    /// File-based routes, features, queries and a test.
    Full,
    /// `Full` with `PostgreSQL` through rok-db.
    Db,
    /// The BLoC architecture: data, features (bloc + view) and thin routes, with a generated
    /// `notes` feature.
    Bloc,
}

impl Template {
    /// Parse a template name.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "minimal" => Some(Self::Minimal),
            "full" => Some(Self::Full),
            "db" => Some(Self::Db),
            "bloc" => Some(Self::Bloc),
            _ => None,
        }
    }

    /// The rok-ui features the template needs.
    pub fn features(self) -> &'static [&'static str] {
        match self {
            Self::Minimal => &[],
            Self::Full => &["router", "query"],
            Self::Db => &["router", "query", "db"],
            Self::Bloc => &["router", "bloc", "form", "typography", "empty"],
        }
    }

    /// Whether the template generates routes in `build.rs`.
    pub fn uses_build_script(self) -> bool {
        self != Self::Minimal
    }

    /// The template's files.
    pub fn files(self) -> Vec<&'static TemplateFile> {
        match self {
            Self::Minimal => MINIMAL.iter().collect(),
            Self::Full => FULL_SHARED.iter().chain(FULL_ONLY).collect(),
            Self::Db => FULL_SHARED.iter().chain(DB_ONLY).collect(),
            Self::Bloc => BLOC.iter().collect(),
        }
    }
}
