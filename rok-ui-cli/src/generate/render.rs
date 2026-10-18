//! The generated files' contents. Output is formatted with rustfmt afterwards; here it only
//! needs to be correct and deterministic.

use std::{collections::BTreeSet, fmt::Write as _};

use super::{
    names,
    resolve::Plan,
    spec::{Kind, ProviderKind},
};

mod data;
mod feature_test;
mod logic;
mod presentation;
mod repository;

use data::{action_label, id_alias_type, is_copy, struct_fields, submit_label};
pub use data::{data_error, model, provider};
pub use feature_test::test;
pub use logic::{bloc, cubit, cubit_method, event, state};
use logic::{event_name, state_has_error_field, state_name, status_name};
pub use presentation::{page, page_name, route};
pub use repository::repository;

/// `use` lines, sorted and deduplicated.
#[derive(Default)]
struct Imports(BTreeSet<String>);

impl Imports {
    fn add(&mut self, path: &str) {
        self.0.insert(path.to_string());
    }

    fn render(&self) -> String {
        self.0.iter().fold(String::new(), |mut out, path| {
            let _ = writeln!(out, "use {path};");
            out
        })
    }
}

/// The type names a type mentions.
fn idents(ty: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current = String::new();
    for character in ty.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            found.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        found.push(current);
    }
    found
}

/// The Rust spelling of a payload type (`bytes` is `Bytes`).
fn rust_type(ty: &str) -> String {
    if ty == "bytes" {
        "Bytes".into()
    } else {
        ty.to_string()
    }
}

/// Where the types a file mentions come from.
fn import_types(plan: &Plan, imports: &mut Imports, types: &[&str], root: &str) {
    for ty in types {
        for ident in idents(&rust_type(ty)) {
            if let Some(path) = type_path(plan, &ident, root) {
                imports.add(&path);
            }
        }
    }
}

fn type_path(plan: &Plan, ident: &str, root: &str) -> Option<String> {
    if plan.models.iter().any(|model| model.names.pascal == ident) {
        let model = names::snake(ident);
        return Some(format!("{root}::data::models::{model}::{ident}"));
    }
    if let Some((_, owner, _)) = plan.ids.iter().find(|(alias, _, _)| alias == ident) {
        let owner = names::snake(owner);
        return Some(format!("{root}::data::models::{owner}::{ident}"));
    }
    if plan.dtos.iter().any(|dto| dto.names.pascal == ident) {
        let provider = &plan.provider.as_ref()?.names.snake;
        return Some(format!("{root}::data::providers::{provider}::{ident}"));
    }
    match ident {
        "DataError" => Some(format!("{root}::data::error::DataError")),
        "ApiError" => Some("rok_ui::http::ApiError".into()),
        "Bytes" => Some("rok_ui::http::Bytes".into()),
        "SharedString" => Some("rok_ui::prelude::SharedString".into()),
        "HashMap" => Some("std::collections::HashMap".into()),
        "BTreeMap" => Some("std::collections::BTreeMap".into()),
        _ => None,
    }
}

/// An expression for `ty`'s default value, spelled out (clippy pedantic rejects a bare
/// `Default::default()` where the type is known).
pub fn default_of(ty: &str) -> String {
    let ty = rust_type(ty).replace(' ', "");
    match ty.as_str() {
        "()" => "()".into(),
        "String" => "String::new()".into(),
        "bool" => "false".into(),
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "isize" => "0".into(),
        "f32" | "f64" => "0.0".into(),
        _ if ty.starts_with("Vec<") => "Vec::new()".into(),
        _ if ty.starts_with("Option<") => "None".into(),
        _ if ty.contains('<') => format!("<{ty}>::default()"),
        _ => format!("{ty}::default()"),
    }
}

/// "Note added" for `NoteAdded`.
fn sentence(name: &str) -> String {
    let words = names::snake(name).replace('_', " ");
    let mut characters = words.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

fn header(text: &str) -> String {
    format!("//! {text}\n\n")
}

/// The line `app.rs` gets at `// rok-ui:repositories`.
pub fn repository_registration(plan: &Plan) -> Option<String> {
    let repository = plan.repository.as_ref()?;
    let provider = plan.provider.as_ref()?;
    let client = if provider.kind == ProviderKind::Http {
        "client.clone()"
    } else {
        ""
    };
    Some(format!(
        ".with::<dyn crate::data::repositories::{0}::{1}>(Arc::new(crate::data::repositories::{0}::{1}Impl::new(crate::data::providers::{2}::{3}::new({client}))))",
        repository.names.snake, repository.names.pascal, provider.names.snake, provider.names.pascal
    ))
}

/// The line `app.rs` gets at `// rok-ui:blocs`.
pub fn bloc_registration(plan: &Plan) -> Option<String> {
    let repository = plan.repository.as_ref()?;
    let (method, suffix, module) = match plan.kind {
        Kind::Bloc => ("with_bloc", "Bloc", "bloc"),
        Kind::Cubit => ("with_cubit", "Cubit", "cubit"),
    };
    Some(format!(
        ".{method}(|scope| crate::features::{0}::bloc::{1}_{module}::{2}{suffix}::new(scope.repository::<dyn crate::data::repositories::{3}::{4}>()))",
        plan.feature.snake, plan.name.snake, plan.name.pascal, repository.names.snake, repository.names.pascal
    ))
}

/// The link `__root.rs` gets at `// rok-ui:nav`.
pub fn nav_link(plan: &Plan) -> String {
    format!(
        "Link::new(\"nav-{0}\", \"/{0}\").child({1:?}),",
        plan.feature.snake,
        sentence(&plan.feature.pascal)
    )
}

/// The plan's doc line for a barrel file, by its path.
pub fn barrel_doc(path: &str) -> &'static str {
    match path {
        "src/data.rs" => "The data layer: models, providers (where data comes from) and repositories (what the business layer uses).",
        "src/data/models.rs" => "Domain models: plain values the whole app shares.",
        "src/data/providers.rs" => "Data providers: databases, APIs and files. Only repositories use them.",
        "src/data/repositories.rs" => "Repositories: traits the business layer depends on, with their implementations.",
        "src/features.rs" => "Features, one module each: business logic (`bloc`) and presentation (`view`).",
        "tests/features/main.rs" => "Feature tests: events in, states out, with fake repositories.",
        _ if path.ends_with("/bloc.rs") => "Business logic: events, states and the bloc. No GPUI, no components.",
        _ if path.ends_with("/view.rs") => "Presentation: pages and widgets that read the bloc and add its events.",
        _ if path.starts_with("tests/") => "Tests for this feature.",
        _ => "This feature: its business logic and its views.",
    }
}
