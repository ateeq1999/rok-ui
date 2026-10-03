//! Whether the user is signed in.

use std::sync::atomic::{AtomicBool, Ordering};

static SIGNED_IN: AtomicBool = AtomicBool::new(false);

/// Whether the user is signed in.
pub fn signed_in() -> bool {
    SIGNED_IN.load(Ordering::Relaxed)
}

/// Sign in or out.
pub fn set_signed_in(signed_in: bool) {
    SIGNED_IN.store(signed_in, Ordering::Relaxed);
}
