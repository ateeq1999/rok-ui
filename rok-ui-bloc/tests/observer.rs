//! The observer sees creation, events, changes, panics and closing. In its own test binary:
//! the observer is process-wide.

use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use rok_ui_bloc::{set_observer, test, Bloc, BlocObserver, Cubit, Emitter};

#[derive(Default)]
struct Recorder(Arc<Mutex<Vec<String>>>);

impl BlocObserver for Recorder {
    fn on_create(&self, bloc: &'static str) {
        self.push(format!("create {}", short(bloc)));
    }

    fn on_event(&self, bloc: &'static str, event: &dyn Debug) {
        self.push(format!("event {} {event:?}", short(bloc)));
    }

    fn on_change(&self, bloc: &'static str, current: &dyn Debug, next: &dyn Debug) {
        self.push(format!("change {} {current:?} -> {next:?}", short(bloc)));
    }

    fn on_error(&self, bloc: &'static str, error: &str) {
        self.push(format!("error {} {error}", short(bloc)));
    }

    fn on_close(&self, bloc: &'static str) {
        self.push(format!("close {}", short(bloc)));
    }
}

impl Recorder {
    fn push(&self, line: String) {
        self.0.lock().unwrap().push(line);
    }
}

fn short(name: &str) -> &str {
    name.rsplit("::").next().unwrap()
}

#[derive(Debug)]
enum CounterEvent {
    Added(i32),
    Exploded,
}

struct CounterBloc;

impl Bloc for CounterBloc {
    type Event = CounterEvent;
    type State = i32;

    fn initial_state(&self) -> i32 {
        0
    }

    async fn on(&self, event: CounterEvent, emit: &Emitter<i32>) {
        match event {
            CounterEvent::Added(by) => {
                emit.update(|count| *count += by);
            }
            CounterEvent::Exploded => panic!("boom"),
        }
    }
}

struct Toggle(Emitter<bool>);

impl Cubit for Toggle {
    type State = bool;

    fn emitter(&self) -> &Emitter<bool> {
        &self.0
    }
}

#[test]
fn the_observer_sees_everything_and_panics_do_not_stop_a_bloc() {
    let lines = Arc::new(Mutex::new(Vec::new()));
    set_observer(Recorder(lines.clone()));

    let states = test::run(
        CounterBloc,
        [
            CounterEvent::Added(2),
            CounterEvent::Exploded,
            CounterEvent::Added(1),
        ],
    );
    assert_eq!(states, [2, 3], "the event after the panic is still handled");
    let seen = lines.lock().unwrap().clone();
    // Events are reported as they are added, changes as they are handled: compare each stream
    // on its own (the two interleave with timing).
    let starting = |prefix: &str| -> Vec<String> {
        seen.iter()
            .filter(|line| line.starts_with(prefix))
            .cloned()
            .collect()
    };
    assert_eq!(seen.first().map(String::as_str), Some("create CounterBloc"));
    assert_eq!(seen.last().map(String::as_str), Some("close CounterBloc"));
    assert_eq!(
        starting("event"),
        [
            "event CounterBloc Added(2)",
            "event CounterBloc Exploded",
            "event CounterBloc Added(1)"
        ]
    );
    let handled: Vec<String> = seen
        .iter()
        .filter(|line| line.starts_with("change") || line.starts_with("error"))
        .cloned()
        .collect();
    assert_eq!(
        handled,
        [
            "change CounterBloc 0 -> 2",
            "error CounterBloc boom",
            "change CounterBloc 2 -> 3"
        ]
    );

    lines.lock().unwrap().clear();
    let states = test::run_cubit(Toggle(Emitter::new(false)), |toggle| {
        let emit = toggle.0.clone();
        toggle.0.spawn(async move {
            emit.emit(true);
        });
    });
    assert_eq!(states, [true]);
    assert_eq!(
        *lines.lock().unwrap(),
        [
            "create Toggle",
            "change Toggle false -> true",
            "close Toggle"
        ]
    );
}
