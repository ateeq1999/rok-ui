//! Typed routes: one Rust type per route, so links and navigation are checked by the compiler.

use std::fmt::Write as _;

use gpui::{App, SharedString};

use super::{location, navigate, replace, Pattern, RouteMatch};

/// A route as a Rust type: its pattern and its parameters.
///
/// Declare routes with [`typed_route!`](crate::typed_route), which checks at compile time that
/// the fields and the pattern's parameters agree:
///
/// ```
/// use rok_ui::{router::Route, typed_route};
///
/// typed_route! {
///     /// A note.
///     pub struct NoteRoute = "/notes/:id" { pub id: u64 }
/// }
///
/// assert_eq!(NoteRoute { id: 3 }.href(), "/notes/3");
/// assert_eq!(NoteRoute::parse("/notes/42"), Some(NoteRoute { id: 42 }));
/// assert_eq!(NoteRoute::parse("/notes/abc"), None);
/// ```
pub trait Route: Sized + 'static {
    /// The pattern, such as `/notes/:id` or `/files/*path`.
    const PATTERN: &'static str;

    /// The parameter values by name, as text.
    fn params(&self) -> Vec<(&'static str, String)>;

    /// The route from matched parameters, or `None` when one does not parse.
    fn from_match(route: &RouteMatch) -> Option<Self>;

    /// The path to this route, with parameters percent-encoded.
    fn href(&self) -> SharedString {
        build_path(Self::PATTERN, &self.params())
    }

    /// The route for `path`, if `path` matches the pattern and its parameters parse.
    #[must_use]
    fn parse(path: &str) -> Option<Self> {
        let location = super::Location::parse(path);
        let (params, _) = Pattern::parse(Self::PATTERN).matches(location.path())?;
        Self::from_match(&RouteMatch {
            pattern: Self::PATTERN.into(),
            params,
            location,
        })
    }
}

/// Fill a pattern's `:name` and `*name` segments.
fn build_path(pattern: &str, params: &[(&'static str, String)]) -> SharedString {
    let value = |name: &str| {
        params
            .iter()
            .find(|(param, _)| *param == name)
            .map(|(_, value)| value.as_str())
            .unwrap_or_default()
    };
    let mut path = String::new();
    for segment in pattern.split('/').filter(|segment| !segment.is_empty()) {
        if let Some(name) = segment.strip_prefix(':') {
            path.push('/');
            path.push_str(&encode(value(name)));
        } else if let Some(name) = segment.strip_prefix('*') {
            let rest = value(name);
            if !rest.is_empty() {
                path.push('/');
                let encoded: Vec<String> = rest.split('/').map(encode).collect();
                path.push_str(&encoded.join("/"));
            }
        } else {
            path.push('/');
            path.push_str(segment);
        }
    }
    if path.is_empty() {
        path.push('/');
    }
    path.into()
}

/// Percent-encode everything but RFC 3986 unreserved characters.
pub(crate) fn encode(component: &str) -> String {
    let mut encoded = String::with_capacity(component.len());
    for byte in component.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").ok();
        }
    }
    encoded
}

/// Go to `route`, adding it to the history.
pub fn navigate_to<R: Route>(route: &R, cx: &mut App) {
    navigate(route.href(), cx);
}

/// Go to `route`, replacing the current history entry.
pub fn replace_to<R: Route>(route: &R, cx: &mut App) {
    replace(route.href(), cx);
}

/// The current location as route `R`, if it matches `R`'s pattern and its parameters parse.
pub fn use_params<R: Route>(cx: &mut App) -> Option<R> {
    let location = location(cx);
    R::parse(&location.to_string())
}

/// Whether `pattern` has a `:name` or `*name` segment. Used by `typed_route!` at compile time.
#[doc(hidden)]
#[must_use]
pub const fn pattern_has_param(pattern: &str, name: &str) -> bool {
    let pattern = pattern.as_bytes();
    let name = name.as_bytes();
    let mut start = 0;
    while start < pattern.len() {
        let mut end = start;
        while end < pattern.len() && pattern[end] != b'/' {
            end += 1;
        }
        if end > start
            && (pattern[start] == b':' || pattern[start] == b'*')
            && end - start - 1 == name.len()
        {
            let mut index = 0;
            while index < name.len() && pattern[start + 1 + index] == name[index] {
                index += 1;
            }
            if index == name.len() {
                return true;
            }
        }
        start = end + 1;
    }
    false
}

/// How many `:name` and `*name` segments `pattern` has. Used by `typed_route!`.
#[doc(hidden)]
#[must_use]
pub const fn pattern_param_count(pattern: &str) -> usize {
    let pattern = pattern.as_bytes();
    let mut count = 0;
    let mut index = 0;
    while index < pattern.len() {
        if (pattern[index] == b':' || pattern[index] == b'*')
            && (index == 0 || pattern[index - 1] == b'/')
        {
            count += 1;
        }
        index += 1;
    }
    count
}

/// Declare a typed route: a struct whose fields are the pattern's parameters.
///
/// ```
/// use rok_ui::typed_route;
///
/// typed_route! {
///     /// The home page.
///     pub struct Home = "/";
/// }
/// typed_route! {
///     /// A file, by its path.
///     pub struct FileRoute = "/files/*path" { pub path: String }
/// }
/// # use rok_ui::router::Route;
/// assert_eq!(Home.href(), "/");
/// assert_eq!(FileRoute { path: "docs/a b.txt".into() }.href(), "/files/docs/a%20b.txt");
/// ```
///
/// A field missing from the pattern, or a parameter without a field, is a compile error.
/// Field types implement `FromStr` and `Display`.
#[macro_export]
macro_rules! typed_route {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident = $pattern:literal;
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        $vis struct $name;

        const _: () = ::core::assert!(
            $crate::router::typed::pattern_param_count($pattern) == 0,
            "this pattern has parameters; declare them as fields: `struct Name = \"..\" {{ field: Type }}`"
        );

        impl $crate::router::Route for $name {
            const PATTERN: &'static str = $pattern;

            fn params(&self) -> ::std::vec::Vec<(&'static str, ::std::string::String)> {
                ::std::vec::Vec::new()
            }

            fn from_match(_: &$crate::router::RouteMatch) -> ::core::option::Option<Self> {
                ::core::option::Option::Some(Self)
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(&$crate::router::Route::href(self))
            }
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident = $pattern:literal {
            $($(#[$field_meta:meta])* $field_vis:vis $field:ident : $type:ty),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        $vis struct $name {
            $($(#[$field_meta])* $field_vis $field: $type,)+
        }

        $(
            const _: () = ::core::assert!(
                $crate::router::typed::pattern_has_param($pattern, ::core::stringify!($field)),
                ::core::concat!("the pattern has no `:", ::core::stringify!($field), "` parameter")
            );
        )+
        const _: () = ::core::assert!(
            $crate::router::typed::pattern_param_count($pattern) == [$(::core::stringify!($field)),+].len(),
            "every parameter in the pattern needs a field"
        );

        impl $crate::router::Route for $name {
            const PATTERN: &'static str = $pattern;

            fn params(&self) -> ::std::vec::Vec<(&'static str, ::std::string::String)> {
                ::std::vec![$((::core::stringify!($field), ::std::string::ToString::to_string(&self.$field)),)+]
            }

            fn from_match(route: &$crate::router::RouteMatch) -> ::core::option::Option<Self> {
                ::core::option::Option::Some(Self {
                    $($field: route.param(::core::stringify!($field))?.parse().ok()?,)+
                })
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(&$crate::router::Route::href(self))
            }
        }
    };
}
