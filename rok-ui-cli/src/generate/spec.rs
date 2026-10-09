//! The `generate` payload: what `-j` accepts, and the flags that override it.
//!
//! Every struct rejects unknown keys, so a typo is an error that names the key. The JSON
//! Schema printed by `cargo rok-ui g schema` is derived from these types.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Deserialize;

/// A feature, or part of one: business logic, data layer, view and tests.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The feature (module under `src/features`): `notes`.
    pub feature: Option<String>,
    /// `bloc` (events) or `cubit` (methods). Default: `bloc`.
    pub kind: Option<Kind>,
    /// The bloc or cubit name without the suffix: `Notes` makes `NotesBloc`. Default: the
    /// feature's name.
    pub name: Option<String>,
    /// How events are handled when others are running. Default: `sequential`.
    pub concurrency: Option<ConcurrencyKind>,
    /// Events, past tense (`NoteAdded`). For a cubit, each becomes a method.
    #[serde(default)]
    pub events: Vec<EventSpec>,
    /// The state.
    pub state: Option<StateSpec>,
    /// Domain models (`Note`). Models used but not listed get fields from their DTO, or from
    /// the events that carry them.
    #[serde(default)]
    pub models: Vec<TypeSpec>,
    /// The repository the bloc uses.
    pub repository: Option<RepositorySpec>,
    /// The data source behind the repository.
    pub provider: Option<ProviderSpec>,
    /// Wire types for an HTTP provider (`NoteDto`).
    #[serde(default)]
    pub dtos: Vec<TypeSpec>,
    /// The view.
    pub view: Option<ViewSpec>,
    /// Write `tests/features/<feature>/` (events in, states out). Default: true.
    pub tests: Option<bool>,
}

/// Bloc or cubit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Events in, states out.
    #[default]
    Bloc,
    /// Methods in, states out.
    Cubit,
}

/// `rok_ui::bloc::Concurrency`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConcurrencyKind {
    /// One at a time, in order.
    #[default]
    Sequential,
    /// Ignore while one of the same event runs.
    Droppable,
    /// Cancel the running one of the same event.
    Restartable,
    /// All at once.
    Concurrent,
}

/// An event.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventSpec {
    /// `PascalCase`, past tense: `NoteAdded`.
    pub name: String,
    /// What it carries.
    #[serde(default)]
    pub fields: Vec<FieldSpec>,
    /// The repository method its handler calls. Default: inferred from the event's verb
    /// (`NoteAdded` calls `add`, `NotesRequested` calls the method that lists).
    pub calls: Option<String>,
    /// Overrides the bloc's concurrency for this event.
    pub concurrency: Option<ConcurrencyKind>,
}

/// A named, typed field, argument or parameter.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldSpec {
    /// `snake_case`.
    pub name: String,
    /// A Rust type: `String`, `Vec<Note>`, `Option<u32>`.
    #[serde(rename = "type")]
    pub ty: String,
    /// The initial value, as a Rust expression. Default: `Default::default()`.
    pub default: Option<String>,
    /// The name on the wire (`serde(rename)`), for DTO fields.
    pub rename: Option<String>,
}

/// The state.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateSpec {
    /// `struct` (a status plus fields) or `enum` (one variant per status). Default: `struct`.
    pub style: Option<StateStyle>,
    /// Status names. Default: `initial`, `loading`, `success`, `failure`.
    #[serde(default)]
    pub status: Vec<String>,
    /// Data fields.
    #[serde(default)]
    pub fields: Vec<FieldSpec>,
    /// The error a failure carries: `ApiError` (HTTP) or the app's `DataError`. Default:
    /// the provider's error type.
    pub error: Option<String>,
}

/// How the state is shaped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateStyle {
    /// `struct NotesState { status, ..fields }`.
    #[default]
    Struct,
    /// `enum NotesState { Initial, Loading, Success { ..fields }, Failure(error) }`.
    Enum,
}

/// A struct (a model or a DTO), or with `variants`, an enum of struct-like variants (for a
/// union response, decoded with `serde(untagged)`).
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypeSpec {
    /// `PascalCase`.
    pub name: String,
    /// Its fields.
    #[serde(default)]
    pub fields: Vec<FieldSpec>,
    /// Variants, for an enum.
    #[serde(default)]
    pub variants: Vec<TypeSpec>,
}

/// A repository: a trait and its implementation over a provider.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositorySpec {
    /// `NotesRepository`.
    pub name: String,
    /// Its methods.
    #[serde(default)]
    pub methods: Vec<MethodSpec>,
}

/// A repository method.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MethodSpec {
    /// `snake_case`: `add`.
    pub name: String,
    /// Arguments.
    #[serde(default)]
    pub args: Vec<FieldSpec>,
    /// What it returns on success. Default: `()`.
    pub returns: Option<String>,
    /// The provider endpoint it calls (HTTP). Default: the endpoint with the same name.
    pub calls: Option<String>,
    /// What the call does to the HTTP session: `sign_in` stores the response's token,
    /// `sign_out` forgets it (even when the call fails).
    pub session: Option<SessionAction>,
    /// The response field holding the token, for `sign_in`. Default: `token`.
    pub token_field: Option<String>,
}

/// What a repository method does to the HTTP session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    /// Store the response's token: the user is signed in.
    SignIn,
    /// Forget the token: the user signed out.
    SignOut,
}

/// Where data comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Kept in memory (a starting point, and for tests).
    #[default]
    Memory,
    /// An HTTP API through `rok_ui::http::HttpClient`.
    Http,
    /// PostgreSQL through rok-db (generated as an in-memory stand-in to replace).
    Db,
    /// A file (generated as an in-memory stand-in to replace).
    File,
}

/// A data provider.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderSpec {
    /// `NotesApiProvider`.
    pub name: String,
    /// What it talks to. Default: `memory`.
    pub kind: Option<ProviderKind>,
    /// For `http`: the path prefix of every endpoint (`/api/v1`).
    pub api_base: Option<String>,
    /// For `http`: the endpoints.
    #[serde(default)]
    pub endpoints: Vec<EndpointSpec>,
}

/// An HTTP endpoint: one provider method.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointSpec {
    /// The provider method: `list`.
    pub method: String,
    /// `GET`, `POST`, `PUT`, `PATCH` or `DELETE`.
    pub verb: String,
    /// The path after `api_base`, with `{param}` placeholders: `/notes/{id}`.
    pub path: String,
    /// Values for the `{param}` placeholders, percent-encoded into the path.
    #[serde(default)]
    pub path_params: Vec<FieldSpec>,
    /// Query parameters; `None` values are left out.
    #[serde(default)]
    pub query: Vec<FieldSpec>,
    /// The JSON body type (a DTO).
    pub body: Option<String>,
    /// Send raw bytes with a content type instead of JSON.
    #[serde(default)]
    pub raw_body: bool,
    /// The response: a DTO, `Vec<..>`, `()` or `bytes`.
    pub response: String,
    /// A 401 here is an answer (wrong password), not an expired session.
    #[serde(default)]
    pub skip_expire: bool,
}

/// The view.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewSpec {
    /// Write a page and a route for it. Default: true.
    pub page: Option<bool>,
    /// A form whose submit adds an event.
    pub form: Option<FormSpec>,
}

/// A form bound to an event.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FormSpec {
    /// The event the submit adds; its fields come from the form.
    pub event: String,
    /// The form's fields (default: the event's).
    #[serde(default)]
    pub fields: Vec<FieldSpec>,
    /// Where server errors land (HTTP).
    pub server_errors: Option<ServerErrorsSpec>,
}

/// `rok_ui::form::ServerErrorOptions`.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServerErrorsSpec {
    /// The field a 409 lands on.
    pub conflict_field: Option<String>,
    /// Server field name to form field name.
    #[serde(default)]
    pub field_map: BTreeMap<String, String>,
    /// The field a 400 lands on.
    pub bad_request_field: Option<String>,
    /// The field a 401 lands on.
    pub unauthorized_field: Option<String>,
}

/// The JSON Schema of the payload.
pub fn schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(Spec)).unwrap_or_default()
}
