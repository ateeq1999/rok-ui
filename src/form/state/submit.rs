//! Submitting: validate, wait for async validators, then run the handler or point at the
//! first invalid field.

use gpui::{AnyWindowHandle, App, Entity, Pixels, Point, ScrollHandle, Window};

use super::{validate_keys, FormError, FormInner, FormValues, ValidationEvent};

/// A submit that runs once the async validators finish.
pub(super) struct WaitingSubmit {
    /// The window it came from, to focus the first invalid field in.
    window: Option<AnyWindowHandle>,
}

/// Scroll `handle` so the field at `position` is at the top of the visible area.
fn scroll_to_field(handle: &ScrollHandle, position: Point<Pixels>) {
    let top = handle.bounds().origin.y - position.y;
    let lowest = -handle.max_offset().height;
    let offset = handle.offset();
    handle.set_offset(gpui::point(offset.x, top.clamp(lowest, gpui::px(0.))));
}

/// Finish a submit that waited for async validators, once none is running.
pub(super) fn settle_submit<V: FormValues>(form: &Entity<FormInner<V>>, cx: &mut App) {
    let waiting = form.update(cx, |inner, _| {
        if inner.is_validating() {
            None
        } else {
            inner.waiting_submit.take()
        }
    });
    let Some(WaitingSubmit { window }) = waiting else {
        return;
    };
    let finished = window.is_some_and(|window| {
        let form = form.clone();
        window
            .update(cx, |_, window, cx| finish_submit(&form, Some(window), cx))
            .is_ok()
    });
    // No window, or it closed while the validators ran.
    if !finished {
        finish_submit(form, None, cx);
    }
}

pub(crate) fn submit<V: FormValues>(
    form: &Entity<FormInner<V>>,
    window: Option<&mut Window>,
    cx: &mut App,
) {
    if form.read(cx).is_submitting {
        return;
    }
    let keys = form.update(cx, |inner, _| {
        inner.attempts += 1;
        inner.is_submitted = false;
        // A server error about the whole form ("try again later") must not block the retry;
        // field errors stay until their field changes.
        inner.form_errors.remove(&ValidationEvent::Server);
        for meta in inner.meta.values_mut() {
            meta.is_touched = true;
        }
        inner.order.clone()
    });
    for event in [
        ValidationEvent::Change,
        ValidationEvent::Blur,
        ValidationEvent::Submit,
    ] {
        validate_keys(form, cx, &keys, event);
    }
    if form.read(cx).is_validating() {
        // Submit once the async validators are done (`settle_submit`).
        let handle = window.as_ref().map(|window| window.window_handle());
        form.update(cx, |inner, cx| {
            inner.is_submitting = true;
            inner.waiting_submit = Some(WaitingSubmit { window: handle });
            cx.notify();
        });
        return;
    }
    finish_submit(form, window, cx);
}

/// With every validator done: run the submit handler if the form is valid, else focus (and
/// scroll to) the first invalid field.
fn finish_submit<V: FormValues>(
    form: &Entity<FormInner<V>>,
    window: Option<&mut Window>,
    cx: &mut App,
) {
    let inner = form.read(cx);
    if !inner.is_valid() {
        let first_invalid = inner
            .order
            .iter()
            .find(|key| inner.meta.get(*key).is_some_and(|meta| !meta.is_valid()));
        let focus = first_invalid.and_then(|key| inner.focus.get(key)).cloned();
        let position = first_invalid
            .and_then(|key| inner.anchors.get(key))
            .and_then(|anchor| anchor.get());
        if let (Some(handle), Some(position)) = (&inner.scroll, position) {
            scroll_to_field(handle, position);
        }
        form.update(cx, |inner, cx| {
            inner.is_submitting = false;
            cx.notify();
        });
        if let Some(window) = window {
            if let Some(focus) = focus {
                window.focus(&focus);
            }
            window.refresh();
        }
        return;
    }
    let Some(handler) = inner.on_submit.clone() else {
        form.update(cx, |inner, cx| {
            inner.is_submitting = false;
            inner.is_submitted = true;
            cx.notify();
        });
        return;
    };
    let values = inner.values.clone();
    form.update(cx, |inner, cx| {
        inner.is_submitting = true;
        cx.notify();
    });
    let running = handler(values, cx);
    let entity = form.clone();
    cx.spawn(async move |cx| {
        let result = running.await;
        cx.update(|cx| {
            entity.update(cx, |inner, cx| {
                inner.is_submitting = false;
                match result {
                    Ok(()) => {
                        inner.is_submitted = true;
                        inner.discard_draft();
                        inner.apply_form_error(ValidationEvent::Server, FormError::default());
                    }
                    Err(errors) => inner.apply_form_error(ValidationEvent::Server, errors),
                }
                cx.notify();
            });
        })
        .ok();
    })
    .detach();
}
