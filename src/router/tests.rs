//! Route matching, redirects and locations.

use super::*;

fn resolve(router: &Router, path: &str) -> Option<(SharedString, Vec<(String, String)>)> {
    router.resolve(&Location::parse(path)).map(|(route, _)| {
        let mut params: Vec<(String, String)> = route
            .params
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        params.sort();
        (route.pattern.clone(), params)
    })
}

fn router() -> Router {
    Router::new()
        .route("/", |_, _, _| div())
        .route("/users/:id", |_, _, _| div())
        .route("/users/new", |_, _, _| div())
        .route("/files/*path", |_, _, _| div())
        .route("/users/:id/posts/:post", |_, _, _| div())
}

#[test]
fn the_most_specific_pattern_wins() {
    let router = router();
    assert_eq!(resolve(&router, "/").unwrap().0.as_ref(), "/");
    assert_eq!(
        resolve(&router, "/users/42").unwrap(),
        ("/users/:id".into(), vec![("id".into(), "42".into())])
    );
    // Declared after `:id`, but a static segment beats a parameter.
    assert_eq!(
        resolve(&router, "/users/new").unwrap().0.as_ref(),
        "/users/new"
    );
    assert_eq!(
        resolve(&router, "/users/7/posts/9").unwrap().1,
        [("id".into(), "7".into()), ("post".into(), "9".into())]
    );
    assert_eq!(
        resolve(&router, "/files/docs/2026/report.pdf").unwrap().1,
        [("path".into(), "docs/2026/report.pdf".into())]
    );
    assert!(resolve(&router, "/missing").is_none());
    assert!(resolve(&router, "/users").is_none());
}

#[test]
fn locations_parse_queries_and_redirects_fill_parameters() {
    let location = Location::parse("/search?q=hello+world&tag=%D8%B9%D8%B1%D8%A8%D9%8A&empty");
    assert_eq!(location.path(), "/search");
    assert_eq!(location.query("q"), Some("hello world"));
    assert_eq!(location.query("tag"), Some("عربي"));
    assert_eq!(location.query("empty"), Some(""));
    assert_eq!(location.query("missing"), None);

    let router = Router::new().redirect("/u/:id", "/users/:id");
    assert_eq!(
        router
            .redirect_for(&Location::parse("/u/5"))
            .as_ref()
            .map(std::convert::AsRef::as_ref),
        Some("/users/5")
    );
    assert_eq!(normalize("users/".into()).as_ref(), "/users");

    // Each parameter fills its own segment, encoded again.
    let router = Router::new().redirect("/old/:id/:identity", "/new/:identity/:id");
    assert_eq!(
        router
            .redirect_for(&Location::parse("/old/1/a%20b"))
            .as_ref()
            .map(std::convert::AsRef::as_ref),
        Some("/new/a%20b/1")
    );
    let router = Router::new().redirect("/docs/*rest", "/manual/*rest?from=docs");
    assert_eq!(
        router
            .redirect_for(&Location::parse("/docs/a/b"))
            .as_ref()
            .map(std::convert::AsRef::as_ref),
        Some("/manual/a/b?from=docs")
    );
    assert_eq!(normalize("/".into()).as_ref(), "/");
}
