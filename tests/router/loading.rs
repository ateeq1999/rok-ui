//! Loaders: once per location, on preload, cancelled when the user leaves, and the pending
//! indicator.

use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rok_ui::{
    prelude::*,
    router::{self, Route},
};

use super::NoteRoute;

struct Loading {
    loads: Rc<RefCell<Vec<u64>>>,
}

impl Render for Loading {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let loads = self.loads.clone();
        Router::new()
            .route_to(|note: NoteRoute, _, _| div().child(note.id.to_string()))
            .loader_to(move |note: &NoteRoute, _| loads.borrow_mut().push(note.id))
    }
}

#[gpui::test]
fn loaders_run_once_per_location_and_on_preload(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let loads = Rc::new(RefCell::new(Vec::new()));
    let view_loads = loads.clone();
    let (_, window) = cx.add_window_view(move |_, _| Loading { loads: view_loads });
    window.update(|_, cx| router::navigate_to(&NoteRoute { id: 1 }, cx));
    window.run_until_parked();
    window.update(|window, _| window.refresh());
    window.run_until_parked();
    assert_eq!(*loads.borrow(), [1], "re-renders do not reload");

    window.update(|_, cx| router::navigate_to(&NoteRoute { id: 2 }, cx));
    window.run_until_parked();
    window.update(|_, cx| router::preload(&NoteRoute { id: 3 }.href(), cx));
    assert_eq!(*loads.borrow(), [1, 2, 3]);
}

fn slow_note(id: u64) -> rok_ui::query::QueryOptions<String> {
    rok_ui::query::QueryOptions::new(rok_ui::query_key!["slow-note", id], move |_| async move {
        rok_ui::gpui::Timer::after(std::time::Duration::from_millis(150)).await;
        Ok::<_, rok_ui::query::QueryError>(format!("Note {id}"))
    })
}

struct SlowLoading;

impl Render for SlowLoading {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Router::new()
            .route_to(|note: NoteRoute, _, _| div().child(note.id.to_string()))
            .route("/elsewhere", |_, _, _| div())
            .loader_to(|note: &NoteRoute, cx| {
                rok_ui::query::prefetch_query(cx, &slow_note(note.id));
            })
    }
}

#[gpui::test]
fn navigating_away_cancels_the_loader(cx: &mut TestAppContext) {
    use rok_ui::query::{self, QueryKey};

    cx.update(rok_ui::init);
    let (_, window) = cx.add_window_view(|_, _| SlowLoading);
    window.update(|_, cx| router::navigate_to(&NoteRoute { id: 1 }, cx));
    window.run_until_parked();
    window.update(|_, cx| assert_eq!(query::fetching_count(cx, &QueryKey::root()), 1));

    window.update(|_, cx| router::navigate("/elsewhere", cx));
    window.run_until_parked();
    window.update(|_, cx| assert_eq!(query::fetching_count(cx, &QueryKey::root()), 0));

    // The cancelled fetch never lands in the cache.
    std::thread::sleep(std::time::Duration::from_millis(250));
    window.run_until_parked();
    window.update(|_, cx| {
        assert!(
            query::get_query_data::<String>(cx, &rok_ui::query_key!["slow-note", 1_u64]).is_none()
        );
    });
}

/// A note loader that takes 600ms.
struct SlowerLoading;

impl Render for SlowerLoading {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Router::new()
            .route_to(|note: NoteRoute, _, _| div().child(note.id.to_string()))
            .route("/elsewhere", |_, _, _| div())
            .loader_to(|note: &NoteRoute, cx| {
                let id = note.id;
                let query = rok_ui::query::QueryOptions::new(
                    rok_ui::query_key!["slower-note", id],
                    move |_| async move {
                        rok_ui::gpui::Timer::after(std::time::Duration::from_millis(600)).await;
                        Ok::<_, rok_ui::query::QueryError>(format!("Note {id}"))
                    },
                )
                .stale_time(std::time::Duration::from_secs(60));
                rok_ui::query::prefetch_query(cx, &query);
            })
    }
}

/// The slower note router, with a pending indicator that records what it showed.
struct PendingView(Rc<RefCell<Vec<(bool, bool)>>>);

impl Render for PendingView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use std::time::Duration;
        let pending = router::use_pending(
            &mut Cx::new(window, cx),
            Duration::from_millis(150),
            Duration::from_millis(1000),
        );
        let loading =
            router::with_window(window.window_handle(), || router::load_state(cx).is_loading);
        self.0.borrow_mut().push((loading, pending));
        div().child(cx.new(|_| SlowerLoading))
    }
}

#[gpui::test]
fn pending_indicators_wait_before_showing_and_stay_a_minimum(cx: &mut TestAppContext) {
    use std::time::Duration;

    cx.update(rok_ui::init);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let view_seen = seen.clone();
    let (_, window) = cx.add_window_view(move |_, _| PendingView(view_seen.clone()));
    let frame = |window: &mut gpui::VisualTestContext, wait: u64| {
        std::thread::sleep(Duration::from_millis(wait));
        window.update(|window, _| window.refresh());
        window.run_until_parked();
        *seen.borrow().last().expect("rendered")
    };

    // The fetch takes 600ms; the indicator waits 150ms and then stays at least 1000ms.
    window.update(|_, cx| router::navigate_to(&NoteRoute { id: 7 }, cx));
    window.run_until_parked();
    assert_eq!(frame(window, 0), (true, false), "a fast load shows nothing");
    assert_eq!(
        frame(window, 300),
        (true, true),
        "a slow one shows after the delay"
    );
    let (loading, pending) = frame(window, 500);
    assert!(
        !loading && pending,
        "kept for the minimum time after loading ends"
    );
    assert_eq!(frame(window, 700), (false, false));

    // Fresh data loads nothing: no indicator at all.
    window.update(|_, cx| router::navigate("/elsewhere", cx));
    window.run_until_parked();
    window.update(|_, cx| router::navigate_to(&NoteRoute { id: 7 }, cx));
    assert_eq!(frame(window, 300), (false, false));
}

/// Links that preload when they come into view, one above the fold and one far below.
struct VisibleLinks(Rc<RefCell<Vec<u64>>>, gpui::ScrollHandle);

impl Render for VisibleLinks {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let loaded = self.0.clone();
        div()
            .child(
                Router::new()
                    .route_to(|note: NoteRoute, _, _| div().child(note.id.to_string()))
                    .route("/", |_, _, _| div())
                    .loader_to(move |note: &NoteRoute, _| loaded.borrow_mut().push(note.id)),
            )
            .child(
                rok_ui::components::ScrollArea::new("list")
                    .track_scroll(&self.1)
                    .h(px(200.))
                    .child(
                        Link::to(&NoteRoute { id: 1 })
                            .preload_visible(true)
                            .child("First"),
                    )
                    .child(div().h(px(2000.)))
                    .child(
                        Link::to(&NoteRoute { id: 2 })
                            .preload_visible(true)
                            .child("Last"),
                    ),
            )
    }
}

#[gpui::test]
fn links_preload_when_they_come_into_view(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let loaded = Rc::new(RefCell::new(Vec::new()));
    let handle = gpui::ScrollHandle::new();
    let (view_loaded, view_handle) = (loaded.clone(), handle.clone());
    let (_, window) =
        cx.add_window_view(move |_, _| VisibleLinks(view_loaded.clone(), view_handle.clone()));
    let frame = |window: &mut gpui::VisualTestContext| {
        window.update(|window, _| window.refresh());
        window.run_until_parked();
    };
    frame(window);
    frame(window);
    assert_eq!(*loaded.borrow(), [1], "only the visible link, once");

    handle.set_offset(gpui::point(px(0.), px(-1900.)));
    frame(window);
    assert_eq!(
        *loaded.borrow(),
        [1, 2],
        "the other one once it is scrolled into view"
    );
}

/// Pages that slide in from above, with a marker recording where the page is drawn.
struct SlidingPages(Rc<std::cell::Cell<Pixels>>);

impl Render for SlidingPages {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let top = self.0.clone();
        Router::new()
            .route("/*page", move |_, _, _| {
                let top = top.clone();
                div().h(px(100.)).child(
                    gpui::canvas(
                        move |bounds, _, _| top.set(bounds.origin.y),
                        |_, (), _, _| {},
                    )
                    .size(px(1.)),
                )
            })
            .transition(
                rok_ui::motion::presets::slide_in(rok_ui::motion::MotionSide::Top, 10.)
                    .duration_ms(400),
            )
    }
}

#[gpui::test]
fn route_transitions_play_when_the_location_changes(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let top = Rc::new(std::cell::Cell::new(px(0.)));
    let view_top = top.clone();
    let (_, window) = cx.add_window_view(move |_, _| SlidingPages(view_top.clone()));
    let frame = |window: &mut gpui::VisualTestContext, wait: u64| {
        std::thread::sleep(std::time::Duration::from_millis(wait));
        window.update(|window, _| window.refresh());
        window.run_until_parked();
        top.get()
    };
    let rest = frame(window, 500);

    window.update(|_, cx| router::navigate("/next", cx));
    window.run_until_parked();
    assert!(
        frame(window, 0) < rest,
        "the new page starts above its place"
    );
    assert_eq!(frame(window, 500), rest, "and slides into it");
}
