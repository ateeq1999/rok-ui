//! The one error type every request returns.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

/// Field-level problems from the server: field name (as the API spells it) to its issues.
pub type FieldDetails = BTreeMap<String, Vec<FieldIssue>>;

/// One problem with one field.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldIssue {
    /// Machine-readable: `"too_short"`.
    #[serde(default)]
    pub code: String,
    /// For people: `"Use at least 8 characters."`.
    #[serde(default)]
    pub message: String,
    /// Values the message mentions: `{"min": 8}`.
    #[serde(default)]
    pub params: BTreeMap<String, serde_json::Value>,
}

/// Why a request failed: the server's answer, or why there was none.
///
/// `status` is the HTTP status, or `0` when no response arrived (the network failed, or the
/// request was cancelled). `code` is machine-readable (`"not_found"`, `"network_error"`);
/// `message` is for people. `details` holds per-field problems, usually with a 422.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    /// The HTTP status, or `0` when there was no response.
    pub status: u16,
    /// Machine-readable; `"unknown"` when the server sent none.
    pub code: String,
    /// For people.
    pub message: String,
    /// Per-field problems, when the server sent them.
    pub details: Option<FieldDetails>,
}

impl ApiError {
    /// `code` of a cancelled request.
    pub const CANCELLED: &'static str = "cancelled";
    /// `code` when the server could not be reached.
    pub const NETWORK_ERROR: &'static str = "network_error";

    /// An error with `status`, `code` and `message` and no details.
    #[must_use]
    pub fn new(status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// The error a cancelled request returns. Never shown, never a failure state: check
    /// [`is_cancelled`](Self::is_cancelled) and do nothing.
    #[must_use]
    pub fn cancelled() -> Self {
        Self::new(0, Self::CANCELLED, "The request was cancelled")
    }

    /// Whether the request was cancelled rather than failed.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.status == 0 && self.code == Self::CANCELLED
    }

    /// The error when no response arrived: status `0`, `"network_error"`.
    #[must_use]
    pub fn network() -> Self {
        Self::new(0, Self::NETWORK_ERROR, "Cannot reach the server")
    }

    /// Whether no response arrived.
    #[must_use]
    pub fn is_network(&self) -> bool {
        self.status == 0 && self.code == Self::NETWORK_ERROR
    }

    /// Add per-field details.
    #[must_use]
    pub fn with_details(mut self, details: FieldDetails) -> Self {
        self.details = Some(details);
        self
    }

    /// The first message the server gave for `field`.
    #[must_use]
    pub fn field_message(&self, field: &str) -> Option<&str> {
        self.details
            .as_ref()?
            .get(field)?
            .first()
            .map(|issue| issue.message.as_str())
    }

    /// Parse a failed response: its status, reason phrase and body.
    ///
    /// The body is the `{"error": {"code", "message", "details"}}` envelope; anything else
    /// (HTML, plain text, nothing) falls back to `"unknown"` and a message from the status.
    #[must_use]
    pub fn from_response(status: u16, reason: Option<&str>, body: &[u8]) -> Self {
        let envelope = serde_json::from_slice::<Envelope>(body)
            .ok()
            .map(|envelope| envelope.error);
        let code = envelope
            .as_ref()
            .and_then(|error| error.code.clone())
            .filter(|code| !code.is_empty())
            .unwrap_or_else(|| "unknown".into());
        let message = envelope
            .as_ref()
            .and_then(|error| error.message.clone())
            .filter(|message| !message.trim().is_empty())
            .unwrap_or_else(|| fallback_message(status, reason));
        let details = envelope
            .and_then(|error| error.details)
            .map(|details| {
                details
                    .into_iter()
                    .map(|(field, issues)| (field, issues.into_vec()))
                    .collect::<FieldDetails>()
            })
            .filter(|details| !details.is_empty());
        Self {
            status,
            code,
            message,
            details,
        }
    }
}

/// The message for a status when the server sent none.
fn fallback_message(status: u16, reason: Option<&str>) -> String {
    match status {
        413 => "The file is too large.".into(),
        415 => "That file type is not supported.".into(),
        _ => reason
            .filter(|reason| !reason.is_empty())
            .map_or_else(|| "Request failed".into(), str::to_string),
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

#[derive(Deserialize)]
struct Envelope {
    error: EnvelopeError,
}

#[derive(Deserialize)]
struct EnvelopeError {
    code: Option<String>,
    message: Option<String>,
    details: Option<BTreeMap<String, Issues>>,
}

/// A field's issues: a list, or a single issue.
#[derive(Deserialize)]
#[serde(untagged)]
enum Issues {
    Many(Vec<FieldIssue>),
    One(FieldIssue),
    Message(String),
}

impl Issues {
    fn into_vec(self) -> Vec<FieldIssue> {
        match self {
            Self::Many(issues) => issues,
            Self::One(issue) => vec![issue],
            Self::Message(message) => vec![FieldIssue {
                message,
                ..FieldIssue::default()
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_envelope_is_parsed() {
        let body = br#"{"error": {"code": "validation_failed", "message": "Check the form",
            "details": {"email": [{"code": "taken", "message": "Already registered",
            "params": {"min": 3}}], "name": "Required"}}}"#;
        let error = ApiError::from_response(422, Some("Unprocessable Entity"), body);
        assert_eq!(error.status, 422);
        assert_eq!(error.code, "validation_failed");
        assert_eq!(error.message, "Check the form");
        assert_eq!(error.field_message("email"), Some("Already registered"));
        assert_eq!(error.field_message("name"), Some("Required"));
        let details = error.details.unwrap();
        assert_eq!(details["email"][0].params["min"], 3);
    }

    #[test]
    fn missing_parts_fall_back() {
        let error = ApiError::from_response(404, Some("Not Found"), b"<html>nope</html>");
        assert_eq!(
            (error.code.as_str(), error.message.as_str()),
            ("unknown", "Not Found")
        );
        let error = ApiError::from_response(413, Some("Payload Too Large"), b"");
        assert_eq!(error.message, "The file is too large.");
        let error = ApiError::from_response(415, None, br#"{"error": {}}"#);
        assert_eq!(error.message, "That file type is not supported.");
        let error = ApiError::from_response(599, None, b"");
        assert_eq!(error.message, "Request failed");
        let error = ApiError::from_response(400, None, br#"{"error": {"message": "Bad"}}"#);
        assert_eq!(
            (error.code.as_str(), error.message.as_str()),
            ("unknown", "Bad")
        );
        assert_eq!(error.details, None);
    }

    #[test]
    fn cancelled_and_network_errors_are_recognized() {
        assert!(ApiError::cancelled().is_cancelled());
        assert!(!ApiError::network().is_cancelled());
        assert!(ApiError::network().is_network());
        assert_eq!(ApiError::network().to_string(), "Cannot reach the server");
    }
}
