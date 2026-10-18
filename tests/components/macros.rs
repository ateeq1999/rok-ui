//! The `#[component]` macro's generated API, default props, the `State` hook handle and
//! keyed hook state.

use std::{cell::RefCell, rc::Rc};

use rok_ui::prelude::*;

/// A component exercising every kind of macro parameter.
#[component]
pub(super) fn Greeting(
    name: SharedString,
    #[prop(optional)] excited: bool,
    #[prop(optional)] title: Option<SharedString>,
    #[prop(optional)] on_wave: Option<EventHandler<ClickEvent>>,
    #[children] children: Vec<AnyElement>,
    #[style] style_overrides: StyleRefinement,
    cx: &mut App,
) -> impl IntoElement {
    let punctuation = if excited { "!" } else { "." };
    div()
        .text_color(cx.theme().colors.foreground)
        .when_some(title, gpui::ParentElement::child)
        .child(format!("Hello, {name}{punctuation}"))
        .when(on_wave.is_some(), |greeting| greeting.child("(waves)"))
        .children(children)
        .apply_style_overrides(&style_overrides)
}

#[test]
fn component_macro_generates_a_builder_api() {
    let greeting = Greeting::new("Ada")
        .excited(true)
        .title("Welcome")
        .on_wave(|_, _, _| {})
        .child("Glad you are here")
        .w_full();

    assert_eq!(greeting.name.as_ref(), "Ada");
    assert!(greeting.excited);
    assert_eq!(
        greeting.title.as_ref().map(std::convert::AsRef::as_ref),
        Some("Welcome")
    );
    assert!(greeting.on_wave.is_some());
    assert_eq!(greeting.children.len(), 1);

    let plain_greeting = Greeting::new("Grace");
    assert!(!plain_greeting.excited);
    assert!(plain_greeting.title.is_none());
    assert_eq!(plain_greeting.children.len(), 0);
}

#[gpui::test]
fn state_handle_sets_and_updates(cx: &mut gpui::TestAppContext) {
    let counter_entity = cx.new(|_| 1_i32);
    let counter = State::from_entity(counter_entity);
    cx.update(|cx| {
        assert_eq!(counter.get(cx), 1);
        counter.set(5, cx);
        assert_eq!(counter.get(cx), 5);
        counter.update(cx, |value| *value *= 2);
        assert_eq!(*counter.read(cx), 10);
    });
}

/// `#[default]` and `#[default(expr)]` props start at their defaults and get builders.
#[component]
fn Badge2(
    label: SharedString,
    #[default] count: u32,
    #[default(px(32.))] size: Pixels,
    #[default(3)] max: u8,
) -> impl IntoElement {
    div().w(size).child(format!("{label} {count}/{max}"))
}

#[test]
fn default_props_start_at_their_defaults() {
    let badge = Badge2::new("Inbox");
    assert_eq!((badge.count, badge.size, badge.max), (0, px(32.), 3));
    let badge = Badge2::new("Inbox").count(5_u32).size(px(10.)).max(9_u8);
    assert_eq!((badge.count, badge.size, badge.max), (5, px(10.), 9));
}

/// Remembers the name it first rendered with, and reports what it remembers.
#[component]
fn Remembering(
    name: SharedString,
    seen: Rc<RefCell<Vec<(String, String)>>>,
    cx: &mut Cx,
) -> impl IntoElement {
    let first = cx.use_state({
        let name = name.clone();
        move || name
    });
    seen.borrow_mut()
        .push((name.to_string(), first.get(cx).to_string()));
    div().child(name)
}

struct KeyedList {
    names: Rc<RefCell<Vec<&'static str>>>,
    seen: Rc<RefCell<Vec<(String, String)>>>,
}

impl Render for KeyedList {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let names = self.names.borrow().clone();
        let seen = self.seen.clone();
        div().children(view! {
            #[key(name)]
            for name in names {
                Remembering(name, seen.clone())
            }
        })
    }
}

#[gpui::test]
fn keyed_loops_keep_hook_state_with_their_items(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let names = Rc::new(RefCell::new(vec!["a", "b", "c"]));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (view_names, view_seen) = (names.clone(), seen.clone());
    let (_, window) = cx.add_window_view(move |_, _| KeyedList {
        names: view_names,
        seen: view_seen,
    });
    window.run_until_parked();
    *names.borrow_mut() = vec!["c", "a", "b"];
    seen.borrow_mut().clear();
    window.update(|window, _| window.refresh());
    window.run_until_parked();
    let seen = seen.borrow();
    assert!(!seen.is_empty());
    for (name, remembered) in seen.iter() {
        assert_eq!(name, remembered, "state moved with its item");
    }
}
