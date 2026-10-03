#![doc = include_str!("../docs/guide/state.md")]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use gpui::{AnyWindowHandle, App, AsyncApp, Context, EntityId, ForegroundExecutor, Window};

pub use rok_ui_hooks as signals;
pub use rok_ui_hooks::{
    batch, create_effect, create_memo, create_signal, create_store, untrack, Effect, Memo,
    ReadSignal, Store, WriteSignal,
};

/// Something to re-render after signals change.
#[derive(Clone, Copy, PartialEq)]
enum Target {
    Entity(EntityId),
    Window(AnyWindowHandle),
}

/// Re-renders requested by effects, applied on the next turn of GPUI's executor.
/// Effects run inside signal writes, often while GPUI is already updating, so
/// they cannot touch the app directly.
#[derive(Default)]
struct Scheduler {
    pending: Vec<Target>,
    flush_queued: bool,
    app: Option<(AsyncApp, ForegroundExecutor)>,
}

thread_local! {
    static SCHEDULER: RefCell<Scheduler> = RefCell::new(Scheduler::default());
}

fn schedule(target: Target) {
    let spawn = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        if !scheduler.pending.contains(&target) {
            scheduler.pending.push(target);
        }
        if scheduler.flush_queued {
            return None;
        }
        let app = scheduler.app.clone()?;
        scheduler.flush_queued = true;
        Some(app)
    });
    if let Some((app, executor)) = spawn {
        executor
            .spawn(async move {
                app.update(flush_pending).ok();
            })
            .detach();
    }
}

fn flush_pending(cx: &mut App) {
    let pending = SCHEDULER.with(|scheduler| {
        let mut scheduler = scheduler.borrow_mut();
        scheduler.flush_queued = false;
        std::mem::take(&mut scheduler.pending)
    });
    for target in pending {
        match target {
            Target::Entity(entity_id) => cx.notify(entity_id),
            Target::Window(handle) => {
                handle.update(cx, |_, window, _| window.refresh()).ok();
            }
        }
    }
}

/// Connect rok-ui-hooks to the app: re-render requests and the `tick` pump.
/// Called by [`crate::init`].
pub(crate) fn init(cx: &mut App) {
    let app = cx.to_async();
    let executor = cx.foreground_executor().clone();
    SCHEDULER.with(|scheduler| scheduler.borrow_mut().app = Some((app, executor)));
    // Timers, debounced values and async resources advance on `tick`. Poll often
    // while tasks are running, a few times a second otherwise.
    cx.spawn(async move |cx| loop {
        signals::tick();
        let delay = if signals::pending_tasks() > 0 {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(50)
        };
        cx.background_executor().timer(delay).await;
    })
    .detach();
}

/// An effect that runs `read` to learn which signals it depends on, and asks for
/// `target` to re-render whenever one of them changes.
fn render_effect(read: impl Fn() + 'static, target: Target) -> Effect {
    let first_run = Cell::new(true);
    create_effect(
        move || {
            read();
            if !first_run.replace(false) {
                schedule(target);
            }
        },
        (),
    )
}

/// Re-render a view when signals change.
pub trait TrackSignals {
    /// Re-render this view whenever a signal (or memo, or store) read by `read`
    /// changes. Reads are tracked again on every change, so conditional reads
    /// work. The tracking lasts as long as the view.
    fn track(&mut self, read: impl Fn() + 'static);
}

impl<V: 'static> TrackSignals for Context<'_, V> {
    fn track(&mut self, read: impl Fn() + 'static) {
        let effect = render_effect(read, Target::Entity(self.entity_id()));
        self.on_release(move |_, _| drop(effect)).detach();
    }
}

/// A signal owned by the calling element, like [`crate::hooks::use_state`]:
/// created on the first render, kept while the element stays rendered, and
/// re-rendering the window when it changes.
///
/// ```ignore
/// #[component]
/// fn Counter(window: &mut Window, cx: &mut App) -> impl IntoElement {
///     let (count, set_count) = use_signal(window, cx, || 0);
///     Button::new("increment")
///         .label(format!("Clicked {} times", count.get()))
///         .on_click(move |_, _, _| set_count.update(|count| *count += 1))
/// }
/// ```
#[track_caller]
pub fn use_signal<T: 'static>(
    window: &mut Window,
    cx: &mut App,
    initial_value: impl FnOnce() -> T,
) -> (ReadSignal<T>, WriteSignal<T>) {
    let entity = window.use_state(cx, |window, _| {
        let (read, write) = create_signal(initial_value());
        let watched = read.clone();
        let effect = render_effect(
            move || watched.with(|_| ()),
            Target::Window(window.window_handle()),
        );
        (read, write, effect)
    });
    let (read, write, _) = entity.read(cx);
    (read.clone(), write.clone())
}

/// Re-render the window whenever a signal read by `read` changes, for components
/// that read signals or stores created elsewhere. Set up on the first render at
/// this call site; `read` keeps the handles it captured then.
#[track_caller]
pub fn use_tracked(window: &mut Window, cx: &mut App, read: impl Fn() + 'static) {
    window.use_state(cx, |window, _| {
        Rc::new(render_effect(read, Target::Window(window.window_handle())))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{prelude::*, TestAppContext};

    struct CounterView {
        count: ReadSignal<i32>,
        renders: Rc<Cell<usize>>,
    }

    impl Render for CounterView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            gpui::div().child(self.count.get_untracked().to_string())
        }
    }

    #[gpui::test]
    fn tracked_views_re_render_when_signals_change(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (count, set_count) = create_signal(0);
        let renders = Rc::new(Cell::new(0));
        let view_renders = renders.clone();
        let (_view, window) = cx.add_window_view(move |_, cx| {
            let watched = count.clone();
            cx.track(move || {
                watched.get();
            });
            CounterView {
                count: count.clone(),
                renders: view_renders.clone(),
            }
        });
        window.run_until_parked();
        let before = renders.get();

        set_count.set(1);
        window.run_until_parked();
        assert!(renders.get() > before, "a change re-renders the view");

        let after_change = renders.get();
        window.run_until_parked();
        assert_eq!(renders.get(), after_change, "no change, no render");
    }
}
