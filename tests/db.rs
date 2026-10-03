//! rok-db integration against a real `PostgreSQL` server. Skipped unless
//! `ROK_UI_TEST_DATABASE_URL` is set, for example:
//!
//! ```sh
//! docker run -d --rm -e POSTGRES_PASSWORD=rok -p 55432:5432 postgres:17-alpine
//! ROK_UI_TEST_DATABASE_URL=postgres://postgres:rok@127.0.0.1:55432/postgres \
//!     cargo test --features db --test db
//! ```

#![cfg(feature = "db")]

use rok_ui::db::{self, DbError};

#[gpui::test]
fn the_app_connects_and_runs_database_work(cx: &mut gpui::TestAppContext) {
    let Ok(url) = std::env::var("ROK_UI_TEST_DATABASE_URL") else {
        eprintln!("ROK_UI_TEST_DATABASE_URL is not set; skipping");
        return;
    };
    // The database answers on real threads, outside the test scheduler.
    cx.executor().allow_parking();

    let before = cx.update(|cx| db::run(cx, |db| async move { db.ping().await }));
    assert_eq!(cx.executor().block_test(before), Err(DbError::NotConnected));

    let connecting = cx.update(|cx| db::connect(url, cx));
    cx.executor()
        .block_test(connecting)
        .expect("connects to the test database");
    assert!(cx.update(|cx| db::connection(cx).is_some()));

    let ping = cx.update(|cx| db::run(cx, |db| async move { db.ping().await }));
    assert_eq!(cx.executor().block_test(ping), Ok(()));

    let broken = cx.update(|cx| {
        db::run(cx, |db| async move {
            db.execute("SELECT * FROM no_such_table").await
        })
    });
    match cx.executor().block_test(broken) {
        Err(DbError::Failed(message)) => assert!(message.contains("no_such_table"), "{message}"),
        other => panic!("expected a database error, got {other:?}"),
    }
}
