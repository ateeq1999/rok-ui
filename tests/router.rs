//! Router v2: typed routes, typed search params, guards and blockers.
//!
//! A test crate's root looks for modules beside it, so each one names its file.

use rok_ui::{router::Search, typed_route};

#[path = "router/loading.rs"]
mod loading;
#[path = "router/navigation.rs"]
mod navigation;
#[path = "router/scroll.rs"]
mod scroll;
#[path = "router/typed.rs"]
mod typed;

typed_route! {
    /// The note list.
    pub struct NotesRoute = "/notes";
}

typed_route! {
    /// One note.
    pub struct NoteRoute = "/notes/:id" {
        /// The note id.
        pub id: u64
    }
}

typed_route! {
    /// A file under a folder.
    pub struct FileRoute = "/folders/:folder/*path" {
        /// The folder name.
        pub folder: String,
        /// The path inside the folder.
        pub path: String,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Sort {
    #[default]
    Newest,
    Oldest,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Sort::Newest => "newest",
            Sort::Oldest => "oldest",
        })
    }
}

impl std::str::FromStr for Sort {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, ()> {
        match text {
            "newest" => Ok(Sort::Newest),
            "oldest" => Ok(Sort::Oldest),
            _ => Err(()),
        }
    }
}

#[derive(Search, Clone, Debug, PartialEq)]
struct NotesSearch {
    #[search(default = 1)]
    page: u32,
    q: Option<String>,
    sort: Sort,
    #[search(rename = "tab")]
    section: Option<String>,
}
