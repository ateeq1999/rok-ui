//! The signed-in session: a bearer token, and whether the server expired it.

use std::{fmt, path::PathBuf, sync::Arc};

use rok_ui_bloc::{Emitter, Observable, Subscription};

/// What a [`Session`] holds. `Debug` never prints the token.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SessionState {
    /// The bearer token, while signed in.
    pub token: Option<String>,
    /// Set when the server rejected the token (a 401), until the next sign-in.
    pub expired: bool,
}

impl fmt::Debug for SessionState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionState")
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("expired", &self.expired)
            .finish()
    }
}

/// The signed-in session: the token [`HttpClient`](super::HttpClient) sends as
/// `Authorization: Bearer <token>`, and the expired flag it sets when the server answers 401.
///
/// Clones share the session. It is `Send + Sync`, so requests on worker threads read and
/// expire it, and [`Observable`], so views watch it like a bloc: a `BlocListener` on the
/// session sends the app to its sign-in page when it expires.
///
/// The token lives in memory unless the session is [`persisted`](Self::persisted).
#[derive(Clone)]
pub struct Session {
    state: Emitter<SessionState>,
    /// Writes a persisted session to its file while any clone is alive.
    #[allow(dead_code)]
    saving: Option<Arc<Subscription>>,
}

impl Session {
    /// A signed-out session, kept in memory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Emitter::new(SessionState::default()),
            saving: None,
        }
    }

    /// A session that starts signed in with `token`.
    #[must_use]
    pub fn with_token(token: impl Into<String>) -> Self {
        let session = Self::new();
        session.set_token(token);
        session
    }

    /// A session saved to `path` (a JSON file) whenever it changes, and read from it now, so
    /// the user stays signed in across launches.
    ///
    /// The token is stored as plain text: anyone who can read the user's files can read it.
    /// Prefer a short-lived token, and keep the file in the app's config directory
    /// (`rok_ui::persist::config_dir()` with the `persist` feature). OS keychain storage is
    /// not available yet.
    #[must_use]
    pub fn persisted(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let token = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Saved>(&bytes).ok())
            .and_then(|saved| saved.token);
        let mut session = Self::new();
        if let Some(token) = token {
            session.set_token(token);
        }
        let subscription = session.state.subscribe(move |state| {
            let saved = Saved {
                token: state.token.clone(),
            };
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            if let Ok(bytes) = serde_json::to_vec(&saved) {
                std::fs::write(&path, bytes).ok();
            }
        });
        session.saving = Some(Arc::new(subscription));
        session
    }

    /// The current token.
    #[must_use]
    pub fn token(&self) -> Option<String> {
        self.state.state().token
    }

    /// Whether there is a token.
    #[must_use]
    pub fn is_signed_in(&self) -> bool {
        self.state.state().token.is_some()
    }

    /// Whether the server rejected the last token (and nobody signed in since).
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.state.state().expired
    }

    /// Sign in: store `token` and clear the expired flag.
    pub fn set_token(&self, token: impl Into<String>) {
        self.state.emit(SessionState {
            token: Some(token.into()),
            expired: false,
        });
    }

    /// Sign out: forget the token. Not an expiry.
    pub fn clear(&self) {
        self.state.emit(SessionState::default());
    }

    /// The server rejected the token: forget it and set the expired flag. The client calls
    /// this on a 401 (see [`Options::skip_expire`](super::Options::skip_expire)).
    pub fn expire(&self) {
        self.state.emit(SessionState {
            token: None,
            expired: true,
        });
    }

    /// Expire the session only if it still holds `token` (a request sent with an older token
    /// must not sign out a newer sign-in).
    pub(super) fn expire_if_current(&self, token: &str) {
        if self.token().as_deref() == Some(token) {
            self.expire();
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Session")
            .field("state", &self.state.state())
            .finish_non_exhaustive()
    }
}

impl Observable for Session {
    type State = SessionState;

    fn emitter(&self) -> &Emitter<SessionState> {
        &self.state
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    token: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn signing_in_clears_expiry() {
        let session = Session::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorder = seen.clone();
        let _subscription =
            session.subscribe(move |state| recorder.lock().unwrap().push(state.expired));
        session.set_token("a");
        session.expire_if_current("old");
        assert!(
            session.is_signed_in(),
            "an older token does not expire a newer session"
        );
        session.expire_if_current("a");
        assert!(session.is_expired() && !session.is_signed_in());
        session.set_token("b");
        assert!(!session.is_expired());
        assert_eq!(*seen.lock().unwrap(), [false, true, false]);
        assert!(
            !format!("{session:?}").contains("\"b\""),
            "the token is redacted"
        );
    }

    #[test]
    fn persisted_sessions_survive_restarts() {
        let path = std::env::temp_dir().join(format!("rok-ui-session-{}.json", std::process::id()));
        std::fs::remove_file(&path).ok();
        let session = Session::persisted(&path);
        assert!(!session.is_signed_in());
        session.set_token("kept");
        drop(session);
        assert_eq!(Session::persisted(&path).token().as_deref(), Some("kept"));
        Session::persisted(&path).clear();
        assert_eq!(Session::persisted(&path).token(), None);
        std::fs::remove_file(&path).ok();
    }
}
