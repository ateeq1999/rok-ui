//! `cargo rok-ui generate` (`g`): write a feature, or part of one, from flags and JSON.
//!
//! The pipeline: flags and `-j` JSON make a [`spec::Spec`] (flags win), [`resolve`] fills in
//! defaults and checks it into a [`resolve::Plan`], [`render`] writes each file's code, and
//! [`output`] works out barrels, `app.rs` lines and conflicts before writing anything.

pub mod layout;
pub mod names;
pub mod output;
pub mod render;
pub mod resolve;
pub mod spec;

use std::{fs, path::PathBuf, process::ExitCode};

use layout::Layout;
use output::{Options, Part};
use spec::Spec;

use crate::Arguments;

mod flags;

pub use flags::from_json;
use flags::{apply_flags, parse_json};

/// `generate`'s help.
pub const USAGE: &str = "cargo rok-ui generate (g): write a feature, or part of one

USAGE:
    cargo rok-ui g feature <name> [flags]      models, provider, repository, bloc, view, route, tests
    cargo rok-ui g bloc <Name> [flags]         a bloc with its tests (data layer when missing)
    cargo rok-ui g cubit <Name> [flags]        a cubit with its tests (data layer when missing)
    cargo rok-ui g repository <Name> [flags]   a repository (provider and models when missing)
    cargo rok-ui g provider <Name> [flags]     a provider (models when missing)
    cargo rok-ui g view <feature> [flags]      a page and its route
    cargo rok-ui g api <feature> [flags]       an HTTP provider, its DTOs and its repository
    cargo rok-ui g schema                      print the JSON Schema of `-j`

FLAGS (flags win over JSON, JSON over defaults):
    -j, --json <JSON|@file|->          the whole spec as JSON (see `g schema`)
    --feature <name>                   the feature, for bloc/cubit/repository/provider
    --event <Name[:field:Type,...]>    an event (repeatable; replaces the JSON's events)
    --kind <bloc|cubit>
    --concurrency <sequential|droppable|restartable|concurrent|debounce:300ms|throttle:1s>
    --state-style <struct|enum>
    --status <a,b,c>                   statuses (default: initial,loading,success,failure)
    --repository <Name>
    --provider <Name[:memory|db|http|file]>
    --api-base <path>                  the HTTP provider's path prefix
    --endpoint <method:VERB:path:Response>   an HTTP endpoint (repeatable); typed path
                                       parameters: /notes/{id:NoteId}
    --dto <Name[:field:Type,...]>      a wire type for the endpoints (repeatable)
    --skip-expire                      the --endpoint ones do not expire the session on 401
    --server-errors                    the form maps API errors onto its fields
    --view / --no-view                 write the page (default: with `feature` only)
    --no-tests                         skip tests/features/<feature>/
    --path <dir>                       the project (default: the current directory)
    --dry-run                          print what would change, write nothing
    --force                            overwrite generated files that changed
    --no-wire                          leave barrel files and src/app.rs alone

EXIT CODES: 0 done, 1 invalid input, 2 conflicts (nothing written).";

/// Run `generate`.
pub fn run(arguments: &Arguments) -> ExitCode {
    match try_run(arguments) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn try_run(arguments: &Arguments) -> Result<ExitCode, String> {
    let Some(subcommand) = arguments.positional.first() else {
        println!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    };
    let part = match subcommand.as_str() {
        "schema" => {
            let schema =
                serde_json::to_string_pretty(&spec::schema()).map_err(|error| error.to_string())?;
            println!("{schema}");
            return Ok(ExitCode::SUCCESS);
        }
        "help" => {
            println!("{USAGE}");
            return Ok(ExitCode::SUCCESS);
        }
        "feature" => Part::Feature,
        "bloc" | "cubit" => Part::Bloc,
        "repository" => Part::Repository,
        "provider" => Part::Provider,
        "view" => Part::View,
        "api" => Part::Api,
        other => return Err(format!("unknown generator `{other}`\n\n{USAGE}")),
    };
    let mut spec = match arguments.flag("json") {
        Some(json) => parse_json(json)?,
        None => Spec::default(),
    };
    apply_flags(
        &mut spec,
        subcommand,
        arguments.positional.get(1).map(String::as_str),
        arguments,
    )?;
    let plan = match resolve::resolve(&spec) {
        Ok(plan) => plan,
        Err(errors) => {
            let mut message = String::from("the spec is invalid:");
            for error in errors {
                message.push_str("\n  ");
                message.push_str(&error);
            }
            return Err(message);
        }
    };
    let root = PathBuf::from(arguments.flag("path").unwrap_or("."));
    let layout = Layout::detect(&root)?;
    let crate_name = crate_name(&root, &layout)?;
    let options = Options {
        force: arguments.has("force"),
        view: if arguments.has("no-view") {
            Some(false)
        } else if arguments.has("view") {
            Some(true)
        } else {
            None
        },
        no_wire: arguments.has("no-wire"),
    };
    let changes = output::plan(&root, &plan, part, &crate_name, &layout, options)?;
    let dry_run = arguments.has("dry-run");
    print!("{}", output::report(&changes, dry_run));
    let conflicts = changes.conflicts();
    if !conflicts.is_empty() {
        eprintln!(
            "error: {} file(s) exist and differ; nothing was written. Pass --force to overwrite them.",
            conflicts.len()
        );
        return Ok(ExitCode::from(2));
    }
    if !dry_run {
        output::write(&root, &changes)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Generate the feature `json` describes into the new app at `root`, quietly.
///
/// # Errors
///
/// When the JSON is invalid or a file cannot be written (both generator bugs here).
pub fn feature_into(root: &std::path::Path, json: &str) -> Result<(), String> {
    let spec = from_json(json)?;
    let plan = resolve::resolve(&spec).map_err(|errors| errors.join("\n"))?;
    let layout = Layout::detect(root)?;
    let crate_name = crate_name(root, &layout)?;
    let changes = output::plan(
        root,
        &plan,
        Part::Feature,
        &crate_name,
        &layout,
        Options::default(),
    )?;
    output::write(root, &changes)
}

/// The app's library crate name: from `rok-ui.toml` in a workspace, else from `Cargo.toml`.
fn crate_name(root: &std::path::Path, layout: &Layout) -> Result<String, String> {
    if let Some(name) = layout.crate_name() {
        return Ok(name.to_string());
    }
    let manifest = fs::read_to_string(root.join("Cargo.toml")).map_err(|_| {
        format!(
            "{} has no Cargo.toml; run this in your app (or pass --path)",
            root.display()
        )
    })?;
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]" || line == "[lib]";
            continue;
        }
        if in_package {
            if let Some(value) = line
                .strip_prefix("name")
                .map(str::trim)
                .and_then(|rest| rest.strip_prefix('='))
            {
                return Ok(value.trim().trim_matches('"').replace('-', "_"));
            }
        }
    }
    Err("Cargo.toml has no package name".into())
}

#[cfg(test)]
mod tests;
