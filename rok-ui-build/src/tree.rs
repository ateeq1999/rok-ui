//! The route tree: file names to path segments (TanStack Router's conventions), and the
//! nodes the files become.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use super::{read_declaration, Error};

/// One part of a route file's path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Segment {
    Static(String),
    Param(String),
    Splat,
    Pathless(String),
    Index,
}

impl Segment {
    pub(super) fn parse(part: &str) -> Option<Self> {
        if let Some(inner) = part
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            return Some(Self::Static(inner.to_string()));
        }
        if part.starts_with('(') && part.ends_with(')') {
            return None;
        }
        Some(match part {
            "$" => Self::Splat,
            "index" => Self::Index,
            _ if part.starts_with('$') => Self::Param(part[1..].to_string()),
            _ if part.starts_with('_') && !part.starts_with("__") => {
                Self::Pathless(part[1..].to_string())
            }
            _ => Self::Static(part.to_string()),
        })
    }

    fn is_path(&self) -> bool {
        matches!(self, Self::Static(_) | Self::Param(_) | Self::Splat)
    }
}

/// Split a file stem on dots that are not inside `[..]`.
pub(super) fn split_flat(stem: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut depth = 0_u32;
    for character in stem.chars() {
        match character {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            '.' if depth == 0 => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = parts.last_mut() {
            last.push(character);
        }
    }
    parts
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Special {
    None,
    Root,
    NotFound,
}

/// One route file.
#[derive(Debug)]
pub(super) struct Node {
    /// The path relative to the routes directory, with `/` separators.
    pub(super) relative: String,
    pub(super) absolute: PathBuf,
    pub(super) segments: Vec<Segment>,
    pub(super) special: Special,
    params: BTreeMap<String, String>,
    pub(super) layout: bool,
}

impl Node {
    pub(super) fn read(root: &Path, relative: &Path) -> Result<Self, Error> {
        let relative_text = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let stem = relative_text.strip_suffix(".rs").unwrap_or(&relative_text);
        let special = match stem {
            "__root" => Special::Root,
            "__not_found" => Special::NotFound,
            _ => Special::None,
        };
        let segments = if special == Special::None {
            stem.split('/')
                .flat_map(split_flat)
                .filter_map(|part| Segment::parse(&part))
                .collect()
        } else {
            Vec::new()
        };
        let absolute = root.join(relative);
        let source = fs::read_to_string(&absolute)?;
        let declaration = read_declaration(&source).map_err(|message| Error::Route {
            file: relative_text.clone(),
            message,
        })?;
        let node = Self {
            relative: relative_text,
            absolute,
            segments,
            special,
            params: declaration.params,
            layout: declaration.layout,
        };
        node.check()?;
        Ok(node)
    }

    pub(super) fn check(&self) -> Result<(), Error> {
        let expected_layout = match self.special {
            Special::Root => Some(true),
            Special::NotFound => Some(false),
            Special::None if self.segments.contains(&Segment::Index) => Some(false),
            Special::None => None,
        };
        if expected_layout.is_some_and(|layout| layout != self.layout) {
            return Err(Error::Route {
                file: self.relative.clone(),
                message: if self.layout {
                    "this file is a page; declare `component: ..` instead of `layout: ..`".into()
                } else {
                    "`__root.rs` is a layout; declare `layout: ..`".into()
                },
            });
        }
        let in_path: HashSet<String> = self
            .segments
            .iter()
            .filter_map(|segment| match segment {
                Segment::Param(name) => Some(name.clone()),
                Segment::Splat => Some("splat".into()),
                _ => None,
            })
            .collect();
        for name in self.params.keys() {
            if !in_path.contains(name) {
                return Err(Error::Route {
                    file: self.relative.clone(),
                    message: format!(
                        "`params` declares `{name}`, but the path has no `${name}` segment"
                    ),
                });
            }
        }
        Ok(())
    }

    /// The segments a child must start with to be inside this layout.
    pub(super) fn prefix(&self) -> &[Segment] {
        &self.segments
    }

    pub(super) fn pattern(&self) -> String {
        let mut pattern = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Static(text) => {
                    pattern.push('/');
                    pattern.push_str(text);
                }
                Segment::Param(name) => {
                    pattern.push_str("/:");
                    pattern.push_str(name);
                }
                Segment::Splat => pattern.push_str("/*splat"),
                Segment::Pathless(_) | Segment::Index => {}
            }
        }
        if pattern.is_empty() {
            pattern.push('/');
        }
        pattern
    }

    /// The route type's name: `NotesId` for `/notes/:id`, `Index` for `/`.
    pub(super) fn type_name(&self) -> String {
        let mut name = String::new();
        for segment in self.segments.iter().filter(|segment| segment.is_path()) {
            match segment {
                Segment::Static(text) | Segment::Param(text) => name.push_str(&camel_case(text)),
                Segment::Splat => name.push_str("Splat"),
                _ => {}
            }
        }
        if name.is_empty() {
            name.push_str("Index");
        }
        if name.starts_with(|character: char| character.is_ascii_digit()) {
            name.insert(0, 'R');
        }
        name
    }

    /// The path parameters in pattern order, with their types (`String` when undeclared).
    pub(super) fn fields(&self) -> Vec<(String, String)> {
        self.segments
            .iter()
            .filter_map(|segment| match segment {
                Segment::Param(name) => Some(name.clone()),
                Segment::Splat => Some("splat".into()),
                _ => None,
            })
            .map(|name| {
                let ty = self
                    .params
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| "String".into());
                (name, ty)
            })
            .collect()
    }
}

pub(super) fn camel_case(text: &str) -> String {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect()
}

pub(super) fn module_name(relative: &str, used: &mut HashSet<String>) -> String {
    let stem = relative.strip_suffix(".rs").unwrap_or(relative);
    let words: Vec<String> = stem
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let name = format!("file_{}", words.join("_"));
    let mut unique = name.clone();
    let mut counter = 2;
    while !used.insert(unique.clone()) {
        unique = format!("{name}_{counter}");
        counter += 1;
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_follow_tanstack_conventions() {
        assert_eq!(split_flat("notes.$id.edit"), ["notes", "$id", "edit"]);
        assert_eq!(split_flat("[v1.0].index"), ["[v1.0]", "index"]);
        assert_eq!(Segment::parse("$id"), Some(Segment::Param("id".into())));
        assert_eq!(Segment::parse("$"), Some(Segment::Splat));
        assert_eq!(
            Segment::parse("_auth"),
            Some(Segment::Pathless("auth".into()))
        );
        assert_eq!(Segment::parse("(marketing)"), None);
        assert_eq!(
            Segment::parse("[rok-ui]"),
            Some(Segment::Static("rok-ui".into()))
        );
        assert_eq!(camel_case("rok-ui"), "RokUi");
    }
}
