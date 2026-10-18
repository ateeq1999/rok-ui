//! Reading a spec from JSON and from command-line flags, which win over the JSON.

use std::{fs, io::Read as _};

use super::names;
use super::spec::{
    ConcurrencyKind, EndpointSpec, EventSpec, FieldSpec, Kind, ProviderKind, ProviderSpec,
    RepositorySpec, ServerErrorsSpec, Spec, StateSpec, StateStyle, TypeSpec, ViewSpec,
};
use crate::Arguments;

/// `-j`: JSON inline, `@file`, or `-` for standard input.
pub(super) fn parse_json(source: &str) -> Result<Spec, String> {
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
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
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
pub(super) fn parse_endpoint(value: &str, skip_expire: bool) -> Result<EndpointSpec, String> {
    // Split at colons outside `{..}` and `<..>`; the response type keeps any `::`.
    let parts = split_top(value, ':');
    let [method, verb, path, response @ ..] = &parts[..] else {
        return Err(format!(
            "--endpoint {value}: use `method:VERB:/path:ResponseType`"
        ));
    };
    if response.is_empty() {
        return Err(format!(
            "--endpoint {value}: use `method:VERB:/path:ResponseType`"
        ));
    }
    // `{id}` is a `String` parameter; `{id:NoteId}` names its type.
    let mut path_params = Vec::new();
    let mut clean_path = Vec::new();
    for segment in path.split('/') {
        match segment
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
        {
            Some(param) => {
                let (name, ty) = param.split_once(':').unwrap_or((param, "String"));
                path_params.push(FieldSpec {
                    name: name.trim().to_string(),
                    ty: ty.trim().to_string(),
                    ..FieldSpec::default()
                });
                clean_path.push(format!("{{{}}}", name.trim()));
            }
            None => clean_path.push(segment.to_string()),
        }
    }
    Ok(EndpointSpec {
        method: method.clone(),
        verb: verb.to_uppercase(),
        path: clean_path.join("/"),
        path_params,
        response: response.join("::"),
        skip_expire,
        ..EndpointSpec::default()
    })
}

/// `--dto NoteDto:id:NoteId,title:String`.
fn parse_dto(value: &str) -> Result<TypeSpec, String> {
    let event = parse_event(value).map_err(|error| error.replace("--event", "--dto"))?;
    Ok(TypeSpec {
        name: event.name,
        fields: event.fields,
        variants: Vec::new(),
    })
}

/// Apply the subcommand's name and the flags over `spec`.
pub(super) fn apply_flags(
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
        spec.concurrency = Some(
            concurrency
                .parse::<ConcurrencyKind>()
                .map_err(|error| format!("--concurrency: {error}"))?,
        );
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
    let dtos = arguments.flags("dto");
    if !dtos.is_empty() {
        spec.dtos = dtos.into_iter().map(parse_dto).collect::<Result<_, _>>()?;
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
