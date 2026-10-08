//! The generated files' contents. Output is formatted with rustfmt afterwards; here it only
//! needs to be correct and deterministic.

use std::{collections::BTreeSet, fmt::Write as _};

use super::{
    names,
    resolve::{Event, Field, Method, Model, Plan},
    spec::{ConcurrencyKind, Kind, ProviderKind, StateStyle},
};

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

// ---------------------------------------------------------------------------
// Data layer.

/// `src/data/error.rs`, when the app has none.
pub fn data_error() -> String {
    r#"//! Errors the data layer reports to the business layer.

use std::fmt;

/// Why a repository call failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataError {
    /// Nothing has that id.
    NotFound,
    /// The data source failed; the message says why.
    Failed(String),
}

impl fmt::Display for DataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("not found"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DataError {}
"#
    .into()
}

fn struct_fields(out: &mut String, fields: &[Field], public: bool, serde: bool) {
    for field in fields {
        let _ = writeln!(out, "    /// `{}`.", field.name);
        if serde {
            if let Some(rename) = &field.rename {
                let _ = writeln!(out, "    #[serde(rename = {rename:?})]");
            }
        }
        let visibility = if public { "pub " } else { "" };
        let _ = writeln!(
            out,
            "    {visibility}{}: {},",
            field.name,
            rust_type(&field.ty)
        );
    }
}

/// `src/data/models/<model>.rs`.
pub fn model(plan: &Plan, model: &Model) -> String {
    let mut out = header(&format!("The `{}` model.", model.names.pascal));
    let mut imports = Imports::default();
    let mut types: Vec<&str> = model.fields.iter().map(|field| field.ty.as_str()).collect();
    types.extend(
        model
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter().map(|field| field.ty.as_str())),
    );
    let own_ids: Vec<&str> = plan
        .ids
        .iter()
        .filter(|(_, owner, _)| *owner == model.names.pascal)
        .map(|(alias, _, _)| alias.as_str())
        .collect();
    import_types(plan, &mut imports, &types, "crate");
    imports.0.retain(|path| {
        !own_ids
            .iter()
            .any(|alias| path.ends_with(&format!("::{alias}")))
            && !path.ends_with(&format!("::{}", model.names.pascal))
    });
    out.push_str(&imports.render());
    if !imports.0.is_empty() {
        out.push('\n');
    }
    let description = sentence(&model.names.pascal);
    if model.variants.is_empty() {
        let _ = writeln!(out, "/// {description}.");
        out.push_str("#[derive(Clone, Debug, Default, PartialEq)]\n");
        let _ = writeln!(out, "pub struct {} {{", model.names.pascal);
        struct_fields(&mut out, &model.fields, true, false);
        out.push_str("}\n");
    } else {
        let _ = writeln!(out, "/// {description}: one of its variants.");
        out.push_str("#[derive(Clone, Debug, PartialEq)]\n");
        let _ = writeln!(out, "pub enum {} {{", model.names.pascal);
        for variant in &model.variants {
            let _ = writeln!(out, "    /// {}.", sentence(&variant.names.pascal));
            let _ = writeln!(out, "    {} {{", variant.names.pascal);
            for field in &variant.fields {
                let _ = writeln!(out, "        /// `{}`.", field.name);
                let _ = writeln!(out, "        {}: {},", field.name, rust_type(&field.ty));
            }
            out.push_str("    },\n");
        }
        out.push_str("}\n");
    }
    for (alias, _, ty) in plan
        .ids
        .iter()
        .filter(|(_, owner, _)| *owner == model.names.pascal)
    {
        let _ = writeln!(
            out,
            "\n/// A `{}`'s id.\npub type {alias} = {ty};",
            model.names.pascal
        );
    }
    out
}

/// The expression that makes the next id from the counter `next`.
fn next_id(ty: &str) -> &'static str {
    match ty {
        "String" => "next.to_string()",
        "u64" => "next",
        "u32" | "usize" | "i32" | "i64" => "next.try_into().unwrap_or_default()",
        _ => "Default::default()",
    }
}

/// `src/data/providers/<provider>.rs`.
pub fn provider(plan: &Plan) -> String {
    let provider = plan.provider.as_ref().expect("a provider");
    match provider.kind {
        ProviderKind::Http => http_provider(plan),
        _ => memory_provider(plan),
    }
}

fn memory_provider(plan: &Plan) -> String {
    let provider = plan.provider.as_ref().expect("a provider");
    let name = &provider.names.pascal;
    let stand_in = match provider.kind {
        ProviderKind::Db => " A stand-in for the database: replace its bodies with rok-db calls (see the `db` guide) and keep its methods.",
        ProviderKind::File => " A stand-in for a file: replace its bodies with reads and writes of the file and keep its methods.",
        _ => "",
    };
    let Some(model) = plan.primary_model() else {
        return format!(
            "{}/// The data source of the `{}` feature.\n#[derive(Debug, Default)]\npub struct {name};\n\nimpl {name} {{\n    /// An empty source.\n    #[must_use]\n    pub fn new() -> Self {{\n        Self\n    }}\n}}\n",
            header(&format!("`{name}`: where `{}` data comes from.", plan.feature.snake)),
            plan.feature.snake
        );
    };
    let model_name = &model.names.pascal;
    let mut imports = Imports::default();
    imports.add("std::sync::{Mutex, PoisonError}");
    let mut types = vec![model_name.as_str()];
    if let Some(id) = &model.id {
        types.push(id.as_str());
    }
    import_types(plan, &mut imports, &types, "crate");
    let mut out = header(&format!(
        "`{name}`: where `{}` data comes from, kept in memory.",
        plan.feature.snake
    ));
    out.push_str(&imports.render());
    let plural = names::plural(&names::snake(model_name)).replace('_', " ");
    let _ = write!(
        out,
        "\n/// {}, kept in memory: the starting point, and what tests use.{stand_in}\n#[derive(Debug, Default)]\npub struct {name} {{\n    items: Mutex<Vec<{model_name}>>,\n",
        sentence(&plural)
    );
    if model.id.is_some() {
        out.push_str("    next_id: Mutex<u64>,\n");
    }
    let _ = write!(
        out,
        "}}\n\nimpl {name} {{\n    /// An empty source.\n    #[must_use]\n    pub fn new() -> Self {{\n        Self::default()\n    }}\n\n    /// Every item.\n    #[must_use]\n    pub fn all(&self) -> Vec<{model_name}> {{\n        self.items.lock().unwrap_or_else(PoisonError::into_inner).clone()\n    }}\n"
    );
    if let Some(id) = &model.id {
        let next = next_id(&id_alias_type(plan, id));
        let _ = write!(
            out,
            r"
    /// The item with `id`.
    #[must_use]
    pub fn find(&self, id: &{id}) -> Option<{model_name}> {{
        self.all().into_iter().find(|item| &item.id == id)
    }}

    /// Store `item` under a new id and return it.
    pub fn insert(&self, mut item: {model_name}) -> {model_name} {{
        let mut next_id = self.next_id.lock().unwrap_or_else(PoisonError::into_inner);
        *next_id += 1;
        let next = *next_id;
        item.id = {next};
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(item.clone());
        item
    }}

    /// Change the item with `id`, and return it changed.
    pub fn update(&self, id: &{id}, change: impl FnOnce(&mut {model_name})) -> Option<{model_name}> {{
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        let item = items.iter_mut().find(|item| &item.id == id)?;
        change(item);
        Some(item.clone())
    }}

    /// Forget the item with `id`, and return it.
    pub fn remove(&self, id: &{id}) -> Option<{model_name}> {{
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        let index = items.iter().position(|item| &item.id == id)?;
        Some(items.remove(index))
    }}
"
        );
    } else {
        let _ = write!(
            out,
            r"
    /// Store `item` and return it.
    pub fn insert(&self, item: {model_name}) -> {model_name} {{
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(item.clone());
        item
    }}
"
        );
    }
    out.push_str("}\n");
    out
}

/// Whether values of `ty` are `Copy` (and so are not cloned).
fn is_copy(ty: &str) -> bool {
    matches!(
        ty,
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "bool"
            | "char"
            | "f32"
            | "f64"
    )
}

/// The label of a page button for an event without fields: "Refresh" for the loader.
fn action_label(plan: &Plan, event: &Event) -> String {
    let loads = event.calls.as_ref().is_some_and(|call| {
        plan.loader()
            .is_some_and(|(loader, _)| &loader.name == call)
    });
    if loads {
        "Refresh".into()
    } else {
        sentence(&names::event_verb(&event.name))
    }
}

/// The type behind an id alias (`NoteId` -> `u64`).
fn id_alias_type(plan: &Plan, ty: &str) -> String {
    plan.ids
        .iter()
        .find(|(alias, _, _)| alias == ty)
        .map_or_else(|| ty.to_string(), |(_, _, inner)| inner.clone())
}

fn dto(out: &mut String, dto: &Model) {
    let description = sentence(dto.names.pascal.trim_end_matches("Dto"));
    if dto.variants.is_empty() {
        let _ = writeln!(
            out,
            "\n/// {description}, as the API sends and receives it."
        );
        out.push_str(
            "#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]\n",
        );
        let _ = writeln!(out, "pub struct {} {{", dto.names.pascal);
        struct_fields(out, &dto.fields, true, true);
        out.push_str("}\n");
    } else {
        let _ = writeln!(
            out,
            "\n/// {description}, as the API sends it: one of these shapes."
        );
        out.push_str("#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]\n#[serde(untagged)]\n");
        let _ = writeln!(out, "pub enum {} {{", dto.names.pascal);
        for variant in &dto.variants {
            let _ = writeln!(out, "    /// {}.", sentence(&variant.names.pascal));
            let _ = writeln!(out, "    {} {{", variant.names.pascal);
            for field in &variant.fields {
                let _ = writeln!(out, "        /// `{}`.", field.name);
                if let Some(rename) = &field.rename {
                    let _ = writeln!(out, "        #[serde(rename = {rename:?})]");
                }
                let _ = writeln!(out, "        {}: {},", field.name, rust_type(&field.ty));
            }
            out.push_str("    },\n");
        }
        out.push_str("}\n");
    }
}

/// The arguments an endpoint's provider method takes, in order.
fn endpoint_args(endpoint: &super::spec::EndpointSpec) -> Vec<(String, String)> {
    let mut args: Vec<(String, String)> = endpoint
        .path_params
        .iter()
        .chain(&endpoint.query)
        .map(|field| (field.name.clone(), field.ty.clone()))
        .collect();
    if let Some(body) = &endpoint.body {
        args.push(("body".into(), body.clone()));
    }
    if endpoint.raw_body {
        args.push(("body".into(), "Vec<u8>".into()));
        args.push(("content_type".into(), "String".into()));
    }
    args
}

fn http_provider(plan: &Plan) -> String {
    let provider = plan.provider.as_ref().expect("a provider");
    let name = &provider.names.pascal;
    let mut imports = Imports::default();
    imports.add("rok_ui::http::ApiError");
    imports.add("rok_ui::http::HttpClient");
    imports.add("rok_ui::http::Options");
    let mut types = Vec::new();
    for endpoint in &provider.endpoints {
        types.extend(endpoint.path_params.iter().map(|param| param.ty.as_str()));
        types.extend(endpoint.query.iter().map(|query| query.ty.as_str()));
        if endpoint.response == "bytes" {
            imports.add("rok_ui::http::Bytes");
        }
        if !endpoint.path_params.is_empty() {
            imports.add("rok_ui::http::path_segment");
        }
    }
    let dto_names: Vec<&str> = plan
        .dtos
        .iter()
        .map(|dto| dto.names.pascal.as_str())
        .collect();
    import_types(plan, &mut imports, &types, "crate");
    imports.0.retain(|path| {
        !dto_names
            .iter()
            .any(|dto| path.ends_with(&format!("::{dto}")))
    });
    let mut out = header(&format!(
        "`{name}`: the `{}` API. Every path, verb and wire name lives in this file, so a \
         backend change is a one-place fix.",
        plan.feature.snake
    ));
    out.push_str(&imports.render());
    let _ = write!(
        out,
        "\n/// The `{}` endpoints, over the app's [`HttpClient`].\n#[derive(Clone, Debug)]\npub struct {name} {{\n    client: HttpClient,\n}}\n\nimpl {name} {{\n    /// The endpoints, called through `client`.\n    #[must_use]\n    pub fn new(client: HttpClient) -> Self {{\n        Self {{ client }}\n    }}\n",
        plan.feature.snake
    );
    for endpoint in &provider.endpoints {
        let path = format!("{}{}", provider.api_base, endpoint.path);
        let args = endpoint_args(endpoint);
        let signature: Vec<String> = args
            .iter()
            .map(|(arg, ty)| format!("{arg}: {}", rust_type(ty)))
            .collect();
        let (call, returns) = match endpoint.response.as_str() {
            "()" => ("request_empty", "()".to_string()),
            "bytes" => ("request_bytes", "Bytes".to_string()),
            other => ("request", other.to_string()),
        };
        let verb = endpoint.verb.to_lowercase();
        let mut options = format!("Options::{verb}()");
        for query in &endpoint.query {
            let _ = write!(
                options,
                ".query({:?}, {})",
                query.rename.as_deref().unwrap_or(&query.name),
                query.name
            );
        }
        if endpoint.body.is_some() {
            options.push_str(".json(&body)");
        }
        if endpoint.raw_body {
            options.push_str(".raw_body(body, content_type)");
        }
        if endpoint.skip_expire {
            options.push_str(".skip_expire(true)");
        }
        let path_expression = if endpoint.path_params.is_empty() {
            format!("{path:?}")
        } else {
            let mut template = path.clone();
            let mut values = Vec::new();
            for param in &endpoint.path_params {
                template = template.replace(&format!("{{{}}}", param.name), "{}");
                values.push(format!("path_segment(&{})", param.name));
            }
            format!("&format!({template:?}, {})", values.join(", "))
        };
        let skip = if endpoint.skip_expire {
            "\n    ///\n    /// A 401 here is an answer (wrong credentials), not an expired session."
        } else {
            ""
        };
        let _ = write!(
            out,
            "\n    /// `{} {path}`.{skip}\n    ///\n    /// # Errors\n    ///\n    /// The server's error, or why the server could not be reached.\n    pub async fn {}(&self, {}) -> Result<{returns}, ApiError> {{\n        self.client.{call}({path_expression}, {options}).await\n    }}\n",
            endpoint.verb,
            endpoint.method,
            signature.join(", "),
        );
    }
    out.push_str("}\n");
    for each in &plan.dtos {
        dto(&mut out, each);
    }
    out
}

/// `src/data/repositories/<repository>.rs`.
pub fn repository(plan: &Plan) -> String {
    let repository = plan.repository.as_ref().expect("a repository");
    let provider = plan.provider.as_ref().expect("a provider");
    let error = plan.error_type();
    let trait_name = &repository.names.pascal;
    let mut imports = Imports::default();
    imports.add("rok_ui::bloc::BoxFuture");
    let _ = provider;
    imports.add(&format!(
        "crate::data::providers::{}::{}",
        provider.names.snake, provider.names.pascal
    ));
    let mut types: Vec<&str> = vec![error];
    for method in &repository.methods {
        types.push(&method.returns);
        types.extend(method.args.iter().map(|arg| arg.ty.as_str()));
    }
    if plan.is_http() {
        for endpoint in &provider.endpoints {
            if let Some(body) = &endpoint.body {
                types.push(body);
            }
            types.push(&endpoint.response);
        }
    }
    import_types(plan, &mut imports, &types, "crate");
    let mut out = header(&format!(
        "`{trait_name}`: what the `{}` business logic reads and writes.",
        plan.feature.snake
    ));
    out.push_str(&imports.render());
    let _ = write!(
        out,
        "\n/// What the `{}` business logic reads and writes. A trait, so tests and other data \
         sources can stand in.\npub trait {trait_name}: Send + Sync {{\n",
        plan.feature.snake
    );
    for method in &repository.methods {
        let args: Vec<String> = method
            .args
            .iter()
            .map(|arg| format!("{}: {}", arg.name, rust_type(&arg.ty)))
            .collect();
        let comma = if args.is_empty() { "" } else { ", " };
        let _ = write!(
            out,
            "    /// {}.\n    fn {}(&self{comma}{}) -> BoxFuture<'_, Result<{}, {error}>>;\n",
            sentence(&method.name),
            method.name,
            args.join(", "),
            rust_type(&method.returns),
        );
    }
    let implementation = format!("{trait_name}Impl");
    let _ = write!(
        out,
        "}}\n\n/// [`{trait_name}`] over a [`{0}`].\n#[derive(Debug)]\npub struct {implementation} {{\n    provider: {0},\n}}\n\nimpl {implementation} {{\n    /// A repository reading and writing through `provider`.\n    #[must_use]\n    pub fn new(provider: {0}) -> Self {{\n        Self {{ provider }}\n    }}\n\n    /// The data source, for the methods you add.\n    #[must_use]\n    pub fn provider(&self) -> &{0} {{\n        &self.provider\n    }}\n}}\n\nimpl {trait_name} for {implementation} {{\n",
        provider.names.pascal
    );
    for method in &repository.methods {
        let body = if plan.is_http() {
            http_method_body(plan, method)
        } else {
            memory_method_body(plan, method)
        };
        let args: Vec<String> = method
            .args
            .iter()
            .map(|arg| {
                let used = body.contains(&arg.name);
                let name = if used {
                    arg.name.clone()
                } else {
                    format!("_{}", arg.name)
                };
                format!("{name}: {}", rust_type(&arg.ty))
            })
            .collect();
        let comma = if args.is_empty() { "" } else { ", " };
        let _ = write!(
            out,
            "    fn {}(&self{comma}{}) -> BoxFuture<'_, Result<{}, {error}>> {{\n        Box::pin(async move {{ {body} }})\n    }}\n\n",
            method.name,
            args.join(", "),
            rust_type(&method.returns),
        );
    }
    out.push_str("}\n");
    if plan.is_http() {
        for each in &plan.dtos {
            let target = each.names.pascal.trim_end_matches("Dto");
            if let Some(model) = plan.model(target) {
                from_dto(&mut out, each, model);
            }
        }
    }
    out
}

/// How a DTO field becomes a model field.
fn convert(expression: &str, dto_type: &str) -> String {
    let ty = dto_type.replace(' ', "");
    if !ty.contains("Dto") {
        expression.to_string()
    } else if ty.starts_with("Vec<") {
        format!("{expression}.into_iter().map(Into::into).collect()")
    } else if ty.starts_with("Option<") {
        format!("{expression}.map(Into::into)")
    } else {
        format!("{expression}.into()")
    }
}

fn copy_fields(source_fields: &[Field], target_fields: &[Field], source: &str) -> String {
    target_fields
        .iter()
        .map(|field| {
            match source_fields
                .iter()
                .find(|candidate| candidate.name == field.name)
            {
                Some(candidate) => {
                    let value = convert(&format!("{source}{}", candidate.name), &candidate.ty);
                    if value == field.name {
                        field.name.clone()
                    } else {
                        format!("{}: {value}", field.name)
                    }
                }
                None => format!("{}: {}", field.name, default_of(&field.ty)),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn from_dto(out: &mut String, dto: &Model, model: &Model) {
    let (dto_name, model_name) = (&dto.names.pascal, &model.names.pascal);
    if dto.variants.is_empty() {
        let fields = copy_fields(&dto.fields, &model.fields, "dto.");
        let _ = write!(
            out,
            "\nimpl From<{dto_name}> for {model_name} {{\n    fn from(dto: {dto_name}) -> Self {{\n        Self {{ {fields} }}\n    }}\n}}\n"
        );
    } else {
        let mut arms = String::new();
        for variant in &dto.variants {
            let Some(target) = model
                .variants
                .iter()
                .find(|candidate| candidate.names.pascal == variant.names.pascal)
            else {
                continue;
            };
            let bindings: Vec<String> = variant
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect();
            let fields = copy_fields(&variant.fields, &target.fields, "");
            let _ = writeln!(
                arms,
                "            {dto_name}::{0} {{ {1} }} => Self::{0} {{ {fields} }},",
                variant.names.pascal,
                bindings.join(", ")
            );
        }
        let _ = write!(
            out,
            "\nimpl From<{dto_name}> for {model_name} {{\n    fn from(dto: {dto_name}) -> Self {{\n        match dto {{\n{arms}        }}\n    }}\n}}\n"
        );
    }
}

fn http_method_body(plan: &Plan, method: &Method) -> String {
    let provider = plan.provider.as_ref().expect("a provider");
    let call = method.calls.clone().unwrap_or_else(|| method.name.clone());
    let Some(endpoint) = provider
        .endpoints
        .iter()
        .find(|endpoint| endpoint.method == call)
    else {
        return format!("Ok({})", default_of(&method.returns));
    };
    let arg = |name: &str| method.args.iter().find(|arg| arg.name == name);
    let mut values = Vec::new();
    for field in endpoint.path_params.iter().chain(&endpoint.query) {
        values.push(if arg(&field.name).is_some() {
            field.name.clone()
        } else {
            default_of(&field.ty)
        });
    }
    if let Some(body) = &endpoint.body {
        let dto_fields = plan
            .dtos
            .iter()
            .find(|dto| &dto.names.pascal == body)
            .map(|dto| dto.fields.clone())
            .unwrap_or_default();
        if dto_fields.is_empty() {
            values.push(default_of(body));
        } else {
            let fields = copy_fields(&method.args, &dto_fields, "");
            values.push(format!("{body} {{ {fields} }}"));
        }
    }
    if endpoint.raw_body {
        values.push(if arg("body").is_some() {
            "body".into()
        } else {
            "Vec::new()".into()
        });
        values.push(if arg("content_type").is_some() {
            "content_type".into()
        } else {
            "\"application/octet-stream\".into()".into()
        });
    }
    let request = format!("self.provider.{call}({}).await", values.join(", "));
    let response = endpoint.response.replace(' ', "");
    let returns = method.returns.replace(' ', "");
    let mapping = if response == returns || (response == "bytes" && returns == "Bytes") {
        String::new()
    } else if returns == "()" {
        ".map(|_| ())".into()
    } else if response.starts_with("Vec<") && returns.starts_with("Vec<") {
        ".map(|items| items.into_iter().map(Into::into).collect())".into()
    } else if response.ends_with("Dto") {
        ".map(Into::into)".into()
    } else {
        format!(".map(|_| {})", default_of(&method.returns))
    };
    format!("{request}{mapping}")
}

fn memory_method_body(plan: &Plan, method: &Method) -> String {
    let Some(model) = plan.primary_model() else {
        return format!("Ok({})", default_of(&method.returns));
    };
    let model_name = &model.names.pascal;
    let returns = method.returns.replace(' ', "");
    let list_type = format!("Vec<{model_name}>");
    let id_arg = model.id.as_ref().and_then(|id| {
        method
            .args
            .iter()
            .find(|arg| arg.name == "id" || &arg.ty == id)
            .map(|arg| arg.name.clone())
    });
    let data_args: Vec<&Field> = method
        .args
        .iter()
        .filter(|arg| Some(&arg.name) != id_arg.as_ref())
        .filter(|arg| {
            model.fields.iter().any(|field| {
                field.name == arg.name && field.ty.replace(' ', "") == arg.ty.replace(' ', "")
            })
        })
        .collect();
    let verb = method.name.split('_').next().unwrap_or_default();
    let ok_item = |expression: &str| {
        if returns == "()" {
            format!("{expression};\n Ok(())")
        } else if returns == *model_name {
            format!("Ok({expression})")
        } else {
            format!("let _ = {expression};\n Ok({})", default_of(&returns))
        }
    };
    let found = |expression: &str| {
        if returns == "()" {
            format!("{expression}.map(|_| ()).ok_or(DataError::NotFound)")
        } else if returns == *model_name {
            format!("{expression}.ok_or(DataError::NotFound)")
        } else if returns == format!("Option<{model_name}>") {
            format!("Ok({expression})")
        } else {
            format!("let _ = {expression};\n Ok({})", default_of(&returns))
        }
    };
    if method.args.is_empty() && returns == list_type {
        return "Ok(self.provider.all())".into();
    }
    if ["add", "create", "insert", "submit"].contains(&verb) {
        let fields: Vec<String> = data_args.iter().map(|arg| arg.name.clone()).collect();
        let mut parts = fields.clone();
        if fields.len() != model.fields.len() {
            parts.push(format!("..{model_name}::default()"));
        }
        let item = format!("{model_name} {{ {} }}", parts.join(", "));
        return ok_item(&format!("self.provider.insert({item})"));
    }
    if let Some(id) = &id_arg {
        if model.id.is_some() {
            if ["delete", "remove"].contains(&verb) {
                return found(&format!("self.provider.remove(&{id})"));
            }
            if ["get", "find", "load", "fetch"].contains(&verb) {
                return found(&format!("self.provider.find(&{id})"));
            }
            if !data_args.is_empty()
                || ["update", "save", "rename", "edit", "toggle", "change"].contains(&verb)
            {
                let changes: Vec<String> = data_args
                    .iter()
                    .map(|arg| format!("item.{0} = {0};", arg.name))
                    .collect();
                let item = if changes.is_empty() { "_item" } else { "item" };
                return found(&format!(
                    "self.provider.update(&{id}, |{item}| {{ {} }})",
                    changes.join(" ")
                ));
            }
        }
    }
    format!("Ok({})", default_of(&method.returns))
}

// ---------------------------------------------------------------------------
// Business logic.

/// The state's type name.
fn state_name(plan: &Plan) -> String {
    format!("{}State", plan.name.pascal)
}

fn status_name(plan: &Plan) -> String {
    format!("{}Status", plan.name.pascal)
}

/// The fields the state stores, plus `error` for struct states.
fn state_has_error_field(plan: &Plan) -> bool {
    plan.state.style == StateStyle::Struct
        && !plan.state.fields.iter().any(|field| field.name == "error")
}

/// `src/features/<f>/bloc/<name>_event.rs`.
pub fn event(plan: &Plan) -> String {
    let mut imports = Imports::default();
    let types: Vec<&str> = plan
        .events
        .iter()
        .flat_map(|event| event.fields.iter().map(|field| field.ty.as_str()))
        .collect();
    import_types(plan, &mut imports, &types, "crate");
    let owner = match plan.kind {
        Kind::Bloc => format!("{}Bloc", plan.name.pascal),
        Kind::Cubit => format!("{}Cubit", plan.name.pascal),
    };
    let mut out = header(&format!(
        "What happens in `{}`, in the past tense.",
        plan.feature.snake
    ));
    out.push_str(&imports.render());
    let _ = write!(
        out,
        "\n/// Events for `{owner}`.\n#[derive(Clone, Debug)]\npub enum {}Event {{\n",
        plan.name.pascal
    );
    for event in &plan.events {
        let _ = writeln!(out, "    /// {}.", sentence(&event.name));
        if event.fields.is_empty() {
            let _ = writeln!(out, "    {},", event.name);
        } else {
            let _ = writeln!(out, "    {} {{", event.name);
            for field in &event.fields {
                let _ = writeln!(out, "        /// `{}`.", field.name);
                let _ = writeln!(out, "        {}: {},", field.name, rust_type(&field.ty));
            }
            out.push_str("    },\n");
        }
    }
    out.push_str("}\n");
    out
}

/// `src/features/<f>/bloc/<name>_state.rs`.
pub fn state(plan: &Plan) -> String {
    let mut imports = Imports::default();
    let mut types: Vec<&str> = plan
        .state
        .fields
        .iter()
        .map(|field| field.ty.as_str())
        .collect();
    types.push(plan.error_type());
    import_types(plan, &mut imports, &types, "crate");
    let state = state_name(plan);
    let mut out = header(&format!("What the `{}` views show.", plan.feature.snake));
    out.push_str(&imports.render());
    out.push('\n');
    let defaults = plan
        .state
        .fields
        .iter()
        .any(|field| field.default.is_some());
    match plan.state.style {
        StateStyle::Struct => {
            let status = status_name(plan);
            let _ = write!(
                out,
                "/// Where `{state}` is.\n#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\npub enum {status} {{\n"
            );
            for (index, name) in plan.state.status.iter().enumerate() {
                let _ = writeln!(out, "    /// {}.", sentence(name));
                if index == 0 {
                    out.push_str("    #[default]\n");
                }
                let _ = writeln!(out, "    {name},");
            }
            let derive = if defaults {
                "Clone, Debug, PartialEq"
            } else {
                "Clone, Debug, Default, PartialEq"
            };
            let _ = write!(
                out,
                "}}\n\n/// What the views show: an immutable value.\n#[derive({derive})]\npub struct {state} {{\n    /// Where loading is.\n    pub status: {status},\n"
            );
            struct_fields(&mut out, &plan.state.fields, true, false);
            if state_has_error_field(plan) {
                let _ = writeln!(out, "    /// Why the last call failed, while `status` says so.\n    pub error: Option<{}>,", plan.error_type());
            }
            out.push_str("}\n");
            if defaults {
                let mut values = vec![format!("status: {status}::default()")];
                for field in &plan.state.fields {
                    values.push(format!(
                        "{}: {}",
                        field.name,
                        field
                            .default
                            .clone()
                            .unwrap_or_else(|| default_of(&field.ty))
                    ));
                }
                if state_has_error_field(plan) {
                    values.push("error: None".into());
                }
                let _ = write!(
                    out,
                    "\nimpl Default for {state} {{\n    fn default() -> Self {{\n        Self {{ {} }}\n    }}\n}}\n",
                    values.join(", ")
                );
            }
        }
        StateStyle::Enum => {
            let data = format!("{}Data", plan.name.pascal);
            let derive = if defaults {
                "Clone, Debug, PartialEq"
            } else {
                "Clone, Debug, Default, PartialEq"
            };
            let _ = write!(out, "/// The data a successful `{state}` holds.\n#[derive({derive})]\npub struct {data} {{\n");
            struct_fields(&mut out, &plan.state.fields, true, false);
            out.push_str("}\n");
            if defaults {
                let values: Vec<String> = plan
                    .state
                    .fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{}: {}",
                            field.name,
                            field
                                .default
                                .clone()
                                .unwrap_or_else(|| default_of(&field.ty))
                        )
                    })
                    .collect();
                let _ = write!(
                    out,
                    "\nimpl Default for {data} {{\n    fn default() -> Self {{\n        Self {{ {} }}\n    }}\n}}\n",
                    values.join(", ")
                );
            }
            let _ = write!(out, "\n/// What the views show: an immutable value.\n#[derive(Clone, Debug, Default, PartialEq)]\npub enum {state} {{\n");
            for (index, name) in plan.state.status.iter().enumerate() {
                let _ = writeln!(out, "    /// {}.", sentence(name));
                if index == 0 {
                    out.push_str("    #[default]\n");
                }
                match name.as_str() {
                    "Success" => {
                        let _ = writeln!(out, "    {name}({data}),");
                    }
                    "Failure" => {
                        let _ = writeln!(out, "    {name}({}),", plan.error_type());
                    }
                    _ => {
                        let _ = writeln!(out, "    {name},");
                    }
                }
            }
            let _ = write!(
                out,
                "}}\n\nimpl {state} {{\n    /// The data of the last success (empty before one).\n    #[must_use]\n    pub fn data(&self) -> {data} {{\n        match self {{\n            Self::Success(data) => data.clone(),\n            _ => {data}::default(),\n        }}\n    }}\n}}\n"
            );
        }
    }
    out
}

/// The statements that apply one event, using `repository` and `emit`.
fn handler(plan: &Plan, event: &Event, repository: &str) -> String {
    let style = plan.state.style;
    let state = state_name(plan);
    let status = status_name(plan);
    let has = |name: &str| plan.state.has_status(name);
    let mut out = String::new();
    let enum_data = style == StateStyle::Enum;
    let set_status = |out: &mut String, name: &str| {
        if !has(name) {
            return;
        }
        match style {
            StateStyle::Struct => {
                let _ = writeln!(out, "emit.update(|state| state.status = {status}::{name});");
            }
            StateStyle::Enum => {
                if name != "Success" && name != "Failure" {
                    let _ = writeln!(out, "emit.emit({state}::{name});");
                }
            }
        }
    };
    let success = |assign: Option<(&str, &str)>| -> String {
        match style {
            StateStyle::Struct => {
                let mut changes = String::new();
                if has("Success") {
                    let _ = write!(changes, "state.status = {status}::Success; ");
                }
                if let Some((field, value)) = assign {
                    let _ = write!(changes, "state.{field} = {value}; ");
                }
                if state_has_error_field(plan) {
                    changes.push_str("state.error = None;");
                }
                format!("emit.update(|state| {{ {changes} }});")
            }
            StateStyle::Enum => {
                let assign = assign
                    .map(|(field, value)| format!("data.{field} = {value};"))
                    .unwrap_or_default();
                let binding = if assign.is_empty() {
                    "data"
                } else {
                    "mut data"
                };
                format!("let {binding} = previous; {assign} emit.emit({state}::Success(data));")
            }
        }
    };
    let failure = match style {
        StateStyle::Struct => {
            let mut changes = String::new();
            if has("Failure") {
                let _ = write!(changes, "state.status = {status}::Failure; ");
            }
            if state_has_error_field(plan) {
                changes.push_str("state.error = Some(error);");
            } else {
                changes.push_str("let _ = error;");
            }
            format!("emit.update(|state| {{ {changes} }});")
        }
        StateStyle::Enum => format!("emit.emit({state}::Failure(error));"),
    };
    let cancelled = if plan.is_http() {
        "Err(error) if error.is_cancelled() => {}\n"
    } else {
        ""
    };
    let method = event.calls.as_ref().and_then(|name| plan.method(name));
    let Some(method) = method else {
        // No repository call: copy the fields the state also has.
        for field in &event.fields {
            if plan.state.fields.iter().any(|candidate| {
                candidate.name == field.name
                    && candidate.ty.replace(' ', "") == field.ty.replace(' ', "")
            }) {
                match style {
                    StateStyle::Struct => {
                        let _ = writeln!(out, "emit.update(|state| state.{0} = {0});", field.name);
                    }
                    StateStyle::Enum => {
                        let _ = writeln!(out, "let mut data = emit.state().data(); data.{0} = {0}; emit.emit({state}::Success(data));", field.name);
                    }
                }
            }
        }
        return out;
    };
    if enum_data {
        out.push_str("let previous = emit.state().data();\n");
    }
    set_status(&mut out, "Loading");
    let arguments: Vec<String> = method.args.iter().map(|arg| arg.name.clone()).collect();
    let call = format!(
        "{repository}.{}({}).await",
        method.name,
        arguments.join(", ")
    );
    let assigned = plan
        .state
        .fields
        .iter()
        .find(|field| field.ty.replace(' ', "") == method.returns.replace(' ', ""));
    let ignored = if method.returns.replace(' ', "") == "()" {
        "()"
    } else {
        "_"
    };
    match (assigned, plan.loader()) {
        (Some(field), _) => {
            let _ = write!(
                out,
                "match {call} {{\nOk(value) => {{ {} }}\n{cancelled}Err(error) => {{ {failure} }}\n}}\n",
                success(Some((&field.name, "value")))
            );
        }
        // A write: then reload what the state shows.
        (None, Some((loader, field))) if loader.name != method.name => {
            let _ = write!(
                out,
                "let result = match {call} {{\nOk({ignored}) => {repository}.{}().await,\nErr(error) => Err(error),\n}};\nmatch result {{\nOk(value) => {{ {} }}\n{cancelled}Err(error) => {{ {failure} }}\n}}\n",
                loader.name,
                success(Some((&field.name, "value")))
            );
        }
        _ => {
            let _ = write!(
                out,
                "match {call} {{\nOk({ignored}) => {{ {} }}\n{cancelled}Err(error) => {{ {failure} }}\n}}\n",
                success(None)
            );
        }
    }
    out
}

fn concurrency_variant(kind: ConcurrencyKind) -> &'static str {
    match kind {
        ConcurrencyKind::Sequential => "Sequential",
        ConcurrencyKind::Droppable => "Droppable",
        ConcurrencyKind::Restartable => "Restartable",
        ConcurrencyKind::Concurrent => "Concurrent",
    }
}

fn bloc_imports(plan: &Plan, imports: &mut Imports) {
    let event = format!("{}Event", plan.name.pascal);
    let state = state_name(plan);
    let event_module = format!("{}_event", plan.name.snake);
    let state_module = format!("{}_state", plan.name.snake);
    imports.add(&format!("super::{event_module}::{event}"));
    imports.add(&format!("super::{state_module}::{state}"));
    let uses_status = plan.state.style == StateStyle::Struct
        && plan.events.iter().any(|event| {
            event.calls.is_some()
                && plan
                    .state
                    .status
                    .iter()
                    .any(|status| ["Loading", "Success", "Failure"].contains(&status.as_str()))
        });
    if uses_status {
        imports.add(&format!("super::{state_module}::{}", status_name(plan)));
    }
    if let Some(repository) = &plan.repository {
        imports.add("std::sync::Arc");
        imports.add(&format!(
            "crate::data::repositories::{}::{}",
            repository.names.snake, repository.names.pascal
        ));
    }
}

/// `src/features/<f>/bloc/<name>_bloc.rs`.
pub fn bloc(plan: &Plan) -> String {
    let name = format!("{}Bloc", plan.name.pascal);
    let event = format!("{}Event", plan.name.pascal);
    let state = state_name(plan);
    let mut imports = Imports::default();
    imports.add("rok_ui::bloc::Bloc");
    imports.add("rok_ui::bloc::Emitter");
    bloc_imports(plan, &mut imports);
    let custom_concurrency = plan.concurrency != ConcurrencyKind::Sequential
        || plan.events.iter().any(|event| {
            event
                .concurrency
                .is_some_and(|kind| kind != plan.concurrency)
        });
    if custom_concurrency {
        imports.add("rok_ui::bloc::Concurrency");
    }
    let mut out = header(&format!(
        "`{name}`: the business logic of `{}`.",
        plan.feature.snake
    ));
    out.push_str(&imports.render());
    let repository = plan.repository.as_ref().expect("a repository");
    let _ = write!(
        out,
        "\n/// Turns [`{event}`]s into [`{state}`]s, reading and writing through a [`{0}`].\npub struct {name} {{\n    repository: Arc<dyn {0}>,\n}}\n\nimpl {name} {{\n    /// A bloc reading and writing through `repository`.\n    #[must_use]\n    pub fn new(repository: Arc<dyn {0}>) -> Self {{\n        Self {{ repository }}\n    }}\n}}\n\nimpl Bloc for {name} {{\n    type Event = {event};\n    type State = {state};\n\n    fn initial_state(&self) -> {state} {{\n        {state}::default()\n    }}\n",
        repository.names.pascal
    );
    if custom_concurrency {
        let mut arms = String::new();
        for event in &plan.events {
            let kind = event.concurrency.unwrap_or(plan.concurrency);
            let pattern = if event.fields.is_empty() {
                String::new()
            } else {
                " { .. }".into()
            };
            let _ = writeln!(
                arms,
                "{event_type}::{}{pattern} => Concurrency::{},",
                event.name,
                concurrency_variant(kind),
                event_type = event_name(plan)
            );
        }
        let _ = write!(out, "\n    fn concurrency(&self, event: &{event}) -> Concurrency {{\n        match event {{\n{arms}        }}\n    }}\n");
    }
    let _ = write!(out, "\n    async fn on(&self, event: {event}, emit: &Emitter<{state}>) {{\n        match event {{\n");
    for event in &plan.events {
        let body = handler(plan, event, "self.repository");
        let _ = writeln!(out, "{} => {{\n{body}}}", pattern(plan, event, &body));
    }
    out.push_str("        }\n    }\n}\n");
    out
}

fn event_name(plan: &Plan) -> String {
    format!("{}Event", plan.name.pascal)
}

/// The match pattern for `event`, binding the fields `body` uses.
fn pattern(plan: &Plan, event: &Event, body: &str) -> String {
    let name = format!("{}::{}", event_name(plan), event.name);
    if event.fields.is_empty() {
        return name;
    }
    let used: Vec<&str> = event
        .fields
        .iter()
        .filter(|field| body.contains(&field.name))
        .map(|field| field.name.as_str())
        .collect();
    let rest = if used.len() == event.fields.len() {
        ""
    } else {
        ".."
    };
    let separator = if used.is_empty() || rest.is_empty() {
        ""
    } else {
        ", "
    };
    format!("{name} {{ {}{separator}{rest} }}", used.join(", "))
}

/// A cubit method's name for `event`: `NoteAdded` -> `add_note`.
pub fn cubit_method(event: &Event) -> String {
    let verb = names::event_verb(&event.name);
    let subject = names::event_subject(&event.name);
    if subject.is_empty() {
        verb
    } else {
        format!("{verb}_{subject}")
    }
}

/// `src/features/<f>/bloc/<name>_cubit.rs`.
pub fn cubit(plan: &Plan) -> String {
    let name = format!("{}Cubit", plan.name.pascal);
    let state = state_name(plan);
    let mut imports = Imports::default();
    imports.add("rok_ui::bloc::Cubit");
    imports.add("rok_ui::bloc::Emitter");
    bloc_imports(plan, &mut imports);
    imports
        .0
        .retain(|path| !path.ends_with(&format!("::{}", event_name(plan))));
    let types: Vec<&str> = plan
        .events
        .iter()
        .flat_map(|event| event.fields.iter().map(|field| field.ty.as_str()))
        .collect();
    import_types(plan, &mut imports, &types, "crate");
    let repository = plan.repository.as_ref().expect("a repository");
    let mut out = header(&format!(
        "`{name}`: the business logic of `{}`.",
        plan.feature.snake
    ));
    out.push_str(&imports.render());
    let _ = write!(
        out,
        "\n/// Changes [`{state}`] through its methods, reading and writing through a [`{0}`].\npub struct {name} {{\n    state: Emitter<{state}>,\n    repository: Arc<dyn {0}>,\n}}\n\nimpl {name} {{\n    /// A cubit reading and writing through `repository`.\n    #[must_use]\n    pub fn new(repository: Arc<dyn {0}>) -> Self {{\n        Self {{ state: Emitter::new({state}::default()), repository }}\n    }}\n",
        repository.names.pascal
    );
    for event in &plan.events {
        let body = handler(plan, event, "repository");
        let args: Vec<String> = event
            .fields
            .iter()
            .map(|field| {
                let name = if body.contains(&field.name) {
                    field.name.clone()
                } else {
                    format!("_{}", field.name)
                };
                format!("{name}: {}", rust_type(&field.ty))
            })
            .collect();
        let comma = if args.is_empty() { "" } else { ", " };
        let repository_clone = if body.contains("repository.") {
            "let repository = self.repository.clone();\n"
        } else {
            ""
        };
        let _ = write!(
            out,
            "\n    /// {}.\n    pub fn {}(&self{comma}{}) {{\n        {repository_clone}let emit = self.state.clone();\n        self.state.spawn(async move {{\n            let emit = &emit;\n            {body}\n        }});\n    }}\n",
            sentence(&cubit_method(event)),
            cubit_method(event),
            args.join(", ")
        );
    }
    let _ = write!(out, "}}\n\nimpl Cubit for {name} {{\n    type State = {state};\n\n    fn emitter(&self) -> &Emitter<{state}> {{\n        &self.state\n    }}\n}}\n");
    out
}

// ---------------------------------------------------------------------------
// Presentation.

/// The page component's name.
pub fn page_name(plan: &Plan) -> String {
    format!("{}Page", plan.name.pascal)
}

/// The form's values type.
fn form_values(plan: &Plan) -> Option<(String, Vec<Field>, &Event)> {
    let form = plan.form.as_ref()?;
    let event = plan.events.iter().find(|event| event.name == form.event)?;
    let fields = if form.fields.is_empty() {
        event.fields.clone()
    } else {
        form.fields
            .iter()
            .map(|field| Field {
                name: field.name.clone(),
                ty: field.ty.clone(),
                default: field.default.clone(),
                rename: None,
            })
            .collect()
    };
    Some((format!("{}Form", event.name), fields, event))
}

/// `src/features/<f>/view/<name>_page.rs`.
#[allow(clippy::too_many_lines)]
pub fn page(plan: &Plan) -> String {
    let page = page_name(plan);
    let state = state_name(plan);
    let is_cubit = plan.kind == Kind::Cubit;
    let owner = if is_cubit {
        format!("{}Cubit", plan.name.pascal)
    } else {
        format!("{}Bloc", plan.name.pascal)
    };
    let owner_module = if is_cubit {
        format!("{}_cubit", plan.name.snake)
    } else {
        format!("{}_bloc", plan.name.snake)
    };
    let bloc_module = format!("crate::features::{}::bloc", plan.feature.snake);
    let mut imports = Imports::default();
    imports.add("rok_ui::bloc::BlocBuilder");
    imports.add("rok_ui::prelude::*");
    imports.add(&format!("{bloc_module}::{owner_module}::{owner}"));
    let mut types: Vec<&str> = Vec::new();
    let form = form_values(plan);
    if let Some((_, fields, _)) = &form {
        types.extend(fields.iter().map(|field| field.ty.as_str()));
        imports.add("rok_ui::form::{self, FormOptions, FormValues, SubmitButton}");
    }
    import_types(plan, &mut imports, &types, "crate");
    let status = status_name(plan);
    if plan.state.style == StateStyle::Struct {
        imports.add(&format!(
            "{bloc_module}::{}_state::{status}",
            plan.name.snake
        ));
    } else {
        imports.add(&format!(
            "{bloc_module}::{}_state::{state}",
            plan.name.snake
        ));
    }
    let no_field_events: Vec<&Event> = plan
        .events
        .iter()
        .filter(|event| event.fields.is_empty())
        .collect();
    if !is_cubit && (!no_field_events.is_empty() || form.is_some()) {
        imports.add(&format!(
            "{bloc_module}::{}_event::{}",
            plan.name.snake,
            event_name(plan)
        ));
    }
    let mut out = header(&format!("The `{}` page.", plan.feature.snake));
    if form.is_some() {
        imports.0.retain(|path| path != "rok_ui::prelude::*");
        imports.add("rok_ui::prelude::*");
    }
    out.push_str(&imports.render());
    // The form's values type.
    let mut form_fields_ui = String::new();
    if let Some((values, fields, event)) = &form {
        let _ = write!(out, "\n/// What the `{}` form edits.\n#[derive(FormValues, Clone, Default)]\nstruct {values} {{\n", event.name);
        struct_fields(&mut out, fields, false, false);
        out.push_str("}\n");
        let event_fields: Vec<String> = event
            .fields
            .iter()
            .map(|field| {
                if fields.iter().any(|candidate| candidate.name == field.name) {
                    format!("{0}: values.{0}", field.name)
                } else {
                    format!("{}: {}", field.name, default_of(&field.ty))
                }
            })
            .collect();
        let dispatch = if is_cubit {
            let args: Vec<String> = event
                .fields
                .iter()
                .map(|field| {
                    if fields.iter().any(|candidate| candidate.name == field.name) {
                        format!("values.{}", field.name)
                    } else {
                        default_of(&field.ty)
                    }
                })
                .collect();
            format!("submitting.{}({});", cubit_method(event), args.join(", "))
        } else {
            format!(
                "submitting.add({}::{} {{ {} }});",
                event_name(plan),
                event.name,
                event_fields.join(", ")
            )
        };
        let _ = write!(
            form_fields_ui,
            "    let submitting = bloc.clone();\n    let form = form::use_form(\n        cx,\n        FormOptions::new({values}::default()).on_submit(move |values: {values}, _| {{\n            {dispatch}\n            gpui::Task::ready(Ok(()))\n        }}),\n    );\n"
        );
        let mut controls = Vec::new();
        for field in fields {
            let constant = names::snake(&field.name).to_uppercase();
            let label = sentence(&field.name);
            match field.ty.as_str() {
                "String" => controls.push(format!(".child(form::TextField::new(&form.field(cx, {values}::{constant}), {label:?}))")),
                "bool" => controls.push(format!(".child(form::CheckboxField::new(&form.field(cx, {values}::{constant}), {label:?}))")),
                _ => {}
            }
        }
        let _ = writeln!(
            form_fields_ui,
            "    let editor = div().flex().flex_col().gap_2(){}.child(SubmitButton::new(&form, {:?}));",
            controls.join(""),
            sentence(&names::event_verb(&event.name))
        );
    }
    let lookup = if is_cubit { "cubit" } else { "bloc" };
    let _ = write!(out, "\n/// Shows `{}` and {}.\n#[component]\npub fn {page}(cx: &mut Cx) -> impl IntoElement {{\n    let bloc = cx.{lookup}::<{owner}>();\n", plan.feature.snake, if is_cubit { "calls its methods" } else { "adds its events" });
    // Load once.
    if let Some(first) = no_field_events.iter().find(|event| {
        event.calls.as_ref().is_some_and(|call| {
            plan.loader()
                .is_some_and(|(loader, _)| &loader.name == call)
        })
    }) {
        let dispatch = if is_cubit {
            format!("first.{}()", cubit_method(first))
        } else {
            format!("first.add({}::{})", event_name(plan), first.name)
        };
        let _ = write!(out, "    // Once, when the page opens: a state initializer runs on the first render only.\n    let first = bloc.clone();\n    cx.use_state(move || {dispatch});\n");
    }
    out.push_str(&form_fields_ui);
    let mut actions = Vec::new();
    for event in &no_field_events {
        let id = names::snake(&event.name).replace('_', "-");
        let dispatch = if is_cubit {
            format!("handler.{}()", cubit_method(event))
        } else {
            format!("handler.add({}::{})", event_name(plan), event.name)
        };
        actions.push(format!(
            ".child({{ let handler = bloc.clone(); Button::new({id:?}).outline().label({:?}).on_click(move |_, _, _| {{ {dispatch}; }}) }})",
            action_label(plan, event)
        ));
    }
    let list = plan.state.fields.iter().find_map(|field| {
        let inner = super::resolve::vec_inner(&field.ty)?;
        Some((field.name.clone(), plan.model(&inner).cloned()))
    });
    let item_text = |binding: &str| -> String {
        let model = list.as_ref().and_then(|(_, model)| model.as_ref());
        let text_field = model.and_then(|model| {
            model
                .fields
                .iter()
                .find(|field| field.ty == "String" && field.name != "id")
                .map(|field| field.name.clone())
        });
        match text_field {
            Some(field) => format!("{binding}.{field}.clone()"),
            None => format!("format!(\"{{{binding}:?}}\")"),
        }
    };
    // Events that act on one item (`NoteDeleted { id }`) become a button on each row.
    let list_model = list.as_ref().and_then(|(_, model)| model.clone());
    let row_events: Vec<&Event> = list_model
        .as_ref()
        .and_then(|model| model.id.clone())
        .map(|id| {
            plan.events
                .iter()
                .filter(|event| event.fields.len() == 1 && event.fields[0].ty == id)
                .collect()
        })
        .unwrap_or_default();
    let copy_id = list_model
        .as_ref()
        .and_then(|model| model.id.as_ref())
        .is_some_and(|id| is_copy(&id_alias_type(plan, id)));
    let items = list.as_ref().map(|(field, _)| {
        let source = match plan.state.style {
            StateStyle::Struct => format!("state.{field}"),
            StateStyle::Enum => format!("state.data().{field}"),
        };
        if row_events.is_empty() {
            return format!(
                "div().flex().flex_col().gap_1().children({source}.iter().map(|item| div().child({})))",
                item_text("item")
            );
        }
        let mut buttons = String::new();
        for event in &row_events {
            let field = &event.fields[0].name;
            let value = if copy_id { "item.id".to_string() } else { "item.id.clone()".to_string() };
            let dispatch = if is_cubit {
                format!("handler.{}({field})", cubit_method(event))
            } else {
                format!("handler.add({}::{} {{ {field} }})", event_name(plan), event.name)
            };
            let id = names::snake(&event.name).replace('_', "-");
            let _ = write!(
                buttons,
                ".child({{ let handler = bloc.clone(); let {field} = {value}; Button::new(({id:?}, index)).ghost().label({:?}).on_click(move |_, _, _| {{ {dispatch}; }}) }})",
                sentence(&names::event_verb(&event.name))
            );
        }
        format!(
            "div().flex().flex_col().gap_1().children({source}.iter().enumerate().map(|(index, item)| div().flex().gap_2().items_center().child({}){buttons}))",
            item_text("item")
        )
    });
    let items = items.unwrap_or_else(|| "div()".into());
    let body = match plan.state.style {
        StateStyle::Struct => {
            let mut arms = String::new();
            if plan.state.has_status("Loading") {
                let _ = writeln!(arms, "{status}::Loading => div().child(\"Loading...\"),");
            }
            if plan.state.has_status("Failure") {
                let message = if state_has_error_field(plan) {
                    "state.error.as_ref().map_or_else(|| \"Something went wrong.\".to_string(), ToString::to_string)"
                } else {
                    "\"Something went wrong.\".to_string()"
                };
                let _ = writeln!(arms, "{status}::Failure => div().child({message}),");
            }
            if arms.is_empty() {
                items
            } else {
                format!("match state.status {{\n{arms}_ => {items},\n}}")
            }
        }
        StateStyle::Enum => {
            let mut arms = String::new();
            if plan.state.has_status("Loading") {
                let _ = writeln!(arms, "{state}::Loading => div().child(\"Loading...\"),");
            }
            let _ = writeln!(
                arms,
                "{state}::Failure(error) => div().child(error.to_string()),"
            );
            let _ = writeln!(arms, "_ => {items},");
            format!("match state {{\n{arms}}}")
        }
    };
    let editor = if form.is_some() { ".child(editor)" } else { "" };
    let builder = if row_events.is_empty() {
        format!("|state, _, _| {{ {body} }}")
    } else {
        format!("{{ let bloc = bloc.clone(); move |state, _, _| {{ {body} }} }}")
    };
    let _ = write!(
        out,
        "    div()\n        .flex()\n        .flex_col()\n        .gap_4()\n        .p_6()\n        .child(H2::new({:?}))\n        .child(div().flex().gap_2(){})\n        {editor}\n        .child(BlocBuilder::new(&bloc, {builder}))\n}}\n",
        sentence(&plan.feature.pascal),
        actions.join("")
    );
    out
}

/// `src/routes/<feature>.rs`: a thin route rendering the page.
pub fn route(plan: &Plan) -> String {
    let page = page_name(plan);
    format!(
        "use rok_ui::prelude::*;\nuse rok_ui::router::file_route;\n\nfile_route! {{\n    component: {0}Route,\n}}\n\n/// `/{1}`: the {1} page. Routes stay thin; the page lives in `features::{1}::view`.\n#[component]\nfn {0}Route() -> impl IntoElement {{\n    crate::features::{1}::view::{2}::{page}::new()\n}}\n",
        plan.feature.pascal,
        plan.feature.snake,
        names::snake(&page)
    )
}

// ---------------------------------------------------------------------------
// Tests.

/// What a generated test asserts about the states `event` produced, with a fake repository
/// that always succeeds.
fn test_check(plan: &Plan, event: &Event) -> String {
    let state = state_name(plan);
    let status = status_name(plan);
    let calls = event.calls.is_some();
    match plan.state.style {
        StateStyle::Struct if calls && plan.state.has_status("Success") => {
            format!("assert_eq!(states.last().map(|state| state.status), Some({status}::Success));")
        }
        StateStyle::Struct if plan.state.has_status("Failure") => {
            format!("assert!(states.iter().all(|state| state.status != {status}::Failure));")
        }
        StateStyle::Struct if state_has_error_field(plan) => {
            "assert!(states.iter().all(|state| state.error.is_none()));".into()
        }
        StateStyle::Struct => "let _ = states;".into(),
        StateStyle::Enum if calls => {
            format!("assert!(matches!(states.last(), Some({state}::Success(_))));")
        }
        StateStyle::Enum => {
            format!("assert!(states.iter().all(|state| !matches!(state, {state}::Failure(_))));")
        }
    }
}

/// `tests/features/<f>/<name>_test.rs`.
pub fn test(plan: &Plan, crate_name: &str) -> String {
    let repository = plan.repository.as_ref().expect("a repository");
    let error = plan.error_type();
    let mut imports = Imports::default();
    imports.add("std::sync::Arc");
    imports.add("rok_ui::bloc::BoxFuture");
    imports.add(&format!(
        "{crate_name}::data::repositories::{}::{}",
        repository.names.snake, repository.names.pascal
    ));
    let mut types: Vec<&str> = vec![error];
    for method in &repository.methods {
        types.push(&method.returns);
        types.extend(method.args.iter().map(|arg| arg.ty.as_str()));
    }
    let event_types: Vec<&str> = plan
        .events
        .iter()
        .flat_map(|event| event.fields.iter().map(|field| field.ty.as_str()))
        .collect();
    types.extend(event_types);
    import_types(plan, &mut imports, &types, crate_name);
    let bloc_module = format!("{crate_name}::features::{}::bloc", plan.feature.snake);
    let state = state_name(plan);
    let status = status_name(plan);
    let is_cubit = plan.kind == Kind::Cubit;
    if is_cubit {
        imports.add(&format!(
            "{bloc_module}::{}_cubit::{}Cubit",
            plan.name.snake, plan.name.pascal
        ));
        imports.add("rok_ui::bloc::test");
    } else {
        imports.add("rok_ui::bloc::test");
        imports.add(&format!(
            "{bloc_module}::{}_bloc::{}Bloc",
            plan.name.snake, plan.name.pascal
        ));
        imports.add(&format!(
            "{bloc_module}::{}_event::{}",
            plan.name.snake,
            event_name(plan)
        ));
    }
    let checks: Vec<String> = plan
        .events
        .iter()
        .map(|event| test_check(plan, event))
        .collect();
    if checks
        .iter()
        .any(|check| check.contains(&format!("{status}::")))
    {
        imports.add(&format!(
            "{bloc_module}::{}_state::{status}",
            plan.name.snake
        ));
    }
    if checks
        .iter()
        .any(|check| check.contains(&format!("{state}::")))
    {
        imports.add(&format!(
            "{bloc_module}::{}_state::{state}",
            plan.name.snake
        ));
    }
    let fake = format!("Fake{}", repository.names.pascal);
    let subject = if is_cubit {
        format!("{}Cubit", plan.name.pascal)
    } else {
        format!("{}Bloc", plan.name.pascal)
    };
    let mut out = header(&format!(
        "`{subject}`: events in, states out, with a fake repository."
    ));
    out.push_str(&imports.render());
    let _ = write!(out, "\n/// Answers every call with an empty success.\nstruct {fake};\n\nimpl {} for {fake} {{\n", repository.names.pascal);
    for method in &repository.methods {
        let args: Vec<String> = method
            .args
            .iter()
            .map(|arg| format!("_{}: {}", arg.name, rust_type(&arg.ty)))
            .collect();
        let comma = if args.is_empty() { "" } else { ", " };
        let _ = write!(
            out,
            "    fn {}(&self{comma}{}) -> BoxFuture<'_, Result<{}, {error}>> {{\n        Box::pin(async {{ Ok({}) }})\n    }}\n\n",
            method.name,
            args.join(", "),
            rust_type(&method.returns),
            default_of(&method.returns)
        );
    }
    out.push_str("}\n");
    for event in &plan.events {
        let test_name = names::snake(&event.name);
        let value = if event.fields.is_empty() {
            format!("{}::{}", event_name(plan), event.name)
        } else {
            let fields: Vec<String> = event
                .fields
                .iter()
                .map(|field| format!("{}: {}", field.name, default_of(&field.ty)))
                .collect();
            format!(
                "{}::{} {{ {} }}",
                event_name(plan),
                event.name,
                fields.join(", ")
            )
        };
        let check = test_check(plan, event);
        if is_cubit {
            let args: Vec<String> = event
                .fields
                .iter()
                .map(|field| default_of(&field.ty))
                .collect();
            let act = if args.is_empty() {
                format!("{subject}::{}", cubit_method(event))
            } else {
                format!(
                    "|cubit| {{ cubit.{}({}); }}",
                    cubit_method(event),
                    args.join(", ")
                )
            };
            let _ = write!(
                out,
                "\n#[test]\nfn {test_name}() {{\n    let states = test::run_cubit({subject}::new(Arc::new({fake})), {act});\n    {check}\n}}\n"
            );
        } else {
            let _ = write!(
                out,
                "\n#[test]\nfn {test_name}() {{\n    let states = test::run({subject}::new(Arc::new({fake})), [{value}]);\n    {check}\n}}\n"
            );
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Wiring.

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
