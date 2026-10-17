//! `rok_ui::bloc` in GPUI: providers, builders, listeners and the bloc lifecycle.

// Test blocs often emit without awaiting anything; `Bloc::on` is async regardless.
#![allow(clippy::unused_async_trait_impl)]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use gpui::{TestAppContext, VisualTestContext};
use rok_ui::{
    bloc::{
        Bloc, BlocBuilder, BlocConsumer, BlocHandle, BlocListener, BlocProvider, BlocSelector,
        Emitter, MultiBlocProvider, RepositoryProvider,
    },
    prelude::*,
};

/// Where the counter's step comes from.
trait StepRepository: Send + Sync {
    fn step(&self) -> u32;
}

struct FixedStep(u32);

impl StepRepository for FixedStep {
    fn step(&self) -> u32 {
        self.0
    }
}

#[derive(Debug)]
enum CounterEvent {
    Incremented,
    Set(u32),
    SlowlyIncremented,
}

struct CounterBloc {
    steps: Arc<dyn StepRepository>,
}

impl Bloc for CounterBloc {
    type Event = CounterEvent;
    type State = u32;

    fn initial_state(&self) -> u32 {
        0
    }

    fn concurrency(&self, _event: &CounterEvent) -> rok_ui::bloc::Concurrency {
        rok_ui::bloc::Concurrency::Concurrent
    }

    async fn on(&self, event: CounterEvent, emit: &Emitter<u32>) {
        match event {
            CounterEvent::Incremented => {
                let step = self.steps.step();
                emit.update(|count| *count += step);
            }
            CounterEvent::Set(value) => {
                emit.emit(value);
            }
            CounterEvent::SlowlyIncremented => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                emit.update(|count| *count += 1);
            }
        }
    }
}

#[derive(Debug)]
enum LogEvent {
    Logged(&'static str),
}

/// A second bloc, told about the counter by a listener in the view.
struct LogBloc;

impl Bloc for LogBloc {
    type Event = LogEvent;
    type State = Vec<&'static str>;

    fn initial_state(&self) -> Vec<&'static str> {
        Vec::new()
    }

    async fn on(&self, event: LogEvent, emit: &Emitter<Vec<&'static str>>) {
        let LogEvent::Logged(line) = event;
        emit.update(|log| log.push(line));
    }
}

type Seen = Rc<RefCell<Vec<(BlocHandle<CounterBloc>, u32)>>>;

/// Reads the counter and records the handle and state it saw.
#[component]
fn Reader(seen: Seen, renders: Rc<AtomicUsize>, cx: &mut Cx) -> impl IntoElement {
    renders.fetch_add(1, Ordering::SeqCst);
    let counter = cx.bloc::<CounterBloc>();
    seen.borrow_mut()
        .push((counter.clone(), cx.watch_bloc::<CounterBloc>()));
    div()
}

#[derive(Default)]
struct Probe {
    seen: Seen,
    renders: Rc<AtomicUsize>,
    shown: Rc<RefCell<Vec<u32>>>,
    selected: Rc<RefCell<Vec<bool>>>,
    heard: Rc<RefCell<Vec<u32>>>,
    log: Rc<RefCell<Option<BlocHandle<LogBloc>>>>,
}

/// Everything that reads the counter, below the providers.
#[component]
fn Readers(probe: Rc<Probe>, cx: &mut Cx) -> impl IntoElement {
    let counter = cx.bloc::<CounterBloc>();
    let log = cx.bloc::<LogBloc>();
    *probe.log.borrow_mut() = Some(log.clone());
    let (shown, selected, heard) = (
        probe.shown.clone(),
        probe.selected.clone(),
        probe.heard.clone(),
    );
    div()
        .child(Reader::new(probe.seen.clone(), probe.renders.clone()))
        .child(Reader::new(probe.seen.clone(), probe.renders.clone()))
        .child(
            BlocBuilder::new(&counter, move |count, _, _| {
                shown.borrow_mut().push(*count);
                div()
            })
            .build_when(|_, current| current % 2 == 0),
        )
        .child(BlocSelector::new(
            &counter,
            |count| *count >= 10,
            move |big, _, _| {
                selected.borrow_mut().push(*big);
                div()
            },
        ))
        .child(
            // The recommended way for one bloc to react to another.
            BlocListener::new(&counter, move |count, _, _| {
                heard.borrow_mut().push(*count);
                if *count == 3 {
                    log.add(LogEvent::Logged("three"));
                }
            })
            .listen_when(|previous, current| current > previous)
            .child(div()),
        )
}

struct App {
    probe: Rc<Probe>,
    mounted: Rc<RefCell<bool>>,
}

impl Render for App {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let probe = self.probe.clone();
        let body = (*self.mounted.borrow()).then(|| {
            RepositoryProvider::new()
                .provide::<dyn StepRepository>(Arc::new(FixedStep(1)))
                .child(
                    MultiBlocProvider::new()
                        .with_bloc(|scope| CounterBloc {
                            steps: scope.repository::<dyn StepRepository>(),
                        })
                        .with_bloc(|_| LogBloc)
                        .child(Readers::new(probe)),
                )
        });
        div().children(body)
    }
}

fn open(cx: &mut TestAppContext) -> (Rc<Probe>, Rc<RefCell<bool>>, &mut VisualTestContext) {
    cx.update(rok_ui::init);
    cx.executor().allow_parking();
    let probe = Rc::new(Probe::default());
    let mounted = Rc::new(RefCell::new(true));
    let (view_probe, view_mounted) = (probe.clone(), mounted.clone());
    let (_, window) = cx.add_window_view(move |_, _| App {
        probe: view_probe.clone(),
        mounted: view_mounted.clone(),
    });
    window.run_until_parked();
    (probe, mounted, window)
}

fn counter(probe: &Probe) -> BlocHandle<CounterBloc> {
    probe.seen.borrow().last().expect("rendered").0.clone()
}

/// Let handlers finish on the runtime and their state reach the window.
fn settle(window: &mut VisualTestContext, bloc: &BlocHandle<CounterBloc>) {
    rok_ui::runtime::block_on(bloc.idle());
    std::thread::sleep(Duration::from_millis(10));
    window.run_until_parked();
}

#[gpui::test]
fn views_below_a_provider_share_one_bloc(cx: &mut TestAppContext) {
    let (probe, _, window) = open(cx);
    let seen = probe.seen.borrow().clone();
    assert!(seen.len() >= 2, "both readers rendered");
    assert!(
        seen.iter().all(|(handle, _)| handle.ptr_eq(&seen[0].0)),
        "one instance"
    );
    assert!(
        seen.iter().all(|(_, count)| *count == 0),
        "one initial state"
    );

    // Later renders get the same bloc, not a new one.
    window.update(|window, _| window.refresh());
    window.run_until_parked();
    assert!(counter(&probe).ptr_eq(&seen[0].0));
}

#[gpui::test]
fn only_real_changes_re_render(cx: &mut TestAppContext) {
    let (probe, _, window) = open(cx);
    let counter = counter(&probe);
    let renders = probe.renders.load(Ordering::SeqCst);

    counter.add(CounterEvent::Set(0));
    settle(window, &counter);
    assert_eq!(
        probe.renders.load(Ordering::SeqCst),
        renders,
        "an equal state"
    );

    counter.add(CounterEvent::Incremented);
    settle(window, &counter);
    assert!(probe.renders.load(Ordering::SeqCst) > renders);
    assert_eq!(probe.seen.borrow().last().map(|(_, count)| *count), Some(1));
}

#[gpui::test]
fn builders_listeners_and_selectors_follow_the_state(cx: &mut TestAppContext) {
    let (probe, _, window) = open(cx);
    let counter = counter(&probe);
    for event in [
        CounterEvent::Incremented,
        CounterEvent::Incremented,
        CounterEvent::Incremented,
        CounterEvent::Set(1),
        CounterEvent::Set(12),
    ] {
        counter.add(event);
        settle(window, &counter);
    }
    // build_when: odd counts keep showing the last even one.
    assert_eq!(probe.shown.borrow().last(), Some(&12));
    assert!(!probe.shown.borrow().contains(&3));
    assert!(probe.shown.borrow().contains(&2));
    // The selector only saw the part it picked.
    assert_eq!(probe.selected.borrow().last(), Some(&true));
    // listen_when: increases only, once each.
    assert_eq!(*probe.heard.borrow(), [1, 2, 3, 12]);
    // The listener told the other bloc.
    let log = probe.log.borrow().clone().expect("rendered");
    rok_ui::runtime::block_on(log.idle());
    assert_eq!(log.state(), ["three"]);
}

#[gpui::test]
fn removing_the_provider_closes_its_blocs(cx: &mut TestAppContext) {
    let (probe, mounted, window) = open(cx);
    let counter = counter(&probe);
    counter.add(CounterEvent::SlowlyIncremented);

    *mounted.borrow_mut() = false;
    // GPUI drops element state that a frame did not use at the end of the next frame.
    for _ in 0..2 {
        window.update(|window, _| window.refresh());
        window.run_until_parked();
    }
    assert!(counter.is_closed());

    std::thread::sleep(Duration::from_millis(150));
    window.run_until_parked();
    assert_eq!(counter.state(), 0, "the cancelled handler never emitted");
}

#[gpui::test]
fn consumers_listen_and_build(cx: &mut TestAppContext) {
    type Slot = Rc<RefCell<Option<BlocHandle<CounterBloc>>>>;
    struct ConsumerApp(Rc<RefCell<Vec<String>>>, Slot);

    #[component]
    fn Status(events: Rc<RefCell<Vec<String>>>, slot: Slot, cx: &mut Cx) -> impl IntoElement {
        let counter = cx.bloc::<CounterBloc>();
        *slot.borrow_mut() = Some(counter.clone());
        let (heard, built) = (events.clone(), events);
        BlocConsumer::new(
            &counter,
            move |count, _, _| heard.borrow_mut().push(format!("heard {count}")),
            move |count, _, _| {
                built.borrow_mut().push(format!("built {count}"));
                div()
            },
        )
    }

    impl Render for ConsumerApp {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            RepositoryProvider::new()
                .provide::<dyn StepRepository>(Arc::new(FixedStep(5)))
                .child(
                    BlocProvider::bloc(|scope| CounterBloc {
                        steps: scope.repository::<dyn StepRepository>(),
                    })
                    .child(Status::new(self.0.clone(), self.1.clone())),
                )
        }
    }

    cx.update(rok_ui::init);
    cx.executor().allow_parking();
    let events = Rc::new(RefCell::new(Vec::new()));
    let slot: Slot = Rc::default();
    let (view_events, view_slot) = (events.clone(), slot.clone());
    let (_, window) =
        cx.add_window_view(move |_, _| ConsumerApp(view_events.clone(), view_slot.clone()));
    window.run_until_parked();
    assert_eq!(events.borrow().last().map(String::as_str), Some("built 0"));

    let counter = slot.borrow().clone().expect("rendered");
    counter.add(CounterEvent::Incremented);
    settle(window, &counter);
    let events = events.borrow();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.starts_with("heard"))
            .collect::<Vec<_>>(),
        ["heard 5"],
        "the repository's step, heard once"
    );
    assert!(events.iter().any(|event| event == "built 5"));
}
