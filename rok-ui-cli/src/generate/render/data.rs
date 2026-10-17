//! The data layer: the error type, models, DTOs and data providers (in memory and HTTP).

use std::fmt::Write as _;

use super::{header, import_types, rust_type, sentence, Imports};
use crate::generate::{
    names,
    resolve::{Event, Field, Model, Plan},
    spec::ProviderKind,
};

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

pub(super) fn struct_fields(out: &mut String, fields: &[Field], public: bool, serde: bool) {
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
pub(super) fn is_copy(ty: &str) -> bool {
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

/// The submit button's label: "Add" for `NoteAdded`, "Sign in" for `SignInSubmitted`.
pub(super) fn submit_label(event: &Event) -> String {
    let verb = names::event_verb(&event.name);
    let subject = names::event_subject(&event.name);
    if verb == "submit" && !subject.is_empty() {
        sentence(&subject)
    } else {
        sentence(&verb)
    }
}

/// The label of a page button for an event without fields: "Refresh" for the loader.
pub(super) fn action_label(plan: &Plan, event: &Event) -> String {
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
pub(super) fn id_alias_type(plan: &Plan, ty: &str) -> String {
    plan.ids
        .iter()
        .find(|(alias, _, _)| alias == ty)
        .map_or_else(|| ty.to_string(), |(_, _, inner)| inner.clone())
}

pub(super) fn dto(out: &mut String, dto: &Model) {
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
fn endpoint_args(endpoint: &crate::generate::spec::EndpointSpec) -> Vec<(String, String)> {
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
    for each in &plan.dtos {
        types.extend(each.fields.iter().map(|field| field.ty.as_str()));
        types.extend(
            each.variants
                .iter()
                .flat_map(|variant| variant.fields.iter().map(|field| field.ty.as_str())),
        );
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
        "\n/// The `{}` endpoints, over the app's [`HttpClient`].\n#[derive(Clone, Debug)]\npub struct {name} {{\n    client: HttpClient,\n}}\n\nimpl {name} {{\n    /// The endpoints, called through `client`.\n    #[must_use]\n    pub fn new(client: HttpClient) -> Self {{\n        Self {{ client }}\n    }}\n\n    /// The client (and through it, the session).\n    #[must_use]\n    pub fn client(&self) -> &HttpClient {{\n        &self.client\n    }}\n",
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
            options.push_str(".raw_body(body).content_type(content_type)");
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
