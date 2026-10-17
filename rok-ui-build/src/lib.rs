//! Build-script support for rok-ui.
//!
//! [`routes`] scans `src/routes/` and generates a typed route tree, the way TanStack Router's
//! generator writes `routeTree.gen.ts`: one route type per page file, a module per file, and a
//! `tree()` function that builds the `Router`.
//!
//! ```no_run
//! // build.rs
//! rok_ui_build::routes("src/routes").generate().unwrap();
//! ```
//!
//! ```text
//! // src/main.rs
//! rok_ui::routes!();          // includes the generated `routes` module
//! // ... RouterProvider: routes::tree()
//! ```
//!
//! File conventions (names relative to the routes directory):
//!
//! | File | Route |
//! |---|---|
//! | `__root.rs` | The root layout, around every route |
//! | `__not_found.rs` | Shown when nothing matches (inside the root layout) |
//! | `index.rs` | `/` (the index of its folder) |
//! | `about.rs` | `/about` |
//! | `notes.rs` with `layout:` | A layout for every route under `/notes` |
//! | `notes/index.rs` | `/notes` |
//! | `notes/$id.rs` | `/notes/:id` |
//! | `notes.$id.edit.rs` | `/notes/:id/edit` (dots separate segments) |
//! | `files/$.rs` | `/files/*splat` |
//! | `_auth.rs` with `layout:` | A layout without a path segment, around `_auth/..` |
//! | `(marketing)/pricing.rs` | `/pricing` (a group folder only organizes files) |
//! | `-components/..` | Ignored: colocated helpers |
//!
//! A file that declares `layout: ..` in its `file_route!` wraps every route below its path;
//! one that declares `component: ..` is a page.
//! | `[rok-ui].rs` | `/rok-ui` (brackets escape special characters) |

use std::{
    env, fmt, fs, io,
    path::{Path, PathBuf},
};

mod render;
mod tree;

use render::{read_declaration, render};
use tree::{module_name, Node, Special};

/// Generate routes from the files in `dir` (relative to the crate root in a build script).
pub fn routes(dir: impl Into<PathBuf>) -> Routes {
    Routes {
        dir: dir.into(),
        module: "routes".into(),
    }
}

/// The route generator, configured by [`routes`].
#[derive(Clone, Debug)]
pub struct Routes {
    dir: PathBuf,
    module: String,
}

/// Why routes could not be generated.
#[derive(Debug)]
pub enum Error {
    /// Reading the routes directory or writing the output failed.
    Io(io::Error),
    /// `OUT_DIR` is not set: [`Routes::generate`] runs only in a build script.
    NoOutDir,
    /// A route file is invalid.
    Route {
        /// The file, relative to the routes directory.
        file: String,
        /// What is wrong.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::NoOutDir => {
                formatter.write_str("OUT_DIR is not set; call generate() from build.rs")
            }
            Self::Route { file, message } => write!(formatter, "{file}: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl Routes {
    /// The name of the generated module. Default: `routes`.
    #[must_use]
    pub fn module(mut self, name: impl Into<String>) -> Self {
        self.module = name.into();
        self
    }

    /// Write the route tree to `$OUT_DIR/rok_ui_routes.rs` for `rok_ui::routes!()`, and tell
    /// Cargo to rerun the build script when a route file changes.
    ///
    /// # Errors
    ///
    /// Fails outside a build script, on I/O errors, and on invalid route files.
    pub fn generate(self) -> Result<PathBuf, Error> {
        let out_dir = env::var_os("OUT_DIR").ok_or(Error::NoOutDir)?;
        let output = Path::new(&out_dir).join("rok_ui_routes.rs");
        println!("cargo:rerun-if-changed={}", self.dir.display());
        self.write_to(&output)?;
        Ok(output)
    }

    /// Write the route tree to `path`, for teams that check generated code in. Include it with
    /// `rok_ui::routes!("path")`.
    ///
    /// # Errors
    ///
    /// Fails on I/O errors and on invalid route files.
    pub fn write_to(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let source = self.source()?;
        if fs::read_to_string(path.as_ref()).ok().as_deref() != Some(source.as_str()) {
            fs::write(path, source)?;
        }
        Ok(())
    }

    /// The generated Rust source.
    ///
    /// # Errors
    ///
    /// Fails on I/O errors and on invalid route files.
    pub fn source(&self) -> Result<String, Error> {
        let dir = if self.dir.is_absolute() {
            self.dir.clone()
        } else {
            env::var_os("CARGO_MANIFEST_DIR")
                .map_or_else(|| self.dir.clone(), |root| Path::new(&root).join(&self.dir))
        };
        let mut files = Vec::new();
        collect_files(&dir, &dir, &mut files)?;
        files.sort();
        let nodes = files
            .iter()
            .map(|file| Node::read(&dir, file))
            .collect::<Result<Vec<_>, _>>()?;
        render(&self.module, &nodes)
    }
}

/// Every `.rs` file under `dir`, skipping `-ignored` entries.
fn collect_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.starts_with('-') || name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn fixture(name: &str) -> PathBuf {
        let root = env::temp_dir().join(format!("rok-ui-build-{name}-{}", std::process::id()));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).unwrap();
        root
    }

    const PAGE: &str = "file_route! { component: Page }";
    const LAYOUT: &str = "file_route! { layout: Layout }";

    #[test]
    fn generates_types_layouts_and_guards() {
        let root = fixture("tree");
        write(&root, "__root.rs", LAYOUT);
        write(&root, "__not_found.rs", PAGE);
        write(&root, "index.rs", PAGE);
        write(&root, "notes.rs", LAYOUT);
        write(&root, "notes/index.rs", PAGE);
        write(
            &root,
            "notes/$id.rs",
            "file_route! { params: { id: u64 }, component: Page }",
        );
        write(
            &root,
            "notes.$id.edit.rs",
            "file_route! { params: { id: u64 }, component: Page }",
        );
        write(&root, "files/$.rs", PAGE);
        write(
            &root,
            "_auth.rs",
            "file_route! { before_load: |location, _cx| Ok(()), layout: Layout }",
        );
        write(&root, "_auth/settings.rs", PAGE);
        write(&root, "(marketing)/pricing.rs", PAGE);
        write(&root, "-components/card.rs", "not a route");
        write(&root, "[rok-ui].rs", PAGE);

        let source = routes(&root).source().unwrap();
        for expected in [
            "pub struct Index = \"/\";",
            "pub struct Notes = \"/notes\";",
            "pub struct NotesId = \"/notes/:id\" {",
            "pub id: u64,",
            "pub struct NotesIdEdit = \"/notes/:id/edit\" {",
            "pub struct FilesSplat = \"/files/*splat\" {",
            "pub splat: String,",
            "pub struct Settings = \"/settings\";",
            "pub struct Pricing = \"/pricing\";",
            "pub struct RokUi = \"/rok-ui\";",
            ".not_found(",
        ] {
            assert!(
                source.contains(expected),
                "missing {expected:?} in:\n{source}"
            );
        }
        assert!(!source.contains("card"), "ignored files stay out");
        // `/notes/:id/edit` renders inside the root and notes layouts, outermost last.
        let edit = source
            .split(".__file_route(\"/notes/:id/edit\"")
            .nth(1)
            .unwrap();
        let edit = edit.split(".__file_route(").next().unwrap();
        let notes_layout = edit.find("file_notes::__rok_layout").unwrap();
        let root_layout = edit.find("file_root::__rok_layout").unwrap();
        assert!(notes_layout < root_layout);
        assert!(edit.contains("file_root::__rok_guard"));
        // `/settings` runs the pathless `_auth` layout's guard.
        let settings = source.split(".__file_route(\"/settings\"").nth(1).unwrap();
        assert!(settings
            .split(".__file_route(")
            .next()
            .unwrap()
            .contains("file_auth::__rok_guard"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn reports_mistakes_with_the_file_name() {
        let root = fixture("errors");
        write(
            &root,
            "notes/$id.rs",
            "file_route! { params: { note: u64 }, component: Page }",
        );
        let error = routes(&root).source().unwrap_err().to_string();
        assert!(
            error.contains("notes/$id.rs") && error.contains("`note`"),
            "{error}"
        );

        fs::remove_dir_all(&root).ok();
        write(&root, "about.rs", "pub fn nothing() {}");
        let error = routes(&root).source().unwrap_err().to_string();
        assert!(error.contains("has no `file_route!"), "{error}");
        fs::remove_dir_all(root).ok();
    }
}
