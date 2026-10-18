//! The generator, end to end: specs in, files out.

use std::path::{Path, PathBuf};

use super::flags::parse_endpoint;
use super::spec::Kind;
use super::*;
use crate::templates::{Template, BLOC_FEATURE, BLOC_HTTP_FEATURES};

/// A new app from the bloc template's skeleton, without its generated feature.
fn skeleton(name: &str) -> PathBuf {
    skeleton_of(Template::Bloc, name)
}

/// A new app from `template`'s skeleton, without its generated features.
fn skeleton_of(template: Template, name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("rok-ui-generate-{name}-{}", std::process::id()));
    fs::remove_dir_all(&root).ok();
    for file in template.files() {
        let path = root.join(file.path.replace("{{name}}", "demo-app"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let text = file
            .text
            .replace("{{name}}", "demo-app")
            .replace("{{crate_name}}", "demo_app")
            .replace("{{rok_ui}}", "rok-ui = \"*\"")
            .replace("{{rok_ui_bloc}}", "rok-ui-bloc = \"*\"")
            .replace("{{rok_ui_build}}", "rok-ui-build = \"*\"");
        fs::write(path, text).unwrap();
    }
    root
}

fn generate(root: &Path, json: &str, part: Part, options: Options) -> output::Changes {
    let plan = resolve::resolve(&from_json(json).unwrap()).unwrap();
    output::plan(root, &plan, part, "demo_app", &Layout::Single, options).unwrap()
}

fn rustfmt_installed() -> bool {
    std::process::Command::new("rustfmt")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Generate `features` into a new `template` app and compare every file written with
/// `tests/snapshots/<name>/`. Run with `UPDATE_SNAPSHOTS=1` to accept changes.
fn check_snapshots(template: Template, name: &str, features: &[&str]) {
    if !rustfmt_installed() {
        eprintln!("skipped: the snapshots are formatted with rustfmt, which is not installed");
        return;
    }
    let root = skeleton_of(template, name);
    let mut written = std::collections::BTreeSet::new();
    for feature in features {
        let changes = generate(&root, feature, Part::Feature, Options::default());
        for (path, (action, _)) in &changes.files {
            assert_ne!(*action, output::Action::Conflict, "{path}");
            written.insert(path.clone());
        }
        output::write(&root, &changes).unwrap();
    }
    let snapshots = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut mismatches = Vec::new();
    for path in &written {
        // Compared without regard to line endings: a Windows checkout may have CRLF.
        let text = fs::read_to_string(root.join(path)).unwrap();
        let snapshot = snapshots.join(path);
        if update {
            fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
            fs::write(&snapshot, &text).unwrap();
        } else if fs::read_to_string(&snapshot)
            .ok()
            .map(|expected| expected.replace("\r\n", "\n"))
            != Some(text.replace("\r\n", "\n"))
        {
            mismatches.push(path.clone());
        }
    }
    fs::remove_dir_all(&root).ok();
    assert!(
        mismatches.is_empty(),
        "these files differ from tests/snapshots/{name} (UPDATE_SNAPSHOTS=1 accepts them): {mismatches:?}"
    );
}

#[test]
fn notes_fixture_matches_its_snapshots() {
    check_snapshots(Template::Bloc, "notes", &[BLOC_FEATURE]);
}

#[test]
fn http_fixtures_match_their_snapshots() {
    check_snapshots(Template::BlocHttp, "http", BLOC_HTTP_FEATURES);
}

#[test]
fn http_specs_are_checked() {
    let invalid = resolve::resolve(
        &from_json(
            r#"{"feature": "auth",
                "repository": {"name": "AuthRepository", "methods": [
                    {"name": "sign_in", "returns": "User", "session": "sign_in"},
                    {"name": "gone"}]},
                "provider": {"name": "AuthApi", "kind": "http", "endpoints": [
                    {"method": "sign_in", "verb": "POST", "path": "sessions", "response": "SignedInDto"},
                    {"method": "me", "verb": "GET", "path": "/users/{id}", "response": "UserDto"}]},
                "dtos": [{"name": "SignedInDto", "fields": [{"name": "jwt", "type": "String"}]}],
                "view": {"form": {"event": "AuthRequested", "server_errors": {"conflict_field": "email"}}}}"#,
        )
        .unwrap(),
    )
    .unwrap_err()
    .join("\n");
    for expected in [
        "repository.methods[0].session: `sign_in` needs the endpoint's response DTO to have a `token` field",
        "repository.methods[1].name: there is no provider endpoint `gone`",
        "provider.endpoints[0].path",
        "provider.endpoints[1]",
        "view.form.server_errors.conflict_field: the form has no field `email`",
    ] {
        assert!(invalid.contains(expected), "missing {expected:?} in:\n{invalid}");
    }
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
fn the_workspace_layout_puts_each_layer_in_its_crate() {
    let root = skeleton_of(Template::BlocWorkspace, "workspace");
    let layout = Layout::detect(&root).unwrap();
    assert_eq!(layout, Layout::workspace("demo-app"));
    let plan = |json: &str| {
        let plan = resolve::resolve(&from_json(json).unwrap()).unwrap();
        output::plan(
            &root,
            &plan,
            Part::Feature,
            "demo_app",
            &layout,
            Options::default(),
        )
        .unwrap()
    };
    let first = plan(BLOC_FEATURE);
    output::write(&root, &first).unwrap();
    let read = |path: &str| fs::read_to_string(root.join(path)).unwrap();
    for path in [
        "crates/demo-app-data/src/models/note.rs",
        "crates/demo-app-notes/Cargo.toml",
        "crates/demo-app-notes/clippy.toml",
        "crates/demo-app/src/features/notes/view/notes_page.rs",
    ] {
        assert!(root.join(path).exists(), "{path}");
    }
    assert!(!root.join("src").exists() && !root.join("tests").exists());
    let lib = read("crates/demo-app-notes/src/lib.rs");
    assert!(lib.contains("pub mod notes_bloc;"), "{lib}");
    let state = read("crates/demo-app-notes/src/notes_state.rs");
    assert!(
        state.contains("use demo_app_data::models::note::Note;"),
        "{state}"
    );
    let test = read("crates/demo-app-notes/tests/notes_bloc_test.rs");
    assert!(
        test.contains("use demo_app_notes::notes_bloc::NotesBloc;"),
        "{test}"
    );
    assert!(test.contains("use rok_ui_bloc::test;"), "{test}");
    let repository = read("crates/demo-app-data/src/repositories/notes_repository.rs");
    assert!(
        repository.contains("use crate::models::note::Note;"),
        "{repository}"
    );
    assert_eq!(
        read("crates/demo-app/src/features/notes.rs"),
        "//! This feature: its business logic and its views.\n\npub use demo_app_notes as bloc;\npub mod view;\n"
    );
    let lib = read("crates/demo-app/src/lib.rs");
    assert!(
        lib.contains("pub use demo_app_data as data;") && !lib.contains("pub mod data;"),
        "{lib}"
    );
    let manifest = read("crates/demo-app/Cargo.toml");
    assert!(manifest.contains("demo-app-notes = { path = \"../demo-app-notes\" }"));
    // Generating again changes nothing.
    let second = plan(BLOC_FEATURE);
    let changed: Vec<&String> = second
        .files
        .iter()
        .filter(|(_, (action, _))| *action != output::Action::Unchanged)
        .map(|(path, _)| path)
        .collect();
    assert!(changed.is_empty(), "changed on the second run: {changed:?}");
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
    let unknown =
        from_json(r#"{"feature": "notes", "events": [{"name": "A", "feilds": []}]}"#).unwrap_err();
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
        from_json(r#"{"feature": "notes", "kind": "bloc", "events": [{"name": "Old"}]}"#).unwrap();
    apply_flags(&mut spec, "bloc", Some("Notes"), &arguments).unwrap();
    assert_eq!(spec.kind, Some(Kind::Cubit));
    assert_eq!(spec.events.len(), 1);
    assert_eq!(spec.events[0].name, "NoteAdded");
    assert_eq!(spec.events[0].fields[1].ty, "Vec<String>");
}

#[test]
fn timed_concurrency_parses_and_renders() {
    use std::time::Duration;

    use super::spec::ConcurrencyKind;

    assert_eq!(
        "debounce:300ms".parse(),
        Ok(ConcurrencyKind::Debounce(Duration::from_millis(300)))
    );
    assert_eq!(
        "throttle:2s".parse(),
        Ok(ConcurrencyKind::Throttle(Duration::from_secs(2)))
    );
    assert_eq!("droppable".parse(), Ok(ConcurrencyKind::Droppable));
    for invalid in [
        "debounce",
        "debounce:",
        "debounce:300",
        "debounce:0ms",
        "throttle:1m",
        "droppable:1s",
        "eager",
    ] {
        assert!(invalid.parse::<ConcurrencyKind>().is_err(), "{invalid}");
    }
    let bad = from_json(
        r#"{"feature": "search", "events": [{"name": "QueryChanged", "concurrency": "debounce:soon"}]}"#,
    )
    .unwrap_err();
    assert!(
        bad.contains("events[0].concurrency") && bad.contains("debounce:<n>ms"),
        "{bad}"
    );

    let root = skeleton("timed");
    let changes = generate(
        &root,
        r#"{"feature": "search", "concurrency": "debounce:300ms", "events": [
            {"name": "QueryChanged", "fields": [{"name": "query", "type": "String"}]},
            {"name": "ResultsSaved", "concurrency": "throttle:1s"},
            {"name": "SearchCleared", "concurrency": "sequential"}
        ]}"#,
        Part::Feature,
        Options::default(),
    );
    let (_, bloc) = &changes.files["src/features/search/bloc/search_bloc.rs"];
    for expected in [
        "use std::time::Duration;",
        "SearchEvent::QueryChanged { .. } => Concurrency::debounce(Duration::from_millis(300)),",
        "SearchEvent::ResultsSaved => Concurrency::throttle(Duration::from_secs(1)),",
        "SearchEvent::SearchCleared => Concurrency::Sequential,",
    ] {
        assert!(bloc.contains(expected), "{expected}\n{bloc}");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn endpoints_parse_from_flags() {
    let endpoint = parse_endpoint("get_note:get:/notes/{id:NoteId}:NoteDto", true).unwrap();
    assert_eq!(endpoint.verb, "GET");
    assert_eq!(endpoint.path, "/notes/{id}");
    assert_eq!(endpoint.path_params[0].name, "id");
    assert_eq!(endpoint.path_params[0].ty, "NoteId");
    assert!(endpoint.skip_expire);
    let endpoint = parse_endpoint("list:GET:/notes:Vec<NoteDto>", false).unwrap();
    assert_eq!(endpoint.response, "Vec<NoteDto>");
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
