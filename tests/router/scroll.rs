//! Scroll restoration in scroll areas and scaffolds.

use std::rc::Rc;

use gpui::TestAppContext;
use rok_ui::{
    prelude::*,
    router::{self},
};

/// A long page in a scroll area that restores its position per history entry.
struct LongPage(gpui::ScrollHandle);

impl Render for LongPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        rok_ui::components::ScrollArea::new("page")
            .restore_scroll(true)
            .track_scroll(&self.0)
            .h(px(200.))
            .child(div().h(px(2000.)))
    }
}

#[gpui::test]
fn scroll_areas_return_to_where_each_page_was_left(cx: &mut TestAppContext) {
    use gpui::point;

    cx.update(rok_ui::init);
    let handle = gpui::ScrollHandle::new();
    let view_handle = handle.clone();
    let (_, window) = cx.add_window_view(move |_, _| LongPage(view_handle.clone()));
    let scroll = |window: &mut gpui::VisualTestContext, y: f32| {
        handle.set_offset(point(px(0.), px(-y)));
        window.update(|window, _| window.refresh());
        window.run_until_parked();
    };
    let go = |window: &mut gpui::VisualTestContext, step: fn(&mut App)| {
        window.update(|_, cx| step(cx));
        window.run_until_parked();
        -handle.offset().y
    };
    window.run_until_parked();

    scroll(window, 500.);
    assert_eq!(
        go(window, |cx| router::navigate("/b", cx)),
        px(0.),
        "a new page starts at the top"
    );
    scroll(window, 300.);
    assert_eq!(go(window, router::back), px(500.));
    assert_eq!(go(window, router::forward), px(300.));

    // A new branch forgets the abandoned entries.
    assert_eq!(go(window, router::back), px(500.));
    assert_eq!(go(window, |cx| router::navigate("/c", cx)), px(0.));
}

/// A scaffold whose body restores its scroll position, with a marker at the top of the
/// content that records where it is drawn.
struct ScaffoldPage(Rc<std::cell::Cell<Pixels>>);

impl Render for ScaffoldPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let top = self.0.clone();
        rok_ui::components::Scaffold::new("app")
            .restore_scroll(true)
            .h(px(300.))
            .w(px(400.))
            .child(
                div().h(px(3000.)).child(
                    gpui::canvas(
                        move |bounds, _, _| top.set(bounds.origin.y),
                        |_, (), _, _| {},
                    )
                    .size(px(1.)),
                ),
            )
    }
}

#[gpui::test]
fn scaffold_bodies_restore_their_scroll_position(cx: &mut TestAppContext) {
    use gpui::{point, Modifiers, ScrollDelta, ScrollWheelEvent, TouchPhase};

    cx.update(rok_ui::init);
    let top = Rc::new(std::cell::Cell::new(px(0.)));
    let view_top = top.clone();
    let (_, window) = cx.add_window_view(move |_, _| ScaffoldPage(view_top.clone()));
    window.run_until_parked();
    let start = top.get();
    let scrolled = |window: &mut gpui::VisualTestContext| {
        window.update(|window, _| window.refresh());
        window.run_until_parked();
        start - top.get()
    };

    window.simulate_event(ScrollWheelEvent {
        position: point(px(200.), px(150.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-400.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    let on_first = scrolled(window);
    assert!(on_first > px(0.), "the wheel scrolls the body");

    window.update(|_, cx| router::navigate("/next", cx));
    assert_eq!(scrolled(window), px(0.), "a new page starts at the top");
    window.update(|_, cx| router::back(cx));
    assert_eq!(
        scrolled(window),
        on_first,
        "back returns to where the page was left"
    );
}
