//! `#[derive(Store)]`: per-field signals re-render only the views that read a field.

use std::{cell::Cell, rc::Rc};

use gpui::TestAppContext;
use rok_ui::{prelude::*, state::Store};

#[derive(Store, Clone, Debug, PartialEq)]
struct Todo {
    title: String,
    done: bool,
}

struct TitleView {
    todo: TodoStore,
    renders: Rc<Cell<usize>>,
}

impl Render for TitleView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        div().child(self.todo.title().get_untracked())
    }
}

#[gpui::test]
fn views_re_render_only_for_the_fields_they_read(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    let todo = Todo {
        title: "Write docs".into(),
        done: false,
    }
    .into_store();
    let renders = Rc::new(Cell::new(0));
    let (view_todo, view_renders) = (todo.clone(), renders.clone());
    let (_, window) = cx.add_window_view(move |_, cx| {
        let title = view_todo.title();
        cx.track(move || {
            title.get();
        });
        TitleView {
            todo: view_todo.clone(),
            renders: view_renders.clone(),
        }
    });
    window.run_until_parked();
    let before = renders.get();

    todo.set_done(true);
    window.run_until_parked();
    assert_eq!(renders.get(), before, "`done` is not read by the view");

    todo.update_title(|title| title.push('!'));
    window.run_until_parked();
    assert!(renders.get() > before, "`title` is");

    assert_eq!(
        todo.get(),
        Todo {
            title: "Write docs!".into(),
            done: true
        }
    );
    todo.set(Todo {
        title: "Ship".into(),
        done: false,
    });
    assert_eq!(todo.title().get_untracked(), "Ship");
}
