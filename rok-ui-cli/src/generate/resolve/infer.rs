//! What a spec leaves out, filled in: fields, default methods and calls, and inferred
//! models.

use super::{Errors, Event, Field, Method, Model, Names};
use crate::generate::{
    names,
    spec::{EndpointSpec, FieldSpec, MethodSpec, TypeSpec},
};

pub(super) fn field(spec: &FieldSpec) -> Field {
    Field {
        name: spec.name.clone(),
        ty: spec.ty.trim().to_string(),
        default: spec.default.clone(),
        rename: spec.rename.clone(),
    }
}

pub(super) fn singular(word: &str) -> String {
    if let Some(stem) = word.strip_suffix("ies") {
        format!("{stem}y")
    } else if word.len() > 1 && word.ends_with('s') && !word.ends_with("ss") {
        word[..word.len() - 1].to_string()
    } else {
        word.to_string()
    }
}

const LIST_VERBS: &[&str] = &["request", "load", "fetch", "refresh", "list", "reload"];
const ADD_VERBS: &[&str] = &["add", "create", "insert", "submit"];
const DELETE_VERBS: &[&str] = &["delete", "remove", "clear"];
const UPDATE_VERBS: &[&str] = &["update", "save", "rename", "edit", "change", "toggle"];

/// The repository method a default repository gets for an event.
pub(super) fn default_method(event: &Event, model: &str, list_type: &str) -> Method {
    let verb = names::event_verb(&event.name);
    let args = event.fields.clone();
    let (name, returns) = if LIST_VERBS.contains(&verb.as_str()) && args.is_empty() {
        ("list".to_string(), list_type.to_string())
    } else if ADD_VERBS.contains(&verb.as_str()) {
        ("add".to_string(), model.to_string())
    } else if DELETE_VERBS.contains(&verb.as_str()) {
        ("delete".to_string(), "()".to_string())
    } else if UPDATE_VERBS.contains(&verb.as_str()) {
        ("update".to_string(), model.to_string())
    } else {
        (verb, "()".to_string())
    };
    Method {
        name,
        args,
        returns,
        calls: None,
        session: None,
    }
}

/// A repository method per HTTP endpoint: its parameters and body fields as arguments, its
/// response with DTOs mapped to models.
pub(super) fn endpoint_methods(endpoints: &[EndpointSpec], dtos: &[Model]) -> Vec<Method> {
    endpoints
        .iter()
        .map(|endpoint| {
            let mut args: Vec<Field> = endpoint
                .path_params
                .iter()
                .chain(&endpoint.query)
                .map(field)
                .collect();
            if let Some(body) = &endpoint.body {
                if let Some(dto) = dtos.iter().find(|dto| &dto.names.pascal == body) {
                    for dto_field in &dto.fields {
                        if !args.iter().any(|arg| arg.name == dto_field.name) {
                            args.push(Field {
                                name: dto_field.name.clone(),
                                ty: dto_field.ty.replace("Dto", ""),
                                default: None,
                                rename: None,
                            });
                        }
                    }
                }
            }
            if endpoint.raw_body {
                args.push(Field {
                    name: "body".into(),
                    ty: "Vec<u8>".into(),
                    default: None,
                    rename: None,
                });
                args.push(Field {
                    name: "content_type".into(),
                    ty: "String".into(),
                    default: None,
                    rename: None,
                });
            }
            let returns = match endpoint.response.as_str() {
                "bytes" => "Bytes".to_string(),
                other => other.replace("Dto", ""),
            };
            Method {
                name: endpoint.method.clone(),
                args,
                returns,
                calls: None,
                session: None,
            }
        })
        .collect()
}

/// The method an event's handler calls when the payload does not say.
pub(super) fn infer_call(
    event: &Event,
    methods: &[Method],
    loader: Option<&str>,
) -> Option<String> {
    let verb = names::event_verb(&event.name);
    let exists = |name: &str| methods.iter().any(|method| method.name == name);
    if LIST_VERBS.contains(&verb.as_str()) && event.fields.is_empty() {
        if let Some(loader) = loader {
            return Some(loader.to_string());
        }
    }
    let candidates: &[&str] = if ADD_VERBS.contains(&verb.as_str()) {
        &["add", "create", "insert"]
    } else if DELETE_VERBS.contains(&verb.as_str()) {
        &["delete", "remove"]
    } else if UPDATE_VERBS.contains(&verb.as_str()) {
        &["update", "save", "rename"]
    } else {
        &[]
    };
    candidates
        .iter()
        .find(|name| exists(name))
        .map(|name| (*name).to_string())
        .or_else(|| exists(&verb).then_some(verb))
}

pub(super) fn methods(specs: &[MethodSpec], errors: &mut Errors) -> Vec<Method> {
    let mut methods: Vec<Method> = Vec::new();
    for (index, method) in specs.iter().enumerate() {
        let path = format!("repository.methods[{index}]");
        if !names::is_identifier(&method.name) || names::snake(&method.name) != method.name {
            errors.push(
                &format!("{path}.name"),
                format!("`{}` is not a snake_case method name", method.name),
            );
        }
        if methods.iter().any(|existing| existing.name == method.name) {
            errors.push(
                &format!("{path}.name"),
                format!("`{}` appears twice", method.name),
            );
        }
        for (arg_index, arg) in method.args.iter().enumerate() {
            errors.check_field(&format!("{path}.args[{arg_index}]"), arg);
        }
        let returns = method.returns.clone().unwrap_or_else(|| "()".into());
        errors.check_type(&format!("{path}.returns"), &returns);
        methods.push(Method {
            name: method.name.clone(),
            args: method.args.iter().map(field).collect(),
            returns,
            calls: method.calls.clone(),
            session: method.session.map(|action| {
                (
                    action,
                    method.token_field.clone().unwrap_or_else(|| "token".into()),
                )
            }),
        });
    }
    methods
}

pub(super) fn type_spec(path: &str, spec: &TypeSpec, errors: &mut Errors) -> Model {
    if names::pascal(&spec.name) != spec.name || !names::is_identifier(&spec.name) {
        errors.push(
            &format!("{path}.name"),
            format!("`{}` is not a PascalCase type name", spec.name),
        );
    }
    for (index, spec_field) in spec.fields.iter().enumerate() {
        errors.check_field(&format!("{path}.fields[{index}]"), spec_field);
    }
    let fields: Vec<Field> = spec.fields.iter().map(field).collect();
    let id = fields
        .iter()
        .find(|field| field.name == "id")
        .map(|field| field.ty.clone());
    let variants = spec
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| type_spec(&format!("{path}.variants[{index}]"), variant, errors))
        .collect();
    Model {
        names: Names::new(&spec.name),
        fields,
        id,
        variants,
    }
}

/// A model nobody described: its DTO's fields, else an id plus the fields events carry.
pub(super) fn inferred_model(
    name: &str,
    events: &[Event],
    methods: &[Method],
    dtos: &[Model],
) -> Model {
    if let Some(dto) = dtos
        .iter()
        .find(|dto| dto.names.pascal == format!("{name}Dto"))
    {
        let fields: Vec<Field> = dto
            .fields
            .iter()
            .map(|field| Field {
                name: field.name.clone(),
                ty: field.ty.replace("Dto", ""),
                default: None,
                rename: None,
            })
            .collect();
        let id = fields
            .iter()
            .find(|field| field.name == "id")
            .map(|field| field.ty.clone());
        let variants = dto
            .variants
            .iter()
            .map(|variant| Model {
                names: variant.names.clone(),
                fields: variant
                    .fields
                    .iter()
                    .map(|field| Field {
                        name: field.name.clone(),
                        ty: field.ty.replace("Dto", ""),
                        default: None,
                        rename: None,
                    })
                    .collect(),
                id: None,
                variants: Vec::new(),
            })
            .collect();
        return Model {
            names: Names::new(name),
            fields,
            id,
            variants,
        };
    }
    let id_type = format!("{name}Id");
    let mut fields = vec![Field {
        name: "id".into(),
        ty: id_type.clone(),
        default: None,
        rename: None,
    }];
    let candidates = events
        .iter()
        .flat_map(|event| event.fields.iter())
        .chain(methods.iter().flat_map(|method| method.args.iter()));
    for candidate in candidates {
        let simple = matches!(
            candidate.ty.as_str(),
            "String" | "bool" | "u32" | "u64" | "i32" | "i64" | "f32" | "f64" | "usize"
        );
        if simple
            && candidate.name != "id"
            && !fields.iter().any(|field| field.name == candidate.name)
        {
            fields.push(Field {
                name: candidate.name.clone(),
                ty: candidate.ty.clone(),
                default: None,
                rename: None,
            });
        }
    }
    Model {
        names: Names::new(name),
        fields,
        id: Some(id_type),
        variants: Vec::new(),
    }
}
