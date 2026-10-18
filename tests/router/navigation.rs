//! Guards, async guards, blockers and per-window histories.

use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rok_ui::{
    prelude::*,
    router::{self, Location, RouteControl},
};

use super::NoteRoute;

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

/// An account page behind a guard that asks a (slow) session check.
struct SessionShell {
    valid: Rc<RefCell<bool>>,
    checks: Rc<RefCell<u32>>,
    rendered: Rc<RefCell<Vec<String>>>,
}

impl Render for SessionShell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (valid, checks) = (self.valid.clone(), self.checks.clone());
        let (login, account, pending) = (
            self.rendered.clone(),
            self.rendered.clone(),
            self.rendered.clone(),
        );
        Router::new()
            .route("/", |_, _, _| div())
            .route("/login", move |_, _, _| {
                login.borrow_mut().push("login".into());
                div()
            })
            .route("/account", move |_, _, _| {
                account.borrow_mut().push("account".into());
                div()
            })
            .guard_async("/account", move |_, cx| {
                *checks.borrow_mut() += 1;
                let valid = *valid.borrow();
                let delay = cx
                    .background_executor()
                    .timer(std::time::Duration::from_millis(50));
                cx.spawn(async move |_| {
                    delay.await;
                    if valid {
                        Ok(())
                    } else {
                        Err(RouteControl::redirect("/login"))
                    }
                })
            })
            .pending(move |_, _, _| {
                pending.borrow_mut().push("checking".into());
                div()
            })
    }
}

#[gpui::test]
fn async_guards_hold_the_route_until_they_decide(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let valid = Rc::new(RefCell::new(false));
    let checks = Rc::new(RefCell::new(0));
    let rendered = Rc::new(RefCell::new(Vec::new()));
    let (view_valid, view_checks, view_rendered) =
        (valid.clone(), checks.clone(), rendered.clone());
    let (_, window) = cx.add_window_view(move |_, _| SessionShell {
        valid: view_valid,
        checks: view_checks,
        rendered: view_rendered,
    });
    let decide = |window: &mut gpui::VisualTestContext| {
        window
            .executor()
            .advance_clock(std::time::Duration::from_millis(60));
        window.run_until_parked();
    };

    // A refused check redirects; the account page never renders.
    window.update(|_, cx| router::navigate("/account", cx));
    window.run_until_parked();
    assert_eq!(
        rendered.borrow().last().map(String::as_str),
        Some("checking")
    );
    decide(window);
    window.update(|_, cx| assert_eq!(router::location(cx).path(), "/login"));
    assert!(!rendered.borrow().iter().any(|page| page == "account"));

    // Coming back checks again; an accepted check renders the route.
    *valid.borrow_mut() = true;
    window.update(|_, cx| router::navigate("/account", cx));
    window.run_until_parked();
    assert_eq!(
        rendered.borrow().last().map(String::as_str),
        Some("checking")
    );
    decide(window);
    assert_eq!(
        rendered.borrow().last().map(String::as_str),
        Some("account")
    );
    assert_eq!(*checks.borrow(), 2);

    // Later renders of the same visit reuse the decision.
    window.update(|window, _| window.refresh());
    window.run_until_parked();
    assert_eq!(
        rendered.borrow().last().map(String::as_str),
        Some("account")
    );
    assert_eq!(*checks.borrow(), 2);
}
