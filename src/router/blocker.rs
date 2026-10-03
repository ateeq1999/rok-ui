//! Blocking navigation away from unsaved work.

use gpui::{App, Global, SharedString, WeakEntity};

use crate::Cx;

/// A navigation that a blocker held back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingNavigation {
    /// `navigate(path)`.
    Push(SharedString),
    /// `replace(path)`.
    Replace(SharedString),
    /// `back()`.
    Back,
    /// `forward()`.
    Forward,
}

struct BlockerState {
    when: bool,
}

#[derive(Default)]
struct Blockers {
    active: Vec<WeakEntity<BlockerState>>,
    pending: Option<PendingNavigation>,
    bypass: bool,
}

impl Global for Blockers {}

/// Hold back `navigation` if a mounted blocker is active. Returns whether it was held.
pub(super) fn intercept(navigation: PendingNavigation, cx: &mut App) -> bool {
    let Some(blockers) = cx.try_global::<Blockers>() else {
        return false;
    };
    if blockers.bypass {
        return false;
    }
    let active = blockers
        .active
        .iter()
        .filter_map(WeakEntity::upgrade)
        .any(|state| state.read(cx).when);
    if active {
        let blockers = cx.global_mut::<Blockers>();
        blockers.active.retain(|state| state.upgrade().is_some());
        blockers.pending = Some(navigation);
        cx.refresh_windows();
    }
    active
}

/// Run `navigation` without asking blockers.
fn perform(navigation: PendingNavigation, cx: &mut App) {
    cx.default_global::<Blockers>().bypass = true;
    match navigation {
        PendingNavigation::Push(path) => super::navigate(path, cx),
        PendingNavigation::Replace(path) => super::replace(path, cx),
        PendingNavigation::Back => super::back(cx),
        PendingNavigation::Forward => super::forward(cx),
    }
    cx.default_global::<Blockers>().bypass = false;
}

/// A navigation blocker for the calling component, returned by [`use_blocker`].
#[derive(Clone)]
pub struct Blocker {
    pending: Option<PendingNavigation>,
}

impl Blocker {
    /// Whether a navigation is waiting for the user to confirm or cancel.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.pending.is_some()
    }

    /// The navigation that is waiting.
    #[must_use]
    pub fn pending(&self) -> Option<&PendingNavigation> {
        self.pending.as_ref()
    }

    /// Let the waiting navigation happen (the user confirmed leaving).
    pub fn proceed(&self, cx: &mut App) {
        if let Some(navigation) = cx.default_global::<Blockers>().pending.take() {
            perform(navigation, cx);
        }
    }

    /// Drop the waiting navigation (the user chose to stay).
    pub fn reset(&self, cx: &mut App) {
        cx.default_global::<Blockers>().pending = None;
        cx.refresh_windows();
    }
}

/// Hold back navigation while `when` is true, such as while a form has unsaved changes
/// (TanStack Router's `useBlocker`). Show a confirmation while
/// [`Blocker::is_blocked`], then call [`Blocker::proceed`] or [`Blocker::reset`]:
///
/// ```no_run
/// # use rok_ui::{prelude::*, router};
/// #[component]
/// fn Editor(dirty: bool, cx: &mut Cx) -> impl IntoElement {
///     let blocker = router::use_blocker(cx, dirty);
///     let (stay, leave) = (blocker.clone(), blocker.clone());
///     AlertDialog::new("leave")
///         .open(blocker.is_blocked())
///         .title("Discard changes?")
///         .on_cancel(move |_, _, cx| stay.reset(cx))
///         .on_action(move |_, _, cx| leave.proceed(cx))
/// }
/// ```
///
/// The blocker stops applying when the component is no longer rendered.
#[track_caller]
pub fn use_blocker(cx: &mut Cx, when: bool) -> Blocker {
    let state = cx.window.use_state(cx.app, |_, _| BlockerState { when });
    state.update(cx.app, |state, _| state.when = when);
    let weak = state.downgrade();
    let blockers = cx.app.default_global::<Blockers>();
    if !blockers.active.iter().any(|active| active == &weak) {
        blockers.active.push(weak);
    }
    Blocker {
        pending: if when { blockers.pending.clone() } else { None },
    }
}
