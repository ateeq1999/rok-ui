//! The route tree has a typed route per page file.

use rok_ui::router::Route;
use {{crate_name}}::routes;

#[test]
fn route_files_become_typed_routes() {
    assert_eq!(routes::Index.href(), "/");
    assert_eq!(routes::NotesId { id: 2 }.href(), "/notes/2");
}
