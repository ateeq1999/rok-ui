//! The business logic: events, state, and the bloc or cubit that handles them.

use std::fmt::Write as _;
use std::time::Duration;

use super::{default_of, header, import_types, rust_type, sentence, struct_fields, Imports};
use crate::generate::{
    names,
    resolve::{Event, Plan},
    spec::{ConcurrencyKind, Kind, SessionAction, StateStyle},
};

/// The state's type name.
pub(super) fn state_name(plan: &Plan) -> String {
    format!("{}State", plan.name.pascal)
}

pub(super) fn status_name(plan: &Plan) -> String {
    format!("{}Status", plan.name.pascal)
}

/// The fields the state stores, plus `error` for struct states.
pub(super) fn state_has_error_field(plan: &Plan) -> bool {
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
pub(super) fn handler(plan: &Plan, event: &Event, repository: &str) -> String {
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
    let returns = method.returns.replace(' ', "");
    // The state field the result goes to: one of the same type, or an `Option` of it.
    let assigned = plan
        .state
        .fields
        .iter()
        .find(|field| field.ty.replace(' ', "") == returns)
        .map(|field| (field, "value"))
        .or_else(|| {
            plan.state
                .fields
                .iter()
                .find(|field| field.ty.replace(' ', "") == format!("Option<{returns}>"))
                .map(|field| (field, "Some(value)"))
        });
    let ignored = if returns == "()" { "()" } else { "_" };
    if matches!(method.session, Some((SessionAction::SignOut, _))) {
        // Signed out: back to the initial state.
        let reset = match style {
            StateStyle::Struct if has("Success") => {
                format!("emit.emit({state} {{ status: {status}::Success, ..{state}::default() }});")
            }
            StateStyle::Struct => format!("emit.emit({state}::default());"),
            StateStyle::Enum => format!(
                "emit.emit({state}::Success({}Data::default()));",
                plan.name.pascal
            ),
        };
        let _ = write!(
            out,
            "match {call} {{\nOk({ignored}) => {{ {reset} }}\n{cancelled}Err(error) => {{ {failure} }}\n}}\n"
        );
        return out;
    }
    match (assigned, plan.loader()) {
        (Some((field, value)), _) => {
            let _ = write!(
                out,
                "match {call} {{\nOk(value) => {{ {} }}\n{cancelled}Err(error) => {{ {failure} }}\n}}\n",
                success(Some((&field.name, value)))
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

/// The `Concurrency` expression for `kind`, adding the imports it needs.
fn concurrency_expr(kind: ConcurrencyKind, imports: &mut Imports) -> String {
    let duration = |duration: Duration, imports: &mut Imports| {
        imports.add("std::time::Duration");
        if duration.subsec_millis() == 0 {
            format!("Duration::from_secs({})", duration.as_secs())
        } else {
            format!("Duration::from_millis({})", duration.as_millis())
        }
    };
    match kind {
        ConcurrencyKind::Sequential => "Concurrency::Sequential".into(),
        ConcurrencyKind::Droppable => "Concurrency::Droppable".into(),
        ConcurrencyKind::Restartable => "Concurrency::Restartable".into(),
        ConcurrencyKind::Concurrent => "Concurrency::Concurrent".into(),
        ConcurrencyKind::Debounce(delay) => {
            format!("Concurrency::debounce({})", duration(delay, imports))
        }
        ConcurrencyKind::Throttle(window) => {
            format!("Concurrency::throttle({})", duration(window, imports))
        }
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
    let mut arms = String::new();
    if custom_concurrency {
        for event in &plan.events {
            let kind = event.concurrency.unwrap_or(plan.concurrency);
            let pattern = if event.fields.is_empty() {
                String::new()
            } else {
                " { .. }".into()
            };
            let _ = writeln!(
                arms,
                "{event_type}::{}{pattern} => {},",
                event.name,
                concurrency_expr(kind, &mut imports),
                event_type = event_name(plan)
            );
        }
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

pub(super) fn event_name(plan: &Plan) -> String {
    format!("{}Event", plan.name.pascal)
}

/// The match pattern for `event`, binding the fields `body` uses.
pub(super) fn pattern(plan: &Plan, event: &Event, body: &str) -> String {
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
