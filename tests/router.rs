//! Router v2: typed routes, typed search params, guards and blockers.

use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rok_ui::{
    prelude::*,
    router::{self, Location, Route, RouteControl, Search},
    typed_route,
};

typed_route! {
    /// The note list.
    pub struct NotesRoute = "/notes";
}

typed_route! {
    /// One note.
    pub struct NoteRoute = "/notes/:id" {
        /// The note id.
        pub id: u64
    }
}

typed_route! {
    /// A file under a folder.
    pub struct FileRoute = "/folders/:folder/*path" {
        /// The folder name.
        pub folder: String,
        /// The path inside the folder.
        pub path: String,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Sort {
    #[default]
    Newest,
    Oldest,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Sort::Newest => "newest",
            Sort::Oldest => "oldest",
        })
    }
}

impl std::str::FromStr for Sort {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, ()> {
        match text {
            "newest" => Ok(Sort::Newest),
            "oldest" => Ok(Sort::Oldest),
            _ => Err(()),
        }
    }
}

#[derive(Search, Clone, Debug, PartialEq)]
struct NotesSearch {
    #[search(default = 1)]
    page: u32,
    q: Option<String>,
    sort: Sort,
    #[search(rename = "tab")]
    section: Option<String>,
}

#[test]
fn typed_routes_build_and_parse_paths() {
    assert_eq!(NotesRoute.href(), "/notes");
    assert_eq!(NoteRoute { id: 7 }.to_string(), "/notes/7");
    assert_eq!(
        NoteRoute::parse("/notes/7?tab=history"),
        Some(NoteRoute { id: 7 })
    );
    assert_eq!(NoteRoute::parse("/notes/seven"), None);
    assert_eq!(NoteRoute::parse("/notes"), None);

    let file = FileRoute {
        folder: "My Docs".into(),
        path: "2026/report ä.pdf".into(),
    };
    assert_eq!(file.href(), "/folders/My%20Docs/2026/report%20%C3%A4.pdf");
    assert_eq!(FileRoute::parse(&file.href()), Some(file));
}

#[test]
fn search_params_fall_back_to_defaults_and_omit_them() {
    let search = NotesSearch::from_location(&Location::parse("/notes?page=x&sort=oldest&tab=a"));
    assert_eq!(
        search,
        NotesSearch {
            page: 1,
            q: None,
            sort: Sort::Oldest,
            section: Some("a".into())
        }
    );
    assert_eq!(
        search.to_query(),
        vec![
            ("sort".to_string(), "oldest".to_string()),
            ("tab".into(), "a".into())
        ]
    );
    assert_eq!(
        NotesSearch::defaults().to_query(),
        Vec::<(String, String)>::new()
    );
    assert_eq!(NotesSearch::FIELDS, &["page", "q", "sort", "tab"]);
}

#[gpui::test]
fn updating_search_keeps_other_parameters(cx: &mut TestAppContext) {
    cx.update(|cx| {
        rok_ui::init(cx);
        router::navigate("/notes?utm=mail&page=2", cx);
        router::update_search::<NotesSearch>(cx, |search| {
            search.page += 1;
            search.q = Some("milk & honey".into());
        });
        let location = router::location(cx);
        assert_eq!(location.query("utm"), Some("mail"));
        assert_eq!(location.query("q"), Some("milk & honey"));
        let search: NotesSearch = router::use_search(cx);
        assert_eq!(search.page, 3);
        router::back(cx);
        assert_eq!(router::use_search::<NotesSearch>(cx).page, 2);

        assert_eq!(router::use_params::<NoteRoute>(cx), None);
        router::navigate_to(&NoteRoute { id: 9 }, cx);
        assert_eq!(
            router::use_params::<NoteRoute>(cx),
            Some(NoteRoute { id: 9 })
        );
    });
}

struct Shell {
    signed_in: Rc<RefCell<bool>>,
    rendered: Rc<RefCell<Vec<String>>>,
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let signed_in = self.signed_in.clone();
        let (account, note, login, missing) = (
            self.rendered.clone(),
            self.rendered.clone(),
            self.rendered.clone(),
            self.rendered.clone(),
        );
        Router::new()
            .route("/login", move |route, _, _| {
                login
                    .borrow_mut()
                    .push(format!("login next={}", route.query("next").unwrap_or("")));
                div()
            })
            .route("/account/*section", move |_, _, _| {
                account.borrow_mut().push("account".into());
                div()
            })
            .route_to(move |route: NoteRoute, _, _| {
                note.borrow_mut().push(format!("note {}", route.id));
                div()
            })
            .not_found(move |route, _, _| {
                missing
                    .borrow_mut()
                    .push(format!("not found {}", route.path()));
                div()
            })
            .guard("/account", move |location, _| {
                if *signed_in.borrow() {
                    Ok(())
                } else {
                    Err(RouteControl::redirect(Location::build(
                        "/login",
                        &[("next", location.path())],
                    )))
                }
            })
    }
}

#[gpui::test]
fn guards_redirect_before_the_guarded_route_renders(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let signed_in = Rc::new(RefCell::new(false));
    let rendered = Rc::new(RefCell::new(Vec::new()));
    let (view_signed_in, view_rendered) = (signed_in.clone(), rendered.clone());
    let (_, window) = cx.add_window_view(move |_, _| Shell {
        signed_in: view_signed_in,
        rendered: view_rendered,
    });
    window.update(|_, cx| router::navigate("/account/billing", cx));
    window.run_until_parked();
    assert!(!rendered.borrow().iter().any(|page| page == "account"));
    assert_eq!(
        rendered.borrow().last().map(String::as_str),
        Some("login next=/account/billing")
    );
    window.update(|_, cx| assert_eq!(router::location(cx).path(), "/login"));

    *signed_in.borrow_mut() = true;
    window.update(|_, cx| router::navigate("/account", cx));
    window.run_until_parked();
    assert_eq!(
        rendered.borrow().last().map(String::as_str),
        Some("account")
    );

    window.update(|_, cx| router::navigate("/notes/abc", cx));
    window.run_until_parked();
    assert_eq!(
        rendered.borrow().last().map(String::as_str),
        Some("not found /notes/abc")
    );
    window.update(|_, cx| router::navigate_to(&NoteRoute { id: 4 }, cx));
    window.run_until_parked();
    assert_eq!(rendered.borrow().last().map(String::as_str), Some("note 4"));
}

struct Editor {
    dirty: Rc<RefCell<bool>>,
    blocker: Rc<RefCell<Option<router::Blocker>>>,
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cx = Cx::new(window, cx);
        *self.blocker.borrow_mut() = Some(router::use_blocker(&mut cx, *self.dirty.borrow()));
        div()
    }
}

#[gpui::test]
fn blockers_hold_navigation_until_confirmed(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let dirty = Rc::new(RefCell::new(true));
    let blocker = Rc::new(RefCell::new(None));
    let (view_dirty, view_blocker) = (dirty.clone(), blocker.clone());
    let (_, window) = cx.add_window_view(move |_, _| Editor {
        dirty: view_dirty,
        blocker: view_blocker,
    });
    window.run_until_parked();
    window.update(|_, cx| {
        router::navigate("/edit", cx);
        assert_eq!(router::location(cx).path(), "/", "held back while dirty");
    });
    window.run_until_parked();
    let current = blocker.borrow().clone().unwrap();
    assert_eq!(
        current.pending(),
        Some(&router::PendingNavigation::Push("/edit".into()))
    );

    // Staying drops the navigation.
    window.update(|_, cx| current.reset(cx));
    window.run_until_parked();
    assert!(!blocker.borrow().as_ref().unwrap().is_blocked());

    // Confirming lets it through.
    window.update(|_, cx| router::navigate("/elsewhere", cx));
    window.run_until_parked();
    let current = blocker.borrow().clone().unwrap();
    window.update(|_, cx| {
        current.proceed(cx);
        assert_eq!(router::location(cx).path(), "/elsewhere");
    });

    // A clean editor does not block.
    *dirty.borrow_mut() = false;
    window.update(|window, _| window.refresh());
    window.run_until_parked();
    window.update(|_, cx| {
        router::navigate("/free", cx);
        assert_eq!(router::location(cx).path(), "/free");
    });
}

#[test]
fn route_fields_must_match_the_pattern() {
    trybuild::TestCases::new().compile_fail("tests/ui-router/*.rs");
}

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

struct PerWindow {
    seen: Rc<RefCell<Vec<String>>>,
}

impl Render for PerWindow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let seen = self.seen.clone();
        Router::new().route("/*path", move |_, _, cx| {
            // Components inside a window's router read that window's history.
            seen.borrow_mut()
                .push(router::location(cx).path().to_string());
            div()
        })
    }
}

#[gpui::test]
fn windows_keep_their_own_histories(cx: &mut TestAppContext) {
    cx.update(|cx| {
        rok_ui::init(cx);
        router::set_per_window_history(true, cx);
    });
    let (seen_a, seen_b) = (
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(Vec::new())),
    );
    let (view_a, view_b) = (seen_a.clone(), seen_b.clone());
    let (_, window_a) = cx.add_window_view(move |_, _| PerWindow { seen: view_a });
    let window_a = window_a.window_handle();
    let (_, window_b) = cx.add_window_view(move |_, _| PerWindow { seen: view_b });
    let window_b = window_b.window_handle();

    cx.update(|cx| {
        router::with_window(window_a, || router::navigate("/inbox", cx));
        router::with_window(window_b, || {
            router::navigate("/settings", cx);
            router::navigate("/settings/profile", cx);
        });
        router::with_window(window_a, || {
            assert_eq!(router::location(cx).path(), "/inbox");
        });
        router::with_window(window_b, || {
            assert_eq!(router::location(cx).path(), "/settings/profile");
            router::back(cx);
            assert_eq!(router::location(cx).path(), "/settings");
        });
        router::with_window(window_a, || assert!(router::can_go_back(cx)));
    });
    cx.run_until_parked();
    cx.update(gpui::App::refresh_windows);
    cx.run_until_parked();
    assert_eq!(seen_a.borrow().last().map(String::as_str), Some("/inbox"));
    assert_eq!(
        seen_b.borrow().last().map(String::as_str),
        Some("/settings")
    );
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
