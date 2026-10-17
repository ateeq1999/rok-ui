//! List fields: [`ArrayFieldApi`] and moving row state with its row.

use std::collections::HashMap;

use gpui::{App, Entity, SharedString, Window};

use super::{validate_keys, FormInner, FormValues, Path, ValidationEvent};

/// A list field of a [`Form`](super::Form), with operations that keep each row's field state with its row.
pub struct ArrayFieldApi<V, E> {
    pub(super) form: Entity<FormInner<V>>,
    pub(super) path: Path<V, Vec<E>>,
    pub(super) items: Vec<E>,
}

impl<V, E: Clone> Clone for ArrayFieldApi<V, E> {
    fn clone(&self) -> Self {
        Self {
            form: self.form.clone(),
            path: self.path.clone(),
            items: self.items.clone(),
        }
    }
}

impl<V: FormValues, E: Clone + 'static> ArrayFieldApi<V, E> {
    /// The number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items, as of this render.
    #[must_use]
    pub fn items(&self) -> &[E] {
        &self.items
    }

    /// The path of item `index`, for `form.field(cx, list.at(index).then(..))`.
    #[must_use]
    pub fn at(&self, index: usize) -> Path<V, E> {
        self.path.at(index)
    }

    fn edit(&self, cx: &mut App, change: impl FnOnce(&mut Vec<E>) -> Option<Rekey>) {
        let key = self.path.key().clone();
        let path = self.path.clone();
        form_rekey_and_set(&self.form, cx, &path, &key, change);
    }

    /// Add `value` at the end.
    pub fn push_value(&self, cx: &mut App, value: E) {
        self.edit(cx, |list| {
            list.push(value);
            Some(Rekey::None)
        });
    }

    /// Insert `value` at `index`, moving later rows down.
    pub fn insert_value(&self, cx: &mut App, index: usize, value: E) {
        self.edit(cx, |list| {
            (index <= list.len()).then(|| {
                list.insert(index, value);
                Rekey::Insert(index)
            })
        });
    }

    /// Remove the item at `index`, moving later rows up.
    pub fn remove_value(&self, cx: &mut App, index: usize) {
        self.edit(cx, |list| {
            (index < list.len()).then(|| {
                list.remove(index);
                Rekey::Remove(index)
            })
        });
    }

    /// Swap two items.
    pub fn swap_values(&self, cx: &mut App, a: usize, b: usize) {
        self.edit(cx, |list| {
            (a < list.len() && b < list.len()).then(|| {
                list.swap(a, b);
                Rekey::Swap(a, b)
            })
        });
    }

    /// Move the item at `from` to `to`.
    pub fn move_value(&self, cx: &mut App, from: usize, to: usize) {
        self.edit(cx, |list| {
            (from < list.len() && to < list.len()).then(|| {
                let item = list.remove(from);
                list.insert(to, item);
                Rekey::Move(from, to)
            })
        });
    }

    /// Replace the item at `index`.
    pub fn replace_value(&self, cx: &mut App, index: usize, value: E) {
        self.edit(cx, |list| {
            list.get_mut(index).map(|item| {
                *item = value;
                Rekey::None
            })
        });
    }

    /// A click handler that adds a clone of `value`.
    pub fn push_handler<Event>(
        &self,
        value: E,
    ) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
        let list = self.clone();
        move |_, _, cx| list.push_value(cx, value.clone())
    }

    /// A click handler that removes the item at `index`.
    pub fn remove_handler<Event>(
        &self,
        index: usize,
    ) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
        let list = self.clone();
        move |_, _, cx| list.remove_value(cx, index)
    }
}

/// How row indices move after a list operation.
#[derive(Clone, Copy)]
pub(super) enum Rekey {
    None,
    Insert(usize),
    Remove(usize),
    Swap(usize, usize),
    Move(usize, usize),
}

impl Rekey {
    /// Where row `index` is now, or `None` if it was removed.
    pub(super) fn map(self, index: usize) -> Option<usize> {
        match self {
            Self::None => Some(index),
            Self::Insert(at) => Some(if index >= at { index + 1 } else { index }),
            Self::Remove(at) => match index.cmp(&at) {
                std::cmp::Ordering::Less => Some(index),
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => Some(index - 1),
            },
            Self::Swap(a, b) => Some(if index == a {
                b
            } else if index == b {
                a
            } else {
                index
            }),
            Self::Move(from, to) => {
                if index == from {
                    Some(to)
                } else if from < to && index > from && index <= to {
                    Some(index - 1)
                } else if to < from && index >= to && index < from {
                    Some(index + 1)
                } else {
                    Some(index)
                }
            }
        }
    }
}

/// `list.3.email` -> `(3, ".email")` for list key `list`.
pub(super) fn row_of<'a>(list_key: &str, key: &'a str) -> Option<(usize, &'a str)> {
    let rest = key.strip_prefix(list_key)?.strip_prefix('.')?;
    let end = rest.find('.').unwrap_or(rest.len());
    Some((rest[..end].parse().ok()?, &rest[end..]))
}

pub(super) fn rekey_map<T>(map: &mut HashMap<SharedString, T>, list_key: &str, rekey: Rekey) {
    let moved: Vec<(SharedString, T)> = map
        .keys()
        .filter(|key| row_of(list_key, key).is_some())
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|key| map.remove(&key).map(|value| (key, value)))
        .collect();
    for (key, value) in moved {
        let Some((row, suffix)) = row_of(list_key, &key) else {
            continue;
        };
        if let Some(row) = rekey.map(row) {
            map.insert(format!("{list_key}.{row}{suffix}").into(), value);
        }
    }
}

pub(super) fn form_rekey_and_set<V: FormValues, E: 'static>(
    form: &Entity<FormInner<V>>,
    cx: &mut App,
    path: &Path<V, Vec<E>>,
    key: &SharedString,
    change: impl FnOnce(&mut Vec<E>) -> Option<Rekey>,
) {
    let changed = form.update(cx, |inner, _| {
        let list = path.get_mut(&mut inner.values)?;
        let rekey = change(list)?;
        inner.revision += 1;
        rekey_map(&mut inner.meta, key, rekey);
        rekey_map(&mut inner.bindings, key, rekey);
        rekey_map(&mut inner.focus, key, rekey);
        rekey_map(&mut inner.anchors, key, rekey);
        // Row validators are bound to row indices; they register again on the next render.
        inner
            .fields
            .retain(|field_key, _| row_of(key, field_key).is_none());
        inner
            .mounted
            .retain(|field_key| row_of(key, field_key).is_none());
        inner
            .async_runs
            .retain(|(field_key, _), _| row_of(key, field_key).is_none());
        let order: Vec<SharedString> = inner
            .order
            .iter()
            .filter_map(|field_key| match row_of(key, field_key) {
                Some((row, suffix)) => rekey
                    .map(row)
                    .map(|row| SharedString::from(format!("{key}.{row}{suffix}"))),
                None => Some(field_key.clone()),
            })
            .collect();
        inner.order = order;
        let meta = inner.meta_mut(key);
        meta.is_dirty = true;
        meta.is_touched = true;
        Some(())
    });
    if changed.is_some() {
        let mut keys = vec![key.clone()];
        keys.extend(form.read(cx).listeners(key));
        validate_keys(form, cx, &keys, ValidationEvent::Change);
    }
}

#[cfg(test)]
mod tests {
    use super::{row_of, Rekey};

    #[test]
    fn rows_follow_list_operations() {
        assert_eq!(row_of("team", "team.3.email"), Some((3, ".email")));
        assert_eq!(row_of("team", "team.3"), Some((3, "")));
        assert_eq!(row_of("team", "teams.3"), None);
        assert_eq!(Rekey::Remove(1).map(0), Some(0));
        assert_eq!(Rekey::Remove(1).map(1), None);
        assert_eq!(Rekey::Remove(1).map(2), Some(1));
        assert_eq!(Rekey::Insert(1).map(1), Some(2));
        assert_eq!(Rekey::Swap(0, 2).map(2), Some(0));
        assert_eq!(Rekey::Move(0, 2).map(0), Some(2));
        assert_eq!(Rekey::Move(0, 2).map(1), Some(0));
        assert_eq!(Rekey::Move(2, 0).map(0), Some(1));
    }
}
