//! Typed search params: shareable UI state (filters, tabs, pages) in the location.

use gpui::{App, SharedString};

use super::{location, navigate, replace, Location};

/// A struct read from and written to the query string, implemented with
/// `#[derive(Search)]`.
///
/// Reading never fails: a missing or invalid value falls back to the field's default, so a
/// hand-edited or outdated link still opens.
///
/// ```
/// use rok_ui::router::{Location, Search};
///
/// #[derive(Search, Clone, Debug, PartialEq)]
/// struct NotesSearch {
///     #[search(default = 1)]
///     page: u32,
///     q: Option<String>,
///     #[search(default)]
///     archived: bool,
/// }
///
/// let search = NotesSearch::from_location(&Location::parse("/notes?page=3&q=milk&archived=oops"));
/// assert_eq!(search, NotesSearch { page: 3, q: Some("milk".into()), archived: false });
/// assert_eq!(search.to_query(), vec![("page".to_string(), "3".to_string()), ("q".into(), "milk".into())]);
/// ```
///
/// Fields parse with `FromStr` and print with `Display`; enums implement both. Values equal to
/// their default are left out of the query string.
pub trait Search: Clone + PartialEq + 'static {
    /// The query parameter names this type owns.
    const FIELDS: &'static [&'static str];

    /// The value with every field at its default.
    fn defaults() -> Self;

    /// Read the value from query pairs.
    fn from_query(pairs: &[(SharedString, SharedString)]) -> Self;

    /// The query pairs for this value, leaving out defaults.
    fn to_query(&self) -> Vec<(String, String)>;

    /// Read the value from a location's query string.
    #[must_use]
    fn from_location(location: &Location) -> Self {
        Self::from_query(location.query_pairs())
    }
}

/// The current location's search params as `S`.
pub fn use_search<S: Search>(cx: &mut App) -> S {
    S::from_location(&location(cx))
}

/// Change the current search params, adding a history entry. Other query parameters are kept.
pub fn update_search<S: Search>(cx: &mut App, update: impl FnOnce(&mut S)) {
    if let Some(target) = updated_location::<S>(cx, update) {
        navigate(target, cx);
    }
}

/// Change the current search params, replacing the current history entry.
pub fn replace_search<S: Search>(cx: &mut App, update: impl FnOnce(&mut S)) {
    if let Some(target) = updated_location::<S>(cx, update) {
        replace(target, cx);
    }
}

fn updated_location<S: Search>(cx: &mut App, update: impl FnOnce(&mut S)) -> Option<String> {
    let current = location(cx);
    let before = S::from_location(&current);
    let mut after = before.clone();
    update(&mut after);
    if after == before {
        return None;
    }
    let mut pairs: Vec<(String, String)> = current
        .query_pairs()
        .iter()
        .filter(|(key, _)| !S::FIELDS.contains(&key.as_ref()))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    pairs.extend(after.to_query());
    Some(Location::build(current.path(), &pairs))
}

/// Parse one query value, for `#[derive(Search)]`.
#[doc(hidden)]
#[must_use]
pub fn parse_value<T: std::str::FromStr>(
    pairs: &[(SharedString, SharedString)],
    key: &str,
) -> Option<T> {
    pairs
        .iter()
        .find(|(name, _)| name.as_ref() == key)
        .and_then(|(_, value)| value.parse().ok())
}
