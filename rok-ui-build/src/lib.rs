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
    collections::{BTreeMap, HashSet},
    env,
    fmt::{self, Write as _},
    fs, io,
    path::{Path, PathBuf},
};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    Ident, Token, Type,
};

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

/// One part of a route file's path.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Segment {
    Static(String),
    Param(String),
    Splat,
    Pathless(String),
    Index,
}

impl Segment {
    fn parse(part: &str) -> Option<Self> {
        if let Some(inner) = part
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            return Some(Self::Static(inner.to_string()));
        }
        if part.starts_with('(') && part.ends_with(')') {
            return None;
        }
        Some(match part {
            "$" => Self::Splat,
            "index" => Self::Index,
            _ if part.starts_with('$') => Self::Param(part[1..].to_string()),
            _ if part.starts_with('_') && !part.starts_with("__") => {
                Self::Pathless(part[1..].to_string())
            }
            _ => Self::Static(part.to_string()),
        })
    }

    fn is_path(&self) -> bool {
        matches!(self, Self::Static(_) | Self::Param(_) | Self::Splat)
    }
}

/// Split a file stem on dots that are not inside `[..]`.
fn split_flat(stem: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut depth = 0_u32;
    for character in stem.chars() {
        match character {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            '.' if depth == 0 => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = parts.last_mut() {
            last.push(character);
        }
    }
    parts
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Special {
    None,
    Root,
    NotFound,
}

/// One route file.
#[derive(Debug)]
struct Node {
    /// The path relative to the routes directory, with `/` separators.
    relative: String,
    absolute: PathBuf,
    segments: Vec<Segment>,
    special: Special,
    params: BTreeMap<String, String>,
    layout: bool,
}

impl Node {
    fn read(root: &Path, relative: &Path) -> Result<Self, Error> {
        let relative_text = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let stem = relative_text.strip_suffix(".rs").unwrap_or(&relative_text);
        let special = match stem {
            "__root" => Special::Root,
            "__not_found" => Special::NotFound,
            _ => Special::None,
        };
        let segments = if special == Special::None {
            stem.split('/')
                .flat_map(split_flat)
                .filter_map(|part| Segment::parse(&part))
                .collect()
        } else {
            Vec::new()
        };
        let absolute = root.join(relative);
        let source = fs::read_to_string(&absolute)?;
        let declaration = read_declaration(&source).map_err(|message| Error::Route {
            file: relative_text.clone(),
            message,
        })?;
        let node = Self {
            relative: relative_text,
            absolute,
            segments,
            special,
            params: declaration.params,
            layout: declaration.layout,
        };
        node.check()?;
        Ok(node)
    }

    fn check(&self) -> Result<(), Error> {
        let expected_layout = match self.special {
            Special::Root => Some(true),
            Special::NotFound => Some(false),
            Special::None if self.segments.contains(&Segment::Index) => Some(false),
            Special::None => None,
        };
        if expected_layout.is_some_and(|layout| layout != self.layout) {
            return Err(Error::Route {
                file: self.relative.clone(),
                message: if self.layout {
                    "this file is a page; declare `component: ..` instead of `layout: ..`".into()
                } else {
                    "`__root.rs` is a layout; declare `layout: ..`".into()
                },
            });
        }
        let in_path: HashSet<String> = self
            .segments
            .iter()
            .filter_map(|segment| match segment {
                Segment::Param(name) => Some(name.clone()),
                Segment::Splat => Some("splat".into()),
                _ => None,
            })
            .collect();
        for name in self.params.keys() {
            if !in_path.contains(name) {
                return Err(Error::Route {
                    file: self.relative.clone(),
                    message: format!(
                        "`params` declares `{name}`, but the path has no `${name}` segment"
                    ),
                });
            }
        }
        Ok(())
    }

    /// The segments a child must start with to be inside this layout.
    fn prefix(&self) -> &[Segment] {
        &self.segments
    }

    fn pattern(&self) -> String {
        let mut pattern = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Static(text) => {
                    pattern.push('/');
                    pattern.push_str(text);
                }
                Segment::Param(name) => {
                    pattern.push_str("/:");
                    pattern.push_str(name);
                }
                Segment::Splat => pattern.push_str("/*splat"),
                Segment::Pathless(_) | Segment::Index => {}
            }
        }
        if pattern.is_empty() {
            pattern.push('/');
        }
        pattern
    }

    /// The route type's name: `NotesId` for `/notes/:id`, `Index` for `/`.
    fn type_name(&self) -> String {
        let mut name = String::new();
        for segment in self.segments.iter().filter(|segment| segment.is_path()) {
            match segment {
                Segment::Static(text) | Segment::Param(text) => name.push_str(&camel_case(text)),
                Segment::Splat => name.push_str("Splat"),
                _ => {}
            }
        }
        if name.is_empty() {
            name.push_str("Index");
        }
        if name.starts_with(|character: char| character.is_ascii_digit()) {
            name.insert(0, 'R');
        }
        name
    }

    /// The path parameters in pattern order, with their types (`String` when undeclared).
    fn fields(&self) -> Vec<(String, String)> {
        self.segments
            .iter()
            .filter_map(|segment| match segment {
                Segment::Param(name) => Some(name.clone()),
                Segment::Splat => Some("splat".into()),
                _ => None,
            })
            .map(|name| {
                let ty = self
                    .params
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| "String".into());
                (name, ty)
            })
            .collect()
    }
}

fn camel_case(text: &str) -> String {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn module_name(relative: &str, used: &mut HashSet<String>) -> String {
    let stem = relative.strip_suffix(".rs").unwrap_or(relative);
    let words: Vec<String> = stem
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let name = format!("file_{}", words.join("_"));
    let mut unique = name.clone();
    let mut counter = 2;
    while !used.insert(unique.clone()) {
        unique = format!("{name}_{counter}");
        counter += 1;
    }
    unique
}

/// `params: { name: Type, .. }` inside the file's `file_route!` invocation.
struct Declaration {
    params: BTreeMap<String, String>,
    layout: bool,
}

impl Parse for Declaration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut params = BTreeMap::new();
        let mut layout = false;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            layout |= key == "layout";
            if key == "params" {
                let content;
                syn::braced!(content in input);
                while !content.is_empty() {
                    let name: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;
                    let ty: Type = content.parse()?;
                    params.insert(name.to_string(), ty.to_token_stream().to_string());
                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
            } else if key == "search" {
                input.parse::<Type>()?;
            } else {
                input.parse::<syn::Expr>()?;
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { params, layout })
    }
}

fn read_declaration(source: &str) -> Result<Declaration, String> {
    let file = syn::parse_file(source).map_err(|error| format!("does not parse: {error}"))?;
    let invocation = file.items.iter().find_map(|item| match item {
        syn::Item::Macro(item)
            if item
                .mac
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "file_route") =>
        {
            Some(item.mac.tokens.clone())
        }
        _ => None,
    });
    let Some(tokens) = invocation else {
        return Err("has no `file_route! { .. }`".into());
    };
    syn::parse2::<Declaration>(tokens).map_err(|error| format!("invalid `file_route!`: {error}"))
}

/// Render the generated module.
fn render(module: &str, nodes: &[Node]) -> Result<String, Error> {
    let mut used = HashSet::new();
    let modules: Vec<String> = nodes
        .iter()
        .map(|node| module_name(&node.relative, &mut used))
        .collect();
    let routes: Vec<usize> = (0..nodes.len())
        .filter(|&index| nodes[index].special == Special::None)
        .collect();
    let is_layout = |index: usize| nodes[index].layout;
    let root = nodes.iter().position(|node| node.special == Special::Root);

    let mut out = String::new();
    out.push_str("// @generated by rok-ui-build from the route files. Do not edit.\n\n");
    write!(out,
        "/// The app's routes, generated from its route files.\n#[allow(dead_code, unused_imports, missing_docs, non_snake_case, clippy::all, clippy::pedantic)]\npub mod {module} {{\n"
    ).ok();

    let mut names = BTreeMap::new();
    for &index in &routes {
        if is_layout(index) {
            continue;
        }
        let node = &nodes[index];
        let name = node.type_name();
        if let Some(previous) = names.insert(name.clone(), node.relative.clone()) {
            return Err(Error::Route {
                file: node.relative.clone(),
                message: format!("its route type `{name}` is also generated for `{previous}`"),
            });
        }
        let pattern = node.pattern();
        write!(out,
            "    ::rok_ui::typed_route! {{\n        /// `{pattern}`, from `{}`.\n        pub struct {name} = {pattern:?}",
            node.relative
        ).ok();
        let fields = node.fields();
        if fields.is_empty() {
            out.push_str(";\n    }\n\n");
        } else {
            out.push_str(" {\n");
            for (field, ty) in fields {
                write!(out,
                    "            /// The `{field}` path parameter.\n            pub {field}: {ty},\n"
                ).ok();
            }
            out.push_str("        }\n    }\n\n");
        }
    }

    for (index, node) in nodes.iter().enumerate() {
        let module_name = &modules[index];
        write!(
            out,
            "    /// `{}`.\n    pub mod {module_name} {{\n",
            node.relative
        )
        .ok();
        if node.special == Special::None && !is_layout(index) {
            let name = node.type_name();
            write!(out,
                "        /// This file's route.\n        pub type Route = super::{name};\n\n        /// This route's params. Call it while this route is shown.\n        pub fn params(cx: &mut ::rok_ui::gpui::App) -> Route {{\n            ::rok_ui::router::use_params::<Route>(cx).expect(\"the current location is not this route\")\n        }}\n\n"
            ).ok();
        }
        write!(
            out,
            "        include!({:?});\n    }}\n\n",
            node.absolute.to_string_lossy().replace('\\', "/")
        )
        .ok();
    }

    out.push_str("    /// The route tree, for a `Router` or `RouterProvider`.\n    pub fn tree() -> ::rok_ui::router::Router {\n        ::rok_ui::router::Router::new()\n");
    for &index in &routes {
        if is_layout(index) {
            continue;
        }
        let node = &nodes[index];
        let mut chain: Vec<usize> = routes
            .iter()
            .copied()
            .filter(|&layout| {
                layout != index
                    && is_layout(layout)
                    && node.segments.len() > nodes[layout].segments.len()
                    && node.segments.starts_with(nodes[layout].prefix())
            })
            .collect();
        chain.sort_by_key(|&layout| nodes[layout].segments.len());
        if let Some(root) = root {
            chain.insert(0, root);
        }
        let guards: Vec<String> = chain
            .iter()
            .chain(std::iter::once(&index))
            .map(|&file| {
                format!(
                    "::std::rc::Rc::new({}::__rok_guard) as ::rok_ui::router::Guard",
                    modules[file]
                )
            })
            .collect();
        write!(out,
            "            .__file_route({:?}, ::std::vec![{}], ::core::option::Option::Some(::std::rc::Rc::new({}::__rok_loader) as ::rok_ui::router::Loader), |route, window, cx| {{\n                let outlet = {}::__rok_page(route, window, cx)?;\n",
            node.pattern(),
            guards.join(", "),
            modules[index],
            modules[index]
        )
        .ok();
        for &layout in chain.iter().rev() {
            writeln!(
                out,
                "                let outlet = {}::__rok_layout(outlet, route, window, cx);",
                modules[layout]
            )
            .ok();
        }
        out.push_str("                ::core::option::Option::Some(outlet)\n            })\n");
    }
    if let Some(not_found) = nodes
        .iter()
        .position(|node| node.special == Special::NotFound)
    {
        write!(out,
            "            .not_found(|route, window, cx| {{\n                let outlet = {}::__rok_page(route, window, cx)\n                    .unwrap_or_else(|| ::rok_ui::gpui::IntoElement::into_any_element(::rok_ui::gpui::div()));\n",
            modules[not_found]
        ).ok();
        if let Some(root) = root {
            writeln!(
                out,
                "                {}::__rok_layout(outlet, route, window, cx)",
                modules[root]
            )
            .ok();
        } else {
            out.push_str("                outlet\n");
        }
        out.push_str("            })\n");
    }
    out.push_str("    }\n}\n");

    // Make sure the output is valid Rust before handing it to the compiler.
    out.parse::<TokenStream>().map_err(|error| Error::Route {
        file: "(generated)".into(),
        message: format!("generated code does not tokenize: {error}"),
    })?;
    Ok(out)
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
    fn segments_follow_tanstack_conventions() {
        assert_eq!(split_flat("notes.$id.edit"), ["notes", "$id", "edit"]);
        assert_eq!(split_flat("[v1.0].index"), ["[v1.0]", "index"]);
        assert_eq!(Segment::parse("$id"), Some(Segment::Param("id".into())));
        assert_eq!(Segment::parse("$"), Some(Segment::Splat));
        assert_eq!(
            Segment::parse("_auth"),
            Some(Segment::Pathless("auth".into()))
        );
        assert_eq!(Segment::parse("(marketing)"), None);
        assert_eq!(
            Segment::parse("[rok-ui]"),
            Some(Segment::Static("rok-ui".into()))
        );
        assert_eq!(camel_case("rok-ui"), "RokUi");
    }

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
