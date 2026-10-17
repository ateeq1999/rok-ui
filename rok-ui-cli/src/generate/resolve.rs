//! Turn a [`Spec`] into a [`Plan`]: defaults filled in, names normalized, types checked, and
//! every reference resolved. Errors name the JSON path of the value at fault.

use std::collections::BTreeSet;

use super::{
    names,
    spec::{ProviderKind, SessionAction, Spec, StateStyle},
};

mod checks;
mod infer;
mod plan;

use checks::{check_args, check_endpoints, idents_of, is_primitive, Errors, KNOWN};
use infer::{
    default_method, endpoint_methods, field, infer_call, inferred_model, methods, singular,
    type_spec,
};
pub use plan::{vec_inner, Event, Field, Method, Model, Names, Plan, Provider, Repository, State};

/// Resolve `spec` against the defaults.
///
/// # Errors
///
/// Every problem found, each prefixed with its JSON path.
#[allow(clippy::too_many_lines)]
pub fn resolve(spec: &Spec) -> Result<Plan, Vec<String>> {
    let mut errors = Errors::default();
    let Some(feature_name) = spec.feature.as_deref() else {
        return Err(vec![
            "feature: missing (the feature module, like `notes`)".into()
        ]);
    };
    let feature_base = names::base(feature_name, &["feature"]);
    if let Err(message) = names::check(&feature_base, "feature") {
        errors.push("feature", message);
    }
    let feature = Names::new(&feature_base);
    let kind = spec.kind.unwrap_or_default();
    let name_source = spec.name.clone().unwrap_or_else(|| feature.pascal.clone());
    let name_base = names::base(&name_source, &["bloc", "cubit"]);
    if let Err(message) = names::check(&name_base, "bloc") {
        // A name taken from the feature is already reported there.
        if spec.name.is_some() {
            errors.push("name", message);
        }
    }
    let name = Names::new(&name_base);
    let model_name = names::pascal(&singular(&feature.snake));

    // Events.
    let mut events = Vec::new();
    let mut seen_events = BTreeSet::new();
    for (index, event) in spec.events.iter().enumerate() {
        let path = format!("events[{index}]");
        let pascal = names::pascal(&event.name);
        if !names::is_identifier(&event.name) || pascal != event.name {
            errors.push(
                &format!("{path}.name"),
                format!(
                    "`{}` is not a PascalCase event name (like `NoteAdded`)",
                    event.name
                ),
            );
        }
        if !seen_events.insert(pascal.clone()) {
            errors.push(&format!("{path}.name"), format!("`{pascal}` appears twice"));
        }
        for (field_index, spec_field) in event.fields.iter().enumerate() {
            errors.check_field(&format!("{path}.fields[{field_index}]"), spec_field);
        }
        events.push(Event {
            name: pascal,
            fields: event.fields.iter().map(field).collect(),
            calls: event.calls.clone(),
            concurrency: event.concurrency,
        });
    }
    if events.is_empty() {
        events.push(Event {
            name: format!("{}Requested", feature.pascal),
            fields: Vec::new(),
            calls: None,
            concurrency: None,
        });
    }

    // State.
    let state_spec = spec.state.clone().unwrap_or_default();
    let list_type = format!("Vec<{model_name}>");
    let mut state_fields: Vec<Field> = state_spec.fields.iter().map(field).collect();
    for (index, spec_field) in state_spec.fields.iter().enumerate() {
        errors.check_field(&format!("state.fields[{index}]"), spec_field);
    }
    if state_spec.fields.is_empty() {
        state_fields.push(Field {
            name: feature.snake.clone(),
            ty: list_type.clone(),
            default: None,
            rename: None,
        });
    }
    let status: Vec<String> = if state_spec.status.is_empty() {
        ["initial", "loading", "success", "failure"]
            .iter()
            .map(|status| names::pascal(status))
            .collect()
    } else {
        state_spec
            .status
            .iter()
            .enumerate()
            .filter_map(|(index, status)| {
                if names::is_identifier(&names::snake(status)) {
                    Some(names::pascal(status))
                } else {
                    errors.push(
                        &format!("state.status[{index}]"),
                        format!("`{status}` is not a valid name"),
                    );
                    None
                }
            })
            .collect()
    };

    // Provider.
    let provider = spec.provider.as_ref().map(|provider| {
        let kind = provider.kind.unwrap_or_default();
        if let Err(message) = names::check(&provider.name, "provider") {
            errors.push("provider.name", message);
        }
        Provider {
            names: Names::new(&provider.name),
            kind,
            api_base: provider.api_base.clone().unwrap_or_default(),
            endpoints: provider.endpoints.clone(),
        }
    });
    let provider = provider.or_else(|| {
        Some(Provider {
            names: Names::new(&format!("{}MemoryProvider", feature.pascal)),
            kind: ProviderKind::Memory,
            api_base: String::new(),
            endpoints: Vec::new(),
        })
    });
    let is_http = provider
        .as_ref()
        .is_some_and(|provider| provider.kind == ProviderKind::Http);
    let error = state_spec
        .error
        .clone()
        .unwrap_or_else(|| if is_http { "ApiError" } else { "DataError" }.to_string());
    if !matches!(error.as_str(), "ApiError" | "DataError") {
        errors.check_type("state.error", &error);
    }

    // DTOs.
    let mut dtos = Vec::new();
    for (index, dto) in spec.dtos.iter().enumerate() {
        dtos.push(type_spec(&format!("dtos[{index}]"), dto, &mut errors));
    }

    // Repository.
    let repository = spec.repository.as_ref().map_or_else(
        || {
            let mut methods: Vec<Method> = Vec::new();
            if let Some(provider) = spec.provider.as_ref().filter(|_| is_http) {
                methods = endpoint_methods(&provider.endpoints, &dtos);
            }
            for event in events.iter().filter(|_| !is_http) {
                if event.calls.is_some() {
                    continue;
                }
                let method = default_method(event, &model_name, &list_type);
                if !methods.iter().any(|existing| existing.name == method.name) {
                    methods.push(method);
                }
            }
            Repository {
                names: Names::new(&format!("{}Repository", feature.pascal)),
                methods,
            }
        },
        |repository| {
            if let Err(message) = names::check(&repository.name, "repository") {
                errors.push("repository.name", message);
            }
            Repository {
                names: Names::new(&repository.name),
                methods: methods(&repository.methods, &mut errors),
            }
        },
    );

    // Resolve which method each event calls.
    let loader = repository
        .methods
        .iter()
        .find(|method| {
            method.args.is_empty() && state_fields.iter().any(|field| field.ty == method.returns)
        })
        .map(|method| method.name.clone());
    for (index, event) in events.iter_mut().enumerate() {
        if let Some(call) = &event.calls {
            let Some(method) = repository
                .methods
                .iter()
                .find(|method| &method.name == call)
            else {
                errors.push(
                    &format!("events[{index}].calls"),
                    format!("there is no repository method `{call}`"),
                );
                continue;
            };
            check_args(&mut errors, index, event, method);
        } else {
            event.calls = infer_call(event, &repository.methods, loader.as_deref());
            if let Some(method) = event.calls.as_ref().and_then(|call| {
                repository
                    .methods
                    .iter()
                    .find(|method| &method.name == call)
            }) {
                if method
                    .args
                    .iter()
                    .any(|arg| !event.fields.iter().any(|field| field.name == arg.name))
                {
                    // The inferred method needs fields the event lacks: call nothing.
                    event.calls = None;
                }
            }
        }
    }

    // Session actions need an HTTP provider, and sign-in a response with the token.
    for (index, method) in repository.methods.iter().enumerate() {
        let Some((action, token_field)) = &method.session else {
            continue;
        };
        let path = format!("repository.methods[{index}].session");
        if !is_http {
            errors.push(&path, "session actions need an `http` provider");
            continue;
        }
        if *action == SessionAction::SignIn {
            let call = method.calls.clone().unwrap_or_else(|| method.name.clone());
            let response = provider
                .as_ref()
                .and_then(|provider| {
                    provider
                        .endpoints
                        .iter()
                        .find(|endpoint| endpoint.method == call)
                })
                .map(|endpoint| endpoint.response.clone());
            let has_token = response.as_ref().is_some_and(|response| {
                dtos.iter().any(|dto| {
                    &dto.names.pascal == response
                        && dto.fields.iter().any(|field| &field.name == token_field)
                })
            });
            if response.is_some() && !has_token {
                errors.push(
                    &path,
                    format!(
                        "`sign_in` needs the endpoint's response DTO to have a `{token_field}` field (or set `token_field`)"
                    ),
                );
            }
        }
    }

    // HTTP endpoints.
    if let Some(provider) = &provider {
        check_endpoints(provider, &dtos, &mut errors);
        if provider.kind == ProviderKind::Http {
            for (index, method) in repository.methods.iter().enumerate() {
                let call = method.calls.clone().unwrap_or_else(|| method.name.clone());
                if !provider
                    .endpoints
                    .iter()
                    .any(|endpoint| endpoint.method == call)
                {
                    let at = if method.calls.is_some() {
                        "calls"
                    } else {
                        "name"
                    };
                    errors.push(
                        &format!("repository.methods[{index}].{at}"),
                        format!("there is no provider endpoint `{call}`"),
                    );
                }
            }
        }
    }

    // Models: listed ones, then any type mentioned that is not known.
    let mut models: Vec<Model> = spec
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| type_spec(&format!("models[{index}]"), model, &mut errors))
        .collect();
    let dto_names: BTreeSet<String> = dtos.iter().map(|dto| dto.names.pascal.clone()).collect();
    let mut mentioned = Vec::new();
    let mut mention = |ty: &str, errors: &mut Errors, path: &str| {
        for ident in errors.check_type(path, ty) {
            if !mentioned.contains(&ident) {
                mentioned.push(ident);
            }
        }
    };
    for (index, field) in state_fields.iter().enumerate() {
        mention(
            &field.ty,
            &mut errors,
            &format!("state.fields[{index}].type"),
        );
    }
    for (index, event) in events.iter().enumerate() {
        for (field_index, field) in event.fields.iter().enumerate() {
            mention(
                &field.ty,
                &mut errors,
                &format!("events[{index}].fields[{field_index}].type"),
            );
        }
    }
    // Inferred methods repeat the events' types, which are already checked: report only the
    // ones the spec wrote.
    let mut inferred_errors = Errors::default();
    for (index, method) in repository.methods.iter().enumerate() {
        let errors = if spec.repository.is_some() {
            &mut errors
        } else {
            &mut inferred_errors
        };
        mention(
            &method.returns,
            errors,
            &format!("repository.methods[{index}].returns"),
        );
        for (arg_index, arg) in method.args.iter().enumerate() {
            mention(
                &arg.ty,
                errors,
                &format!("repository.methods[{index}].args[{arg_index}].type"),
            );
        }
    }
    if let Some(provider) = &provider {
        for (index, endpoint) in provider.endpoints.iter().enumerate() {
            for (param_index, param) in endpoint.path_params.iter().enumerate() {
                mention(
                    &param.ty,
                    &mut errors,
                    &format!("provider.endpoints[{index}].path_params[{param_index}].type"),
                );
            }
            for (query_index, query) in endpoint.query.iter().enumerate() {
                mention(
                    &query.ty,
                    &mut errors,
                    &format!("provider.endpoints[{index}].query[{query_index}].type"),
                );
            }
        }
    }
    if mentioned.is_empty() || !mentioned.contains(&model_name) && state_spec.fields.is_empty() {
        mentioned.push(model_name.clone());
    }
    let mut ids = Vec::new();
    for ident in &mentioned {
        if KNOWN.contains(&ident.as_str()) || dto_names.contains(ident) {
            continue;
        }
        if let Some(owner) = ident.strip_suffix("Id").filter(|owner| !owner.is_empty()) {
            ids.push((ident.clone(), owner.to_string()));
            if !models.iter().any(|model| model.names.pascal == owner)
                && !mentioned.iter().any(|other| other == owner)
            {
                models.push(inferred_model(owner, &events, &repository.methods, &dtos));
            }
            continue;
        }
        if !models.iter().any(|model| model.names.pascal == *ident) {
            models.push(inferred_model(ident, &events, &repository.methods, &dtos));
        }
    }
    // Types that models mention (`Vec<Attachment>` in `Note`) are models too.
    loop {
        let mut missing = Vec::new();
        for model in &models {
            let fields = model.fields.iter().chain(
                model
                    .variants
                    .iter()
                    .flat_map(|variant| variant.fields.iter()),
            );
            for field in fields {
                for ident in idents_of(&field.ty) {
                    let known = KNOWN.contains(&ident.as_str())
                        || dto_names.contains(&ident)
                        || models.iter().any(|model| model.names.pascal == ident)
                        || ids.iter().any(|(alias, _)| *alias == ident)
                        || missing.contains(&ident);
                    if !known {
                        missing.push(ident);
                    }
                }
            }
        }
        if missing.is_empty() {
            break;
        }
        for ident in missing {
            if let Some(owner) = ident.strip_suffix("Id").filter(|owner| !owner.is_empty()) {
                ids.push((ident.clone(), owner.to_string()));
                if !models.iter().any(|model| model.names.pascal == owner) {
                    models.push(inferred_model(owner, &events, &repository.methods, &dtos));
                }
            } else {
                models.push(inferred_model(&ident, &events, &repository.methods, &dtos));
            }
        }
    }
    let ids = ids
        .into_iter()
        .map(|(alias, owner)| {
            let ty = models
                .iter()
                .find(|model| model.names.pascal == owner)
                .and_then(|model| model.id.clone())
                .filter(|ty| ty != &alias)
                .unwrap_or_else(|| "u64".to_string());
            (alias, owner, ty)
        })
        .collect::<Vec<_>>();
    // Ids no field mentions still need their alias (`CounterId` of an inferred `Counter`).
    let mut ids = ids;
    for model in &models {
        let Some(id) = &model.id else { continue };
        let known = KNOWN.contains(&id.as_str())
            || is_primitive(id)
            || models.iter().any(|other| &other.names.pascal == id);
        if !known && !ids.iter().any(|(alias, _, _)| alias == id) {
            ids.push((id.clone(), model.names.pascal.clone(), "u64".into()));
        }
    }

    let style = state_spec.style.unwrap_or_default();
    if style == StateStyle::Enum
        && !(status.iter().any(|s| s == "Success") && status.iter().any(|s| s == "Failure"))
    {
        errors.push(
            "state.status",
            "an `enum` state needs `success` and `failure` statuses",
        );
    }

    // View.
    let view = spec.view.clone().unwrap_or_default();
    if let Some(form) = &view.form {
        if !events.iter().any(|event| event.name == form.event) {
            errors.push(
                "view.form.event",
                format!("there is no event `{}`", form.event),
            );
        }
        for (index, spec_field) in form.fields.iter().enumerate() {
            errors.check_field(&format!("view.form.fields[{index}]"), spec_field);
        }
        // Server errors land on form fields: they must exist.
        let form_fields: Vec<String> = if form.fields.is_empty() {
            events
                .iter()
                .find(|event| event.name == form.event)
                .map(|event| {
                    event
                        .fields
                        .iter()
                        .map(|field| field.name.clone())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            form.fields.iter().map(|field| field.name.clone()).collect()
        };
        if let Some(server_errors) = &form.server_errors {
            let mut named: Vec<(String, &String)> = server_errors
                .field_map
                .iter()
                .map(|(api, field)| (format!("view.form.server_errors.field_map.{api}"), field))
                .collect();
            for (key, field) in [
                ("conflict_field", &server_errors.conflict_field),
                ("bad_request_field", &server_errors.bad_request_field),
                ("unauthorized_field", &server_errors.unauthorized_field),
            ] {
                if let Some(field) = field {
                    named.push((format!("view.form.server_errors.{key}"), field));
                }
            }
            for (path, field) in named {
                if !form_fields.contains(field) {
                    errors.push(&path, format!("the form has no field `{field}`"));
                }
            }
        }
    }

    if !errors.0.is_empty() {
        return Err(errors.0);
    }
    Ok(Plan {
        feature,
        kind,
        name,
        concurrency: spec.concurrency.unwrap_or_default(),
        events,
        state: State {
            style,
            status,
            fields: state_fields,
            error,
        },
        models,
        ids,
        repository: Some(repository),
        provider,
        dtos,
        page: view.page.unwrap_or(true),
        form: view.form,
        tests: spec.tests.unwrap_or(true),
    })
}
