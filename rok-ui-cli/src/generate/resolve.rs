//! Turn a [`Spec`] into a [`Plan`]: defaults filled in, names normalized, types checked, and
//! every reference resolved. Errors name the JSON path of the value at fault.

use std::collections::BTreeSet;

use super::{
    names,
    spec::{
        ConcurrencyKind, EndpointSpec, FieldSpec, FormSpec, Kind, MethodSpec, ProviderKind,
        SessionAction, Spec, StateStyle, TypeSpec,
    },
};

/// A name in its two spellings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    /// `notes_repository`: the file and module.
    pub snake: String,
    /// `NotesRepository`: the type.
    pub pascal: String,
}

impl Names {
    pub fn new(name: &str) -> Self {
        Self {
            snake: names::snake(name),
            pascal: names::pascal(name),
        }
    }
}

/// A typed name.
#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: String,
    pub default: Option<String>,
    pub rename: Option<String>,
}

/// An event and the repository method its handler calls.
#[derive(Clone, Debug)]
pub struct Event {
    pub name: String,
    pub fields: Vec<Field>,
    pub calls: Option<String>,
    pub concurrency: Option<ConcurrencyKind>,
}

/// A repository method.
#[derive(Clone, Debug)]
pub struct Method {
    pub name: String,
    pub args: Vec<Field>,
    pub returns: String,
    pub calls: Option<String>,
    /// What the call does to the HTTP session, and the response's token field.
    pub session: Option<(SessionAction, String)>,
}

/// The repository.
#[derive(Clone, Debug)]
pub struct Repository {
    pub names: Names,
    pub methods: Vec<Method>,
}

/// The provider.
#[derive(Clone, Debug)]
pub struct Provider {
    pub names: Names,
    pub kind: ProviderKind,
    pub api_base: String,
    pub endpoints: Vec<EndpointSpec>,
}

/// A struct with fields (a model or a DTO), or an enum of struct-like variants.
#[derive(Clone, Debug)]
pub struct Model {
    pub names: Names,
    pub fields: Vec<Field>,
    /// The type of its id, when it has an `id` field.
    pub id: Option<String>,
    /// For an enum: its variants.
    pub variants: Vec<Model>,
}

/// The state.
#[derive(Clone, Debug)]
pub struct State {
    pub style: StateStyle,
    /// `PascalCase` status names.
    pub status: Vec<String>,
    pub fields: Vec<Field>,
    pub error: String,
}

impl State {
    pub fn has_status(&self, status: &str) -> bool {
        self.status.iter().any(|candidate| candidate == status)
    }
}

/// Everything the files are written from.
#[derive(Clone, Debug)]
pub struct Plan {
    pub feature: Names,
    pub kind: Kind,
    /// `Notes`: the bloc is `NotesBloc`, the state `NotesState`.
    pub name: Names,
    pub concurrency: ConcurrencyKind,
    pub events: Vec<Event>,
    pub state: State,
    pub models: Vec<Model>,
    /// `*Id` aliases: (alias, the model it belongs to, its type).
    pub ids: Vec<(String, String, String)>,
    pub repository: Option<Repository>,
    pub provider: Option<Provider>,
    pub dtos: Vec<Model>,
    pub page: bool,
    pub form: Option<FormSpec>,
    pub tests: bool,
}

impl Plan {
    /// The error type repositories return.
    pub fn error_type(&self) -> &str {
        &self.state.error
    }

    /// Whether the data layer talks HTTP.
    pub fn is_http(&self) -> bool {
        self.provider
            .as_ref()
            .is_some_and(|provider| provider.kind == ProviderKind::Http)
    }

    /// The model the provider stores and the page lists.
    pub fn primary_model(&self) -> Option<&Model> {
        self.state
            .fields
            .iter()
            .find_map(|field| vec_inner(&field.ty))
            .and_then(|inner| self.models.iter().find(|model| model.names.pascal == inner))
            .or_else(|| self.models.first())
    }

    /// The model or DTO named `name`.
    pub fn model(&self, name: &str) -> Option<&Model> {
        self.models.iter().find(|model| model.names.pascal == name)
    }

    pub fn method(&self, name: &str) -> Option<&Method> {
        self.repository
            .as_ref()
            .and_then(|repository| repository.methods.iter().find(|method| method.name == name))
    }

    /// The method that loads a state field without arguments (to refresh after a write).
    pub fn loader(&self) -> Option<(&Method, &Field)> {
        let repository = self.repository.as_ref()?;
        repository.methods.iter().find_map(|method| {
            if !method.args.is_empty() {
                return None;
            }
            self.state
                .fields
                .iter()
                .find(|field| field.ty == method.returns)
                .map(|field| (method, field))
        })
    }
}

/// `Note` for `Vec<Note>`.
pub fn vec_inner(ty: &str) -> Option<String> {
    let ty = ty.replace(' ', "");
    ty.strip_prefix("Vec<")
        .and_then(|rest| rest.strip_suffix('>'))
        .map(str::to_string)
}

/// Types that are not models.
const KNOWN: &[&str] = &[
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
struct Errors(Vec<String>);

impl Errors {
    fn push(&mut self, path: &str, message: impl std::fmt::Display) {
        let message = format!("{path}: {message}");
        if !self.0.contains(&message) {
            self.0.push(message);
        }
    }

    fn check_type(&mut self, path: &str, ty: &str) -> Vec<String> {
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

    fn check_field(&mut self, path: &str, field: &FieldSpec) -> Vec<String> {
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

fn field(spec: &FieldSpec) -> Field {
    Field {
        name: spec.name.clone(),
        ty: spec.ty.trim().to_string(),
        default: spec.default.clone(),
        rename: spec.rename.clone(),
    }
}

fn singular(word: &str) -> String {
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
fn default_method(event: &Event, model: &str, list_type: &str) -> Method {
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
fn endpoint_methods(endpoints: &[EndpointSpec], dtos: &[Model]) -> Vec<Method> {
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
fn infer_call(event: &Event, methods: &[Method], loader: Option<&str>) -> Option<String> {
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

fn methods(specs: &[MethodSpec], errors: &mut Errors) -> Vec<Method> {
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

fn check_args(errors: &mut Errors, index: usize, event: &Event, method: &Method) {
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

fn type_spec(path: &str, spec: &TypeSpec, errors: &mut Errors) -> Model {
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
fn inferred_model(name: &str, events: &[Event], methods: &[Method], dtos: &[Model]) -> Model {
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

/// The type names `ty` mentions (nothing when it does not parse; that is reported elsewhere).
fn idents_of(ty: &str) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(parsed) = syn::parse_str::<syn::Type>(ty) {
        type_idents(&parsed, &mut found);
    }
    found
}

/// Whether `ty` is a primitive Rust type.
fn is_primitive(ty: &str) -> bool {
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
fn check_endpoints(provider: &Provider, dtos: &[Model], errors: &mut Errors) {
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
