//! Typed routes and typed search params.

use gpui::TestAppContext;
use rok_ui::router::{self, Location, Route, Search};

use super::{FileRoute, NoteRoute, NotesRoute, NotesSearch, Sort};

#[test]
fn typed_routes_build_and_parse_paths() {
    assert_eq!(NotesRoute.href(), "/notes");
    assert_eq!(NoteRoute { id: 7 }.to_string(), "/notes/7");
    assert_eq!(
        NoteRoute::parse("/notes/7?tab=history"),
        Some(NoteRoute { id: 7 })
    );
    assert_eq!(NoteRoute::parse("/notes/seven"), None);
    assert_eq!(NoteRoute::parse("/notes"), None);

    let file = FileRoute {
        folder: "My Docs".into(),
        path: "2026/report ä.pdf".into(),
    };
    assert_eq!(file.href(), "/folders/My%20Docs/2026/report%20%C3%A4.pdf");
    assert_eq!(FileRoute::parse(&file.href()), Some(file));
}

#[test]
fn search_params_fall_back_to_defaults_and_omit_them() {
    let search = NotesSearch::from_location(&Location::parse("/notes?page=x&sort=oldest&tab=a"));
    assert_eq!(
        search,
        NotesSearch {
            page: 1,
            q: None,
            sort: Sort::Oldest,
            section: Some("a".into())
        }
    );
    assert_eq!(
        search.to_query(),
        vec![
            ("sort".to_string(), "oldest".to_string()),
            ("tab".into(), "a".into())
        ]
    );
    assert_eq!(
        NotesSearch::defaults().to_query(),
        Vec::<(String, String)>::new()
    );
    assert_eq!(NotesSearch::FIELDS, &["page", "q", "sort", "tab"]);
}

#[gpui::test]
fn updating_search_keeps_other_parameters(cx: &mut TestAppContext) {
    cx.update(|cx| {
        rok_ui::init(cx);
        router::navigate("/notes?utm=mail&page=2", cx);
        router::update_search::<NotesSearch>(cx, |search| {
            search.page += 1;
            search.q = Some("milk & honey".into());
        });
        let location = router::location(cx);
        assert_eq!(location.query("utm"), Some("mail"));
        assert_eq!(location.query("q"), Some("milk & honey"));
        let search: NotesSearch = router::use_search(cx);
        assert_eq!(search.page, 3);
        router::back(cx);
        assert_eq!(router::use_search::<NotesSearch>(cx).page, 2);

        assert_eq!(router::use_params::<NoteRoute>(cx), None);
        router::navigate_to(&NoteRoute { id: 9 }, cx);
        assert_eq!(
            router::use_params::<NoteRoute>(cx),
            Some(NoteRoute { id: 9 })
        );
    });
}
