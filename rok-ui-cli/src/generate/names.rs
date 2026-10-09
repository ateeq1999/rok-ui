//! Names: `notes`, `Notes`, `notes_bloc` and `NotesBloc` all mean the feature `notes`.

/// Rust's keywords, which cannot be module or type names.
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "try", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield",
];

/// Split `NotesBloc`, `notes_bloc` or `notes-bloc` into lowercase words.
fn words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let characters: Vec<char> = name.chars().collect();
    for (index, &character) in characters.iter().enumerate() {
        if character == '_' || character == '-' || character == ' ' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let starts_word = character.is_uppercase()
            && !current.is_empty()
            && (characters[index - 1].is_lowercase()
                || characters[index - 1].is_ascii_digit()
                || characters
                    .get(index + 1)
                    .is_some_and(|next| next.is_lowercase()));
        if starts_word {
            words.push(std::mem::take(&mut current));
        }
        current.extend(character.to_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// `notes_bloc`.
pub fn snake(name: &str) -> String {
    words(name).join("_")
}

/// `NotesBloc`.
pub fn pascal(name: &str) -> String {
    words(name)
        .iter()
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| first.to_uppercase().chain(characters).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}

/// `name` without a trailing `bloc`, `cubit`, `repository` or `provider` word.
pub fn base(name: &str, suffixes: &[&str]) -> String {
    let mut parts = words(name);
    if parts.len() > 1
        && parts
            .last()
            .is_some_and(|last| suffixes.contains(&last.as_str()))
    {
        parts.pop();
    }
    parts.join("_")
}

/// Check that `name` makes a valid module and type name.
pub fn check(name: &str, what: &str) -> Result<(), String> {
    let snake = snake(name);
    if snake.is_empty() {
        return Err(format!("{what} name is empty"));
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(format!(
            "{what} name `{name}` may only use letters, digits, `_` and `-`"
        ));
    }
    if snake.starts_with(|character: char| character.is_ascii_digit()) {
        return Err(format!("{what} name `{name}` cannot start with a digit"));
    }
    if KEYWORDS.contains(&snake.as_str()) {
        return Err(format!("{what} name `{name}` is a Rust keyword"));
    }
    Ok(())
}

/// Whether `name` is a valid identifier (for fields and methods).
pub fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(|character: char| character.is_ascii_digit())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        && !KEYWORDS.contains(&name)
}

/// The base verb of a past-tense event name's last word: `NoteAdded` -> `add`,
/// `NotesRequested` -> `request`.
pub fn event_verb(event: &str) -> String {
    let last = words(event).pop().unwrap_or_default();
    let irregular = [
        ("added", "add"),
        ("created", "create"),
        ("deleted", "delete"),
        ("removed", "remove"),
        ("updated", "update"),
        ("saved", "save"),
        ("renamed", "rename"),
        ("loaded", "load"),
        ("fetched", "fetch"),
        ("requested", "request"),
        ("refreshed", "refresh"),
        ("submitted", "submit"),
        ("changed", "change"),
        ("toggled", "toggle"),
        ("cleared", "clear"),
        ("selected", "select"),
        ("reset", "reset"),
        ("set", "set"),
    ];
    if let Some((_, verb)) = irregular.iter().find(|(past, _)| *past == last) {
        return (*verb).to_string();
    }
    last.strip_suffix("ed").map_or(last.clone(), str::to_string)
}

/// The words of an event name before its verb: `NoteAdded` -> `note`.
pub fn event_subject(event: &str) -> String {
    let mut parts = words(event);
    parts.pop();
    parts.join("_")
}

/// A simple English plural: `note` -> `notes`, `entry` -> `entries`.
pub fn plural(word: &str) -> String {
    if word.ends_with('s') {
        word.to_string()
    } else if let Some(stem) = word.strip_suffix('y') {
        format!("{stem}ies")
    } else {
        format!("{word}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_normalize() {
        for name in ["notes", "Notes", "notes_bloc", "NotesBloc", "notes-bloc"] {
            assert_eq!(base(name, &["bloc"]), "notes", "{name}");
        }
        assert_eq!(pascal("note_added"), "NoteAdded");
        assert_eq!(snake("HTTPServerError"), "http_server_error");
        assert_eq!(snake("NoteId"), "note_id");
        assert!(check("type", "feature").is_err());
        assert!(check("9lives", "feature").is_err());
        assert!(check("my notes", "feature").is_err());
        assert!(check("todo_list", "feature").is_ok());
    }

    #[test]
    fn event_verbs() {
        assert_eq!(event_verb("NoteAdded"), "add");
        assert_eq!(event_verb("NotesRequested"), "request");
        assert_eq!(event_verb("ItemArchived"), "archiv");
        assert_eq!(event_subject("NoteAdded"), "note");
        assert_eq!(plural("entry"), "entries");
    }
}
