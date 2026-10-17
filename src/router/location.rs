//! Locations: a path and its decoded query.

use gpui::SharedString;

use super::typed;

/// A path and its query string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub(super) path: SharedString,
    query: Vec<(SharedString, SharedString)>,
}

impl Location {
    /// A path with query pairs, encoded: `build("/search", &[("q", "a b")])` is
    /// `/search?q=a%20b`.
    #[must_use]
    pub fn build(path: &str, query: &[(impl AsRef<str>, impl AsRef<str>)]) -> String {
        let mut built = path.to_string();
        for (index, (key, value)) in query.iter().enumerate() {
            built.push(if index == 0 { '?' } else { '&' });
            built.push_str(&typed::encode(key.as_ref()));
            built.push('=');
            built.push_str(&typed::encode(value.as_ref()));
        }
        built
    }

    /// The path and query string, encoded.
    #[must_use]
    pub fn href(&self) -> SharedString {
        Self::build(&self.path, &self.query).into()
    }

    /// Split `"/users/42?tab=posts"` into its path and decoded query pairs.
    #[must_use]
    pub fn parse(full_path: &str) -> Self {
        let without_fragment = full_path.split('#').next().unwrap_or_default();
        let (path, query) = without_fragment
            .split_once('?')
            .unwrap_or((without_fragment, ""));
        let query = query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                (decode(key).into(), decode(value).into())
            })
            .collect();
        Self {
            path: SharedString::from(path.to_string()),
            query,
        }
    }

    /// The path without the query string: `/users/42`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The first value of query parameter `key`.
    #[must_use]
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(name, _)| name.as_ref() == key)
            .map(|(_, value)| value.as_ref())
    }

    /// Every query parameter, decoded, in order.
    #[must_use]
    pub fn query_pairs(&self) -> &[(SharedString, SharedString)] {
        &self.query
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.href())
    }
}

/// Decode `%XX` escapes and `+` (as a space) in a query component.
pub(super) fn decode(component: &str) -> String {
    let bytes = component.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            b'%' if index + 2 < bytes.len() => {
                match u8::from_str_radix(&component[index + 1..index + 3], 16) {
                    Ok(byte) => {
                        decoded.push(byte);
                        index += 2;
                    }
                    Err(_) => decoded.push(b'%'),
                }
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}
