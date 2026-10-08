//! `cargo rok-ui generate` (`g`): write a feature, or part of one, from flags and JSON.
//!
//! The pipeline: flags and `-j` JSON make a [`spec::Spec`] (flags win), [`resolve`] fills in
//! defaults and checks it into a [`resolve::Plan`], [`render`] writes each file's code, and
//! [`output`] works out barrels, `app.rs` lines and conflicts before writing anything.

pub mod names;
pub mod output;
pub mod render;
pub mod resolve;
pub mod spec;

use std::{fs, io::Read as _, path::PathBuf, process::ExitCode};

use output::{Options, Part};
use spec::{
    ConcurrencyKind, EndpointSpec, EventSpec, FieldSpec, Kind, ProviderKind, ProviderSpec,
    RepositorySpec, ServerErrorsSpec, Spec, StateSpec, StateStyle, ViewSpec,
};

use crate::Arguments;

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
    --concurrency <sequential|droppable|restartable|concurrent>
    --state-style <struct|enum>
    --status <a,b,c>                   statuses (default: initial,loading,success,failure)
    --repository <Name>
    --provider <Name[:memory|db|http|file]>
    --api-base <path>                  the HTTP provider's path prefix
    --endpoint <method:VERB:path:Response>   an HTTP endpoint (repeatable)
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
    let crate_name = crate_name(&root)?;
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
    let changes = output::plan(&root, &plan, part, &crate_name, options)?;
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
    let crate_name = crate_name(root)?;
    let changes = output::plan(root, &plan, Part::Feature, &crate_name, Options::default())?;
    output::write(root, &changes)
}

/// The library crate's name, from `Cargo.toml`.
fn crate_name(root: &std::path::Path) -> Result<String, String> {
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

/// `-j`: JSON inline, `@file`, or `-` for standard input.
fn parse_json(source: &str) -> Result<Spec, String> {
    let text = if source == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| error.to_string())?;
        text
    } else if let Some(path) = source.strip_prefix('@') {
        fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?
    } else {
        source.to_string()
    };
    from_json(&text)
}

/// Parse a spec, with errors that name the JSON path.
///
/// # Errors
///
/// When the JSON is malformed, has unknown keys or wrong types.
pub fn from_json(text: &str) -> Result<Spec, String> {
    let deserializer = &mut serde_json::Deserializer::from_str(text);
    serde_path_to_error::deserialize(deserializer).map_err(|error| {
        let path = error.path().to_string();
        let inner = error.into_inner();
        if path == "." {
            format!("invalid JSON: {inner}")
        } else {
            format!("invalid JSON at `{path}`: {inner}")
        }
    })
}

fn parse_choice<T: serde::de::DeserializeOwned>(flag: &str, value: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|_| format!("--{flag}: `{value}` is not one of the choices (see --help)"))
}

/// Split at top-level commas: `a:Vec<u8>,b:HashMap<K, V>` into two.
fn split_top(text: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut current = String::new();
    for character in text.chars() {
        match character {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            _ => {}
        }
        if character == separator && depth == 0 {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(character);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .collect()
}

/// `--event NoteAdded:title:String,body:String`.
fn parse_event(value: &str) -> Result<EventSpec, String> {
    let (name, rest) = value.split_once(':').unwrap_or((value, ""));
    let mut fields = Vec::new();
    for field in split_top(rest, ',') {
        let Some((field_name, ty)) = field.split_once(':') else {
            return Err(format!("--event {value}: `{field}` should be `name:Type`"));
        };
        fields.push(FieldSpec {
            name: field_name.trim().to_string(),
            ty: ty.trim().to_string(),
            ..FieldSpec::default()
        });
    }
    Ok(EventSpec {
        name: name.trim().to_string(),
        fields,
        ..EventSpec::default()
    })
}

/// `--endpoint list_notes:GET:/notes:Vec<NoteDto>`.
fn parse_endpoint(value: &str, skip_expire: bool) -> Result<EndpointSpec, String> {
    let parts: Vec<&str> = value.splitn(4, ':').collect();
    let [method, verb, path, response] = parts[..] else {
        return Err(format!(
            "--endpoint {value}: use `method:VERB:/path:ResponseType`"
        ));
    };
    let path_params = path
        .split('/')
        .filter_map(|segment| {
            segment
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
        })
        .map(|name| FieldSpec {
            name: name.to_string(),
            ty: "String".into(),
            ..FieldSpec::default()
        })
        .collect();
    Ok(EndpointSpec {
        method: method.to_string(),
        verb: verb.to_uppercase(),
        path: path.to_string(),
        path_params,
        response: response.to_string(),
        skip_expire,
        ..EndpointSpec::default()
    })
}

/// Apply the subcommand's name and the flags over `spec`.
fn apply_flags(
    spec: &mut Spec,
    subcommand: &str,
    name: Option<&str>,
    arguments: &Arguments,
) -> Result<(), String> {
    let feature_flag = arguments.flag("feature").map(str::to_string);
    let derived_feature =
        |name: &str, suffixes: &[&str]| names::snake(&names::base(name, suffixes));
    match (subcommand, name) {
        ("feature" | "view" | "api", Some(name)) => spec.feature = Some(name.to_string()),
        ("bloc" | "cubit", Some(name)) => {
            spec.kind = Some(if subcommand == "cubit" {
                Kind::Cubit
            } else {
                Kind::Bloc
            });
            spec.name = Some(name.to_string());
            if spec.feature.is_none() {
                spec.feature = Some(derived_feature(name, &["bloc", "cubit"]));
            }
        }
        ("repository", Some(name)) => {
            let repository = spec.repository.get_or_insert_with(RepositorySpec::default);
            repository.name = format!(
                "{}Repository",
                names::pascal(&names::base(name, &["repository"]))
            );
            if spec.feature.is_none() {
                spec.feature = Some(derived_feature(name, &["repository"]));
            }
        }
        ("provider", Some(name)) => {
            let provider = spec.provider.get_or_insert_with(ProviderSpec::default);
            provider.name = names::pascal(name);
            if spec.feature.is_none() {
                spec.feature = Some(derived_feature(name, &["provider"]));
            }
        }
        ("bloc" | "cubit", None) if spec.name.is_some() => {
            spec.kind = Some(if subcommand == "cubit" {
                Kind::Cubit
            } else {
                Kind::Bloc
            });
        }
        (_, None) if spec.feature.is_some() || feature_flag.is_some() => {}
        (_, None) => return Err(format!("usage: cargo rok-ui g {subcommand} <name> [flags]")),
        _ => {}
    }
    if let Some(feature) = feature_flag {
        spec.feature = Some(feature);
    }
    if subcommand == "api" {
        let provider = spec.provider.get_or_insert_with(|| ProviderSpec {
            name: String::new(),
            ..ProviderSpec::default()
        });
        if provider.name.is_empty() {
            let feature = names::pascal(spec.feature.as_deref().unwrap_or_default());
            provider.name = format!("{feature}Api");
        }
        provider.kind = Some(ProviderKind::Http);
    }
    if let Some(kind) = arguments.flag("kind") {
        spec.kind = Some(parse_choice::<Kind>("kind", kind)?);
    }
    if let Some(concurrency) = arguments.flag("concurrency") {
        spec.concurrency = Some(parse_choice::<ConcurrencyKind>("concurrency", concurrency)?);
    }
    let events = arguments.flags("event");
    if !events.is_empty() {
        spec.events = events
            .into_iter()
            .map(parse_event)
            .collect::<Result<_, _>>()?;
    }
    if let Some(style) = arguments.flag("state-style") {
        spec.state.get_or_insert_with(StateSpec::default).style =
            Some(parse_choice::<StateStyle>("state-style", style)?);
    }
    if let Some(status) = arguments.flag("status") {
        spec.state.get_or_insert_with(StateSpec::default).status = split_top(status, ',');
    }
    if let Some(repository) = arguments.flag("repository") {
        spec.repository
            .get_or_insert_with(RepositorySpec::default)
            .name = names::pascal(repository);
    }
    if let Some(provider) = arguments.flag("provider") {
        let (name, kind) = provider.split_once(':').unwrap_or((provider, ""));
        let spec_provider = spec.provider.get_or_insert_with(ProviderSpec::default);
        spec_provider.name = names::pascal(name);
        if !kind.is_empty() {
            spec_provider.kind = Some(parse_choice::<ProviderKind>("provider", kind)?);
        }
    }
    if let Some(base) = arguments.flag("api-base") {
        spec.provider
            .get_or_insert_with(ProviderSpec::default)
            .api_base = Some(base.to_string());
    }
    let endpoints = arguments.flags("endpoint");
    if !endpoints.is_empty() {
        let skip_expire = arguments.has("skip-expire");
        let parsed = endpoints
            .into_iter()
            .map(|endpoint| parse_endpoint(endpoint, skip_expire))
            .collect::<Result<Vec<_>, _>>()?;
        let provider = spec.provider.get_or_insert_with(ProviderSpec::default);
        if provider.kind.is_none() {
            provider.kind = Some(ProviderKind::Http);
        }
        provider.endpoints = parsed;
    }
    if arguments.has("server-errors") {
        let form = spec
            .view
            .get_or_insert_with(ViewSpec::default)
            .form
            .as_mut()
            .ok_or("--server-errors needs a form (`view.form` in the JSON)")?;
        form.server_errors
            .get_or_insert_with(ServerErrorsSpec::default);
    }
    if arguments.has("no-tests") {
        spec.tests = Some(false);
    }
    if let Some(provider) = &spec.provider {
        if provider.name.is_empty() {
            return Err("provider.name: missing".into());
        }
    }
    if let Some(repository) = &spec.repository {
        if repository.name.is_empty() {
            return Err("repository.name: missing".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::templates::{Template, BLOC_FEATURE};

    /// A new app from the bloc template's skeleton, without its generated feature.
    fn skeleton(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("rok-ui-generate-{name}-{}", std::process::id()));
        fs::remove_dir_all(&root).ok();
        for file in Template::Bloc.files() {
            let path = root.join(file.path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let text = file
                .text
                .replace("{{name}}", "demo-app")
                .replace("{{crate_name}}", "demo_app")
                .replace("{{rok_ui}}", "rok-ui = \"*\"")
                .replace("{{rok_ui_build}}", "rok-ui-build = \"*\"");
            fs::write(path, text).unwrap();
        }
        root
    }

    fn generate(root: &Path, json: &str, part: Part, options: Options) -> output::Changes {
        let plan = resolve::resolve(&from_json(json).unwrap()).unwrap();
        output::plan(root, &plan, part, "demo_app", options).unwrap()
    }

    fn rustfmt_installed() -> bool {
        std::process::Command::new("rustfmt")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// Compare every file the notes fixture writes with `tests/snapshots/notes/`. Run with
    /// `UPDATE_SNAPSHOTS=1` to accept changes.
    #[test]
    fn notes_fixture_matches_its_snapshots() {
        if !rustfmt_installed() {
            eprintln!("skipped: the snapshots are formatted with rustfmt, which is not installed");
            return;
        }
        let root = skeleton("snapshots");
        let changes = generate(&root, BLOC_FEATURE, Part::Feature, Options::default());
        output::write(&root, &changes).unwrap();
        let snapshots = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/notes");
        let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
        let mut mismatches = Vec::new();
        for (path, (action, _)) in &changes.files {
            assert_ne!(*action, output::Action::Conflict, "{path}");
            let text = fs::read_to_string(root.join(path)).unwrap();
            let snapshot = snapshots.join(path);
            if update {
                fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
                fs::write(&snapshot, &text).unwrap();
            } else if fs::read_to_string(&snapshot).ok().as_deref() != Some(text.as_str()) {
                mismatches.push(path.clone());
            }
        }
        fs::remove_dir_all(&root).ok();
        assert!(
            mismatches.is_empty(),
            "these files differ from tests/snapshots/notes (UPDATE_SNAPSHOTS=1 accepts them): {mismatches:?}"
        );
    }

    #[test]
    fn generating_twice_changes_nothing() {
        let root = skeleton("twice");
        let first = generate(&root, BLOC_FEATURE, Part::Feature, Options::default());
        output::write(&root, &first).unwrap();
        let second = generate(&root, BLOC_FEATURE, Part::Feature, Options::default());
        let changed: Vec<&String> = second
            .files
            .iter()
            .filter(|(_, (action, _))| *action != output::Action::Unchanged)
            .map(|(path, _)| path)
            .collect();
        assert!(changed.is_empty(), "changed on the second run: {changed:?}");
        let app = fs::read_to_string(root.join("src/app.rs")).unwrap();
        assert_eq!(app.matches("NotesRepositoryImpl::new").count(), 1);
        let features = fs::read_to_string(root.join("src/features.rs")).unwrap();
        assert_eq!(features.matches("pub mod notes;").count(), 1);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn edited_files_conflict_unless_forced() {
        let root = skeleton("conflict");
        let first = generate(&root, BLOC_FEATURE, Part::Feature, Options::default());
        output::write(&root, &first).unwrap();
        let edited = root.join("src/features/notes/bloc/notes_bloc.rs");
        let mut text = fs::read_to_string(&edited).unwrap();
        text.push_str("// mine\n");
        fs::write(&edited, text).unwrap();
        let again = generate(&root, BLOC_FEATURE, Part::Feature, Options::default());
        assert_eq!(again.conflicts(), ["src/features/notes/bloc/notes_bloc.rs"]);
        let forced = generate(
            &root,
            BLOC_FEATURE,
            Part::Feature,
            Options {
                force: true,
                ..Options::default()
            },
        );
        assert_eq!(forced.conflicts(), Vec::<&str>::new());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn barrels_stay_sorted() {
        let text = "//! Features.\n\npub mod notes;\npub mod todos;\n";
        assert_eq!(
            output::declare(text, "settings", true),
            "//! Features.\n\npub mod notes;\npub mod settings;\npub mod todos;\n"
        );
        assert_eq!(output::declare(text, "notes", true), text);
        assert_eq!(
            output::declare("//! Tests.\n", "notes", false),
            "//! Tests.\n\nmod notes;\n"
        );
    }

    #[test]
    fn json_errors_name_the_path() {
        let unknown = from_json(r#"{"feature": "notes", "events": [{"name": "A", "feilds": []}]}"#)
            .unwrap_err();
        assert!(
            unknown.contains("events[0]") && unknown.contains("feilds"),
            "{unknown}"
        );
        let wrong_type = from_json(r#"{"feature": "notes", "tests": "yes"}"#).unwrap_err();
        assert!(wrong_type.contains("tests"), "{wrong_type}");
        let invalid = resolve::resolve(
            &from_json(
                r#"{"feature": "type", "events": [{"name": "noteAdded", "fields": [{"name": "x", "type": "Vec<"}]}]}"#,
            )
            .unwrap(),
        )
        .unwrap_err();
        let joined = invalid.join("\n");
        assert!(
            joined.contains("feature: feature name `type` is a Rust keyword"),
            "{joined}"
        );
        assert!(joined.contains("events[0].name"), "{joined}");
        assert!(joined.contains("events[0].fields[0].type"), "{joined}");
    }

    #[test]
    fn flags_win_over_json() {
        let arguments = Arguments::parse(
            [
                "bloc",
                "Notes",
                "--kind",
                "cubit",
                "--event",
                "NoteAdded:title:String,tags:Vec<String>",
            ]
            .into_iter()
            .map(String::from),
        );
        let mut spec =
            from_json(r#"{"feature": "notes", "kind": "bloc", "events": [{"name": "Old"}]}"#)
                .unwrap();
        apply_flags(&mut spec, "bloc", Some("Notes"), &arguments).unwrap();
        assert_eq!(spec.kind, Some(Kind::Cubit));
        assert_eq!(spec.events.len(), 1);
        assert_eq!(spec.events[0].name, "NoteAdded");
        assert_eq!(spec.events[0].fields[1].ty, "Vec<String>");
    }

    #[test]
    fn endpoints_parse_from_flags() {
        let endpoint = parse_endpoint("get_note:get:/notes/{id}:NoteDto", true).unwrap();
        assert_eq!(endpoint.verb, "GET");
        assert_eq!(endpoint.path_params[0].name, "id");
        assert!(endpoint.skip_expire);
        assert!(parse_endpoint("get_note:GET", false).is_err());
    }

    #[test]
    fn the_schema_describes_every_key() {
        let schema = spec::schema();
        let properties = schema["properties"].as_object().unwrap();
        for key in [
            "feature",
            "kind",
            "events",
            "state",
            "repository",
            "provider",
            "dtos",
            "view",
        ] {
            assert!(properties.contains_key(key), "{key}");
        }
        assert_eq!(
            schema["additionalProperties"],
            serde_json::Value::Bool(false)
        );
    }
}
