//! Checks on a spec: known types, collected errors, call arguments and HTTP endpoints.

use std::collections::BTreeSet;

use super::{Event, Method, Model, Provider};
use crate::generate::{names, spec::FieldSpec};

/// Types that are not models.
pub(super) const KNOWN: &[&str] = &[
    "String",
    "str",
    "Vec",
    "Option",
    "Result",
    "Box",
    "Arc",
    "Rc",
    "HashMap",
    "BTreeMap",
    "HashSet",
    "BTreeSet",
    "bool",
    "char",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "f32",
    "f64",
    "Bytes",
    "SharedString",
    "ApiError",
    "DataError",
    "Duration",
    "Self",
    "bytes",
];

/// The type names `ty` mentions.
fn type_idents(ty: &syn::Type, found: &mut Vec<String>) {
    match ty {
        syn::Type::Path(path) => {
            if path.qself.is_none() && path.path.segments.len() == 1 {
                let segment = &path.path.segments[0];
                found.push(segment.ident.to_string());
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        if let syn::GenericArgument::Type(inner) = argument {
                            type_idents(inner, found);
                        }
                    }
                }
            }
        }
        syn::Type::Reference(reference) => type_idents(&reference.elem, found),
        syn::Type::Slice(slice) => type_idents(&slice.elem, found),
        syn::Type::Array(array) => type_idents(&array.elem, found),
        syn::Type::Paren(paren) => type_idents(&paren.elem, found),
        syn::Type::Group(group) => type_idents(&group.elem, found),
        syn::Type::Tuple(tuple) => {
            for element in &tuple.elems {
                type_idents(element, found);
            }
        }
        _ => {}
    }
}

/// Collects errors, each with its JSON path.
#[derive(Default)]
pub(super) struct Errors(pub(super) Vec<String>);

impl Errors {
    pub(super) fn push(&mut self, path: &str, message: impl std::fmt::Display) {
        let message = format!("{path}: {message}");
        if !self.0.contains(&message) {
            self.0.push(message);
        }
    }

    pub(super) fn check_type(&mut self, path: &str, ty: &str) -> Vec<String> {
        if ty == "bytes" || ty == "()" {
            return Vec::new();
        }
        match syn::parse_str::<syn::Type>(ty) {
            Ok(parsed) => {
                let mut found = Vec::new();
                type_idents(&parsed, &mut found);
                found
            }
            Err(error) => {
                self.push(path, format!("`{ty}` is not a Rust type ({error})"));
                Vec::new()
            }
        }
    }

    pub(super) fn check_field(&mut self, path: &str, field: &FieldSpec) -> Vec<String> {
        if !names::is_identifier(&field.name) {
            self.push(
                &format!("{path}.name"),
                format!("`{}` is not a valid field name", field.name),
            );
        }
        if let Some(default) = &field.default {
            if syn::parse_str::<syn::Expr>(default).is_err() {
                self.push(
                    &format!("{path}.default"),
                    format!("`{default}` is not a Rust expression"),
                );
            }
        }
        self.check_type(&format!("{path}.type"), &field.ty)
    }
}

pub(super) fn check_args(errors: &mut Errors, index: usize, event: &Event, method: &Method) {
    for arg in &method.args {
        match event.fields.iter().find(|field| field.name == arg.name) {
            None => errors.push(
                &format!("events[{index}].fields"),
                format!(
                    "`{}` calls `{}`, which takes `{}: {}`; add that field to the event",
                    event.name, method.name, arg.name, arg.ty
                ),
            ),
            Some(field) if field.ty.replace(' ', "") != arg.ty.replace(' ', "") => errors.push(
                &format!("events[{index}].fields"),
                format!(
                    "`{}` has `{}: {}`, but `{}` takes `{}: {}`",
                    event.name, field.name, field.ty, method.name, arg.name, arg.ty
                ),
            ),
            Some(_) => {}
        }
    }
}

/// The type names `ty` mentions (nothing when it does not parse; that is reported elsewhere).
pub(super) fn idents_of(ty: &str) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(parsed) = syn::parse_str::<syn::Type>(ty) {
        type_idents(&parsed, &mut found);
    }
    found
}

/// Whether `ty` is a primitive Rust type.
pub(super) fn is_primitive(ty: &str) -> bool {
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
            | "String"
    )
}

/// HTTP endpoint rules.
pub(super) fn check_endpoints(provider: &Provider, dtos: &[Model], errors: &mut Errors) {
    let mut seen = BTreeSet::new();
    for (index, endpoint) in provider.endpoints.iter().enumerate() {
        let path = format!("provider.endpoints[{index}]");
        if !names::is_identifier(&endpoint.method)
            || names::snake(&endpoint.method) != endpoint.method
        {
            errors.push(
                &format!("{path}.method"),
                format!("`{}` is not a snake_case method name", endpoint.method),
            );
        }
        if !seen.insert(endpoint.method.clone()) {
            errors.push(
                &format!("{path}.method"),
                format!("`{}` appears twice", endpoint.method),
            );
        }
        if !matches!(
            endpoint.verb.as_str(),
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE"
        ) {
            errors.push(
                &format!("{path}.verb"),
                format!("`{}` is not GET, POST, PUT, PATCH or DELETE", endpoint.verb),
            );
        }
        if !endpoint.path.starts_with('/') {
            errors.push(&format!("{path}.path"), "must start with `/`");
        }
        let placeholders: Vec<&str> = endpoint
            .path
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect();
        for placeholder in &placeholders {
            if !endpoint
                .path_params
                .iter()
                .any(|param| param.name == *placeholder)
            {
                errors.push(
                    &format!("{path}.path"),
                    format!("`{{{placeholder}}}` has no entry in `path_params`"),
                );
            }
        }
        for (param_index, param) in endpoint.path_params.iter().enumerate() {
            if !placeholders.contains(&param.name.as_str()) {
                errors.push(
                    &format!("{path}.path_params[{param_index}].name"),
                    format!(
                        "`{}` does not appear in the path as `{{{}}}`",
                        param.name, param.name
                    ),
                );
            }
            errors.check_field(&format!("{path}.path_params[{param_index}]"), param);
        }
        for (query_index, query) in endpoint.query.iter().enumerate() {
            errors.check_field(&format!("{path}.query[{query_index}]"), query);
        }
        if endpoint.raw_body && endpoint.body.is_some() {
            errors.push(
                &format!("{path}.raw_body"),
                "an endpoint takes `body` or `raw_body`, not both",
            );
        }
        if let Some(body) = &endpoint.body {
            errors.check_type(&format!("{path}.body"), body);
            if body.ends_with("Dto") && !dtos.iter().any(|dto| &dto.names.pascal == body) {
                errors.push(
                    &format!("{path}.body"),
                    format!("there is no DTO `{body}` in `dtos`"),
                );
            }
        }
        errors.check_type(&format!("{path}.response"), &endpoint.response);
        let mut response_types = Vec::new();
        if let Ok(parsed) = syn::parse_str::<syn::Type>(&endpoint.response) {
            type_idents(&parsed, &mut response_types);
        }
        for ident in response_types {
            if ident.ends_with("Dto") && !dtos.iter().any(|dto| dto.names.pascal == ident) {
                errors.push(
                    &format!("{path}.response"),
                    format!("there is no DTO `{ident}` in `dtos`"),
                );
            }
        }
    }
}
