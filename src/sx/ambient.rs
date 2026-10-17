//! What styles read without a context: the active theme, and whether focus rings show
//! (like CSS `:focus-visible`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::theme::Theme;

thread_local! {
    static THEME_SNAPSHOT: RefCell<Option<Rc<Theme>>> = const { RefCell::new(None) };
}

/// Record the theme that `.sx()` resolves tokens against. Called by
/// [`Theme::set_global`] and [`crate::init`]; GPUI renders on one thread.
#[doc(hidden)]
pub fn set_theme_snapshot(theme: &Theme) {
    THEME_SNAPSHOT.with(|snapshot| *snapshot.borrow_mut() = Some(Rc::new(theme.clone())));
}

/// The theme `.sx()` resolves against (the default theme before `init`).
#[must_use]
pub fn current_theme() -> Rc<Theme> {
    THEME_SNAPSHOT.with(|snapshot| {
        snapshot
            .borrow_mut()
            .get_or_insert_with(|| Rc::new(Theme::default()))
            .clone()
    })
}

/// When `focus:` styles (focus rings) show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusRingMode {
    /// Only after keyboard input, like CSS `:focus-visible`: clicking a button
    /// focuses it without a ring, tabbing to it shows one.
    #[default]
    KeyboardOnly,
    /// Whenever the element is focused, however focus got there.
    Always,
}

thread_local! {
    static FOCUS_RING_MODE: Cell<FocusRingMode> = const { Cell::new(FocusRingMode::KeyboardOnly) };
    /// Whether the most recent input was the keyboard (rather than a pointer).
    static KEYBOARD_MODALITY: Cell<bool> = const { Cell::new(false) };
}

/// Choose when focus rings show. The default is [`FocusRingMode::KeyboardOnly`].
pub fn set_focus_ring_mode(mode: FocusRingMode) {
    FOCUS_RING_MODE.with(|current| current.set(mode));
}

/// Whether `focus:` styles apply right now. Text inputs show their ring whenever
/// they are focused, as browsers do, and do not depend on this.
pub fn focus_visible() -> bool {
    FOCUS_RING_MODE.with(Cell::get) == FocusRingMode::Always || KEYBOARD_MODALITY.with(Cell::get)
}

/// Record whether the latest input came from the keyboard. Returns whether that
/// changed, in which case the window needs a redraw.
#[doc(hidden)]
#[must_use]
pub fn set_keyboard_modality(keyboard: bool) -> bool {
    KEYBOARD_MODALITY.with(|current| current.replace(keyboard) != keyboard)
}
