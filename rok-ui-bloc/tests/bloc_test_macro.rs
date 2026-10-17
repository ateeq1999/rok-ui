//! `bloc_test!` expands to working tests.

// Test blocs often emit without awaiting anything; `Bloc::on` is async regardless.
#![allow(clippy::unused_async_trait_impl)]

use std::time::Duration;

use rok_ui_bloc::{bloc_test, Bloc, Concurrency, Emitter};

struct Counter;

impl Bloc for Counter {
    type Event = i32;
    type State = i32;

    fn initial_state(&self) -> i32 {
        0
    }

    async fn on(&self, by: i32, emit: &Emitter<i32>) {
        emit.update(|count| *count += by);
    }
}

/// Debounces every event.
struct Search;

impl Bloc for Search {
    type Event = &'static str;
    type State = String;

    fn initial_state(&self) -> String {
        String::new()
    }

    fn concurrency(&self, _event: &&'static str) -> Concurrency {
        Concurrency::debounce(Duration::from_millis(30))
    }

    async fn on(&self, query: &'static str, emit: &Emitter<String>) {
        emit.emit(query.to_string());
    }
}

bloc_test! {
    counting,
    build: Counter,
    act: [1, 2, 0],
    expect: [1, 3],
}

bloc_test! {
    /// Nothing added, nothing emitted.
    no_events_no_states,
    build: Counter,
    act: [],
    expect: [],
}

bloc_test! {
    typing_fast_searches_once,
    build: Search,
    act: ["r", "ru", "rust"],
    expect: [String::from("rust")],
}

bloc_test! {
    settled_events_each_run,
    build: Search,
    settle: true,
    act: ["a", "b"],
    verify: |states: Vec<String>| assert_eq!(states, ["a", "b"]),
}
