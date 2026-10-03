//! Motion: `keyframes!`, `.motion(..)`, `use_transition` and `use_presence`.

#![allow(clippy::float_cmp)] // Finished transitions land exactly on their targets.

use std::{cell::Cell, rc::Rc};

use rok_ui::prelude::*;

keyframes! {
    FADE_UP = {
        from: { opacity: 0, y: 2 },
        50%: { background: accent/50 },
        to: { opacity: 1, y: 0, radius: md, width: px(40.) },
    }
}

#[test]
fn keyframes_macro_builds_interpolated_frames() {
    let mut start = FADE_UP.apply(div(), 0.);
    let start = start.style();
    assert_eq!(start.opacity, Some(0.));
    assert_eq!(start.inset.top, Some(px(8.).into()));

    let mut middle = FADE_UP.apply(div(), 0.5);
    let middle = middle.style();
    assert_eq!(middle.opacity, Some(0.5));
    assert_eq!(middle.inset.top, Some(px(4.).into()));
    assert!(middle.background.is_some());
}

struct TransitionView {
    target: f32,
    present: bool,
    seen_value: Rc<Cell<f32>>,
    seen_mounted: Rc<Cell<bool>>,
}

impl Render for TransitionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let value = use_transition("width", window, cx, self.target, Transition::ease_out(500));
        self.seen_value.set(value);
        let presence = use_presence("panel", window, cx, self.present, Transition::ease_out(500));
        self.seen_mounted.set(presence.is_mounted());
        AppRoot::new()
            .child(div().w(px(value)).motion("enter", motion::fade_in()))
            .child(div().child("pulse").motion("pulse", motion::pulse()))
            .child(
                div()
                    .child("bounce")
                    .motion("bounce", motion::bounce().alternate()),
            )
            .when(presence.is_mounted(), |root| {
                root.child(div().opacity(presence.progress()).child("panel"))
            })
    }
}

#[gpui::test]
fn transitions_start_from_the_current_value(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let seen_value = Rc::new(Cell::new(-1.));
    let seen_mounted = Rc::new(Cell::new(false));
    let (view, window_context) = cx.add_window_view(|_, _| TransitionView {
        target: 0.,
        present: true,
        seen_value: seen_value.clone(),
        seen_mounted: seen_mounted.clone(),
    });
    window_context.run_until_parked();
    // The first render shows the target without animating.
    assert_eq!(seen_value.get(), 0.);
    assert!(seen_mounted.get());

    // A new target starts from the current value; 500ms have not passed yet.
    window_context.update(|window, cx| {
        view.update(cx, |view, _| {
            view.target = 100.;
            view.present = false;
        });
        window.refresh();
    });
    window_context.run_until_parked();
    assert!(seen_value.get() < 50., "value {}", seen_value.get());
    // Still mounted while animating out.
    assert!(seen_mounted.get());

    // Reduced motion finishes transitions immediately.
    rok_ui::motion::set_reduced_motion(true);
    window_context.update(|window, _| window.refresh());
    window_context.run_until_parked();
    assert_eq!(seen_value.get(), 100.);
    assert!(!seen_mounted.get());
    rok_ui::motion::set_reduced_motion(false);
}
