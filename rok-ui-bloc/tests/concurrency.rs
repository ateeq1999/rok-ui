//! The concurrency modes, closing, and change-only notification.

// Test blocs often emit without awaiting anything; `Bloc::on` is async regardless.
#![allow(clippy::unused_async_trait_impl)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use rok_ui_bloc::{test, Bloc, BlocHandle, Concurrency, Emitter, Observable};

/// Appends each event's label after waiting its delay.
struct Log {
    mode: Concurrency,
}

#[derive(Clone, Debug)]
struct Step {
    label: &'static str,
    delay_ms: u64,
}

impl Bloc for Log {
    type Event = Step;
    type State = Vec<&'static str>;

    fn initial_state(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn concurrency(&self, _event: &Step) -> Concurrency {
        self.mode
    }

    async fn on(&self, step: Step, emit: &Emitter<Vec<&'static str>>) {
        tokio::time::sleep(Duration::from_millis(step.delay_ms)).await;
        emit.update(|log| log.push(step.label));
    }
}

fn steps(steps: &[(&'static str, u64)]) -> Vec<Step> {
    steps
        .iter()
        .map(|&(label, delay_ms)| Step { label, delay_ms })
        .collect()
}

fn last(states: &[Vec<&'static str>]) -> Vec<&'static str> {
    states.last().cloned().unwrap_or_default()
}

#[test]
fn debounced_events_run_once_after_the_last() {
    let mode = Concurrency::debounce(Duration::from_millis(40));
    let states = test::run(Log { mode }, steps(&[("a", 1), ("ab", 1), ("abc", 1)]));
    assert_eq!(states, [vec!["abc"]], "only the last event is handled");
    // Events further apart than the delay each run.
    let settled = test::run_settled(Log { mode }, steps(&[("a", 1), ("b", 1)]));
    assert_eq!(last(&settled), ["a", "b"]);
}

#[test]
fn throttled_events_are_dropped_inside_the_window() {
    let mode = Concurrency::throttle(Duration::from_secs(5));
    let states = test::run_settled(
        Log { mode },
        steps(&[("first", 1), ("second", 1), ("third", 1)]),
    );
    assert_eq!(last(&states), ["first"]);
    let mode = Concurrency::throttle(Duration::ZERO);
    let states = test::run_settled(Log { mode }, steps(&[("first", 1), ("second", 1)]));
    assert_eq!(
        last(&states),
        ["first", "second"],
        "an elapsed window lets events through"
    );
}

#[test]
fn sequential_events_keep_their_order() {
    let mode = Concurrency::Sequential;
    let states = test::run(
        Log { mode },
        steps(&[("slow", 60), ("fast", 1), ("mid", 20)]),
    );
    assert_eq!(last(&states), ["slow", "fast", "mid"]);
}

#[test]
fn concurrent_events_finish_in_their_own_time() {
    let mode = Concurrency::Concurrent;
    let states = test::run(Log { mode }, steps(&[("slow", 80), ("fast", 1)]));
    assert_eq!(last(&states), ["fast", "slow"]);
}

#[test]
fn droppable_events_are_ignored_while_one_runs() {
    let mode = Concurrency::Droppable;
    let states = test::run(
        Log { mode },
        steps(&[("first", 50), ("second", 1), ("third", 1)]),
    );
    assert_eq!(last(&states), ["first"]);
    // Once it finished, the next one runs.
    let settled = test::run_settled(Log { mode }, steps(&[("first", 5), ("second", 1)]));
    assert_eq!(last(&settled), ["first", "second"]);
}

#[test]
fn restartable_events_cancel_the_running_one() {
    let mode = Concurrency::Restartable;
    let states = test::run(Log { mode }, steps(&[("stale", 80), ("fresh", 5)]));
    assert_eq!(
        states,
        [vec!["fresh"]],
        "the cancelled handler emitted nothing"
    );
}

#[test]
fn closing_cancels_running_handlers_and_ignores_later_events() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let bloc = BlocHandle::start(
        Log {
            mode: Concurrency::Concurrent,
        },
        runtime.handle(),
    );
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let _subscription = bloc.subscribe(move |state: &Vec<&'static str>| {
        recorder.lock().unwrap().push(state.clone());
    });
    bloc.add(Step {
        label: "late",
        delay_ms: 50,
    });
    bloc.close();
    bloc.add(Step {
        label: "after",
        delay_ms: 0,
    });
    runtime.block_on(async { tokio::time::sleep(Duration::from_millis(100)).await });
    assert!(seen.lock().unwrap().is_empty());
    assert!(bloc.is_closed());
    assert_eq!(bloc.state(), Vec::<&str>::new());
}

#[test]
fn equal_states_are_not_announced() {
    let emitter = Emitter::new(1);
    let calls = Arc::new(Mutex::new(0));
    let counter = calls.clone();
    let subscription = emitter.subscribe(move |_| *counter.lock().unwrap() += 1);
    emitter.emit(1);
    emitter.emit(2);
    emitter.update(|value| *value = 2);
    assert_eq!(*calls.lock().unwrap(), 1);
    drop(subscription);
    emitter.emit(3);
    assert_eq!(
        *calls.lock().unwrap(),
        1,
        "dropped subscriptions hear nothing"
    );
}

#[test]
fn a_bloc_cannot_hold_another_bloc() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
