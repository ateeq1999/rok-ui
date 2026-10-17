//! The resolved plan the renderers read: names, fields, events, methods, models and state.

use crate::generate::{
    names,
    spec::{
        ConcurrencyKind, EndpointSpec, FormSpec, Kind, ProviderKind, SessionAction, StateStyle,
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
