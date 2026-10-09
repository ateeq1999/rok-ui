//! `cargo rok-ui`: create rok-ui apps, vendor components and write route trees.
//!
//! ```text
//! cargo rok-ui new <name> [--template minimal|full|db|bloc [--http]] [--rok-ui-path <path>]
//! cargo rok-ui add <component>... [--dir src/components/ui] [--force]
//! cargo rok-ui routes [--dir src/routes] [--out src/route_tree.rs]
//! cargo rok-ui generate <feature|bloc|cubit|repository|provider|view|api|schema> ...
//! ```

mod add;
mod generate;
mod templates;

use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use templates::Template;

const USAGE: &str = "cargo rok-ui: create rok-ui apps, vendor components and write route trees

USAGE:
    cargo rok-ui new <name> [--template minimal|full|db|bloc [--http]] [--rok-ui-path <path>]
    cargo rok-ui add <component>... [--dir <dir>] [--force]
    cargo rok-ui routes [--dir <routes dir>] [--out <file>]
    cargo rok-ui generate <what> <name> [flags]     (alias: g; see `cargo rok-ui g help`)

COMMANDS:
    new       Create an app. Templates: minimal (one window), full (file-based routes,
              features, queries; the default), db (full plus PostgreSQL), bloc (the
              BLoC architecture: data, features and thin routes, with a notes feature;
              add --http for a notes API client, sign-in and a session guard).
    add       Copy components' source into the app (default: src/components/ui/) to change
              them, and declare them in the barrel file.
    routes    Write the route tree for src/routes to a checked-in file (default:
              src/route_tree.rs), for `rok_ui::routes!(\"route_tree.rs\")`.
    generate  Write a BLoC feature, or part of one (bloc, cubit, repository, provider, view,
              HTTP api), from flags and JSON, and wire it into the barrels and src/app.rs.";

/// Flags that take no value.
const SWITCHES: &[&str] = &[
    "force",
    "dry-run",
    "view",
    "no-view",
    "no-tests",
    "no-wire",
    "skip-expire",
    "server-errors",
    "http",
];

/// Options shared by the commands: positional arguments and `--name value` flags.
pub struct Arguments {
    /// Arguments that are not flags, in order.
    pub positional: Vec<String>,
    flags: Vec<(String, Option<String>)>,
}

impl Arguments {
    fn parse(arguments: impl Iterator<Item = String>) -> Self {
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        let mut arguments = arguments.peekable();
        while let Some(argument) = arguments.next() {
            // `-j` is `--json`.
            let argument = if argument == "-j" {
                "--json".to_string()
            } else {
                argument
            };
            if let Some(name) = argument.strip_prefix("--") {
                let value = match name.split_once('=') {
                    Some((name, value)) => {
                        flags.push((name.to_string(), Some(value.to_string())));
                        continue;
                    }
                    None if SWITCHES.contains(&name) => None,
                    // A value may be `-` (standard input), but not another flag.
                    None => arguments.next_if(|next| !next.starts_with("--") && next != "-j"),
                };
                flags.push((name.to_string(), value));
            } else {
                positional.push(argument);
            }
        }
        Self { positional, flags }
    }

    /// The last value of a flag.
    #[must_use]
    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .rev()
            .find(|(flag, _)| flag == name)
            .and_then(|(_, value)| value.as_deref())
    }

    /// Every value of a repeatable flag.
    #[must_use]
    pub fn flags(&self, name: &str) -> Vec<&str> {
        self.flags
            .iter()
            .filter(|(flag, _)| flag == name)
            .filter_map(|(_, value)| value.as_deref())
            .collect()
    }

    fn has(&self, name: &str) -> bool {
        self.flags.iter().any(|(flag, _)| flag == name)
    }
}

fn main() -> ExitCode {
    // `cargo rok-ui ..` runs `cargo-rok-ui rok-ui ..`.
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().map(String::as_str) == Some("rok-ui") {
        arguments.remove(0);
    }
    let Some(command) = arguments.first().cloned() else {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    };
    let arguments = Arguments::parse(arguments.into_iter().skip(1));
    if matches!(command.as_str(), "generate" | "g") {
        return generate::run(&arguments);
    }
    let result = match command.as_str() {
        "new" => new(&arguments),
        "add" => add(&arguments),
        "routes" => routes(&arguments),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The dependency lines for rok-ui and rok-ui-build.
fn dependencies(template: Template, local: Option<&Path>) -> (String, String) {
    let features = template.features();
    let version = env!("CARGO_PKG_VERSION");
    let source = |crate_dir: &str| match local {
        Some(root) => {
            let path = root.join(crate_dir);
            format!("path = {:?}", path.to_string_lossy().replace('\\', "/"))
        }
        None => format!("version = \"{version}\""),
    };
    let rok_ui = if features.is_empty() {
        format!("rok-ui = {{ {} }}", source("."))
    } else {
        let list: Vec<String> = features
            .iter()
            .map(|feature| format!("\"{feature}\""))
            .collect();
        format!(
            "rok-ui = {{ {}, features = [{}] }}",
            source("."),
            list.join(", ")
        )
    };
    let build = format!("rok-ui-build = {{ {} }}", source("rok-ui-build"));
    (rok_ui, build)
}

/// Write `template` into `directory` for a crate called `name`.
fn create(
    directory: &Path,
    name: &str,
    template: Template,
    local: Option<&Path>,
) -> Result<(), String> {
    if directory.exists()
        && fs::read_dir(directory)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err(format!("{} exists and is not empty", directory.display()));
    }
    let (rok_ui, rok_ui_build) = dependencies(template, local);
    let crate_name = name.replace('-', "_");
    for file in template.files() {
        let text = file
            .text
            .replace("{{name}}", name)
            .replace("{{crate_name}}", &crate_name)
            .replace("{{rok_ui}}", &rok_ui)
            .replace("{{rok_ui_build}}", &rok_ui_build);
        let path = directory.join(file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(&path, text).map_err(|error| error.to_string())?;
    }
    fs::write(directory.join(".gitignore"), "/target\n").map_err(|error| error.to_string())?;
    Ok(())
}

fn new(arguments: &Arguments) -> Result<(), String> {
    let name = arguments
        .positional
        .first()
        .ok_or("usage: cargo rok-ui new <name> [--template minimal|full|db|bloc [--http]]")?;
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(format!("`{name}` is not a valid crate name"));
    }
    let template_name = match (arguments.flag("template"), arguments.has("http")) {
        (Some("bloc"), true) => "bloc-http",
        (_, true) => return Err("--http goes with --template bloc".into()),
        (template, false) => template.unwrap_or("full"),
    };
    let template = Template::parse(template_name).ok_or_else(|| {
        format!("unknown template `{template_name}`; use minimal, full, db, bloc or bloc-http")
    })?;
    let local = arguments
        .flag("rok-ui-path")
        .map(|path| fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path)));
    let directory = PathBuf::from(arguments.flag("path").unwrap_or(name));
    create(&directory, name, template, local.as_deref())?;
    match template {
        Template::Bloc => generate::feature_into(&directory, templates::BLOC_FEATURE)?,
        Template::BlocHttp => {
            for feature in templates::BLOC_HTTP_FEATURES {
                generate::feature_into(&directory, feature)?;
            }
        }
        _ => {}
    }
    println!(
        "Created {} ({template_name} template).",
        directory.display()
    );
    println!("  cd {} && cargo run", directory.display());
    if template.uses_build_script() {
        println!("Routes live in src/routes; build.rs turns them into the `routes` module.");
    }
    Ok(())
}

fn add(arguments: &Arguments) -> Result<(), String> {
    if arguments.positional.is_empty() {
        return Err("usage: cargo rok-ui add <component>... [--dir src/components/ui]".into());
    }
    let project = std::env::current_dir().map_err(|error| error.to_string())?;
    let source = add::rok_ui_source(&project)?;
    let target = project.join(arguments.flag("dir").unwrap_or("src/components/ui"));
    let mut dependencies: Vec<&str> = Vec::new();
    for component in &arguments.positional {
        for path in add::add(&source, component, &target, arguments.has("force"))? {
            let text = fs::read_to_string(&path).unwrap_or_default();
            for dependency in add::extra_dependencies(&text) {
                if !dependencies.contains(&dependency) {
                    dependencies.push(dependency);
                }
            }
            println!(
                "wrote {}",
                path.strip_prefix(&project).unwrap_or(&path).display()
            );
        }
    }
    if !dependencies.is_empty() {
        println!("The copies use these crates; add them to [dependencies]:");
        for dependency in dependencies {
            println!("    {dependency}");
        }
    }
    println!("Use the copies through your `components::ui` module (declare `pub mod ui;` in src/components.rs).");
    Ok(())
}

fn routes(arguments: &Arguments) -> Result<(), String> {
    let project = std::env::current_dir().map_err(|error| error.to_string())?;
    let directory = project.join(arguments.flag("dir").unwrap_or("src/routes"));
    let output = project.join(arguments.flag("out").unwrap_or("src/route_tree.rs"));
    rok_ui_build::routes(&directory)
        .write_to(&output)
        .map_err(|error| error.to_string())?;
    println!(
        "wrote {}",
        output.strip_prefix(&project).unwrap_or(&output).display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_parse_flags_and_positionals() {
        let arguments = Arguments::parse(
            ["my-app", "--template", "db", "--force", "--dir=src/ui"]
                .into_iter()
                .map(String::from),
        );
        assert_eq!(arguments.positional, ["my-app"]);
        assert_eq!(arguments.flag("template"), Some("db"));
        assert!(arguments.has("force"));
        assert_eq!(arguments.flag("dir"), Some("src/ui"));
    }

    #[test]
    fn templates_fill_their_placeholders() {
        let directory = std::env::temp_dir().join(format!("rok-ui-cli-new-{}", std::process::id()));
        fs::remove_dir_all(&directory).ok();
        create(
            &directory,
            "my-app",
            Template::Db,
            Some(Path::new("/src/rok-ui")),
        )
        .unwrap();
        let manifest = fs::read_to_string(directory.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("name = \"my-app\""));
        assert!(manifest.contains(
            "rok-ui = { path = \"/src/rok-ui/.\", features = [\"router\", \"query\", \"db\"] }"
        ));
        assert!(manifest.contains("rok-ui-build = { path = \"/src/rok-ui/rok-ui-build\" }"));
        let main = fs::read_to_string(directory.join("src/main.rs")).unwrap();
        assert!(main.contains("use my_app::routes;"));
        assert!(directory.join("src/routes/notes/$id.rs").exists());
        assert!(directory.join("migrations/0001_create_notes.sql").exists());
        for entry in walk(&directory) {
            let text = fs::read_to_string(&entry).unwrap();
            assert!(
                !text.contains("{{"),
                "{} has an unfilled placeholder",
                entry.display()
            );
        }
        assert!(
            create(&directory, "my-app", Template::Db, None).is_err(),
            "refuses a non-empty directory"
        );
        fs::remove_dir_all(directory).ok();
    }

    fn walk(directory: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.extend(walk(&path));
            } else {
                files.push(path);
            }
        }
        files
    }
}
