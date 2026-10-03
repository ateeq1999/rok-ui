//! Queries, mutations, procedures, memoize and Suspense against a real background runtime.

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
    memoize,
    prelude::*,
    procedure,
    query::{self, ErrorBoundary, MutationOptions, QueryError, QueryKey, QueryOptions, Suspense},
    query_key,
};

/// Run the app until `done` holds, letting background tasks finish on their threads.
fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut App) -> bool) {
    for _ in 0..400 {
        cx.run_until_parked();
        if cx.update(|_, cx| done(cx)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for background work");
}

fn counting_query(
    key: QueryKey,
    fetches: Arc<AtomicUsize>,
    value: &'static str,
) -> QueryOptions<String> {
    QueryOptions::new(key, move |_| {
        let fetches = fetches.clone();
        async move {
            fetches.fetch_add(1, Ordering::SeqCst);
            Ok::<_, QueryError>(value.to_string())
        }
    })
}

/// A component that reads a query and records what it saw.
#[component]
fn Reader(
    options: QueryOptions<String>,
    seen: Rc<RefCell<Vec<String>>>,
    cx: &mut Cx,
) -> impl IntoElement {
    let result = query::use_query(cx, options);
    if let Some(text) = result.data() {
        seen.borrow_mut().push(text.clone());
    }
    div().child(result.data().cloned().unwrap_or_default())
}

struct Readers {
    queries: Vec<QueryOptions<String>>,
    seen: Rc<RefCell<Vec<String>>>,
}

impl Render for Readers {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().children(
            self.queries
                .iter()
                .map(|options| Reader::new(options.clone(), self.seen.clone())),
        )
    }
}

fn open(
    cx: &mut TestAppContext,
    queries: Vec<QueryOptions<String>>,
) -> (Rc<RefCell<Vec<String>>>, &mut VisualTestContext) {
    cx.update(rok_ui::init);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let view_seen = seen.clone();
    let (_, window) = cx.add_window_view(move |_, _| Readers {
        queries,
        seen: view_seen,
    });
    (seen, window)
}

#[gpui::test]
fn two_readers_of_one_key_share_one_fetch(cx: &mut TestAppContext) {
    let fetches = Arc::new(AtomicUsize::new(0));
    let options = counting_query(query_key!["shared"], fetches.clone(), "hello")
        .stale_time(Duration::from_secs(60));
    let (seen, window) = open(cx, vec![options.clone(), options]);
    settle(window, |_| seen.borrow().len() >= 2);
    assert_eq!(fetches.load(Ordering::SeqCst), 1);
    assert!(seen.borrow().iter().all(|text| text == "hello"));

    // Fresh data is served from the cache on later renders.
    window.update(|window, _| window.refresh());
    window.run_until_parked();
    assert_eq!(fetches.load(Ordering::SeqCst), 1);
}

#[gpui::test]
fn prefix_invalidation_refetches_every_matching_query(cx: &mut TestAppContext) {
    let notes = Arc::new(AtomicUsize::new(0));
    let other = Arc::new(AtomicUsize::new(0));
    let day = Duration::from_secs(86_400);
    let (seen, window) = open(
        cx,
        vec![
            counting_query(query_key!["notes", 1], notes.clone(), "one").stale_time(day),
            counting_query(query_key!["notes", "list"], notes.clone(), "list").stale_time(day),
            counting_query(query_key!["users"], other.clone(), "users").stale_time(day),
        ],
    );
    settle(window, |_| seen.borrow().len() >= 3);
    assert_eq!(
        (notes.load(Ordering::SeqCst), other.load(Ordering::SeqCst)),
        (2, 1)
    );

    window.update(|_, cx| query::invalidate(cx, &query_key!["notes"]));
    settle(window, |_| notes.load(Ordering::SeqCst) == 4);
    settle(window, |cx| {
        query::fetching_count(cx, &QueryKey::root()) == 0
    });
    assert_eq!(other.load(Ordering::SeqCst), 1, "other keys stay cached");
}

#[gpui::test]
fn fetch_query_dedups_and_errors_come_back(cx: &mut TestAppContext) {
    let fetches = Arc::new(AtomicUsize::new(0));
    let options = counting_query(query_key!["imperative"], fetches.clone(), "value")
        .stale_time(Duration::from_secs(60));
    let (_, window) = open(cx, Vec::new());
    let (first, second) = window.update(|_, cx| {
        (
            query::fetch_query(cx, &options),
            query::fetch_query(cx, &options),
        )
    });
    let results = Rc::new(RefCell::new(Vec::new()));
    for task in [first, second] {
        let results = results.clone();
        window
            .update(|_, cx| {
                cx.spawn(async move |_| {
                    let result = task.await;
                    results.borrow_mut().push(result);
                })
            })
            .detach();
    }
    settle(window, |_| results.borrow().len() == 2);
    assert_eq!(fetches.load(Ordering::SeqCst), 1);
    assert!(results
        .borrow()
        .iter()
        .all(|result| result.as_deref().map(String::as_str) == Ok("value")));

    let failing = QueryOptions::new(query_key!["failing"], |_| async {
        Err::<String, _>(QueryError::msg("boom"))
    })
    .retry(1)
    .retry_delay(Duration::from_millis(1));
    let task = window.update(|_, cx| query::fetch_query(cx, &failing));
    let outcome = Rc::new(RefCell::new(None));
    let slot = outcome.clone();
    window
        .update(|_, cx| cx.spawn(async move |_| *slot.borrow_mut() = Some(task.await)))
        .detach();
    settle(window, |_| outcome.borrow().is_some());
    let error = outcome.borrow_mut().take().unwrap().unwrap_err();
    assert_eq!(error.message().as_ref(), "boom");
}

struct Form {
    mutation: Rc<RefCell<Option<query::Mutation<String, (), QueryError>>>>,
    key: QueryKey,
}
impl Render for Form {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key = self.key.clone();
        let mut cx = Cx::new(window, cx);
        let mutation = query::use_mutation(
            &mut cx,
            MutationOptions::new(|_, _: String| async { Err::<(), _>(QueryError::msg("offline")) }),
        )
        .optimistic(move |todo: &String, cache| {
            cache.update(&key, |todos: &mut Vec<String>| todos.push(todo.clone()));
        });
        *self.mutation.borrow_mut() = Some(mutation);
        div()
    }
}

#[gpui::test]
fn optimistic_updates_roll_back_when_a_mutation_fails(cx: &mut TestAppContext) {
    let (_, window) = open(cx, Vec::new());
    let key = query_key!["todos"];
    window.update(|_, cx| query::set_query_data(cx, &key, vec!["write tests".to_string()]));

    let slot = Rc::new(RefCell::new(None));
    let view_slot = slot.clone();
    let view_key = key.clone();
    let (_, form_window) = cx.add_window_view(move |_, _| Form {
        mutation: view_slot,
        key: view_key,
    });
    form_window.run_until_parked();
    let mutation = slot.borrow().clone().expect("rendered");
    form_window.update(|_, cx| {
        mutation.mutate(cx, "ship it".to_string());
        let todos = query::get_query_data::<Vec<String>>(cx, &key).unwrap();
        assert_eq!(todos.len(), 2, "applied before the mutation runs");
    });
    settle(form_window, |cx| {
        query::get_query_data::<Vec<String>>(cx, &key).is_some_and(|todos| todos.len() == 1)
    });
    form_window.run_until_parked();
    let mutation = slot.borrow().clone().unwrap();
    assert!(mutation.is_error());
    assert_eq!(
        mutation.error().map(|error| error.message().to_string()),
        Some("offline".into())
    );
}

#[derive(Clone, Debug)]
struct NewTodo {
    title: String,
}

#[procedure(invalidates = [query_key!["todo-count"]])]
async fn add_todo(_cx: rok_ui::query::TaskCx, input: NewTodo) -> Result<usize, QueryError> {
    if input.title.is_empty() {
        return Err(QueryError::msg("empty title"));
    }
    Ok(input.title.len())
}

#[gpui::test]
fn procedures_run_and_invalidate_their_queries(cx: &mut TestAppContext) {
    use rok_ui::query::Procedure;

    let fetches = Arc::new(AtomicUsize::new(0));
    let count = counting_query(query_key!["todo-count"], fetches.clone(), "3")
        .stale_time(Duration::from_secs(60));
    let (seen, window) = open(cx, vec![count]);
    settle(window, |_| !seen.borrow().is_empty());

    let task = window.update(|_, cx| {
        add_todo.call(
            cx,
            NewTodo {
                title: "four".into(),
            },
        )
    });
    let outcome = Rc::new(RefCell::new(None));
    let slot = outcome.clone();
    window
        .update(|_, cx| cx.spawn(async move |_| *slot.borrow_mut() = Some(task.await)))
        .detach();
    settle(window, |_| outcome.borrow().is_some());
    assert!(matches!(outcome.borrow().as_ref(), Some(Ok(4))));
    settle(window, |_| fetches.load(Ordering::SeqCst) == 2);

    let failing = window.update(|_, cx| {
        add_todo.call(
            cx,
            NewTodo {
                title: String::new(),
            },
        )
    });
    let outcome = Rc::new(RefCell::new(None));
    let slot = outcome.clone();
    window
        .update(|_, cx| cx.spawn(async move |_| *slot.borrow_mut() = Some(failing.await)))
        .detach();
    settle(window, |_| outcome.borrow().is_some());
    assert!(matches!(outcome.borrow().as_ref(), Some(Err(Some(_)))));
}

static LOADS: AtomicUsize = AtomicUsize::new(0);

#[memoize]
async fn expensive(id: u32) -> u32 {
    LOADS.fetch_add(1, Ordering::SeqCst);
    sleep().await;
    id * 2
}

/// Long enough that the three calls overlap.
async fn sleep() {
    rok_ui::gpui::Timer::after(Duration::from_millis(20)).await;
}

#[test]
fn memoized_calls_share_one_future() {
    let first = rok_ui::runtime::spawn(expensive(21));
    let second = rok_ui::runtime::spawn(expensive(21));
    let third = rok_ui::runtime::spawn(expensive(5));
    let results = rok_ui::runtime::block_on(async {
        (
            first.await.unwrap(),
            second.await.unwrap(),
            third.await.unwrap(),
        )
    });
    assert_eq!(results, (42, 42, 10));
    assert_eq!(LOADS.load(Ordering::SeqCst), 2);
    rok_ui::query::memo::invalidate(module_path!());
    assert_eq!(rok_ui::runtime::block_on(expensive(21)), 42);
    assert_eq!(LOADS.load(Ordering::SeqCst), 3);
}

struct Boundaries {
    log: Rc<RefCell<Vec<&'static str>>>,
}

impl Render for Boundaries {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let failing = QueryOptions::new(query_key!["boundary", "failing"], |_| async {
            Err::<String, _>(QueryError::msg("down"))
        });
        let (pending_log, error_log, sibling_log, boundary_log) = (
            self.log.clone(),
            self.log.clone(),
            self.log.clone(),
            self.log.clone(),
        );
        div()
            .child(
                Suspense::new(move |cx| {
                    let text = query::use_suspense_query(cx, failing)?;
                    Ok(div().child(text.to_string()))
                })
                .fallback({
                    pending_log.borrow_mut().push("fallback");
                    div()
                })
                .error(move |_, _| {
                    error_log.borrow_mut().push("error view");
                    div()
                }),
            )
            .child(ErrorBoundary::new(
                move |_| -> Result<_, std::num::ParseIntError> {
                    let value: u8 = "300".parse()?;
                    Ok(div().child(value.to_string()))
                },
                move |_, _| {
                    boundary_log.borrow_mut().push("boundary fallback");
                    div()
                },
            ))
            .child({
                sibling_log.borrow_mut().push("sibling");
                div()
            })
    }
}

#[gpui::test]
fn failures_stay_inside_their_boundaries(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let log = Rc::new(RefCell::new(Vec::new()));
    let view_log = log.clone();
    let (_, window) = cx.add_window_view(move |_, _| Boundaries { log: view_log });
    settle(window, |_| log.borrow().contains(&"error view"));
    let log = log.borrow();
    assert!(log.contains(&"boundary fallback"));
    assert!(log.contains(&"sibling"));
}

#[gpui::test]
fn refocusing_a_window_refetches_marked_queries(cx: &mut TestAppContext) {
    let fetches = Arc::new(AtomicUsize::new(0));
    let options = counting_query(query_key!["focus"], fetches.clone(), "hello")
        .stale_time(Duration::from_secs(3600))
        .refetch_on_window_focus(true);
    let (seen, window) = open(cx, vec![options]);
    settle(window, |_| !seen.borrow().is_empty());
    assert_eq!(fetches.load(Ordering::SeqCst), 1);

    window.deactivate_window();
    window.update(|window, _| window.activate_window());
    settle(window, |_| fetches.load(Ordering::SeqCst) == 2);
}
