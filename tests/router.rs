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
