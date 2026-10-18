//! The feature's generated test file.

use std::fmt::Write as _;

use super::{
    cubit_method, default_of, event_name, header, import_types, rust_type, state_has_error_field,
    state_name, status_name, Imports,
};
use crate::generate::{
    names,
    resolve::{Event, Plan},
    spec::{Kind, StateStyle},
};

/// What a generated test asserts about the states `event` produced, with a fake repository
/// that always succeeds.
fn test_check(plan: &Plan, event: &Event) -> String {
    let state = state_name(plan);
    let status = status_name(plan);
    let calls = event.calls.is_some();
    match plan.state.style {
        StateStyle::Struct if calls && plan.state.has_status("Success") => {
            format!("assert_eq!(states.last().map(|state| state.status), Some({status}::Success));")
        }
        StateStyle::Struct if plan.state.has_status("Failure") => {
            format!("assert!(states.iter().all(|state| state.status != {status}::Failure));")
        }
        StateStyle::Struct if state_has_error_field(plan) => {
            "assert!(states.iter().all(|state| state.error.is_none()));".into()
        }
        StateStyle::Struct => "let _ = states;".into(),
        StateStyle::Enum if calls => {
            format!("assert!(matches!(states.last(), Some({state}::Success(_))));")
        }
        StateStyle::Enum => {
            format!("assert!(states.iter().all(|state| !matches!(state, {state}::Failure(_))));")
        }
    }
}

/// `tests/features/<f>/<name>_test.rs`.
pub fn test(plan: &Plan, crate_name: &str) -> String {
    let repository = plan.repository.as_ref().expect("a repository");
    let error = plan.error_type();
    let mut imports = Imports::default();
    imports.add("std::sync::Arc");
    imports.add("rok_ui::bloc::BoxFuture");
    imports.add(&format!(
        "{crate_name}::data::repositories::{}::{}",
        repository.names.snake, repository.names.pascal
    ));
    let mut types: Vec<&str> = vec![error];
    for method in &repository.methods {
        types.push(&method.returns);
        types.extend(method.args.iter().map(|arg| arg.ty.as_str()));
    }
    let event_types: Vec<&str> = plan
        .events
        .iter()
        .flat_map(|event| event.fields.iter().map(|field| field.ty.as_str()))
        .collect();
    types.extend(event_types);
    import_types(plan, &mut imports, &types, crate_name);
    let bloc_module = format!("{crate_name}::features::{}::bloc", plan.feature.snake);
    let state = state_name(plan);
    let status = status_name(plan);
    let is_cubit = plan.kind == Kind::Cubit;
    if is_cubit {
        imports.add(&format!(
            "{bloc_module}::{}_cubit::{}Cubit",
            plan.name.snake, plan.name.pascal
        ));
        imports.add("rok_ui::bloc::test");
    } else {
        imports.add("rok_ui::bloc::test");
        imports.add(&format!(
            "{bloc_module}::{}_bloc::{}Bloc",
            plan.name.snake, plan.name.pascal
        ));
        imports.add(&format!(
            "{bloc_module}::{}_event::{}",
            plan.name.snake,
            event_name(plan)
        ));
    }
    let checks: Vec<String> = plan
        .events
        .iter()
        .map(|event| test_check(plan, event))
        .collect();
    if checks
        .iter()
        .any(|check| check.contains(&format!("{status}::")))
    {
        imports.add(&format!(
            "{bloc_module}::{}_state::{status}",
            plan.name.snake
        ));
    }
    if checks
        .iter()
        .any(|check| check.contains(&format!("{state}::")))
    {
        imports.add(&format!(
            "{bloc_module}::{}_state::{state}",
            plan.name.snake
        ));
    }
    let fake = format!("Fake{}", repository.names.pascal);
    let subject = if is_cubit {
        format!("{}Cubit", plan.name.pascal)
    } else {
        format!("{}Bloc", plan.name.pascal)
    };
    let mut out = header(&format!(
        "`{subject}`: events in, states out, with a fake repository."
    ));
    out.push_str(&imports.render());
    let _ = write!(out, "\n/// Answers every call with an empty success.\nstruct {fake};\n\nimpl {} for {fake} {{\n", repository.names.pascal);
    for method in &repository.methods {
        let args: Vec<String> = method
            .args
            .iter()
            .map(|arg| format!("_{}: {}", arg.name, rust_type(&arg.ty)))
            .collect();
        let comma = if args.is_empty() { "" } else { ", " };
        let _ = write!(
            out,
            "    fn {}(&self{comma}{}) -> BoxFuture<'_, Result<{}, {error}>> {{\n        Box::pin(async {{ Ok({}) }})\n    }}\n\n",
            method.name,
            args.join(", "),
            rust_type(&method.returns),
            default_of(&method.returns)
        );
    }
    out.push_str("}\n");
    for event in &plan.events {
        let test_name = names::snake(&event.name);
        let value = if event.fields.is_empty() {
            format!("{}::{}", event_name(plan), event.name)
        } else {
            let fields: Vec<String> = event
                .fields
                .iter()
                .map(|field| format!("{}: {}", field.name, default_of(&field.ty)))
                .collect();
            format!(
                "{}::{} {{ {} }}",
                event_name(plan),
                event.name,
                fields.join(", ")
            )
        };
        let check = test_check(plan, event);
        if is_cubit {
            let args: Vec<String> = event
                .fields
                .iter()
                .map(|field| default_of(&field.ty))
                .collect();
            let act = if args.is_empty() {
                format!("{subject}::{}", cubit_method(event))
            } else {
                format!(
                    "|cubit| {{ cubit.{}({}); }}",
                    cubit_method(event),
                    args.join(", ")
                )
            };
            let _ = write!(
                out,
                "\n#[test]\nfn {test_name}() {{\n    let states = test::run_cubit({subject}::new(Arc::new({fake})), {act});\n    {check}\n}}\n"
            );
        } else {
            let _ = write!(
                out,
                "\n#[test]\nfn {test_name}() {{\n    let states = test::run({subject}::new(Arc::new({fake})), [{value}]);\n    {check}\n}}\n"
            );
        }
    }
    out
}
