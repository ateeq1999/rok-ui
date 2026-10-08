//! From a [`Plan`] to the files on disk: which files a subcommand writes, the barrel files
//! that declare them, the lines `app.rs` gets, formatting, conflicts and atomic writes.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use super::{render, resolve::Plan, spec::Kind};

/// What a subcommand writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// Models, the data error, provider, repository, bloc, view, route and tests.
    Feature,
    /// The bloc or cubit with its tests; the data layer when missing.
    Bloc,
    /// The repository; its provider and models when missing.
    Repository,
    /// The provider; its models when missing.
    Provider,
    /// The page and its route.
    View,
    /// An HTTP provider and its repository.
    Api,
}

/// How a file is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// The file the command is for: an existing different file is a conflict.
    Own,
    /// A file the command needs: written only when missing.
    Support,
}

/// A file the generator wants.
struct File {
    path: String,
    text: String,
    mode: Mode,
}

/// What happens to a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// A new file.
    Create,
    /// An existing file changes (barrels, `app.rs`, or `--force`).
    Update,
    /// The file already says this.
    Unchanged,
    /// An existing file the command would overwrite (without `--force`).
    Conflict,
}

/// The outcome of [`plan`]: every path and what happens to it.
#[derive(Default)]
pub struct Changes {
    /// Path (relative to the project) to its action and new text.
    pub files: BTreeMap<String, (Action, String)>,
    /// Lines to add to `app.rs` by hand, when it has no markers.
    pub snippets: Vec<String>,
}

impl Changes {
    /// The paths that conflict.
    pub fn conflicts(&self) -> Vec<&str> {
        self.files
            .iter()
            .filter(|(_, (action, _))| *action == Action::Conflict)
            .map(|(path, _)| path.as_str())
            .collect()
    }
}

/// Options from the command line.
#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Options {
    /// Overwrite files that differ.
    pub force: bool,
    /// Write the page with a bloc (`--view`), or not with a feature (`--no-view`).
    pub view: Option<bool>,
    /// Do not touch barrel files or `app.rs`.
    pub no_wire: bool,
}

fn files(plan: &Plan, part: Part, crate_name: &str, options: Options) -> Vec<File> {
    use Mode::{Own, Support};
    let mut out = Vec::new();
    let mut add = |path: String, text: String, mode: Mode| out.push(File { path, text, mode });
    let data =
        |mode_provider: Mode, mode_repository: Mode, add: &mut dyn FnMut(String, String, Mode)| {
            for model in &plan.models {
                add(
                    format!("src/data/models/{}.rs", model.names.snake),
                    render::model(plan, model),
                    Support,
                );
            }
            if !plan.is_http() || plan.error_type() == "DataError" {
                add("src/data/error.rs".into(), render::data_error(), Support);
            }
            if let Some(provider) = &plan.provider {
                add(
                    format!("src/data/providers/{}.rs", provider.names.snake),
                    render::provider(plan),
                    mode_provider,
                );
            }
            if let Some(repository) = &plan.repository {
                add(
                    format!("src/data/repositories/{}.rs", repository.names.snake),
                    render::repository(plan),
                    mode_repository,
                );
            }
        };
    let feature = &plan.feature.snake;
    let name = &plan.name.snake;
    let logic = |add: &mut dyn FnMut(String, String, Mode)| {
        let directory = format!("src/features/{feature}/bloc");
        if plan.kind == Kind::Bloc {
            add(
                format!("{directory}/{name}_event.rs"),
                render::event(plan),
                Own,
            );
            add(
                format!("{directory}/{name}_bloc.rs"),
                render::bloc(plan),
                Own,
            );
        } else {
            add(
                format!("{directory}/{name}_cubit.rs"),
                render::cubit(plan),
                Own,
            );
        }
        add(
            format!("{directory}/{name}_state.rs"),
            render::state(plan),
            Own,
        );
        if plan.tests {
            let suffix = if plan.kind == Kind::Bloc {
                "bloc"
            } else {
                "cubit"
            };
            add(
                format!("tests/features/{feature}/{name}_{suffix}_test.rs"),
                render::test(plan, crate_name),
                Own,
            );
        }
    };
    let view = |add: &mut dyn FnMut(String, String, Mode)| {
        add(
            format!(
                "src/features/{feature}/view/{}.rs",
                super::names::snake(&render::page_name(plan))
            ),
            render::page(plan),
            Own,
        );
        add(format!("src/routes/{feature}.rs"), render::route(plan), Own);
    };
    match part {
        Part::Feature => {
            data(Own, Own, &mut add);
            logic(&mut add);
            if options.view.unwrap_or(plan.page) {
                view(&mut add);
            }
        }
        Part::Bloc => {
            data(Support, Support, &mut add);
            logic(&mut add);
            if options.view.unwrap_or(false) {
                view(&mut add);
            }
        }
        Part::Repository | Part::Api => data(Own, Own, &mut add),
        Part::Provider => {
            for model in &plan.models {
                add(
                    format!("src/data/models/{}.rs", model.names.snake),
                    render::model(plan, model),
                    Support,
                );
            }
            if !plan.is_http() {
                add("src/data/error.rs".into(), render::data_error(), Support);
            }
            if let Some(provider) = &plan.provider {
                add(
                    format!("src/data/providers/{}.rs", provider.names.snake),
                    render::provider(plan),
                    Own,
                );
            }
        }
        Part::View => view(&mut add),
    }
    out
}

/// Format Rust source with rustfmt, if it is installed.
fn format(path: &str, text: &str) -> Result<String, String> {
    let child = Command::new("rustfmt")
        .args(["--edition", "2021", "--emit", "stdout", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        return Ok(text.to_string());
    };
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "{path}: the generated code does not parse (a generator bug):\n{}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// The module a file declares in its parent, and the parent's path, for files under `src/`
/// and `tests/<root>/`.
fn parent_barrel(path: &str) -> Option<(String, String)> {
    let without = path.strip_suffix(".rs")?;
    let (parent, module) = without.rsplit_once('/')?;
    if parent == "src"
        || parent == "tests"
        || parent == "src/routes"
        || parent.starts_with("src/routes/")
    {
        return None;
    }
    if module == "main" && parent.starts_with("tests/") {
        return None;
    }
    // `tests/<suite>/main.rs` is the root of the `<suite>` test (Cargo finds it), so that its
    // modules can live in `tests/<suite>/`.
    if let Some(suite) = parent
        .strip_prefix("tests/")
        .filter(|suite| !suite.contains('/'))
    {
        return Some((format!("tests/{suite}/main.rs"), module.to_string()));
    }
    Some((format!("{parent}.rs"), module.to_string()))
}

/// `text` declaring `module` (`pub mod` in `src`, `mod` in tests), with the declarations kept
/// sorted where they are.
pub fn declare(text: &str, module: &str, public: bool) -> String {
    let keyword = if public { "pub mod" } else { "mod" };
    let line = format!("{keyword} {module};");
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let is_declaration = |line: &str| {
        let line = line.trim();
        (line.starts_with("pub mod ") || line.starts_with("mod ")) && line.ends_with(';')
    };
    if lines.iter().any(|existing| existing.trim() == line) {
        return text.to_string();
    }
    let positions: Vec<usize> = (0..lines.len())
        .filter(|&index| is_declaration(&lines[index]))
        .collect();
    if let Some(&last) = positions.last() {
        lines.insert(last + 1, line);
        let positions: Vec<usize> = (0..lines.len())
            .filter(|&index| is_declaration(&lines[index]))
            .collect();
        let mut sorted: Vec<String> = positions
            .iter()
            .map(|&index| lines[index].clone())
            .collect();
        sorted.sort_by_key(|line| line.trim().trim_start_matches("pub ").to_string());
        for (index, line) in positions.into_iter().zip(sorted) {
            lines[index] = line;
        }
    } else {
        while lines.last().is_some_and(|line| line.trim().is_empty()) {
            lines.pop();
        }
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(line);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// A new barrel file for `path`.
fn new_barrel(path: &str) -> String {
    let mut out = format!("//! {}\n", render::barrel_doc(path));
    if path.starts_with("src/features/") && path.ends_with("/bloc.rs") {
        out.push_str("\n// The business layer stays free of GPUI: see `clippy.toml`.\n#![deny(clippy::disallowed_types)]\n");
    }
    out
}

/// Insert `line` before `marker` in `text`, at the marker's indentation, unless `text` has it.
fn insert_at_marker(text: &str, marker: &str, line: &str) -> Option<String> {
    let at = text
        .lines()
        .position(|existing| existing.trim() == marker)?;
    if text.lines().any(|existing| existing.trim() == line.trim()) {
        return Some(text.to_string());
    }
    let mut lines: Vec<&str> = text.lines().collect();
    let indentation: String = lines[at]
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect();
    let new_line = format!("{indentation}{line}");
    lines.insert(at, &new_line);
    let mut out = lines.join("\n");
    out.push('\n');
    Some(out)
}

/// Work out every change, reading the project at `root` and writing nothing.
///
/// # Errors
///
/// When rustfmt rejects generated code, or a file cannot be read.
pub fn plan(
    root: &Path,
    plan: &Plan,
    part: Part,
    crate_name: &str,
    options: Options,
) -> Result<Changes, String> {
    let mut changes = Changes::default();
    let read = |path: &str| fs::read_to_string(root.join(path)).ok();
    let mut wanted: Vec<File> = files(plan, part, crate_name, options);
    // Barrels.
    if !options.no_wire {
        let mut barrels: BTreeMap<String, String> = BTreeMap::new();
        let mut pending: Vec<String> = wanted.iter().map(|file| file.path.clone()).collect();
        while let Some(path) = pending.pop() {
            let Some((barrel, module)) = parent_barrel(&path) else {
                // Top-level modules go in lib.rs.
                if let Some(module) = path
                    .strip_prefix("src/")
                    .and_then(|rest| rest.strip_suffix(".rs"))
                {
                    if !module.contains('/')
                        && module != "lib"
                        && module != "main"
                        && module != "app"
                    {
                        let current = barrels
                            .get("src/lib.rs")
                            .cloned()
                            .or_else(|| read("src/lib.rs"));
                        match current {
                            Some(text) => {
                                barrels.insert("src/lib.rs".into(), declare(&text, module, true));
                            }
                            None => changes
                                .snippets
                                .push(format!("declare `pub mod {module};` in your crate root")),
                        }
                    }
                }
                continue;
            };
            let public = barrel.starts_with("src/");
            let current = barrels
                .get(&barrel)
                .cloned()
                .or_else(|| read(&barrel))
                .unwrap_or_else(|| {
                    pending.push(barrel.clone());
                    new_barrel(&barrel)
                });
            barrels.insert(barrel, declare(&current, &module, public));
        }
        for (path, text) in barrels {
            wanted.push(File {
                path,
                text,
                mode: Mode::Own,
            });
        }
        // Registrations at `// rok-ui:` markers.
        let has_view = matches!(part, Part::View)
            || (matches!(part, Part::Feature) && options.view.unwrap_or(plan.page))
            || (matches!(part, Part::Bloc) && options.view == Some(true));
        let mut registrations = match part {
            Part::Feature | Part::Bloc => vec![
                (
                    "src/app.rs",
                    "// rok-ui:repositories",
                    render::repository_registration(plan),
                ),
                (
                    "src/app.rs",
                    "// rok-ui:blocs",
                    render::bloc_registration(plan),
                ),
            ],
            Part::Repository | Part::Api => vec![(
                "src/app.rs",
                "// rok-ui:repositories",
                render::repository_registration(plan),
            )],
            Part::Provider | Part::View => Vec::new(),
        };
        if has_view {
            registrations.push((
                "src/routes/__root.rs",
                "// rok-ui:nav",
                Some(render::nav_link(plan)),
            ));
        }
        let mut edited: BTreeMap<&str, (String, String)> = BTreeMap::new();
        for (path, marker, line) in registrations {
            let Some(line) = line else { continue };
            let current = match edited.get(path) {
                Some((_, text)) => Some(text.clone()),
                None => read(path),
            };
            match current
                .as_deref()
                .and_then(|text| insert_at_marker(text, marker, &line))
            {
                Some(text) => {
                    let original = edited.get(path).map_or_else(
                        || current.clone().unwrap_or_default(),
                        |(original, _)| original.clone(),
                    );
                    edited.insert(path, (original, text));
                }
                None if path == "src/app.rs" => changes.snippets.push(format!("{marker}\n{line}")),
                None => {}
            }
        }
        for (path, (original, text)) in edited {
            if text != original {
                changes.files.insert(path.into(), (Action::Update, text));
            }
        }
    }
    for file in wanted {
        let is_barrel = file.mode == Mode::Own
            && file.text.lines().all(|line| {
                let line = line.trim();
                line.is_empty()
                    || line.starts_with("//")
                    || line.starts_with("#!")
                    || line.starts_with("pub mod ")
                    || line.starts_with("mod ")
            });
        let text = if is_barrel {
            file.text
        } else {
            format(&file.path, &file.text)?
        };
        let action = match read(&file.path) {
            None => Action::Create,
            Some(existing) if existing == text => Action::Unchanged,
            Some(_) if file.mode == Mode::Support => continue,
            Some(_) if is_barrel || options.force => Action::Update,
            Some(_) => Action::Conflict,
        };
        changes.files.insert(file.path, (action, text));
    }
    Ok(changes)
}

/// Write every created or updated file, each through a temporary file and a rename.
///
/// # Errors
///
/// When a file cannot be written.
pub fn write(root: &Path, changes: &Changes) -> Result<(), String> {
    for (path, (action, text)) in &changes.files {
        if !matches!(action, Action::Create | Action::Update) {
            continue;
        }
        let target: PathBuf = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        let temporary = target.with_extension("rs.rok-ui-tmp");
        fs::write(&temporary, text).map_err(|error| format!("{path}: {error}"))?;
        fs::rename(&temporary, &target).map_err(|error| format!("{path}: {error}"))?;
    }
    Ok(())
}

/// The summary printed after planning.
pub fn report(changes: &Changes, dry_run: bool) -> String {
    let mut out = String::new();
    for (path, (action, _)) in &changes.files {
        let verb = match (action, dry_run) {
            (Action::Create, false) => "created",
            (Action::Create, true) => "would create",
            (Action::Update, false) => "updated",
            (Action::Update, true) => "would update",
            (Action::Unchanged, _) => "unchanged",
            (Action::Conflict, _) => "conflict",
        };
        let _ = writeln!(out, "{verb:>12}  {path}");
    }
    if !changes.snippets.is_empty() {
        out.push_str(
            "\nAdd these to src/app.rs by hand (it has no `// rok-ui:` marker for them):\n",
        );
        for snippet in &changes.snippets {
            let _ = writeln!(out, "{snippet}");
        }
    }
    out
}
