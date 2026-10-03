//! `cargo rok-ui add`: copy component sources into the app.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// The rok-ui sources the app builds against: its `cargo metadata`, or `ROK_UI_SOURCE`.
pub fn rok_ui_source(project: &Path) -> Result<PathBuf, String> {
    if let Some(source) = std::env::var_os("ROK_UI_SOURCE") {
        return Ok(PathBuf::from(source));
    }
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["metadata", "--format-version", "1"])
        .current_dir(project)
        .output()
        .map_err(|error| format!("could not run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("unexpected cargo metadata output: {error}"))?;
    metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|package| package["name"] == "rok-ui")
        .and_then(|package| package["manifest_path"].as_str())
        .and_then(|manifest| Path::new(manifest).parent().map(Path::to_path_buf))
        .ok_or_else(|| "this project does not depend on rok-ui".to_string())
}

/// Rewrite a component file from inside rok-ui to code in the app: `crate::` paths become
/// `rok_ui::` paths, sibling components (`super::`) become `rok_ui::components::`, and the test
/// module (which uses rok-ui internals) is dropped. In a component's sub-module (`child`),
/// `super::` keeps pointing at the copied parent.
pub fn rewrite(source: &str, child: bool) -> String {
    let without_tests = match source.find("\n#[cfg(test)]\nmod tests") {
        Some(start) => &source[..=start],
        None => source,
    };
    let mut rewritten = String::with_capacity(without_tests.len());
    for line in without_tests.lines() {
        let line = line.replace("crate::", "rok_ui::");
        let line = if child {
            line
        } else {
            line.replace("super::", "rok_ui::components::")
        };
        rewritten.push_str(&line);
        rewritten.push('\n');
    }
    if rewritten.contains("#[cfg(feature") {
        rewritten.insert_str(
            0,
            "// Copied from rok-ui. `#[cfg(feature = ..)]` below refers to this crate's features;\n// enable or remove those blocks as needed.\n",
        );
    } else {
        rewritten.insert_str(
            0,
            "// Copied from rok-ui with `cargo rok-ui add`; edit freely.\n",
        );
    }
    rewritten
}

/// Crates a copied file uses directly, which the app must depend on too.
pub fn extra_dependencies(source: &str) -> Vec<&'static str> {
    [
        ("jiff::", "jiff = \"0.2\""),
        ("unicode_segmentation", "unicode-segmentation = \"1.12\""),
        ("unicode_bidi", "unicode-bidi = \"0.3\""),
        ("ttf_parser", "ttf-parser = \"0.25\""),
    ]
    .into_iter()
    .filter(|(usage, _)| source.contains(usage))
    .map(|(_, dependency)| dependency)
    .collect()
}

/// Copy `component` (`button`, `date-picker`) into `target` (a `ui` module directory), and
/// declare it in the barrel file next to it. Returns the written files.
pub fn add(
    source_root: &Path,
    component: &str,
    target: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, String> {
    let module = component.replace('-', "_");
    let file = source_root
        .join("src/components")
        .join(format!("{module}.rs"));
    let text = fs::read_to_string(&file).map_err(|_| {
        format!(
            "rok-ui has no component `{component}` ({} not found)",
            file.display()
        )
    })?;
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    let mut written = Vec::new();
    let destination = target.join(format!("{module}.rs"));
    if destination.exists() && !force {
        return Err(format!(
            "{} exists; pass --force to overwrite",
            destination.display()
        ));
    }
    fs::write(&destination, rewrite(&text, false)).map_err(|error| error.to_string())?;
    written.push(destination);

    // Components split across files keep their sub-modules (`input/textarea.rs`).
    let directory = source_root.join("src/components").join(&module);
    if directory.is_dir() {
        let child_target = target.join(&module);
        fs::create_dir_all(&child_target).map_err(|error| error.to_string())?;
        for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                let text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
                let destination = child_target.join(path.file_name().unwrap_or_default());
                fs::write(&destination, rewrite(&text, true)).map_err(|error| error.to_string())?;
                written.push(destination);
            }
        }
    }

    // Declare the module in the barrel file (`src/components/ui.rs` for `src/components/ui/`).
    let barrel = target.with_extension("rs");
    let declaration = format!("pub mod {module};");
    let current = fs::read_to_string(&barrel).unwrap_or_default();
    if !current.lines().any(|line| line.trim() == declaration) {
        let mut updated = current;
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&declaration);
        updated.push('\n');
        fs::write(&barrel, updated).map_err(|error| error.to_string())?;
        written.push(barrel);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::rewrite;

    #[test]
    fn crate_paths_point_at_rok_ui() {
        let source = "use super::{spinner::Spinner};\nuse crate::{styles, sx::Sx};\ncrate::implement_style_overrides!(Button);\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n}\n";
        let rewritten = rewrite(source, false);
        assert!(rewritten.contains("use rok_ui::components::{spinner::Spinner};"));
        assert!(rewritten.contains("use rok_ui::{styles, sx::Sx};"));
        assert!(rewritten.contains("rok_ui::implement_style_overrides!(Button);"));
        assert!(!rewritten.contains("mod tests"));
        assert!(rewrite("use super::InputState;\n", true).contains("use super::InputState;"));
    }

    #[test]
    fn extra_crates_are_reported() {
        assert_eq!(
            super::extra_dependencies("use jiff::civil::Date;"),
            ["jiff = \"0.2\""]
        );
        assert_eq!(super::extra_dependencies("use gpui::div;"), [] as [&str; 0]);
    }
}
