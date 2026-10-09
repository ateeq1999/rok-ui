//! `Provide` and `cx.context`: values scoped to a subtree; `cx.keyed`: state that follows a
//! key.

use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;
use rok_ui::{context::Provide, prelude::*};

#[derive(Clone, Debug, PartialEq)]
struct Theme(&'static str);

type Seen = Rc<RefCell<Vec<Option<Theme>>>>;

/// Records the theme it sees each render.
#[component]
fn Reader(seen: Seen, cx: &mut Cx) -> impl IntoElement {
    seen.borrow_mut().push(cx.context::<Theme>());
    div()
}

struct Tree {
    seen: Seen,
}

impl Render for Tree {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let seen = self.seen.clone();
        div()
            .child(Reader::new(seen.clone()))
            .child(
                Provide::new().value(Theme("outer")).child(
                    div().child(Reader::new(seen.clone())).child(
                        Provide::new()
                            .value(Theme("inner"))
                            .child(Reader::new(seen.clone())),
                    ),
                ),
            )
            .child(Reader::new(seen))
    }
}

#[gpui::test]
fn the_nearest_provider_wins_and_scopes_end(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let seen: Seen = Rc::default();
    let view_seen = seen.clone();
    let (_, window) = cx.add_window_view(move |_, _| Tree { seen: view_seen });
    window.run_until_parked();
    let first: Vec<Option<Theme>> = seen.borrow().iter().take(4).cloned().collect();
    assert_eq!(
        first,
        [None, Some(Theme("outer")), Some(Theme("inner")), None,]
    );
}

type Rows = Rc<RefCell<Vec<u64>>>;
type Values = Rc<RefCell<Vec<(u64, u32)>>>;

/// One state per row, keyed by the row id, holding the serial number it was created with.
struct List {
    rows: Rows,
    values: Values,
    next: Rc<RefCell<u32>>,
}

impl Render for List {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut cx = Cx::new(window, cx);
        self.values.borrow_mut().clear();
        for id in self.rows.borrow().iter().copied() {
            let next = self.next.clone();
            let serial = cx.keyed(("row", id as usize)).use_state(move || {
                *next.borrow_mut() += 1;
                *next.borrow()
            });
            let value = serial.get(&cx);
            self.values.borrow_mut().push((id, value));
        }
        div()
    }
}

#[gpui::test]
fn keyed_state_follows_its_key(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let rows: Rows = Rc::new(RefCell::new(vec![1, 2]));
    let values: Values = Rc::default();
    let (view_rows, view_values) = (rows.clone(), values.clone());
    let (view, window) = cx.add_window_view(move |_, _| List {
        rows: view_rows,
        values: view_values,
        next: Rc::default(),
    });
    window.run_until_parked();
    assert_eq!(*values.borrow(), [(1, 1), (2, 2)]);
    // Reverse the rows and add one: each id keeps its own state, the new one gets a new one.
    *rows.borrow_mut() = vec![3, 2, 1];
    window.update(|_, cx| view.update(cx, |_, cx| cx.notify()));
    window.run_until_parked();
    assert_eq!(*values.borrow(), [(3, 3), (2, 2), (1, 1)]);
}
