//! The presentation layer: the page and its route.

use std::fmt::Write as _;

use super::{
    action_label, cubit_method, default_of, event_name, header, id_alias_type, import_types,
    is_copy, sentence, state_has_error_field, state_name, status_name, struct_fields, submit_label,
    Imports,
};
use crate::generate::{
    names,
    resolve::{Event, Field, Plan},
    spec::{Kind, StateStyle},
};

/// The page component's name.
pub fn page_name(plan: &Plan) -> String {
    format!("{}Page", plan.name.pascal)
}

/// The form's values type.
fn form_values(plan: &Plan) -> Option<(String, Vec<Field>, &Event)> {
    let form = plan.form.as_ref()?;
    let event = plan.events.iter().find(|event| event.name == form.event)?;
    let fields = if form.fields.is_empty() {
        event.fields.clone()
    } else {
        form.fields
            .iter()
            .map(|field| Field {
                name: field.name.clone(),
                ty: field.ty.clone(),
                default: field.default.clone(),
                rename: None,
            })
            .collect()
    };
    Some((format!("{}Form", event.name), fields, event))
}

/// `src/features/<f>/view/<name>_page.rs`.
#[allow(clippy::too_many_lines)]
pub fn page(plan: &Plan) -> String {
    let page = page_name(plan);
    let state = state_name(plan);
    let is_cubit = plan.kind == Kind::Cubit;
    let owner = if is_cubit {
        format!("{}Cubit", plan.name.pascal)
    } else {
        format!("{}Bloc", plan.name.pascal)
    };
    let owner_module = if is_cubit {
        format!("{}_cubit", plan.name.snake)
    } else {
        format!("{}_bloc", plan.name.snake)
    };
    let bloc_module = format!("crate::features::{}::bloc", plan.feature.snake);
    let mut imports = Imports::default();
    imports.add("rok_ui::bloc::BlocBuilder");
    imports.add("rok_ui::prelude::*");
    imports.add(&format!("{bloc_module}::{owner_module}::{owner}"));
    let mut types: Vec<&str> = Vec::new();
    let form = form_values(plan);
    if let Some((_, fields, _)) = &form {
        types.extend(fields.iter().map(|field| field.ty.as_str()));
        imports.add("rok_ui::form::{self, FormOptions, FormValues, SubmitButton}");
    }
    let server_errors = plan
        .form
        .as_ref()
        .and_then(|form| form.server_errors.clone())
        .filter(|_| form.is_some());
    if server_errors.is_some() {
        imports.add("rok_ui::bloc::BlocListener");
        imports.add("rok_ui::form::ServerErrorOptions");
    }
    import_types(plan, &mut imports, &types, "crate");
    let status = status_name(plan);
    if plan.state.style == StateStyle::Struct {
        imports.add(&format!(
            "{bloc_module}::{}_state::{status}",
            plan.name.snake
        ));
    } else {
        imports.add(&format!(
            "{bloc_module}::{}_state::{state}",
            plan.name.snake
        ));
    }
    let no_field_events: Vec<&Event> = plan
        .events
        .iter()
        .filter(|event| event.fields.is_empty())
        .collect();
    if !is_cubit && (!no_field_events.is_empty() || form.is_some()) {
        imports.add(&format!(
            "{bloc_module}::{}_event::{}",
            plan.name.snake,
            event_name(plan)
        ));
    }
    let mut out = header(&format!("The `{}` page.", plan.feature.snake));
    if form.is_some() {
        imports.0.retain(|path| path != "rok_ui::prelude::*");
        imports.add("rok_ui::prelude::*");
    }
    out.push_str(&imports.render());
    // The form's values type.
    let mut form_fields_ui = String::new();
    if let Some((values, fields, event)) = &form {
        let _ = write!(out, "\n/// What the `{}` form edits.\n#[derive(FormValues, Clone, Default)]\nstruct {values} {{\n", event.name);
        struct_fields(&mut out, fields, false, false);
        out.push_str("}\n");
        if let Some(options) = &server_errors {
            let constant =
                |field: &str| format!("&{values}::{}", names::snake(field).to_uppercase());
            let mut chain = String::from("ServerErrorOptions::new()");
            for (api_field, field) in &options.field_map {
                let _ = write!(chain, ".map({api_field:?}, {})", constant(field));
            }
            if let Some(field) = &options.conflict_field {
                let _ = write!(chain, ".conflict_field({})", constant(field));
            }
            if let Some(field) = &options.bad_request_field {
                let _ = write!(chain, ".bad_request_field({})", constant(field));
            }
            if let Some(field) = &options.unauthorized_field {
                let _ = write!(chain, ".unauthorized_field({})", constant(field));
            }
            let _ = write!(
                out,
                "\n/// Where API errors from submitting the form are shown.\nfn server_errors() -> ServerErrorOptions {{\n    {chain}\n}}\n"
            );
        }
        let event_fields: Vec<String> = event
            .fields
            .iter()
            .map(|field| {
                if fields.iter().any(|candidate| candidate.name == field.name) {
                    format!("{0}: values.{0}", field.name)
                } else {
                    format!("{}: {}", field.name, default_of(&field.ty))
                }
            })
            .collect();
        let dispatch = if is_cubit {
            let args: Vec<String> = event
                .fields
                .iter()
                .map(|field| {
                    if fields.iter().any(|candidate| candidate.name == field.name) {
                        format!("values.{}", field.name)
                    } else {
                        default_of(&field.ty)
                    }
                })
                .collect();
            format!("submitting.{}({});", cubit_method(event), args.join(", "))
        } else {
            format!(
                "submitting.add({}::{} {{ {} }});",
                event_name(plan),
                event.name,
                event_fields.join(", ")
            )
        };
        let _ = write!(
            form_fields_ui,
            "    let submitting = bloc.clone();\n    let form = form::use_form(\n        cx,\n        FormOptions::new({values}::default()).on_submit(move |values: {values}, _| {{\n            {dispatch}\n            gpui::Task::ready(Ok(()))\n        }}),\n    );\n"
        );
        let mut controls = Vec::new();
        for field in fields {
            let constant = names::snake(&field.name).to_uppercase();
            let label = sentence(&field.name);
            match field.ty.as_str() {
                "String" if field.name.contains("password") => controls.push(format!(".child(form::TextField::new(&form.field(cx, {values}::{constant}), {label:?}).password())")),
                "String" => controls.push(format!(".child(form::TextField::new(&form.field(cx, {values}::{constant}), {label:?}))")),
                "bool" => controls.push(format!(".child(form::CheckboxField::new(&form.field(cx, {values}::{constant}), {label:?}))")),
                _ => {}
            }
        }
        let _ = writeln!(
            form_fields_ui,
            "    let editor = div().flex().flex_col().gap_2(){}.child(SubmitButton::new(&form, {:?}));",
            controls.join(""),
            submit_label(event)
        );
        if server_errors.is_some() {
            let (pattern, listen_when) = match plan.state.style {
                StateStyle::Struct => (
                    "Some(error) = &state.error".to_string(),
                    "previous.error != current.error",
                ),
                StateStyle::Enum => (
                    format!("{state}::Failure(error) = state"),
                    "previous != current",
                ),
            };
            let _ = writeln!(
                form_fields_ui,
                "    // The bloc's error, on the form's fields.\n    let errors_form = form.clone();\n    let errors = BlocListener::new(&bloc, move |state, _, cx| {{ if let {pattern} {{ errors_form.apply_server_errors(cx, &form::to_server_errors(error, &server_errors())); }} }}).listen_when(|previous, current| {listen_when});"
            );
        }
    }
    let lookup = if is_cubit { "cubit" } else { "bloc" };
    let _ = write!(out, "\n/// Shows `{}` and {}.\n#[component]\npub fn {page}(cx: &mut Cx) -> impl IntoElement {{\n    let bloc = cx.{lookup}::<{owner}>();\n", plan.feature.snake, if is_cubit { "calls its methods" } else { "adds its events" });
    // Load once.
    if let Some(first) = no_field_events.iter().find(|event| {
        event.calls.as_ref().is_some_and(|call| {
            plan.loader()
                .is_some_and(|(loader, _)| &loader.name == call)
        })
    }) {
        let dispatch = if is_cubit {
            format!("first.{}()", cubit_method(first))
        } else {
            format!("first.add({}::{})", event_name(plan), first.name)
        };
        let _ = write!(out, "    // Once, when the page opens: a state initializer runs on the first render only.\n    let first = bloc.clone();\n    cx.use_state(move || {dispatch});\n");
    }
    out.push_str(&form_fields_ui);
    let mut actions = Vec::new();
    for event in &no_field_events {
        let id = names::snake(&event.name).replace('_', "-");
        let dispatch = if is_cubit {
            format!("handler.{}()", cubit_method(event))
        } else {
            format!("handler.add({}::{})", event_name(plan), event.name)
        };
        actions.push(format!(
            ".child({{ let handler = bloc.clone(); Button::new({id:?}).outline().label({:?}).on_click(move |_, _, _| {{ {dispatch}; }}) }})",
            action_label(plan, event)
        ));
    }
    let list = plan.state.fields.iter().find_map(|field| {
        let inner = crate::generate::resolve::vec_inner(&field.ty)?;
        Some((field.name.clone(), plan.model(&inner).cloned()))
    });
    let item_text = |binding: &str| -> String {
        let model = list.as_ref().and_then(|(_, model)| model.as_ref());
        let text_field = model.and_then(|model| {
            model
                .fields
                .iter()
                .find(|field| field.ty == "String" && field.name != "id")
                .map(|field| field.name.clone())
        });
        match text_field {
            Some(field) => format!("{binding}.{field}.clone()"),
            None => format!("format!(\"{{{binding}:?}}\")"),
        }
    };
    // Events that act on one item (`NoteDeleted { id }`) become a button on each row.
    let list_model = list.as_ref().and_then(|(_, model)| model.clone());
    let row_events: Vec<&Event> = list_model
        .as_ref()
        .and_then(|model| model.id.clone())
        .map(|id| {
            plan.events
                .iter()
                .filter(|event| event.fields.len() == 1 && event.fields[0].ty == id)
                .collect()
        })
        .unwrap_or_default();
    let copy_id = list_model
        .as_ref()
        .and_then(|model| model.id.as_ref())
        .is_some_and(|id| is_copy(&id_alias_type(plan, id)));
    let items = list.as_ref().map(|(field, _)| {
        let source = match plan.state.style {
            StateStyle::Struct => format!("state.{field}"),
            StateStyle::Enum => format!("state.data().{field}"),
        };
        if row_events.is_empty() {
            return format!(
                "div().flex().flex_col().gap_1().children({source}.iter().map(|item| div().child({})))",
                item_text("item")
            );
        }
        let mut buttons = String::new();
        for event in &row_events {
            let field = &event.fields[0].name;
            let value = if copy_id { "item.id".to_string() } else { "item.id.clone()".to_string() };
            let dispatch = if is_cubit {
                format!("handler.{}({field})", cubit_method(event))
            } else {
                format!("handler.add({}::{} {{ {field} }})", event_name(plan), event.name)
            };
            let id = names::snake(&event.name).replace('_', "-");
            let _ = write!(
                buttons,
                ".child({{ let handler = bloc.clone(); let {field} = {value}; Button::new(({id:?}, index)).ghost().label({:?}).on_click(move |_, _, _| {{ {dispatch}; }}) }})",
                sentence(&names::event_verb(&event.name))
            );
        }
        format!(
            "div().flex().flex_col().gap_1().children({source}.iter().enumerate().map(|(index, item)| div().flex().gap_2().items_center().child({}){buttons}))",
            item_text("item")
        )
    });
    let items = items.unwrap_or_else(|| "div()".into());
    let body = match plan.state.style {
        StateStyle::Struct => {
            let mut arms = String::new();
            if plan.state.has_status("Loading") {
                let _ = writeln!(arms, "{status}::Loading => div().child(\"Loading...\"),");
            }
            if plan.state.has_status("Failure") {
                let message = if state_has_error_field(plan) {
                    "state.error.as_ref().map_or_else(|| \"Something went wrong.\".to_string(), ToString::to_string)"
                } else {
                    "\"Something went wrong.\".to_string()"
                };
                let _ = writeln!(arms, "{status}::Failure => div().child({message}),");
            }
            if arms.is_empty() {
                items
            } else {
                format!("match state.status {{\n{arms}_ => {items},\n}}")
            }
        }
        StateStyle::Enum => {
            let mut arms = String::new();
            if plan.state.has_status("Loading") {
                let _ = writeln!(arms, "{state}::Loading => div().child(\"Loading...\"),");
            }
            let _ = writeln!(
                arms,
                "{state}::Failure(error) => div().child(error.to_string()),"
            );
            let _ = writeln!(arms, "_ => {items},");
            format!("match state {{\n{arms}}}")
        }
    };
    let editor = match (&form, &server_errors) {
        (Some(_), Some(_)) => ".child(editor).child(errors)",
        (Some(_), None) => ".child(editor)",
        _ => "",
    };
    let builder = if row_events.is_empty() {
        format!("|state, _, _| {{ {body} }}")
    } else {
        format!("{{ let bloc = bloc.clone(); move |state, _, _| {{ {body} }} }}")
    };
    let _ = write!(
        out,
        "    div()\n        .flex()\n        .flex_col()\n        .gap_4()\n        .p_6()\n        .child(H2::new({:?}))\n        .child(div().flex().gap_2(){})\n        {editor}\n        .child(BlocBuilder::new(&bloc, {builder}))\n}}\n",
        sentence(&plan.feature.pascal),
        actions.join("")
    );
    out
}

/// `src/routes/<feature>.rs`: a thin route rendering the page.
pub fn route(plan: &Plan) -> String {
    let page = page_name(plan);
    format!(
        "use rok_ui::prelude::*;\nuse rok_ui::router::file_route;\n\nfile_route! {{\n    component: {0}Route,\n}}\n\n/// `/{1}`: the {1} page. Routes stay thin; the page lives in `features::{1}::view`.\n#[component]\nfn {0}Route() -> impl IntoElement {{\n    crate::features::{1}::view::{2}::{page}::new()\n}}\n",
        plan.feature.pascal,
        plan.feature.snake,
        names::snake(&page)
    )
}
