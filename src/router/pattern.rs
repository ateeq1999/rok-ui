//! Route patterns (`/users/:id`, `/files/*path`) and what a match captured.

use std::collections::HashMap;

use gpui::SharedString;

use super::{decode_path, normalize, Location};

/// One segment of a route pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Segment {
    Static(SharedString),
    Param(SharedString),
    Wildcard(SharedString),
}

/// A compiled route pattern such as `/users/:id`.
#[derive(Clone, Debug)]
pub(super) struct Pattern {
    pub(super) source: SharedString,
    segments: Vec<Segment>,
}

impl Pattern {
    pub(super) fn parse(source: &str) -> Self {
        let segments = source
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                if let Some(name) = segment.strip_prefix(':') {
                    Segment::Param(name.to_string().into())
                } else if let Some(name) = segment.strip_prefix('*') {
                    Segment::Wildcard(name.to_string().into())
                } else {
                    Segment::Static(segment.to_string().into())
                }
            })
            .collect();
        Self {
            source: normalize(SharedString::from(source.to_string())),
            segments,
        }
    }

    /// The captured parameters and a specificity score, if `path` matches.
    pub(super) fn matches(
        &self,
        path: &str,
    ) -> Option<(HashMap<SharedString, SharedString>, usize)> {
        let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
        let mut params = HashMap::new();
        let mut score = 0;
        for (index, segment) in self.segments.iter().enumerate() {
            match segment {
                Segment::Wildcard(name) => {
                    let rest = parts.get(index..).unwrap_or_default().join("/");
                    params.insert(name.clone(), decode_path(&rest).into());
                    return Some((params, score * 3 + 1));
                }
                Segment::Static(text) => {
                    if parts.get(index) != Some(&text.as_ref()) {
                        return None;
                    }
                    score = score * 3 + 3;
                }
                Segment::Param(name) => {
                    let part = parts.get(index)?;
                    params.insert(name.clone(), decode_path(part).into());
                    score = score * 3 + 2;
                }
            }
        }
        (parts.len() == self.segments.len()).then_some((params, score))
    }
}

/// The route that matched the current location, passed to route builders.
#[derive(Clone, Debug)]
pub struct RouteMatch {
    pub(super) pattern: SharedString,
    pub(super) params: HashMap<SharedString, SharedString>,
    pub(super) location: Location,
}

impl RouteMatch {
    /// The pattern that matched, such as `/users/:id` (empty for "not found").
    #[must_use]
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// A `:name` or `*name` parameter.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<SharedString> {
        self.params.get(name).cloned()
    }

    /// A parameter parsed as `T` (numbers, ids).
    #[must_use]
    pub fn param_as<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.params.get(name)?.parse().ok()
    }

    /// The matched path.
    #[must_use]
    pub fn path(&self) -> SharedString {
        self.location.path.clone()
    }

    /// The first value of query parameter `key`.
    #[must_use]
    pub fn query(&self, key: &str) -> Option<&str> {
        self.location.query(key)
    }

    /// The full location, with its query.
    #[must_use]
    pub fn location(&self) -> &Location {
        &self.location
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plus_in_a_path_segment_is_a_plus() {
        let (params, _) = Pattern::parse("/tags/:tag").matches("/tags/c++").unwrap();
        assert_eq!(params["tag"].as_ref(), "c++");
        let (params, _) = Pattern::parse("/files/*path")
            .matches("/files/a+b/c%20d")
            .unwrap();
        assert_eq!(params["path"].as_ref(), "a+b/c d");
    }
}
