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
    percent_decode(component, true)
}

/// Decode `%XX` escapes in a path segment. A `+` stays a `+`: only queries use it for a space.
pub(super) fn decode_path(component: &str) -> String {
    percent_decode(component, false)
}

fn percent_decode(component: &str, plus_is_space: bool) -> String {
    let bytes = component.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escaped = match bytes.get(index + 1..index + 3) {
            Some(&[high, low]) if bytes[index] == b'%' => hex_value(high)
                .zip(hex_value(low))
                .map(|(high, low)| high << 4 | low),
            _ => None,
        };
        match (escaped, bytes[index]) {
            (Some(byte), _) => {
                decoded.push(byte);
                index += 2;
            }
            (None, b'+') if plus_is_space => decoded.push(b' '),
            (None, byte) => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(digit: u8) -> Option<u8> {
    char::from(digit)
        .to_digit(16)
        .and_then(|value| u8::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percent_before_a_multibyte_character_is_kept() {
        // `%a` then `é`: the two bytes after `%` are not a whole escape.
        assert_eq!(decode("%a\u{e9}"), "%a\u{e9}");
        assert_eq!(decode("%\u{e9}x"), "%\u{e9}x");
        assert_eq!(
            Location::parse("/s?q=%a\u{e9}").query("q"),
            Some("%a\u{e9}")
        );
    }
}
