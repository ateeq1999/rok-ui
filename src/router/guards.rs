//! Guards: code that runs before a route renders and may redirect or refuse it.

use std::{cell::Cell, collections::HashMap, rc::Rc};

use gpui::{AnyWindowHandle, App, Global, SharedString, Task};

use super::{history::CURRENT_WINDOW, Location, Route, Router};

/// A guard: `Ok(())` lets the route render.
pub type Guard = Rc<dyn Fn(&Location, &mut App) -> Result<(), RouteControl>>;
/// A guard that decides in the background (a session check, a permission lookup).
pub type AsyncGuard = Rc<dyn Fn(&Location, &mut App) -> Task<Result<(), RouteControl>>>;

/// Async guard checks per window: the location each guard checked and what it decided.
#[derive(Default)]
pub(super) struct GuardChecks {
    checks: HashMap<(Option<AnyWindowHandle>, SharedString), GuardCheck>,
}

impl Global for GuardChecks {}

struct GuardCheck {
    href: SharedString,
    /// `None` while the guard runs.
    outcome: Option<Result<(), RouteControl>>,
    _task: Task<()>,
}

/// What a guard does instead of rendering the route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteControl {
    /// Go to this path instead, replacing the current history entry.
    Redirect(SharedString),
    /// Render the "not found" route.
    NotFound,
}

impl RouteControl {
    /// Redirect to `path`.
    #[must_use]
    pub fn redirect(path: impl Into<SharedString>) -> Self {
        Self::Redirect(path.into())
    }

    /// Redirect to a typed route.
    #[must_use]
    pub fn redirect_to<R: Route>(route: &R) -> Self {
        Self::Redirect(route.href())
    }
}

impl Router {
    /// The first guard over `location` that refuses it.
    pub(super) fn check_guards(
        &self,
        location: &Location,
        cx: &mut App,
    ) -> Result<(), RouteControl> {
        for (pattern, guard) in &self.guards {
            if pattern.matches(location.path()).is_some() {
                guard(location, cx)?;
            }
        }
        if let Some((_, route)) = self.resolve(location) {
            for guard in &route.guards {
                guard(location, cx)?;
            }
        }
        Ok(())
    }

    /// What the async guards over `location` decided: `None` while one still runs.
    pub(super) fn check_async_guards(
        &self,
        location: &Location,
        cx: &mut App,
    ) -> Option<Result<(), RouteControl>> {
        if self.async_guards.is_empty() {
            return Some(Ok(()));
        }
        let window = CURRENT_WINDOW.with(Cell::get);
        let href = location.href();
        // A check holds for one visit: leaving the location forgets it.
        cx.default_global::<GuardChecks>()
            .checks
            .retain(|(owner, _), check| *owner != window || check.href == href);
        let mut pending = false;
        for (pattern, key, guard) in &self.async_guards {
            if pattern.matches(location.path()).is_none() {
                continue;
            }
            let id = (window, key.clone());
            let outcome = cx
                .default_global::<GuardChecks>()
                .checks
                .get(&id)
                .map(|check| check.outcome.clone());
            match outcome {
                Some(Some(Ok(()))) => {}
                Some(Some(Err(control))) => {
                    // Decided once: a later visit checks again.
                    cx.default_global::<GuardChecks>().checks.remove(&id);
                    return Some(Err(control));
                }
                Some(None) => pending = true,
                None => {
                    let deciding = guard(location, cx);
                    let task_id = id.clone();
                    let task = cx.spawn(async move |cx| {
                        let outcome = deciding.await;
                        cx.update(|cx| {
                            if let Some(check) =
                                cx.default_global::<GuardChecks>().checks.get_mut(&task_id)
                            {
                                check.outcome = Some(outcome);
                            }
                            cx.refresh_windows();
                        })
                        .ok();
                    });
                    cx.default_global::<GuardChecks>().checks.insert(
                        id,
                        GuardCheck {
                            href: href.clone(),
                            outcome: None,
                            _task: task,
                        },
                    );
                    pending = true;
                }
            }
        }
        (!pending).then_some(Ok(()))
    }
}
