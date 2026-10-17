//! The repository: what blocs call, over a data provider.

use std::fmt::Write as _;

use super::{default_of, header, import_types, rust_type, sentence, Imports};
use crate::generate::{
    resolve::{Field, Method, Model, Plan},
    spec::SessionAction,
};

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
            // Request bodies are built by name; responses are only converted.
            if let Some(body) = &endpoint.body {
                types.push(body);
            }
        }
        // The `From` impls at the end of the file.
        for each in &plan.dtos {
            if let Some(model) = plan.model(each.names.pascal.trim_end_matches("Dto")) {
                types.push(&each.names.pascal);
                types.push(&model.names.pascal);
            }
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
    // How a response value becomes the method's return value.
    let convert = if response == returns || (response == "bytes" && returns == "Bytes") {
        None
    } else if returns == "()" {
        Some(("()".to_string(), true))
    } else if response.starts_with("Vec<") && returns.starts_with("Vec<") {
        Some((
            "value.into_iter().map(Into::into).collect()".to_string(),
            false,
        ))
    } else if let Some(field) = picked_field(plan, &response, &returns) {
        // An envelope (`{ "token": .., "user": .. }`): take the field with the return type.
        Some((field, false))
    } else if response.ends_with("Dto") {
        Some(("value.into()".to_string(), false))
    } else {
        Some((default_of(&method.returns), true))
    };
    let mapping = match &convert {
        None => String::new(),
        Some((value, true)) => format!(".map(|_| {value})"),
        Some((value, false)) if value == "value.into()" => ".map(Into::into)".into(),
        Some((value, false)) => format!(".map(|value| {value})"),
    };
    match &method.session {
        Some((SessionAction::SignIn, token)) => {
            let value = match &convert {
                None => "value".to_string(),
                Some((value, _)) => value.clone(),
            };
            format!(
                "let value = {request}?;\nif let Some(session) = self.provider.client().session() {{ session.set_token(value.{token}.clone()); }}\nOk({value})"
            )
        }
        Some((SessionAction::SignOut, _)) => format!(
            "let result = {request}{mapping};\n// Signed out locally even when the server call fails.\nif let Some(session) = self.provider.client().session() {{ session.clear(); }}\nresult"
        ),
        None => format!("{request}{mapping}"),
    }
}

/// `value.user.into()` when the `response` DTO has one field of the `returns` type (or its
/// DTO).
fn picked_field(plan: &Plan, response: &str, returns: &str) -> Option<String> {
    let dto = plan.dtos.iter().find(|dto| dto.names.pascal == response)?;
    let matching: Vec<&Field> = dto
        .fields
        .iter()
        .filter(|field| {
            let ty = field.ty.replace(' ', "");
            ty == returns || ty == format!("{returns}Dto")
        })
        .collect();
    let [field] = matching[..] else {
        return None;
    };
    let ty = field.ty.replace(' ', "");
    Some(if ty == returns {
        format!("value.{}", field.name)
    } else {
        format!("value.{}.into()", field.name)
    })
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
